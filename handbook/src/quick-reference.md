Based on the documentation and diff, here's the updated chapter that aligns with the source of truth:

# Appendix A: Quick Reference

This appendix provides quick lookup tables and code snippets for common migration patterns and Stylus operations.

```rust
// Common imports for examples below
use stylus_sdk::prelude::*;
use stylus_sdk::{evm, msg, block, contract};
use stylus_sdk::call::{Call, static_call, transfer_eth};
use stylus_sdk::crypto::keccak;
use alloy_primitives::{Address, U256, B256, FixedBytes, address};
use alloy_sol_types::sol;
```

## Type Mapping Table

| Solana Type | Stylus Type | Notes | Example Conversion |
|-------------|-------------|-------|-------------------|
| `Pubkey` | `FixedBytes<32>` (recommended) or `Address` | No canonical conversion exists. Store original 32 bytes, or derive shadow address | See deterministic mapping pattern below |
| `u64` | `U256` | Consider size implications | `U256::from(value)` |
| `u128` | `U256` | Direct conversion | `U256::from(value)` |
| `i64` | `I256` | Signed integer conversion | `I256::from(value)` |
| `[u8; 32]` | `FixedBytes<32>` | Fixed-size byte arrays | `FixedBytes::from_slice(&bytes)` |
| `Vec<u8>` | `Bytes` | Dynamic byte arrays | `Bytes::from(vec)` |
| `String` | `String` | Direct equivalent | No conversion needed |
| `bool` | `bool` | Direct equivalent | No conversion needed |
| `AccountInfo` | `Address` | Reference to account converts to address | Use contract storage instead |
| `ProgramResult` | `Result<T, Vec<u8>>` | Error handling pattern | `Result<(), MyError>` |
| `lamports` | `U256` (wei-scale) | Decimal scaling (9→18), NOT currency exchange | `fn to_18dp_from_9dp(x: U256) -> U256 { x * U256::from(10u64.pow(9)) }` |

**Note**: Converting lamports to wei examples demonstrate decimal scaling (9→18), not economic exchange rates. Keep prices and configurations explicit per chain.

**Pubkey to Address Conversion** (if absolutely needed):
```rust
// Deterministic, but NOT a cryptographic identity mapping
use stylus_sdk::crypto::keccak;
let h = keccak(pubkey_bytes);                // [u8; 32]
let evm_addr = Address::from_slice(&h[12..]); // last 20 bytes
```

## Common Patterns

### Account Creation → Storage Init
```rust
// Solana: Create account with Anchor
#[account(
    init,
    payer = signer,
    space = 8 + 32 + 8,
    seeds = [b"user", signer.key().as_ref()],
    bump
)]
pub user_account: Account<'info, UserAccount>,

// Stylus: Initialize storage
#[storage]
#[entrypoint]
pub struct Contract {
    users: StorageMap<Address, UserData>,
}

#[storage]
pub struct UserData {
    balance: StorageU256,
    status: StorageBool,
    created_at: StorageU256,
}

#[public]
impl Contract {
    pub fn initialize_user(&mut self) {
        let user = msg::sender();
        let mut data = self.users.setter(user);
        data.balance.set(U256::ZERO);
        data.status.set(true);
        data.created_at.set(U256::from(block::timestamp()));
    }
}
```

### CPI → External Call
```rust
// Solana: Cross-program invocation with Anchor
let cpi_accounts = Transfer {
    from: ctx.accounts.from.to_account_info(),
    to: ctx.accounts.to.to_account_info(),
    authority: ctx.accounts.authority.to_account_info(),
};
let cpi_program = ctx.accounts.token_program.to_account_info();
let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
token::transfer(cpi_ctx, amount)?;

// Stylus: External contract call
sol_interface! {
    interface IERC20 {
        function transfer(address to, uint256 amount) returns (bool);
    }
}

let token = IERC20::new(token_address);
// Provide a call config (gas/value if needed)
let ok = token.transfer(Call::new_in(self), recipient, amount)?;
if !ok { return Err(b"ERC20 transfer returned false".to_vec()); }
```

