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
    /// - InvalidSchedule: if the provided schedule is empty, contains a zero timestamp or amount, or the total amount overflows 256 bits.
    /// - TokenDepositTransferFailed: if there is an error transferring the total vesting amount from the caller to the contract
    pub fn create(
        &mut self,
        token: Address,
        owner: Address,
        source: Address,
        destination: Address,
        schedule: Vec<(U64, U256)>,
    ) -> Result<U256, ContractError> {
        // Step 1: validate inputs
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

        // Step 2/3: calculate total vested amount & write schedule state
        let schedule_id = self.schedule_count.get() + U256::ONE;

        // Only iterate through schedule unlocks once
        let mut schedule_store = self.schedule.setter(schedule_id);
        let mut total_vested_amount = U256::ZERO;
        for (timestamp, amount) in schedule {
            if amount.is_zero() || timestamp.is_zero() {
                return Err(InvalidSchedule {}.into());
            }

            total_vested_amount = total_vested_amount
                .checked_add(amount)
                .ok_or(InvalidSchedule {})?;

            let mut schedule_item = schedule_store.grow();

            schedule_item.timestamp.set(timestamp);
            schedule_item.amount.set(amount);
        }

        self.schedule_count.set(schedule_id);
        self.token.insert(schedule_id, token);
        self.owner.insert(schedule_id, owner);
        self.destination.insert(schedule_id, destination);

        // Step 4: Transfer the total vesting amount to the contract
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
        // Step 1: Check that the schedule exits
        let token = self.token.get(schedule_id);

        if token.is_zero() {
            return Err(ScheduleNotFound {}.into());
        }

        // Step 2: Determine unlocked token amount & zero newly unlocked amounts
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

        // Step 3: Check that unlocks are available
        if unlocked_token_amount.is_zero() {
            return Err(NoUnlocksAvailable {}.into());
        }

        // Step 4: Transfer the unlocked amount to the current destination account
        let destination = self.destination.get(schedule_id);
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
        destination: Address,
    ) -> Result<(), ContractError> {
        // Step 1: Check that the proposed destination is valid
        if destination == Address::ZERO {
            return Err(InvalidDestination {}.into());
        }

        // Step 2: Check that the schedule exists
        if self.token.get(schedule_id).is_zero() {
            return Err(ScheduleNotFound {}.into());
        }

        // Step 3: Check that the caller is the current owner
        if self.vm().msg_sender() != self.owner.get(schedule_id) {
            return Err(Unauthorized {}.into());
        }

        // Step 4: Overwrite the stored destination
        self.destination.insert(schedule_id, destination);

        Ok(())
    }

    /// Change the `owner` associated with the `schedule_id`, this can only be called by the current `owner`.
    ///
    /// Note: setting a zero address for `owner` means the `destination` is now immutable.
    ///
    /// # Errors
    /// - ScheduleNotFound: if the provided `schedule_id` is not associated with a schedule
    /// - Unauthorized: if the caller is not the owner of the schedule
    pub fn change_owner(&mut self, schedule_id: U256, owner: Address) -> Result<(), ContractError> {
        // Step 1: Check that the schedule exists
        if self.token.get(schedule_id).is_zero() {
            return Err(ScheduleNotFound {}.into());
        }

        // Step 2: Check that the caller is the current owner
        if self.vm().msg_sender() != self.owner.get(schedule_id) {
            return Err(Unauthorized {}.into());
        }

        // Step 3: Overwrite the stored owner
        self.owner.insert(schedule_id, owner);

        Ok(())
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
