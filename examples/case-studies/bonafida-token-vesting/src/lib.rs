#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use openzeppelin_stylus::token::erc20::interface::Erc20Interface;
use stylus_sdk::{alloy_primitives::*, alloy_sol_types::sol, prelude::*, storage::*};

sol! {
    #[derive(Debug)]
    error InvalidToken();
    #[derive(Debug)]
    error InvalidDestination();
    #[derive(Debug)]
    error InvalidSchedule();
    #[derive(Debug)]
    error TokenDepositTransferFailed();
    #[derive(Debug)]
    error ScheduleNotFound();
    #[derive(Debug)]
    error NoUnlocksAvailable();
    #[derive(Debug)]
    error Unauthorized();

    event ScheduleCreated(
        uint256 schedule_id,
        address token,
        address owner,
        address destination,
        uint64[] timestamps,
        uint256[] amounts
    );

    event TokensUnlocked(
        uint256 schedule_id,
        address destination,
        uint256 unlocked_token_amount,
    );

    event DestinationChanged(
        uint256 schedule_id,
        address old_destination,
        address new_destination,
    );

    event OwnerChanged(
        uint256 schedule_id,
        address old_owner,
        address new_owner,
    );
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    InvalidToken(InvalidToken),
    InvalidDestination(InvalidDestination),
    InvalidSchedule(InvalidSchedule),
    TokenDepositFailed(TokenDepositTransferFailed),
    ScheduleNotFound(ScheduleNotFound),
    NoUnlocksAvailable(NoUnlocksAvailable),
    Unauthorized(Unauthorized),
}

#[storage]
pub struct Schedule {
    /// Timestamp after which tokens are unlocked
    timestamp: StorageU64,
    /// Amount of tokens unlocked (set to zero afterwards)
    amount: StorageU256,
}

#[storage]
#[entrypoint]
pub struct TokenVestingContract {
    /// Incremented to determine the schedule identifier
    schedule_count: StorageU256,
    /// Token vested by the schedule
    token: StorageMap<U256, StorageAddress>,
    /// Owner and benefactor of the schedule
    owner: StorageMap<U256, StorageAddress>,
    /// Destination address for unlocked tokens
    destination: StorageMap<U256, StorageAddress>,
    /// Scheduled token unlocks
    schedule: StorageMap<U256, StorageVec<Schedule>>,
}

#[public]
impl TokenVestingContract {
    /// Create a vesting schedule for the specified `token` and initial `destination`, returning the schedule identifier.
    /// Attempts to transfer the total amount of tokens scheduled from the sender to this contract.
    ///
    /// Note: setting a zero address for `owner` means the `destination` is immutable.
    ///
    /// # Errors
    /// - InvalidToken: if the provided token address is zero
    /// - InvalidDestination: if the provided destination address is zero
    /// - InvalidSchedule: if the provided schedule is empty, contains a zero amount, is not ordered chronologically or the total amount overflows 256 bits.
    /// - TokenDepositTransferFailed: if there is an error transferring the total vesting amount from the sender to the contract
    pub fn create(
        &mut self,
        token: Address,
        owner: Address,
        destination: Address,
        schedule: Vec<(u64, U256)>,
    ) -> Result<U256, ContractError> {
        if token == Address::ZERO {
            return Err(InvalidToken {}.into());
        }

        if destination == Address::ZERO {
            return Err(InvalidDestination {}.into());
        }

        if schedule.is_empty() {
            return Err(InvalidSchedule {}.into());
        }

        let schedule_id = self.schedule_count.get() + U256::ONE;

        let mut schedule_store = self.schedule.setter(schedule_id);
        let mut total_vested_amount = U256::ZERO;
        let mut last_timestamp = 0u64;
        let mut timestamps = Vec::with_capacity(schedule.len());
        let mut amounts = Vec::with_capacity(schedule.len());
        for (timestamp, amount) in schedule {
            if amount.is_zero() || timestamp < last_timestamp {
                return Err(InvalidSchedule {}.into());
            }

            last_timestamp = timestamp;
            total_vested_amount = total_vested_amount
                .checked_add(amount)
                .ok_or(InvalidSchedule {})?;

            timestamps.push(timestamp);
            amounts.push(amount);

            let mut schedule_item = schedule_store.grow();
            schedule_item.timestamp.set(U64::from(timestamp));
            schedule_item.amount.set(amount);
        }

        self.schedule_count.set(schedule_id);
        self.token.insert(schedule_id, token);
        self.owner.insert(schedule_id, owner);
        self.destination.insert(schedule_id, destination);

        log(
            self.vm(),
            ScheduleCreated {
                schedule_id,
                token,
                owner,
                destination,
                timestamps,
                amounts,
            },
        );

        let contract_addr = self.vm().contract_address();
        let sender = self.vm().msg_sender();
        Erc20Interface::new(token)
            .transfer_from(self, sender, contract_addr, total_vested_amount)
            .map_err(|_| TokenDepositTransferFailed {})?;

        Ok(schedule_id)
    }

