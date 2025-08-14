# State storage

One of the most significant differences between Solana and Stylus involves how state storage and access work. This chapter covers the transformation from Solana's account-based storage model to Stylus's contract storage variables, including type mappings, access patterns, and cost considerations.

## Storage model comparison

### Solana account model
- **Separate Accounts**: Each piece of state lives in a dedicated account
- **Explicit Size**: Account size must have predetermined allocation
- **Rent-Exempt Balance**: Accounts must maintain a rent-exempt balance proportional to their size, or risk deallocation. Closing accounts refunds Lamports
- **Manual Management**: Developers handle account creation and validation

### Stylus storage model  
- **Contract Storage**: All state lives within the contract's storage slots
- **Dynamic Growth**: Storage can grow as needed within gas limits
- **Gas-based Cost**: Storage operations cost gas, which users pay during transaction execution
- **Automatic Management**: The EVM handles storage
- **No Ongoing Rent**: State persists without rent requirements. Users pay costs when writing data and for transaction calldata

## Basic type mappings

### Primitive types

| Solana Type | Stylus Storage Type | Rust Parameter/Return Type | Notes |
|-------------|---------------------|---------------------------|-------|
| `u8` | `StorageU8` | `u8` | Direct mapping |
| `u16` | `StorageU16` | `u16` | Direct mapping |
| `u32` | `StorageU32` | `u32` | Direct mapping |
| `u64` | `StorageU64` or `StorageU256` | `U64` or `U256` | Use U256 where ERC standards expect it |
| `u128` | `StorageU128` or `StorageU256` | `U128` or `U256` | Prefer U256 for interoperability |
| `bool` | `StorageBool` | `bool` | Direct mapping |
| `Pubkey` | `StorageAddress` or `StorageFixedBytes<32>` | `Address` or `FixedBytes<32>` | Use Address for EVM accounts, FixedBytes<32> to preserve Solana pubkeys |
| `String` | `StorageString` | `String` | Direct mapping |
| `Vec<u8>` | `StorageBytes` | `Vec<u8>` or `Bytes` | Direct mapping |

### Complex types

| Solana Pattern | Stylus Storage Pattern |
|----------------|------------------------|
| Several accounts | Single storage struct with several fields |
| Account arrays | `StorageVec<T>` for dynamic arrays |
| Hash maps | `StorageMap<K, V>` |
| Nested structures | Nested storage structs |

## Account-to-storage transformation

### From Solana accounts

**Solana Native:**
```rust
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

// Separate account structures
#[derive(BorshSerialize, BorshDeserialize)]
pub struct UserProfile {
    pub owner: Pubkey,
    pub username: String,
    pub email: String,
    pub created_at: u64,
    pub is_verified: bool,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct UserStats {
    pub owner: Pubkey,
    pub login_count: u64,
    pub last_login: u64,
    pub total_spent: u64,
}

// Manual account creation and management
fn create_user_profile(
    accounts: &[AccountInfo],
    username: String,
    email: String,
) -> ProgramResult {
    let profile_account = &accounts[0];
    let owner = &accounts[1];
    
    // Calculate space needed
    let space = 32 + 4 + username.len() + 4 + email.len() + 8 + 1;
    
    // Create and initialize account
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(space);
    
    // ... account creation logic
    
    let profile = UserProfile {
        owner: *owner.key,
        username,
        email,
        created_at: Clock::get()?.unix_timestamp as u64,
        is_verified: false,
    };
    
    profile.serialize(&mut &mut profile_account.data.borrow_mut()[..])?;
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::prelude::*;

// Account structures with constraints
#[account]
#[derive(InitSpace)]
pub struct UserProfile {
    pub owner: Pubkey,
    #[max_len(50)]
    pub username: String,
    #[max_len(100)] 
    pub email: String,
    pub created_at: u64,
    pub is_verified: bool,
}

#[account]
#[derive(InitSpace)]
pub struct UserStats {
    pub owner: Pubkey,
    pub login_count: u64,
    pub last_login: u64,
    pub total_spent: u64,
}

// Account contexts
#[derive(Accounts)]
#[instruction(username: String, email: String)]
pub struct CreateProfile<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + UserProfile::INIT_SPACE
    )]
    pub profile: Account<'info, UserProfile>,
    #[account(
        init,
        payer = owner, 
        space = 8 + UserStats::INIT_SPACE
    )]
    pub stats: Account<'info, UserStats>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

### To Stylus storage

**Stylus:**
```rust
use stylus_sdk::{
    alloy_primitives::{Address, U256},
    block, msg,
    prelude::*,
    storage::{StorageAddress, StorageBool, StorageString, StorageU256, StorageMap},
};

