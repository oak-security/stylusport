#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use openzeppelin_stylus::token::erc20::interface::Erc20Interface;
use stylus_sdk::{alloy_primitives::*, alloy_sol_types::sol, prelude::*, storage::*};

sol! {
    #[derive(Debug)]
    error InvalidToken();
    #[derive(Debug)]
    error InvalidSource();
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
        address source,
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
    InvalidSource(InvalidSource),
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
    /// Attempts to transfer the total amount of tokens scheduled from `source` to this contract.
    ///
    /// Note: setting a zero address for `owner` means the `destination` is immutable.
    ///
    /// # Errors
    /// - InvalidToken: if the provided token address is zero
    /// - InvalidSource: if the provided source address is zero
    /// - InvalidDestination: if the provided destination address is zero
    /// - InvalidSchedule: if the provided schedule is empty, contains a zero amount, is not ordered chronologically or the total amount overflows 256 bits.
    /// - TokenDepositTransferFailed: if there is an error transferring the total vesting amount from the caller to the contract
    pub fn create(
        &mut self,
        token: Address,
        owner: Address,
        source: Address,
        destination: Address,
        schedule: Vec<(u64, U256)>,
    ) -> Result<U256, ContractError> {
        if token == Address::ZERO {
            return Err(InvalidToken {}.into());
        }

        if source == Address::ZERO {
            return Err(InvalidSource {}.into());
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
                source,
                destination,
                timestamps,
                amounts,
            },
        );

        let contract_addr = self.vm().contract_address();
        Erc20Interface::new(token)
            .transfer_from(self, source, contract_addr, total_vested_amount)
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

    use motsu::prelude::*;

    #[motsu::test]
    fn test_contract(
        contract: Contract<TokenVestingContract>,
        owner: Address,
        destination: Address,
    ) {
    }
}