    /// Unlock any vested tokens associated with the `schedule_id` and transfers them to the set `destination`
    ///
    /// # Errors
    /// - ScheduleNotFound: if the provided `schedule_id` is not associated with a schedule
    /// - NoUnlocksAvailable: if there a zero unlocked tokens to transfer
    pub fn unlock(&mut self, schedule_id: U256) -> Result<(), ContractError> {
        let token = self.token.get(schedule_id);

        if token.is_zero() {
            return Err(ScheduleNotFound {}.into());
        }

        let now = U64::from(self.vm().block_timestamp());

        let mut schedule = self.schedule.setter(schedule_id);
        let mut idx = 0;
        let mut unlocked_token_amount = U256::ZERO;
        loop {
            let Some(mut schedule_item) = schedule.setter(idx) else {
                break;
            };

            idx += 1;

            if schedule_item.timestamp.get() > now {
                break;
            }

            let amount = schedule_item.amount.get();

            if amount.is_zero() {
                continue;
            }

            schedule_item.amount.set(U256::ZERO);

            // Overflow not possible because: escrow total <= U256::MAX checked during creation
            unlocked_token_amount += amount;
        }

        if unlocked_token_amount.is_zero() {
            return Err(NoUnlocksAvailable {}.into());
        }

        let destination = self.destination.get(schedule_id);

        log(
            self.vm(),
            TokensUnlocked {
                schedule_id,
                destination,
                unlocked_token_amount,
            },
        );

        Erc20Interface::new(token)
            .transfer(self, destination, unlocked_token_amount)
            .expect("Invariant: the contract always has sufficient balance to satisfy unlocks");

        Ok(())
    }

    /// Change the `destination` associated with the `schedule_id`, this can only be called by the associated `owner`.
    ///
    /// # Errors
    /// - ScheduleNotFound: if the provided `schedule_id` is not associated with a schedule
    /// - InvalidDestination: if the provided destination address is zero
    /// - Unauthorized: if the caller is not the owner of the schedule
    pub fn change_destination(
        &mut self,
        schedule_id: U256,
        new_destination: Address,
    ) -> Result<(), ContractError> {
        if new_destination == Address::ZERO {
            return Err(InvalidDestination {}.into());
        }

        if self.token.get(schedule_id).is_zero() {
            return Err(ScheduleNotFound {}.into());
        }

        if self.vm().msg_sender() != self.owner.get(schedule_id) {
            return Err(Unauthorized {}.into());
        }

        let old_destination = self.destination.replace(schedule_id, new_destination);

        log(
            self.vm(),
            DestinationChanged {
                schedule_id,
                old_destination,
                new_destination,
            },
        );

        Ok(())
    }

    /// Change the `owner` associated with the `schedule_id`, this can only be called by the current `owner`.
    ///
    /// Note: setting a zero address for `owner` means the `destination` is now immutable.
    ///
    /// # Errors
    /// - ScheduleNotFound: if the provided `schedule_id` is not associated with a schedule
    /// - Unauthorized: if the caller is not the owner of the schedule
    pub fn change_owner(
        &mut self,
        schedule_id: U256,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        if self.token.get(schedule_id).is_zero() {
            return Err(ScheduleNotFound {}.into());
        }

        if self.vm().msg_sender() != self.owner.get(schedule_id) {
            return Err(Unauthorized {}.into());
        }

        let old_owner = self.owner.replace(schedule_id, new_owner);

        log(
            self.vm(),
            OwnerChanged {
                schedule_id,
                old_owner,
                new_owner,
            },
        );

        Ok(())
    }

    // View functions
    fn schedule_count(&self) -> U256 {
        self.schedule_count.get()
    }

    fn token(&self, schedule_id: U256) -> Address {
        self.token.get(schedule_id)
    }

    fn destination(&self, schedule_id: U256) -> Address {
        self.destination.get(schedule_id)
    }

    fn owner(&self, schedule_id: U256) -> Address {
        self.owner.get(schedule_id)
    }

