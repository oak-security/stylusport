# State storage

One of the most significant differences between Solana and Stylus involves how state storage and access work. This chapter covers the transformation from Solana's account-based storage model to Stylus's contract storage variables, including type mappings, access patterns, and cost considerations.

## Solana account model

Solana stores each piece of state in a separate, dedicated account with predetermined size allocation. Accounts must maintain a rent-exempt balance proportional to their size to avoid automatic deallocation, though closing accounts refunds the Lamports. Programs can only access the accounts provided in the instruction, which is fully client-controlled. Extreme care must be taken to validate the accounts received when processing the instruction in order to prevent exploits.

### Native

Solana programs can group related state together to be stored in accounts owned by the program. Native programs are required to explicitly create and initialize those accounts.

```rust
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Data {
    pub bool: bool,
    pub uint8: u8,
    pub uint16: u16,
    pub uint32: u32,
    pub uint64: u64,
    pub uint128: u128,
    pub int8: i8,
    pub int16: i16,
    pub int32: i32,
    pub int64: i64,
    pub int128: i128,
    pub string: String,
    pub bytes: Vec<u8>,
    pub address: Pubkey,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if Data::try_from_slice(instruction_data).is_err() {
        return Err(ProgramError::InvalidInstructionData);
    };

    let [payer, data_account, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    let lamports_required = Rent::get()?.minimum_balance(instruction_data.len());

    invoke(
        &system_instruction::create_account(
            payer.key,
            data_account.key,
            lamports_required,
            instruction_data.len() as u64,
            program_id,
        ),
        &[payer.clone(), data_account.clone(), system_program.clone()],
    )?;

    let mut data_account_buffer = data_account.try_borrow_mut_data()?;

    data_account_buffer.copy_from_slice(instruction_data);

    Ok(())
}
```

State that is to be maintained against other accounts such as user EOAs or other programs stored in accounts associated with Program Derived Addresses (PDAs). Each PDA is derived from a set of seeds, which can be viewed as a prefixed key and a 'bump' byte. In native Solana programs, the program owning the PDA must be careful to create and verify those accounts against the canonical bump seed, as well as protect against re-initialization attacks.

```rust
pub static SEED_SEPARATOR: &[u8] = b"-";
pub static PLAYER_PDA_ACCOUNT_SEED: &[u8] = b"player";

pub const STARTING_LIVES: u8 = 10;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct PlayerAccountState {
    pub lives: u8,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let Ok(args) = PlayerAccountState::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };

    // ensure correct initial player state is provided
    if args.lives != STARTING_LIVES {
        return Err(ProgramError::InvalidInstructionData);
    }

    let [payer, player_pda_account, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    // ensure PDA has not already been initialized
    if !player_pda_account.data_is_empty()
        || player_pda_account.lamports() > 0
        || *player_pda_account.owner == ID
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let (player_pda_account_key, bump) = Pubkey::find_program_address(
        &[PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key.as_ref()],
        &ID,
    );

    if player_pda_account_key != *player_pda_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let lamports_required = Rent::get()?.minimum_balance(instruction_data.len());

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            player_pda_account.key,
            lamports_required,
            instruction_data.len() as u64,
            program_id,
        ),
        &[
            payer.clone(),
            player_pda_account.clone(),
            system_program.clone(),
        ],
        &[&[
            PLAYER_PDA_ACCOUNT_SEED,
            SEED_SEPARATOR,
            payer.key.as_ref(),
            &[bump],
        ]],
    )?;

    let mut data_account_buffer = player_pda_account.try_borrow_mut_data()?;

    data_account_buffer.copy_from_slice(instruction_data);

    Ok(())
}
```

### Anchor

When defining Solana program instructions using the Anchor framework, program-owned accounts can be created automatically using the `#[account(init, ...)]` attribute. This also implicitly adds checks for already initialized accounts and always uses the canonical bump seed unless otherwise specified. 