// Combined storage structure
#[storage]
#[entrypoint]
pub struct UserManager {
    // User profiles mapping
    profiles: StorageMap<Address, UserProfile>,
    // User statistics mapping
    stats: StorageMap<Address, UserStats>,
    // Total users counter
    total_users: StorageU256,
    // Contract owner
    owner: StorageAddress,
}

// Nested storage structures
#[storage]
pub struct UserProfile {
    username: StorageString,
    email: StorageString,
    created_at: StorageU256,
    is_verified: StorageBool,
}

#[storage]
pub struct UserStats {
    login_count: StorageU256,
    last_login: StorageU256,
    total_spent: StorageU256,
}

#[public]
impl UserManager {
    pub fn constructor(&mut self) {
        self.owner.set(msg::sender());
        self.total_users.set(U256::ZERO);
    }
    
    pub fn create_profile(
        &mut self,
        username: String,
        email: String,
    ) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        
        // Check if profile already exists
        if !self.profiles.getter(user).username.get_string().is_empty() {
            return Err(b"Profile already exists".to_vec());
        }
        
        // Create profile
        let mut profile = self.profiles.setter(user);
        profile.username.set_str(username);
        profile.email.set_str(email);
        profile.created_at.set(U256::from(block::timestamp()));
        profile.is_verified.set(false);
        
        // Initialize stats
        let mut stats = self.stats.setter(user);
        stats.login_count.set(U256::ZERO);
        stats.last_login.set(U256::from(block::timestamp()));
        stats.total_spent.set(U256::ZERO);
        
        // Update total users
        self.total_users.set(self.total_users.get() + U256::from(1));
        