### PDA Generation → Deterministic Address
```rust
// Solana: Program Derived Address with Anchor
#[account(
    seeds = [b"vault", user.key().as_ref()],
    bump
)]
pub vault: SystemAccount<'info>,

// Stylus: Virtual deterministic addresses (like Solana PDAs)
pub fn compute_virtual_vault_address(&self, user: Address) -> Address {
    let mut data = Vec::new();
    data.extend_from_slice(contract::address().as_bytes());
    data.extend_from_slice(user.as_bytes());
    data.extend_from_slice(b"vault");
    
    let hash = keccak(&data);
    Address::from_slice(&hash[12..])
}

// Alternative: Real CREATE2 deployment (if you need actual contracts)
pub fn deploy_vault_contract(&mut self, user: Address, salt: FixedBytes<32>) -> Address {
    // Note: Requires factory pattern or CREATE2 precompile access
    let init_code_hash = keccak(b"vault_bytecode");
    let mut data = Vec::new();
    data.extend_from_slice(&[0xff]);
    data.extend_from_slice(contract::address().as_bytes());
    data.extend_from_slice(salt.as_bytes());
    data.extend_from_slice(&init_code_hash);
    
    let hash = keccak(&data);
    Address::from_slice(&hash[12..])
}
```

### Event Logging → Event Emission
```rust
// Solana: Anchor events
#[event]
pub struct TransferEvent {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
}

emit!(TransferEvent {
    from: *ctx.accounts.from.key,
    to: *ctx.accounts.to.key,
    amount,
});

// Stylus: Event emission
sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

evm::log(Transfer {
    from: sender,
    to: recipient,
    value: amount,
});
```

## Attribute Reference

| Attribute | Purpose | Example |
|-----------|---------|---------|
| `#[storage]` | Define contract storage | `#[storage] pub struct MyContract { ... }` |
| `#[entrypoint]` | Mark main contract struct | `#[entrypoint] pub struct MyContract { ... }` |
| `#[public]` | Mark impl blocks as externally callable | `#[public] impl MyContract { ... }` |
| `#[payable]` | Function can receive ETH | `#[payable] pub fn deposit(&mut self) { ... }` |
| `#[constructor]` | Constructor function | `#[constructor] pub fn constructor(&mut self, param: U256) { ... }` |
| `#[fallback]` | Handle unknown function calls | `#[fallback] pub fn fallback(&mut self, input: &[u8]) -> ArbResult { ... }` |
| `#[receive]` | Handle plain ETH transfers | `#[receive] #[payable] pub fn receive(&mut self) -> Result<(), Vec<u8>> { ... }` |
| `#[inherit]` | Inherit from other contracts | `#[inherit(AccessControl)] impl MyToken { ... }` |
| `#[borrow]` | Borrow storage from inherited contract | `#[borrow] AccessControl access;` |
| `#[derive(SolidityError)]` | Create custom error types | `#[derive(SolidityError)] pub enum MyError { ... }` |

## Storage Patterns

### Simple Value Storage
```rust
#[storage]
pub struct SimpleStorage {
    owner: StorageAddress,
    total_supply: StorageU256,
    is_paused: StorageBool,
}

// Access patterns
let current_owner = self.owner.get();
self.owner.set(new_owner);

let supply = self.total_supply.get();
self.total_supply.set(supply + amount);
```

### Mapping Storage
```rust
#[storage]
pub struct MappingStorage {
    balances: StorageMap<Address, StorageU256>,
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
}

// Access patterns
let balance = self.balances.get(user);
self.balances.setter(user).set(new_balance);

let allowance = self.allowances.getter(owner).get(spender);
self.allowances.setter(owner).setter(spender).set(amount);
```