```rust
#[derive(InitSpace)]
#[account]
pub struct Data {
    pub bool: bool,
    pub uint8: u8,
    pub uint16: u16,
    pub uint32: u32,
    pub uint64: u64,
    pub uint128: u128,
    pub int8: i8,
    pub int16: i16,
    pub int32: i32,
    pub int64: i64,
    pub int128: i128,
    #[max_len(200)]
    pub string: String,
    #[max_len(200)]
    pub bytes: Vec<u8>,
    pub address: Pubkey,
}

#[derive(Accounts)]
#[instruction(data: Data)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Data::INIT_SPACE
    )]
    pub data_account: Account<'info, Data>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod data_storage {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: Data) -> Result<()> {
        *ctx.accounts.data_account = data;
        Ok(())
    }
}
```

The Anchor framework abstracts the boilerplate required to manually create PDA accounts and automatically checks for initialization as well as correct seeds.

```rust
#[derive(InitSpace)]
#[account]
pub struct PlayerAccountState {
    pub lives: u8,
}

#[derive(Accounts)]
#[instruction()]
pub struct CreatePlayerAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + PlayerAccountState::INIT_SPACE,
        seeds = [PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key().as_ref()],
        bump,
    )]
    pub player_account: Account<'info, PlayerAccountState>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod data_storage {
    use super::*;

    pub fn create_player_account(ctx: Context<CreatePlayerAccount>) -> Result<()> {
        ctx.accounts.player_account.lives = STARTING_LIVES;
        Ok(())
    }
}
```


## Stylus storage model
 
Stylus stores all state within the contract's storage slots, allowing dynamic growth as needed within gas limits. Storage operations cost gas that users pay during transaction execution, with the smart contract execution VM automatically handling storage accessibility. State persists without ongoing rent requirements, as users only pay costs when writing data and for transaction calldata.