        Ok(())
    }
}
```

## Storage access patterns

### Reading data

**Solana Pattern:**
```rust
// Load account data
let profile_data = &profile_account.data.borrow();
let profile = UserProfile::try_from_slice(profile_data)?;
let username = profile.username.clone();
```

**Stylus Pattern:**
```rust
// Direct storage access
let username = self.profiles.getter(user).username.get_string();
```

### Writing data

**Solana Pattern:**
```rust
// Modify and serialize back
let mut profile = UserProfile::try_from_slice(&profile_account.data.borrow())?;
profile.username = new_username;
profile.serialize(&mut &mut profile_account.data.borrow_mut()[..])?;
```

**Stylus Pattern:**
```rust
// Direct storage modification
self.profiles.setter(user).username.set_str(new_username);
```

### Collections and mappings

**Solana**: Many accounts or program-derived addresses (PDAs)
```rust
// Finding user by index requires PDA derivation
let (user_pda, _) = Pubkey::find_program_address(
    &[b"user", &index.to_le_bytes()],
    program_id
);
```

**Stylus**: Native mapping support
```rust
// Direct mapping access
let user_profile = self.profiles.getter(user_address);
let username = user_profile.username.get_string();
```

## Working example: token vault migration

Here's a complete migration of a token vault from Solana to Stylus:

**Solana Native:**
```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Vault {
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub total_deposited: u64,
    pub is_paused: bool,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct UserDeposit {
    pub owner: Pubkey,
    pub amount: u64,
    pub deposited_at: u64,
    pub rewards_claimed: u64,
}
```

**Stylus:**
```rust
use stylus_sdk::storage::{StorageAddress, StorageBool, StorageU256, StorageMap, StorageVec};

#[storage]
#[entrypoint]
pub struct TokenVault {
    authority: StorageAddress,
    token_mint: StorageAddress,
    total_deposited: StorageU256,
    is_paused: StorageBool,
    
    // User deposits mapping
    deposits: StorageMap<Address, UserDeposit>,
    // Track all depositors
    depositors: StorageVec<StorageAddress>,
}

#[storage]
pub struct UserDeposit {
    amount: StorageU256,
    deposited_at: StorageU256,
    rewards_claimed: StorageU256,
}
```

## Advanced storage patterns

### Arrays and vectors

**Dynamic Arrays:**
```rust
use stylus_sdk::storage::{StorageVec, StorageString, StorageBool, StorageU256};

#[storage]
pub struct MyContract {
    // Dynamic array of addresses
    users: StorageVec<StorageAddress>,
    // Dynamic array of custom structs  
    items: StorageVec<Item>,
}

#[storage]
pub struct Item {
    id: StorageU256,
    name: StorageString,
    active: StorageBool,
}

#[public]
impl MyContract {
    pub fn add_user(&mut self, user: Address) {
        self.users.push(StorageAddress::new(user, 0));
    }
    
    pub fn get_user_count(&self) -> U256 {
        U256::from(self.users.len())
    }
    
    pub fn get_user(&self, index: U256) -> Result<Address, Vec<u8>> {
        self.users.get(index)
            .map(|addr| addr.get())
            .ok_or_else(|| b"Index out of bounds".to_vec())
    }
}
```

**Fixed Arrays:**
```rust
use stylus_sdk::storage::{StorageArray, StorageU256, StorageAddress};

#[storage]
pub struct MyContract {
    // Fixed-size array
    scores: StorageArray<StorageU256, 10>,
    // Array of structs
    top_players: StorageArray<Player, 5>,
}

#[storage]
pub struct Player {
    addr: StorageAddress,
    score: StorageU256,
}

#[public]
impl MyContract {
    pub fn set_score(&mut self, index: U256, score: U256) -> Result<(), Vec<u8>> {
        if index >= U256::from(10) {
            return Err(b"Index out of bounds".to_vec());
        }
        self.scores.setter(index).unwrap().set(score);
        Ok(())
    }
}
```

### Nested mappings

```rust
use stylus_sdk::storage::{StorageMap, StorageU256, StorageBool};

#[storage]
pub struct TokenContract {
    // Nested mappings for allowances
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    // Complex nested structure
    user_transactions: StorageMap<Address, StorageMap<U256, Transaction>>,
}

#[storage]
pub struct Transaction {
    amount: StorageU256,
    timestamp: StorageU256,
    completed: StorageBool,
}

#[public]
impl TokenContract {
    pub fn approve(&mut self, spender: Address, amount: U256) {
        self.allowances.setter(msg::sender()).insert(spender, StorageU256::new(amount, 0));
    }
    
    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.getter(owner).get(spender).get()
    }
    
    pub fn record_transaction(
        &mut self, 
        tx_id: U256, 
        amount: U256
    ) -> Result<(), Vec<u8>> {
        let mut tx = self.user_transactions
            .setter(msg::sender())
            .setter(tx_id);
            
        tx.amount.set(amount);
        tx.timestamp.set(U256::from(block::timestamp()));
        tx.completed.set(false);
        
        Ok(())
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

## Migration checklist

### Planning phase
- [ ] Map all account structures to storage variables
- [ ] Identify relationships between accounts
- [ ] Plan mapping keys and access patterns
- [ ] Consider storage packing opportunities

### Implementation phase
- [ ] Define storage structure with `#[storage]`
- [ ] Replace account loading with storage access
- [ ] Update all read operations
- [ ] Update all write operations
- [ ] Add proper initialization in constructor

### Testing phase
- [ ] Test storage read/write operations
- [ ] Verify mapping functionality
- [ ] Check storage packing efficiency
- [ ] Measure gas costs vs. Solana rent

## Common pitfalls

### Forgetting storage initialization
```rust
#[storage]
#[entrypoint]
pub struct MyContract {
    value: StorageU256,
    is_initialized: StorageBool,
}

#[public]
impl MyContract {
    // Missing constructor - storage never initialized!
    pub fn set_value(&mut self, value: U256) {
        self.value.set(value); // May fail or behave unexpectedly
    }
}
```

**Fix:**
```rust
#[public]
impl MyContract {
    pub fn constructor(&mut self) {
        self.value.set(U256::ZERO);
        self.is_initialized.set(true);
    }
}
```

### Inefficient storage access
```rust
// Don't repeatedly access the same storage
pub fn process_users(&self) -> U256 {
    let mut total = U256::ZERO;
    for i in 0..self.users.len() {
        let user = self.users.get(i).unwrap(); // Expensive repeated reads
        let balance = self.balances.getter(user.get()).get(); // More reads
        total = total + balance;
    }
    total
}
```

**Fix:**
```rust
// Cache frequently accessed data
pub fn process_users(&self) -> U256 {
    let user_count = self.users.len();
    let mut total = U256::ZERO;
    
    // Single pass through users
    for i in 0..user_count {
        if let Some(user) = self.users.get(i) {
            total = total + self.balances.getter(user.get()).get();
        }
    }
    total
}
```

### Wrong storage types
```rust
#[storage]
pub struct BadTypes {
    authority: Pubkey,     // Wrong - Solana type
    data: Vec<u8>,        // Wrong - Use StorageBytes instead
    amount: u64,          // Wrong - Use StorageU64
}
```

**Fix:**
```rust
#[storage]
pub struct GoodTypes {
    authority: StorageAddress,   // Correct - EVM address type
    data: StorageBytes,         // Correct - Storage bytes
    amount: StorageU256,        // Correct - Storage uint
}
```

## Best practices

### 1. Group related data
```rust
#[storage]
pub struct OptimalStorage {
    // Good: Related data in same struct
    users: StorageMap<Address, UserData>,
}

#[storage]
pub struct UserData {
    name: StorageString,
    email: StorageString,
    balance: StorageU256,
    active: StorageBool,
    last_activity: StorageU256,
}
```

### 2. Use appropriate access patterns
```rust
#[public]
impl OptimalStorage {
    // Good: Direct mapping access
    pub fn get_user_balance(&self, user: Address) -> U256 {
        self.users.getter(user).balance.get()
    }
    
    // Avoid: Linear searches
    pub fn find_user_by_name(&self, name: String) -> Option<Address> {
        // This would require iterating all users - expensive!
        // Consider additional mapping if needed:
        // username_to_address: StorageMap<StorageString, StorageAddress>
    }
}
```

### 3. Initialize storage correctly
```rust
#[public]
impl UserManager {
    pub fn constructor(&mut self, initial_admin: Address) {
        self.admin.set(initial_admin);
        self.total_users.set(U256::ZERO);
        self.is_initialized.set(true);
        
        // Initialize any default values
        self.max_users_per_page.set(U256::from(100));
        self.registration_fee.set(U256::from(1_000_000)); // 1 USDC
    }
}
```

## Next steps

With storage patterns understood, the next chapter covers [Access Control](./access-control.md) - transforming Solana's signer checks and PDA patterns to Stylus access control mechanisms.

## Reference

- [Example Code: state-storage](/examples/concepts/state-storage/)
- [Stylus SDK Storage Documentation](https://docs.rs/stylus-sdk/latest/stylus_sdk/storage/index.html)
- [Solidity Storage Layout](https://docs.soliditylang.org/en/latest/internals/layout_in_storage.html)