    fn schedule(&self, schedule_id: U256) -> Vec<(U64, U256)> {
        if self.token(schedule_id).is_zero() {
            return vec![];
        }

        let schedule_store = self.schedule.getter(schedule_id);

        let mut schedule = vec![];
        let mut idx = 0;
        while let Some(schedule_item) = schedule_store.getter(idx) {
            schedule.push((schedule_item.timestamp.get(), schedule_item.amount.get()));
            idx += 1;
        }

        schedule
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloy_primitives::{Address, U256, U64};
    use motsu::prelude::*;
    use openzeppelin_stylus::token::erc20::{Erc20, IErc20};

    pub const TOTAL_SUPPLY: u64 = 1_000_000;

    fn setup_env(token: &Contract<Erc20>, source: Address) {
        // Environment always starts at timestamp 1 for simplicity
        VM::context().set_block_timestamp(1);

        // Mint total supply of tokens to source account
        token
            .sender(source)
            ._mint(source, U256::from(TOTAL_SUPPLY))
            .motsu_unwrap();
    }

    #[motsu::test]
    fn test_create_vesting_schedule(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner: Address,
        source: Address,
        destination: Address,
    ) {
        setup_env(&token, source);

        // Approve vesting contract to transfer tokens
        let vesting_amount = U256::from(60u64);
        token
            .sender(source)
            .approve(vesting.address(), vesting_amount)
            .motsu_unwrap();

        // Create vesting schedule with 3 unlocks
        let schedule = vec![
            (0u64, U256::from(20u64)),   // Immediate unlock
            (100u64, U256::from(20u64)), // After timestamp 100
            (200u64, U256::from(20u64)), // After timestamp 200
        ];

        let schedule_id = vesting
            .sender(source)
            .create(token.address(), owner, destination, schedule.clone())
            .motsu_unwrap();

        // Verify schedule was created
        assert_eq!(schedule_id, U256::from(1u64));
        assert_eq!(vesting.sender(source).schedule_count(), U256::from(1u64));
        assert_eq!(vesting.sender(source).token(schedule_id), token.address());
        assert_eq!(vesting.sender(source).owner(schedule_id), owner);
        assert_eq!(vesting.sender(source).destination(schedule_id), destination);

        // Verify schedule details
        let stored_schedule = vesting.sender(source).schedule(schedule_id);
        assert_eq!(stored_schedule.len(), 3);
        assert_eq!(stored_schedule[0], (U64::from(0u64), U256::from(20u64)));
        assert_eq!(stored_schedule[1], (U64::from(100u64), U256::from(20u64)));
        assert_eq!(stored_schedule[2], (U64::from(200u64), U256::from(20u64)));

        // Verify tokens were transferred to vesting contract
        assert_eq!(
            token.sender(source).balance_of(vesting.address()),
            vesting_amount
        );
        assert_eq!(
            token.sender(source).balance_of(source),
            U256::from(TOTAL_SUPPLY) - vesting_amount
        );
    }

    #[motsu::test]
    fn test_unlock_tokens(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner: Address,
        source: Address,
        destination: Address,
    ) {
        setup_env(&token, source);

        let vesting_amount = U256::from(60u64);
        token
            .sender(source)
            .approve(vesting.address(), vesting_amount)
            .motsu_unwrap();

        let schedule = vec![
            (0u64, U256::from(20u64)),
            (100u64, U256::from(20u64)),
            (200u64, U256::from(20u64)),
        ];

        let schedule_id = vesting
            .sender(source)
            .create(token.address(), owner, destination, schedule)
            .motsu_unwrap();

        // Test 1: Unlock at timestamp 1 (immediate unlock for first tranche)
        vesting.sender(source).unlock(schedule_id).motsu_unwrap();

        assert_eq!(
            token.sender(source).balance_of(destination),
            U256::from(20u64)
        );
        assert_eq!(
            token.sender(source).balance_of(vesting.address()),
            U256::from(40u64)
        );

        // Verify first unlock is now zero in schedule
        let stored_schedule = vesting.sender(source).schedule(schedule_id);
        assert_eq!(stored_schedule[0].1, U256::ZERO);

        // Test 2: Try to unlock again at same timestamp (should fail - no unlocks available)
        let err = vesting
            .sender(source)
            .unlock(schedule_id)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::NoUnlocksAvailable(_)));

        // Test 3: Unlock at timestamp 150 (should unlock second tranche)
        VM::context().set_block_timestamp(150);
        vesting.sender(source).unlock(schedule_id).motsu_unwrap();

        assert_eq!(
            token.sender(source).balance_of(destination),
            U256::from(40u64)
        );
        assert_eq!(
            token.sender(source).balance_of(vesting.address()),
            U256::from(20u64)
        );

        // Test 4: Unlock at timestamp 250 (should unlock final tranche)
        VM::context().set_block_timestamp(250);
        vesting.sender(source).unlock(schedule_id).motsu_unwrap();