### Array Storage
```rust
#[storage]
pub struct ArrayStorage {
    participants: StorageVec<StorageAddress>,
    scores: StorageArray<StorageU256, 10>, // Fixed size
}

// Access patterns
self.participants.push(new_participant);
let participant_count = self.participants.len();
let first_participant = self.participants.get(0).unwrap();

let score = self.scores.get(index).unwrap();
self.scores.setter(index).unwrap().set(new_score);
```

### Complex Storage Structures
```rust
#[storage]
pub struct ComplexStorage {
    users: StorageMap<Address, UserData>,
    tokens: StorageMap<U256, TokenData>,
    authorized_minters: StorageVec<StorageAddress>,
}

#[storage]
pub struct UserData {
    balance: StorageU256,
    last_action_time: StorageU256,
    is_active: StorageBool,
}

#[storage]
pub struct TokenData {
    owner: StorageAddress,
    uri: StorageString,
    price: StorageU256,
}
```

## Error Handling Patterns

### Custom Error Types
```rust
sol! {
    error InsufficientBalance(uint256 requested, uint256 available);
    error Unauthorized(address caller);
    error InvalidAddress();
    error TransferFailed();
}

#[derive(SolidityError)]
pub enum TokenError {
    InsufficientBalance(InsufficientBalance),
    Unauthorized(Unauthorized),
    InvalidAddress(InvalidAddress),
    TransferFailed(TransferFailed),
}

// Usage in functions
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), TokenError> {
    let balance = self.balances.get(msg::sender());
    if amount > balance {
        return Err(TokenError::InsufficientBalance(InsufficientBalance {
            requested: amount,
            available: balance,
        }));
    }
    // ... transfer logic
    Ok(())
}
```

## Command Reference

### Project Management
```bash
# Create new Stylus project
cargo stylus new my-project
cd my-project

# Check contract validity
cargo stylus check

# Export contract ABI
cargo stylus export-abi

# Run local tests
cargo test

# Build optimized contract
cargo build --release --target wasm32-unknown-unknown
```

### Deployment Commands
```bash
# Deploy to Arbitrum Sepolia testnet
cargo stylus deploy \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc \
    --private-key-path ~/.wallet/key.txt

# Deploy with constructor arguments
cargo stylus deploy \
    --endpoint https://sepolia-rollup.arbitrum.io/rpc \
    --private-key-path ~/.wallet/key.txt \
    --constructor-args 0xdeadbeef

# Verify on Arbiscan
cargo stylus verify \
    --contract-address 0x... \
    --verifier-url https://api-sepolia.arbiscan.io/api \
    --arbiscan-api-key YOUR_API_KEY
```

### Development Commands
```bash
# Run tests with output
cargo test -- --nocapture

# Check for common issues
cargo clippy --all-targets --all-features

# Format code
cargo fmt

# Update dependencies
cargo update

# Clean build artifacts
cargo clean
```

## Gas Optimization Quick Tips

### Storage Packing
```rust
// Inefficient: Each value uses full 32-byte slot
#[storage]
struct Inefficient {
    is_active: StorageBool,      // 32 bytes
    user_type: StorageU8,        // 32 bytes  
    score: StorageU16,           // 32 bytes
}

// Note: Storage packing optimization requires careful consideration
// of access patterns and may require custom implementations
```

### Caching Storage Reads
```rust
// Inefficient: Multiple storage reads
pub fn calculate_reward(&self, user: Address) -> U256 {
    let balance = self.balances.get(user);
    let multiplier = self.multipliers.get(user);
    let base_rate = self.base_rate.get();
    
    if self.balances.get(user) > U256::from(1000) { // Second read
        return balance * multiplier * base_rate * U256::from(2);
    }
    balance * multiplier * base_rate
}

// Efficient: Cache values
pub fn calculate_reward(&self, user: Address) -> U256 {
    let balance = self.balances.get(user);
    let multiplier = self.multipliers.get(user);
    let base_rate = self.base_rate.get();
    
    if balance > U256::from(1000) { // Use cached value
        return balance * multiplier * base_rate * U256::from(2);
    }
    balance * multiplier * base_rate
}
```

