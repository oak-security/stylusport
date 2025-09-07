# Case Study - Migrating Bonafida's Token Vesting to Stylus

In this chapter, we will walk through the complete migration of an audited [token vesting program](https://github.com/Bonfida/token-vesting/tree/master/program), built by [Bonafida](https://www.bonfida.org/) with native Solana, to Arbitrum Stylus. This case study demonstrates the practical application of the concepts we have covered in following chapters:

- [Program Structure Migration](./program-structure.md)
- [State Storage Patterns](./state-storage.md)
- [Access Control Migration](./access-control.md)
- [Fungible Token Handling](./fungible-tokens.md)
- [Errors and Events](./errors-events.md)

The program allows any account to setup a token escrow where amounts of a the token will be released to a single destination account according to a pre-defined schedule.

Once the token escrow is established, it cannot be cancelled. Additionally, any account is able to trigger token unlocks. 

## Migration Strategy

We will migrate the program to Stylus phases:
1. **Program Structure**: Convert neccesary instructions to `#[public]` functions.
1. **State Storage**: Assess the data structures stored in accounts and the use of PDAs, then convert to idiomatic Stylus state management.
1. **Business Logic**: Once state and token operations are setup, port the platform-agnostic business logic from instruction handlers to the equivalent functions.
1. **View Functions**: Unlike Solana, view functions need to be added to allow users and clients to easily read the contract storage. 
1. **Events**: It is best practice to emit an event when the contract state changes. 
1. **Testing**: Ensure feature parity with automated testing.

## Phase 1: Program Structure

The token vesting program defines the following [instructions](https://github.com/Bonfida/token-vesting/blob/6234f98229196d1c785dfd2198bb58afc60bca10/program/src/instruction.rs#L61-L115):

```rust
pub enum VestingInstruction {
    /// Initializes an empty program account for the token_vesting program
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The system program account
    ///   1. `[]` The sysvar Rent account
    ///   1. `[signer]` The fee payer account
    ///   1. `[]` The vesting account
    Init {
        // The seed used to derive the vesting accounts address
        seeds: [u8; 32],
        // The number of release schedules for this contract to hold
        number_of_schedules: u32,
    },

    /// Creates a new vesting schedule contract
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The spl-token program account
    ///   1. `[writable]` The vesting account
    ///   2. `[writable]` The vesting spl-token account
    ///   3. `[signer]` The source spl-token account owner
    ///   4. `[writable]` The source spl-token account
    Create {
        seeds: [u8; 32],
        mint_address: Pubkey,
        destination_token_address: Pubkey,
        schedules: Vec<Schedule>,
    },

    /// Unlocks a simple vesting contract (SVC) - can only be invoked by the program itself
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The spl-token program account
    ///   1. `[]` The clock sysvar account
    ///   1. `[writable]` The vesting account
    ///   2. `[writable]` The vesting spl-token account
    ///   3. `[writable]` The destination spl-token account
    Unlock { seeds: [u8; 32] },

    /// Change the destination account of a given simple vesting contract (SVC)
    /// - can only be invoked by the present destination address of the contract.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The vesting account
    ///   1. `[]` The current destination token account
    ///   2. `[signer]` The destination spl-token account owner
    ///   3. `[]` The new destination spl-token account
    ChangeDestination { seeds: [u8; 32] },
}
```

We can see that there are three core functions that users of the program can perform:

- `Create`: sets up the token escrow and specifies the release schedule.
- `Unlock`: check the schedule and send any newly unlocked funds to the associated destination.
- `ChangeDestination`: the owner of the destination account can elect to change the destination. Note: this also potentially changes the owner.

The `Init` instruction is specific to Solana state management as Stylus contract manage their own state which can grow as required. The `seeds` parameter for each instruction is used to create a unique identifier for the vesting schedule in the form of a PDA assigned to the vesting schedule state account. 

The instructions can be converted to Stylus functions as follows:

```rust
#[derive(SolidityError, Debug)]
pub enum ContractError {
    // TODO: declare error variants
}

#[storage]
#[entrypoint]
pub struct TokenVestingContract {
    // TODO: declare storage schema
}

#[public]
impl TokenVestingContract {
    /// Create a vesting schedule for the specified `token` and initial `destination`, returning the schedule identifier.
    ///
    /// # Errors
    /// - TBD
    pub fn create(
        &mut self,
        token: Address,
        owner: Address,
        destination: Address,
        schedule: Vec<(U64, U256)>,
    ) -> Result<U256, ContractError> {
        todo!()
    }

    /// Unlock any vested tokens associated with the `schedule_id`.
    ///
    /// # Errors
    /// - TBD
    pub fn unlock(&mut self, schedule_id: U256) -> Result<(), ContractError> {
        todo!()
    }

    /// Change the `destination` associated with the `schedule_id`, this can only be called by the associated `owner`.
    ///
    /// # Errors
    /// - TBD
    pub fn change_destination(
        &mut self,
        schedule_id: U256,
        destination: Address,
    ) -> Result<(), ContractError> {
        todo!()
    }

    /// Change the `owner` associated with the `schedule_id`, this can only be called by the current `owner`.
    ///
    /// # Errors
    /// - TBD
    pub fn change_owner(&mut self, schedule_id: U256, owner: Address) -> Result<(), ContractError> {
        todo!()
    }
}
```

## Phase 2: State Storage

Aside from the escrowed token balance which is stored in the associated token account, the vesting schedule state is represented in the following form:

```rust
pub struct VestingSchedule {
    pub release_time: u64,
    pub amount: u64,
}

pub struct VestingScheduleHeader {
    pub destination_address: Pubkey,
    pub mint_address: Pubkey,
    pub is_initialized: bool,
}
```

The PDA derived from the `seeds` is associated with a data account arranged in the following [packed format](https://github.com/Bonfida/token-vesting/blob/6234f98229196d1c785dfd2198bb58afc60bca10/program/src/state.rs#L23-L110) where `N` is set in the `Init` instruction:

```
[Header: 65 bytes] [Schedule 0: 16 bytes] [Schedule 1: 16 bytes] ... [Schedule N: 16 bytes]

Header (65 bytes):
[0..32] destination_address | [32..64] mint_address | [64] is_initialized

Schedule (16 bytes each):
[0..8] release_time (u64 LE) | [8..16] amount (u64 LE)
```

When porting this state management to Stylus, it is idiomatic to store each element in a `StorageMap` using the schedule identifier as the key:

```rust
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
```

## Phase 3: Business Logic

### Create token vesting schedule

The [`Create` instruction handler](https://github.com/Bonfida/token-vesting/blob/6234f98229196d1c785dfd2198bb58afc60bca10/program/src/processor.rs#L72-L183) from the native solana program is as follows:

```rust
pub fn process_create(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    seeds: [u8; 32],
    mint_address: &Pubkey,
    destination_token_address: &Pubkey,
    schedules: Vec<Schedule>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let spl_token_account = next_account_info(accounts_iter)?;
    let vesting_account = next_account_info(accounts_iter)?;
    let vesting_token_account = next_account_info(accounts_iter)?;
    let source_token_account_owner = next_account_info(accounts_iter)?;
    let source_token_account = next_account_info(accounts_iter)?;

    let vesting_account_key = Pubkey::create_program_address(&[&seeds], program_id)?;
    if vesting_account_key != *vesting_account.key {
        msg!("Provided vesting account is invalid");
        return Err(ProgramError::InvalidArgument);
    }

    if !source_token_account_owner.is_signer {
        msg!("Source token account owner should be a signer.");
        return Err(ProgramError::InvalidArgument);
    }

    if *vesting_account.owner != *program_id {
        msg!("Program should own vesting account");
        return Err(ProgramError::InvalidArgument);
    }

    // Verifying that no SVC was already created with this seed
    let is_initialized =
        vesting_account.try_borrow_data()?[VestingScheduleHeader::LEN - 1] == 1;

    if is_initialized {
        msg!("Cannot overwrite an existing vesting contract.");
        return Err(ProgramError::InvalidArgument);
    }

    let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;

    if vesting_token_account_data.owner != vesting_account_key {
        msg!("The vesting token account should be owned by the vesting account.");
        return Err(ProgramError::InvalidArgument);
    }

    if vesting_token_account_data.delegate.is_some() {
        msg!("The vesting token account should not have a delegate authority");
        return Err(ProgramError::InvalidAccountData);
    }

    if vesting_token_account_data.close_authority.is_some() {
        msg!("The vesting token account should not have a close authority");
        return Err(ProgramError::InvalidAccountData);
    }

    let state_header = VestingScheduleHeader {
        destination_address: *destination_token_address,
        mint_address: *mint_address,
        is_initialized: true,
    };

    let mut data = vesting_account.data.borrow_mut();
    if data.len() != VestingScheduleHeader::LEN + schedules.len() * VestingSchedule::LEN {
        return Err(ProgramError::InvalidAccountData)
    }
    state_header.pack_into_slice(&mut data);

    let mut offset = VestingScheduleHeader::LEN;
    let mut total_amount: u64 = 0;

    for s in schedules.iter() {
        let state_schedule = VestingSchedule {
            release_time: s.release_time,
            amount: s.amount,
        };
        state_schedule.pack_into_slice(&mut data[offset..]);
        let delta = total_amount.checked_add(s.amount);
        match delta {
            Some(n) => total_amount = n,
            None => return Err(ProgramError::InvalidInstructionData), // Total amount overflows u64
        }
        offset += SCHEDULE_SIZE;
    }
    
    if Account::unpack(&source_token_account.data.borrow())?.amount < total_amount {
        msg!("The source token account has insufficient funds.");
        return Err(ProgramError::InsufficientFunds)
    };

    let transfer_tokens_to_vesting_account = transfer(
        spl_token_account.key,
        source_token_account.key,
        vesting_token_account.key,
        source_token_account_owner.key,
        &[],
        total_amount,
    )?;

    invoke(
        &transfer_tokens_to_vesting_account,
        &[
            source_token_account.clone(),
            vesting_token_account.clone(),
            spl_token_account.clone(),
            source_token_account_owner.clone(),
        ],
    )?;
    Ok(())
}
```

This can be boiled down the following steps:

1. Validate inputs such as token and destination accounts.
1. Compute total amount to be held in escrow, return an error if the schedule is empty.
1. Write schedule state to storage.
1. Transfer the computed total token amount to the escrow account, reverting on transfer failure.

```rust
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
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    InvalidToken(InvalidToken),
    InvalidSource(InvalidSource),
    InvalidDestination(InvalidDestination),
    InvalidSchedule(InvalidSchedule),
    TokenDepositFailed(TokenDepositTransferFailed),
}

#[public]
impl TokenVestingContract {
    /// Create a vesting schedule for the specified `token` and initial `destination`, returning the schedule identifier.
    /// Attempts to transfer the total amount of tokens scheduled from `source` to this contract.
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

    // ...
}
```

### Unlock tokens

The [`Unlock` instruction handler](https://github.com/Bonfida/token-vesting/blob/6234f98229196d1c785dfd2198bb58afc60bca10/program/src/processor.rs#L185-L268) is implemented as follows:

```rust
pub fn process_unlock(
    program_id: &Pubkey,
    _accounts: &[AccountInfo],
    seeds: [u8; 32],
) -> ProgramResult {
    let accounts_iter = &mut _accounts.iter();

    let spl_token_account = next_account_info(accounts_iter)?;
    let clock_sysvar_account = next_account_info(accounts_iter)?;
    let vesting_account = next_account_info(accounts_iter)?;
    let vesting_token_account = next_account_info(accounts_iter)?;
    let destination_token_account = next_account_info(accounts_iter)?;

    let vesting_account_key = Pubkey::create_program_address(&[&seeds], program_id)?;
    if vesting_account_key != *vesting_account.key {
        msg!("Invalid vesting account key");
        return Err(ProgramError::InvalidArgument);
    }

    if spl_token_account.key != &spl_token::id() {
        msg!("The provided spl token program account is invalid");
        return Err(ProgramError::InvalidArgument)
    }

    let packed_state = &vesting_account.data;
    let header_state =
        VestingScheduleHeader::unpack(&packed_state.borrow()[..VestingScheduleHeader::LEN])?;

    if header_state.destination_address != *destination_token_account.key {
        msg!("Contract destination account does not matched provided account");
        return Err(ProgramError::InvalidArgument);
    }

    let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;

    if vesting_token_account_data.owner != vesting_account_key {
        msg!("The vesting token account should be owned by the vesting account.");
        return Err(ProgramError::InvalidArgument);
    }

    // Unlock the schedules that have reached maturity
    let clock = Clock::from_account_info(&clock_sysvar_account)?;
    let mut total_amount_to_transfer = 0;
    let mut schedules = unpack_schedules(&packed_state.borrow()[VestingScheduleHeader::LEN..])?;

    for s in schedules.iter_mut() {
        if clock.unix_timestamp as u64 >= s.release_time {
            total_amount_to_transfer += s.amount;
            s.amount = 0;
        }
    }
    if total_amount_to_transfer == 0 {
        msg!("Vesting contract has not yet reached release time");
        return Err(ProgramError::InvalidArgument);
    }

    let transfer_tokens_from_vesting_account = transfer(
        &spl_token_account.key,
        &vesting_token_account.key,
        destination_token_account.key,
        &vesting_account_key,
        &[],
        total_amount_to_transfer,
    )?;

    invoke_signed(
        &transfer_tokens_from_vesting_account,
        &[
            spl_token_account.clone(),
            vesting_token_account.clone(),
            destination_token_account.clone(),
            vesting_account.clone(),
        ],
        &[&[&seeds]],
    )?;

    // Reset released amounts to 0. This makes the simple unlock safe with complex scheduling contracts
    pack_schedules_into_slice(
        schedules,
        &mut packed_state.borrow_mut()[VestingScheduleHeader::LEN..],
    );

    Ok(())
}
```

Looking past the Solana-specific account validation and deserialization logic, the handler needs to do the following:

1. Check that the specified schedule exists.
1. Iterate over the schedule unlocks, summing the unlocked token amount and zeroing newly unlocked tokens.
1. Check that a non-zero amount of tokens needs to be transferred to the destination.
1. Transfer the unlocked amount from the escrow account to the current destination account. 

Invariant: The escrow account **MUST** have enough tokens to complete the transfer.

Implemented in Stylus, it can look like this:

```rust
sol! {
    // ...
    #[derive(Debug)]
    error ScheduleNotFound();
    #[derive(Debug)]
    error NoUnlocksAvailable();
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    // ...
    ScheduleNotFound(ScheduleNotFound),
    NoUnlocksAvailable(NoUnlocksAvailable),
}

#[public]
impl TokenVestingContract {
    // ...

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

    // ...
}
```

### Change destination

The [`ChangeDestination` instruction handler](https://github.com/Bonfida/token-vesting/blob/6234f98229196d1c785dfd2198bb58afc60bca10/program/src/processor.rs#L270-L318) looks like:

```rust
pub fn process_change_destination(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    seeds: [u8; 32],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let vesting_account = next_account_info(accounts_iter)?;
    let destination_token_account = next_account_info(accounts_iter)?;
    let destination_token_account_owner = next_account_info(accounts_iter)?;
    let new_destination_token_account = next_account_info(accounts_iter)?;

    if vesting_account.data.borrow().len() < VestingScheduleHeader::LEN {
        return Err(ProgramError::InvalidAccountData)
    }
    let vesting_account_key = Pubkey::create_program_address(&[&seeds], program_id)?;
    let state = VestingScheduleHeader::unpack(
        &vesting_account.data.borrow()[..VestingScheduleHeader::LEN],
    )?;

    if vesting_account_key != *vesting_account.key {
        msg!("Invalid vesting account key");
        return Err(ProgramError::InvalidArgument);
    }

    if state.destination_address != *destination_token_account.key {
        msg!("Contract destination account does not matched provided account");
        return Err(ProgramError::InvalidArgument);
    }

    if !destination_token_account_owner.is_signer {
        msg!("Destination token account owner should be a signer.");
        return Err(ProgramError::InvalidArgument);
    }

    let destination_token_account = Account::unpack(&destination_token_account.data.borrow())?;

    if destination_token_account.owner != *destination_token_account_owner.key {
        msg!("The current destination token account isn't owned by the provided owner");
        return Err(ProgramError::InvalidArgument);
    }

    let mut new_state = state;
    new_state.destination_address = *new_destination_token_account.key;
    new_state
        .pack_into_slice(&mut vesting_account.data.borrow_mut()[..VestingScheduleHeader::LEN]);

    Ok(())
}
```

This boils down to:

1. Check the proposed destination is valid
1. Check the schedule exits
1. Check the caller is the owner
1. Overwrite the existing destination

As mentioned in Phase 1, as the owner is determined by checking the owner of the destination associated token account, the `ChangeDestination` instruction also potentially changes the owner. In order to have feature parity, a seperate `change_owner` function is added to the Stylus implememtation.

```rust
#[public]
impl TokenVestingContract {
    // ...

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

    // ...
}
```

## Phase 4: View Functions

TODO

## Phase 5: Events

TODO

## Phase 6: Testing

TODO