        assert_eq!(
            token.sender(source).balance_of(destination),
            U256::from(60u64)
        );
        assert_eq!(
            token.sender(source).balance_of(vesting.address()),
            U256::ZERO
        );

        // All tokens should be unlocked now
        let final_schedule = vesting.sender(source).schedule(schedule_id);
        assert!(final_schedule.iter().all(|(_, amount)| amount.is_zero()));
    }

    #[motsu::test]
    fn test_unlock_multiple_at_once(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner: Address,
        source: Address,
        destination: Address,
    ) {
        setup_env(&token, source);

        let vesting_amount = U256::from(60u64);
        token
            .sender(source)
            .approve(vesting.address(), vesting_amount)
            .motsu_unwrap();

        let schedule = vec![
            (50u64, U256::from(20u64)),
            (100u64, U256::from(20u64)),
            (150u64, U256::from(20u64)),
        ];

        let schedule_id = vesting
            .sender(source)
            .create(token.address(), owner, destination, schedule)
            .motsu_unwrap();

        // Jump to timestamp 120 - should unlock first two tranches at once
        VM::context().set_block_timestamp(120);
        vesting.sender(source).unlock(schedule_id).motsu_unwrap();

        assert_eq!(
            token.sender(source).balance_of(destination),
            U256::from(40u64)
        );
        assert_eq!(
            token.sender(source).balance_of(vesting.address()),
            U256::from(20u64)
        );
    }

    #[motsu::test]
    fn test_change_destination(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner: Address,
        source: Address,
        destination: Address,
        new_destination: Address,
    ) {
        setup_env(&token, source);

        let vesting_amount = U256::from(40u64);
        token
            .sender(source)
            .approve(vesting.address(), vesting_amount)
            .motsu_unwrap();

        let schedule = vec![(100u64, U256::from(20u64)), (200u64, U256::from(20u64))];

        let schedule_id = vesting
            .sender(source)
            .create(token.address(), owner, destination, schedule)
            .motsu_unwrap();

        // Test 1: Unauthorized change (not owner)
        let err = vesting
            .sender(source)
            .change_destination(schedule_id, new_destination)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        // Test 2: Authorized change by owner
        vesting
            .sender(owner)
            .change_destination(schedule_id, new_destination)
            .motsu_unwrap();

        assert_eq!(
            vesting.sender(owner).destination(schedule_id),
            new_destination
        );

        // Test 3: Unlock tokens to new destination
        VM::context().set_block_timestamp(150);
        vesting.sender(owner).unlock(schedule_id).motsu_unwrap();

        assert_eq!(
            token.sender(source).balance_of(new_destination),
            U256::from(20u64)
        );
        assert_eq!(token.sender(source).balance_of(destination), U256::ZERO);
    }

    #[motsu::test]
    fn test_change_owner(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner: Address,
        new_owner: Address,
        source: Address,
        destination: Address,
    ) {
        setup_env(&token, source);

        token
            .sender(source)
            .approve(vesting.address(), U256::from(20u64))
            .motsu_unwrap();

        let schedule = vec![(100u64, U256::from(20u64))];

        let schedule_id = vesting
            .sender(source)
            .create(token.address(), owner, destination, schedule)
            .motsu_unwrap();

        // Test 1: Unauthorized change
        let err = vesting
            .sender(source)
            .change_owner(schedule_id, new_owner)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        // Test 2: Authorized change by current owner
        vesting
            .sender(owner)
            .change_owner(schedule_id, new_owner)
            .motsu_unwrap();

        assert_eq!(vesting.sender(new_owner).owner(schedule_id), new_owner);

        // Test 3: New owner can now change destination
        let another_destination = Address::from([5u8; 20]);
        vesting
            .sender(new_owner)
            .change_destination(schedule_id, another_destination)
            .motsu_unwrap();

        assert_eq!(
            vesting.sender(new_owner).destination(schedule_id),
            another_destination
        );
    }

    #[motsu::test]
    fn test_create_validation_errors(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner: Address,
        source: Address,
        destination: Address,
    ) {
        setup_env(&token, source);

        // Test 1: Invalid token (zero address)
        let err = vesting
            .sender(source)
            .create(
                Address::ZERO,
                owner,
                destination,
                vec![(100u64, U256::from(20u64))],
            )
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidToken(_)));

        // Test 3: Invalid destination (zero address)
        let err = vesting
            .sender(source)
            .create(
                token.address(),
                owner,
                Address::ZERO,
                vec![(100u64, U256::from(20u64))],
            )
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidDestination(_)));

        // Test 4: Empty schedule
        let err = vesting
            .sender(source)
            .create(token.address(), owner, destination, vec![])
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidSchedule(_)));

        // Test 5: Zero amount in schedule
        let err = vesting
            .sender(source)
            .create(
                token.address(),
                owner,
                destination,
                vec![(100u64, U256::ZERO)],
            )
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidSchedule(_)));

        // Test 6: Non-chronological schedule
        let err = vesting
            .sender(source)
            .create(
                token.address(),
                owner,
                destination,
                vec![
                    (200u64, U256::from(10u64)),
                    (100u64, U256::from(10u64)), // Earlier timestamp after later one
                ],
            )
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidSchedule(_)));

        // Test 7: Insufficient allowance
        token
            .sender(source)
            .approve(vesting.address(), U256::from(10u64))
            .motsu_unwrap();

        let err = vesting
            .sender(source)
            .create(
                token.address(),
                owner,
                destination,
                vec![(100u64, U256::from(20u64))], // Needs 20 but only approved 10
            )
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::TokenDepositFailed(_)));
    }

    #[motsu::test]
    fn test_multiple_schedules(
        token: Contract<Erc20>,
        vesting: Contract<TokenVestingContract>,
        owner1: Address,
        owner2: Address,
        source: Address,
        destination1: Address,
        destination2: Address,
    ) {
        setup_env(&token, source);

        // Create first schedule
        token
            .sender(source)
            .approve(vesting.address(), U256::from(30u64))
            .motsu_unwrap();

        let schedule_id1 = vesting
            .sender(source)
            .create(
                token.address(),
                owner1,
                destination1,
                vec![(100u64, U256::from(30u64))],
            )
            .motsu_unwrap();

        // Create second schedule
        token
            .sender(source)
            .approve(vesting.address(), U256::from(50u64))
            .motsu_unwrap();

        let schedule_id2 = vesting
            .sender(source)
            .create(
                token.address(),
                owner2,
                destination2,
                vec![(200u64, U256::from(50u64))],
            )
            .motsu_unwrap();

        // Verify separate schedule IDs
        assert_eq!(schedule_id1, U256::from(1u64));
        assert_eq!(schedule_id2, U256::from(2u64));
        assert_eq!(vesting.sender(source).schedule_count(), U256::from(2u64));

        // Verify schedules are independent
        assert_eq!(vesting.sender(source).owner(schedule_id1), owner1);
        assert_eq!(vesting.sender(source).owner(schedule_id2), owner2);
        assert_eq!(
            vesting.sender(source).destination(schedule_id1),
            destination1
        );
        assert_eq!(
            vesting.sender(source).destination(schedule_id2),
            destination2
        );

        // Unlock first schedule
        VM::context().set_block_timestamp(150);
        vesting.sender(source).unlock(schedule_id1).motsu_unwrap();
        assert_eq!(
            token.sender(source).balance_of(destination1),
            U256::from(30u64)
        );
        assert_eq!(token.sender(source).balance_of(destination2), U256::ZERO);

        // Unlock second schedule
        VM::context().set_block_timestamp(200);
        vesting.sender(source).unlock(schedule_id2).motsu_unwrap();
        assert_eq!(
            token.sender(source).balance_of(destination1),
            U256::from(30u64)
        );
        assert_eq!(
            token.sender(source).balance_of(destination2),
            U256::from(50u64)
        );
    }

    #[motsu::test]
    fn test_nonexistent_schedule_operations(
        vesting: Contract<TokenVestingContract>,
        caller: Address,
        new_destination: Address,
        new_owner: Address,
    ) {
        let nonexistent_id = U256::from(999u64);

        // Test unlock on nonexistent schedule
        let err = vesting
            .sender(caller)
            .unlock(nonexistent_id)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::ScheduleNotFound(_)));

        // Test change_destination on nonexistent schedule
        let err = vesting
            .sender(caller)
            .change_destination(nonexistent_id, new_destination)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::ScheduleNotFound(_)));

        // Test change_owner on nonexistent schedule
        let err = vesting
            .sender(caller)
            .change_owner(nonexistent_id, new_owner)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::ScheduleNotFound(_)));

        // Test view functions return sensible defaults
        assert_eq!(vesting.sender(caller).token(nonexistent_id), Address::ZERO);
        assert_eq!(vesting.sender(caller).owner(nonexistent_id), Address::ZERO);
        assert_eq!(
            vesting.sender(caller).destination(nonexistent_id),
            Address::ZERO
        );
        assert_eq!(vesting.sender(caller).schedule(nonexistent_id), vec![]);
    }
}