The `#[storage]` attribute macro can be used to implement [`StorageType`](https://docs.rs/stylus-sdk/latest/stylus_sdk/storage/trait.StorageType.html) for user-defined types, allowing state to be logically grouped together.

```rust
#[storage]
pub struct IntegerStore {
    uint8: StorageU8,
    uint16: StorageU16,
    uint32: StorageU32,
    uint64: StorageU64,
    uint128: StorageU128,
    uint256: StorageU256,
    int8: StorageI8,
    int16: StorageI16,
    int32: StorageI32,
    int64: StorageI64,
    int128: StorageI128,
    int256: StorageI256,
}

#[storage]
#[entrypoint]
pub struct DataStorage {
    // Types that implement `StorageType` can be nested
    // in order to namespace and organize related storage items
    integers: IntegerStore,
    bool: StorageBool,
    string: StorageString,
    bytes: StorageBytes,
    fixed_bytes: StorageFixedBytes<4>,
    vec: StorageVec<StorageU64>,
    address: StorageAddress,
}

#[public]
impl DataStorage {
    #[constructor]
    // for example purposes only, avoid using this many parameters to functions
    #[allow(clippy::too_many_arguments)]
    pub fn constructor(
        &mut self,
        bool: bool,
        uint8: U8,
        uint16: U16,
        uint32: U32,
        uint64: U64,
        uint128: U128,
        uint256: U256,
        int8: I8,
        int16: I16,
        int32: I32,
        int64: I64,
        int128: I128,
        int256: I256,
        string: String,
        bytes: Vec<u8>,
        fixed_bytes: FixedBytes<4>,
        vec: Vec<U64>,
        address: Address,
    ) {
        // unless explicitly set, all storage is initialized to the types respective zero-value
        self.bool.set(bool);
        self.integers.uint8.set(uint8);
        self.integers.uint16.set(uint16);
        self.integers.uint32.set(uint32);
        self.integers.uint64.set(uint64);
        self.integers.uint128.set(uint128);
        self.integers.uint256.set(uint256);
        self.integers.int8.set(int8);
        self.integers.int16.set(int16);
        self.integers.int32.set(int32);
        self.integers.int64.set(int64);
        self.integers.int128.set(int128);
        self.integers.int256.set(int256);
        self.string.set_str(string);
        self.bytes.set_bytes(bytes);
        self.fixed_bytes.set(fixed_bytes);

        for x in vec {
            self.vec.push(x);
        }

        self.address.set(address);
    }

    fn get_bool(&self) -> bool { self.bool.get() }
    fn get_uint8(&self) -> U8 { self.integers.uint8.get() }
    fn get_uint16(&self) -> U16 { self.integers.uint16.get() }
    fn get_uint32(&self) -> U32 { self.integers.uint32.get() }
    fn get_uint64(&self) -> U64 { self.integers.uint64.get() }
    fn get_uint128(&self) -> U128 { self.integers.uint128.get() }
    fn get_uint256(&self) -> U256 { self.integers.uint256.get() }
    fn get_int8(&self) -> I8 { self.integers.int8.get() }
    fn get_int16(&self) -> I16 { self.integers.int16.get() }
    fn get_int32(&self) -> I32 { self.integers.int32.get() }
    fn get_int64(&self) -> I64 { self.integers.int64.get() }
    fn get_int128(&self) -> I128 { self.integers.int128.get() }
    fn get_int256(&self) -> I256 { self.integers.int256.get() }
    fn get_string(&self) -> String { self.string.get_string() }
    fn get_bytes(&self) -> Vec<u8> { self.bytes.get_bytes() }
    fn get_fixed_bytes(&self) -> FixedBytes<4> { self.fixed_bytes.get() }
    fn get_address(&self) -> Address { self.address.get() }

    // Option<T> is not available as a return or a public function parameter type
    // as `None` cannot be EVM ABI-encoded, hence the use of (bool, T)
    fn get_vec_item(&self, idx: u32) -> (bool, U64) {
        self.vec.get(idx).map_or((false, U64::ZERO), |x| (true, x))
    }
}
```

The `StorageMap` type can be used to store state using keys that are calculated at runtime, for example the `Address` of the caller. Care needs to be taken as the mapped type's zero value will be returned if an entry does not exist for the provided key. For some data this is fine and expected, such as token balances or allowances. 

```rust
#[storage]
#[entrypoint]
pub struct Mappings {
    player_lives: StorageMap<Address, StorageU8>,
    player_is_dead: StorageMap<Address, StorageBool>,
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    error PlayerAlreadyExists(address player);

    #[derive(Debug, PartialEq, Eq)]
    error PlayerNotFound(address player);
}

#[derive(SolidityError, Debug, PartialEq, Eq)]
pub enum ContractError {
    PlayerAlreadyExists(PlayerAlreadyExists),
    PlayerNotFound(PlayerNotFound),
}

impl Mappings {
    fn player_exists(&self, player: Address) -> bool {
        self.player_lives.get(player) > U8::ZERO || self.player_is_dead.get(player)
    }
}

#[public]
impl Mappings {
    pub fn create_player_account(&mut self) -> Result<(), ContractError> {
        let msg_sender = self.vm().msg_sender();

        if self.player_exists(msg_sender) {
            return Err(PlayerAlreadyExists { player: msg_sender }.into());
        }

        self.player_lives
            .insert(self.vm().msg_sender(), U8::from(STARTING_LIVES));

        Ok(())
    }

    pub fn get_is_dead(&self, player: Address) -> Result<bool, ContractError> {
        if !self.player_exists(player) {
            return Err(PlayerNotFound { player }.into());
        }

        Ok(self.player_is_dead.get(player))
    }

    pub fn get_lives(&self, player: Address) -> Result<U8, ContractError> {
        if !self.player_exists(player) {
            return Err(PlayerNotFound { player }.into());
        }

        Ok(self.player_lives.get(player))
    }
}
```

## Solana to Stylus type mappings

### Primitive types

| Solana Type | Stylus Storage Type | Rust Parameter/Return Type | Notes |
|-------------|---------------------|---------------------------|-------|
| `u8` | `StorageU8` | `U8` | Direct mapping |
| `u16` | `StorageU16` | `U16` | Direct mapping |
| `u32` | `StorageU32` | `U32` | Direct mapping |
| `u64` | `StorageU64` or `StorageU256` | `U64` or `U256` | Use U256 where ERC standards expect it |
| `u128` | `StorageU128` or `StorageU256` | `U128` or `U256` | Prefer U256 for interoperability |
| `bool` | `StorageBool` | `bool` | Direct mapping |
| `Pubkey` | `StorageAddress` | `Address` | Use `Address` for EOAs and other contracts |
| `String` | `StorageString` | `String` | Direct mapping |
| `Vec<u8>` | `StorageBytes` | `Vec<u8>` or `Bytes` | Direct mapping |
| `[u8; N]` | `StorageFixedBytes<N>` | `[u8; N]` or `FixedBytes<N>` | Direct mapping |

### More complex schemas

| Solana Pattern | Stylus Storage Pattern |
|----------------|------------------------|
| Several PDAs with fixed seeds | Multiple structs tagged with `#[storage]` nested under the struct marked `#[entrypoint]` |
| PDAs with dynamic seeds, like a user `Pubkey` | Use `StorageMap<K, V>` where `K` consists of the dynamic seed component and `V` implements `StorageType` |


### Nested mappings

In cases where there are multiple dynamic components of a key, a nested `StorageMap` can be used.

```rust
use stylus_sdk::storage::{StorageMap, StorageU256, StorageBool};

#[storage]
pub struct Transaction {
    amount: StorageU256,
    timestamp: StorageU256,
    completed: StorageBool,
}

#[storage]
#[entrypoint]
pub struct TokenContract {
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    user_transactions: StorageMap<Address, StorageMap<U256, Transaction>>,
}

#[public]
impl TokenContract {
    pub fn approve(&mut self, spender: Address, amount: U256) {
        self.allowances
            .setter(self.vm().msg_sender())
            .insert(spender, amount);
    }

    pub fn record_transaction(&mut self, tx_id: U256, amount: U256) {
        let block_time = self.vm().block_timestamp();

        // a nested `setter` cannot be called in a single expression
        let mut txs = self.user_transactions.setter(self.vm().msg_sender());
        let mut tx = txs.setter(tx_id);

        tx.amount.set(amount);
        tx.timestamp.set(U256::from(block_time));
        tx.completed.set(true);
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.getter(owner).get(spender)
    }

    pub fn transaction(&self, address: Address, tx_id: U256) -> (U256, U256, bool) {
        let txs = self.user_transactions.getter(address);
        let tx = txs.get(tx_id);

        (tx.amount.get(), tx.timestamp.get(), tx.completed.get())
    }
}
```

## Cost considerations

### Solana costs
- **Account Creation**: Rent-exempt balance amount (approximately 0.002 SOL per account)
- **Storage Rent**: Ongoing cost for keeping accounts alive
- **No Cost for Reads**: Reading account data requires no fee

### Stylus costs
- **Storage Writes**: Gas cost for storing data (approximately 20,000 gas per 32-byte slot)
- **Storage Reads**: Much cheaper than writes (approximately 200 gas per read)
- **One-time Cost**: Pay when writing, no ongoing costs

### Optimization strategies

**Pack Related Data:**
```rust
// Instead of separate mappings
#[storage]
pub struct Inefficient {
    usernames: StorageMap<Address, StorageString>,
    emails: StorageMap<Address, StorageString>,
    created_at: StorageMap<Address, StorageU256>,
}

// Use a single struct
#[storage]
pub struct Efficient {
    profiles: StorageMap<Address, UserProfile>,
}

#[storage]
pub struct UserProfile {
    username: StorageString,
    email: StorageString,
    created_at: StorageU256,
}
```

**Use Appropriate Types:**
```rust
#[storage]
pub struct OptimizedStorage {
    // Don't waste space with oversized types
    small_counter: StorageU8,    // for values 0-255
    timestamp: StorageU32,       // sufficient for timestamps
    large_value: StorageU256,    // when needed
    
    // Pack booleans together
    flag1: StorageBool,
    flag2: StorageBool,
    flag3: StorageBool,
    // These will be packed into a single storage slot
}
```

## Next steps

With storage patterns understood, the next chapter covers [Access Control](./access-control.md) - transforming Solana's signer checks and PDA patterns to Stylus access control mechanisms.