## Common Constants

### Units and Limits
```rust
// Ethereum units
pub const WEI_PER_GWEI: U256 = U256::from(1_000_000_000u64);
pub const WEI_PER_ETHER: U256 = U256::from(1_000_000_000_000_000_000u128);

// Common gas limits
pub const TRANSFER_GAS: u64 = 21_000;
pub const ERC20_TRANSFER_GAS: u64 = 65_000;
pub const STORAGE_WRITE_GAS: u64 = 20_000;

// Time constants
pub const SECONDS_PER_DAY: u64 = 86_400;
pub const SECONDS_PER_WEEK: u64 = 604_800;
pub const SECONDS_PER_YEAR: u64 = 31_536_000;
```

### Common Addresses
```rust
// Special addresses
pub const ZERO_ADDRESS: Address = Address::ZERO;

// Precompiles
pub const ECRECOVER_PRECOMPILE: Address = address!("0000000000000000000000000000000000000001");
pub const SHA256_PRECOMPILE: Address = address!("0000000000000000000000000000000000000002");
```

## Testing Patterns

### Basic Test Setup
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stylus_sdk::testing::*;
    
    #[test]
    fn test_basic_functionality() {
        let vm = TestVM::new();
        let mut contract = MyContract::from(&vm);
        
        // Set up test environment
        vm.set_sender(address!("1234567890123456789012345678901234567890"));
        vm.set_value(U256::from(1_000_000));
        
        // Execute function
        let result = contract.my_function(test_param).unwrap();
        
        // Assert results
        assert_eq!(result, expected_value);
    }
}
```

### Event Testing
```rust
#[test]
fn test_event_emission() {
    let vm = TestVM::new();
    let mut contract = MyContract::from(&vm);
    
    // Execute function that emits event
    contract.transfer(recipient, amount).unwrap();
    
    // Check emitted logs
    let logs = vm.get_emitted_logs();
    assert_eq!(logs.len(), 1);
    
    // Verify event data
    let transfer_event = &logs[0];
    // Check event signature matches expected keccak hash
    let expected_sig = B256::from(keccak("Transfer(address,address,uint256)".as_bytes()));
    assert_eq!(transfer_event.0[0], expected_sig);
}
```

### Mock External Calls
```rust
#[test]
fn test_with_mock_external_call() {
    let vm = TestVM::new();
    let mut contract = MyContract::from(&vm);
    
    // Mock external contract response
    let token_address = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let selector = vec![0xa9, 0x05, 0x9c, 0xbb]; // transfer(address,uint256)
    let return_data = vec![0; 32]; // Return true (padded to 32 bytes)
    
    vm.mock_call(
        token_address,
        selector,
        Ok(return_data),
    );
    
    // Test function that makes external call
    let result = contract.interact_with_token(token_address).unwrap();
    assert!(result);
}
```

## Debugging Helpers

### Console Logging (Development Only)
```rust
use stylus_sdk::console;

#[public]
impl Contract {
    pub fn debug_function(&mut self, value: U256) -> Result<(), MyError> {
        console!("Starting function with value: {}", value);
        
        let intermediate = self.complex_calculation(value)?;
        console!("Intermediate result: {}", intermediate);
        
        if intermediate.is_zero() {
            console!("Warning: Zero intermediate result!");
        }
        
        Ok(())
    }
}
```

### State Inspection Helper
```rust
#[derive(Debug)]
pub struct ContractState {
    pub owner: Address,
    pub total_supply: U256,
    pub is_paused: bool,
    pub user_count: U256,
}

#[public]
impl Contract {
    pub fn get_state(&self) -> ContractState {
        ContractState {
            owner: self.owner.get(),
            total_supply: self.total_supply.get(),
            is_paused: self.is_paused.get(),
            user_count: U256::from(self.users.len()),
        }
    }
}
```

This quick reference provides essential patterns and conversions for migrating from Solana to Stylus. Keep this resource available as you work through your migration projects.