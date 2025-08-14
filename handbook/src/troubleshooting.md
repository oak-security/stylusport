# Appendix B: Troubleshooting Guide

This guide helps you diagnose and resolve common issues encountered when migrating from Solana to Stylus.

## Common Migration Errors

### Type Conversion Issues

#### Problem: "cannot convert Pubkey to Address"
When migrating from Solana, you'll encounter type mismatches between Solana's `Pubkey` (32 bytes) and Ethereum's `Address` (20 bytes).

**Important:** You cannot import `solana_program` inside a Stylus contract. Treat Solana Pubkey as raw 32-byte data you received off-chain or from a bridge.

```rust
// ❌ This won't compile in Stylus contracts
use solana_program::pubkey::Pubkey; // Not available!
let address: Address = pubkey; // Error: can't convert
```

**Solutions:**
```rust
// ✅ Working with raw pubkey bytes (received from off-chain)
use alloy_primitives::Address;
use stylus_sdk::crypto::keccak;

// Deterministic but NOT an identity mapping; for namespacing only.
fn pubkey_bytes_to_address(pubkey32: &[u8; 32]) -> Address {
    let h = keccak(pubkey32);
    Address::from_slice(&h[12..])
}

// ✅ If you need to preserve Solana identity, store the full key
use alloy_primitives::FixedBytes;

#[storage]
#[entrypoint]
pub struct CrossChainUser {
    solana_pubkey: StorageFixedBytes<32>,  // Store original 32 bytes
    evm_address: StorageAddress,            // Derived address for EVM operations  
    balance: StorageU256,
}

// Note: Verify Solana signatures off-chain or via precompiles, not by truncation
```

#### Problem: "mismatched types: expected U256, found u64"
Solana commonly uses `u64` while Stylus uses `U256` for token amounts and balances.

```rust
// ❌ Type mismatch
let value: u64 = 1000;
self.balance.set(value); // Error: expected U256
```

**Solutions:**
```rust
// ✅ Explicit conversion
let value: u64 = 1000;
self.balance.set(U256::from(value));

// ✅ Direct U256 creation
self.balance.set(U256::from(1000));

// ✅ For large numbers
self.balance.set(U256::from_str("1000000000000000000").unwrap()); // 1 ETH in wei
```

#### Problem: String/bytes conversion errors
Solana and Stylus handle strings and byte arrays differently.

```rust
// ❌ Incorrect string handling
let data: &[u8] = b"hello";
let string_value: String = data; // Error: can't convert
```

**Solutions:**
```rust
use stylus_sdk::abi::Bytes;
use alloc::string::String;

// ✅ Safe string conversion
fn safe_bytes_to_string(data: &[u8]) -> Result<String, Vec<u8>> {
    core::str::from_utf8(data).map(|s| s.to_owned()).map_err(|_| b"utf8".to_vec())
}

// ✅ Convert to Bytes type
fn to_bytes(data: &[u8]) -> Bytes { 
    Bytes::from(data.to_vec()) 
}

// ✅ Safe conversion with fallback
let data: &[u8] = b"hello";
let string_value = safe_bytes_to_string(data).unwrap_or_else(|_| "<invalid>".to_owned());
```

### Storage Access Errors

#### Problem: "forgot to gate functions on initialization"
Stylus storage is zero-initialized. Reads on unseen keys return zero/empty. The issue is usually forgetting to gate functions on proper initialization.

```rust
// ❌ Functions not gated on initialization check
pub fn get_balance(&self, user: Address) -> U256 {
    self.balances.get(user) // Returns zero for new users - this may be correct!
    // Problem: if contract itself isn't initialized, this might be wrong behavior
}
```

**Solutions:**
```rust
// ✅ Initialize storage in constructor
#[storage]
#[entrypoint]
pub struct Contract {
    initialized: StorageBool,
    balances: StorageMap<Address, StorageU256>,
    owner: StorageAddress,
}

#[public]
impl Contract {
    #[constructor]
    pub fn constructor(&mut self, owner: Address) {
        assert!(owner != Address::ZERO, "owner=0");
        self.owner.set(owner);
        self.initialized.set(true);
    }
    
    // Alternative: legacy initialization pattern
    pub fn initialize(&mut self) -> Result<(), Vec<u8>> {
        if self.initialized.get() {
            return Err(b"Already initialized".to_vec());
        }
        self.initialized.set(true);
        Ok(())
    }

    // ✅ Check initialization in functions
    pub fn get_balance(&self, user: Address) -> Result<U256, Vec<u8>> {
        if !self.initialized.get() {
            return Err(b"Not initialized".to_vec());
        }
        Ok(self.balances.get(user))
    }
}
```

#### Problem: "storage slot collision"
Collisions occur when you delegatecall/upgrade or compose storages in one entrypoint. For simple single contracts, this rarely matters.

```rust
// ❌ Multiple contracts using same storage slots
#[storage]
pub struct Contract1 {
    value1: StorageU256,  // Slot 0
}

#[storage]
pub struct Contract2 {
    value2: StorageU256,  // Also slot 0!
}
```

**Solutions:**
```rust
// ✅ The gap pattern is relevant for proxies/upgrades
#[storage]
pub struct SafeContract {
    _gap1: StorageArray<StorageU256, 100>,  // Reserve slots 0-99 for future fields
    value1: StorageU256,                     // Slot 100
    _gap2: StorageArray<StorageU256, 50>,   // Reserve slots 101-150
    value2: StorageU256,                     // Slot 151
}

// ✅ For most cases, prefer composition via #[borrow]
sol_storage! {
    #[entrypoint]
    pub struct ChildContract {
        #[borrow]
        ParentContract parent;  // Inherits parent's storage layout
        
        uint256 child_value;    // Automatically uses next available slot
    }
}
```

### Compilation Errors

#### Problem: "method not found in scope"
Missing imports or incorrect method usage is common when migrating.

```rust
// ❌ Missing trait implementation
impl MyContract {
    pub fn transfer(&mut self) {
        self.emit_event(); // Error: method not found
    }
}
```

**Solutions:**
```rust
// ✅ Import required modules
use stylus_sdk::{evm, alloy_sol_types::sol};

// ✅ Define events properly
sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

// ✅ Use correct event emission
#[public]
impl MyContract {
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        // Emit event correctly
        evm::log(Transfer {
            from: msg::sender(),
            to,
            value: amount,
        });
        Ok(())
    }
}
```

#### Problem: "cannot find macro `sol_storage!` in this scope"
```rust
// ❌ Missing imports
sol_storage! {  // Error: macro not found
    pub struct MyContract {
        uint256 value;
    }
}
```

**Solutions:**
```rust
// ✅ Use the prelude and avoid pinning old versions
use stylus_sdk::prelude::*; // brings sol_storage!, #[public], etc.

// ✅ Verify Cargo.toml dependencies
[dependencies]
stylus-sdk = "0.6.0"         # or latest compatible
alloy-primitives = "0.7.6"   # keep in sync with stylus-sdk
alloy-sol-types = "0.7.6"

[features]
export-abi = ["stylus-sdk/export-abi"]
debug = ["stylus-sdk/debug"] # for console!()
```

## Deployment Issues

### Gas Estimation Failures

#### Problem: "intrinsic gas too low"
Stylus contracts may require more gas than expected, especially during deployment.

```bash
Error: Transaction gas limit is too low. Expected at least 1234567 gas.
```

**Solutions:**
```bash
# ✅ Check contract size first
cargo stylus check --endpoint $RPC_URL

# ✅ Deploy with explicit gas limit
cargo stylus deploy \
    --endpoint $RPC_URL \
    --private-key $PRIVATE_KEY \
    --gas-limit 5000000

# ✅ Optimize contract to reduce deployment cost
# - Remove unused imports
# - Minimize constructor logic
# - Use efficient storage patterns
```

#### Problem: "insufficient funds for gas * price + value"
```bash
Error: sender doesn't have enough funds to send tx
```

**Solutions:**
```bash
# ✅ Check account balance
cast balance $YOUR_ADDRESS --rpc-url $RPC_URL

# ✅ Get testnet funds
# For Arbitrum Sepolia: https://faucet.arbitrum.io/

# ✅ Calculate required funds
# Deployment typically requires 0.01-0.05 ETH on testnet
```

### Constructor Parameter Errors

#### Problem: "constructor argument encoding failed"
Stylus constructors require specific parameter encoding.

**Solutions:**
```rust
// ✅ Constructor signatures: take &mut self, don't return Result
#[public]
impl Contract {
    #[constructor]
    pub fn constructor(&mut self, owner: Address, initial_supply: U256) {
        assert!(owner != Address::ZERO, "owner=0");
        assert!(initial_supply > U256::ZERO, "supply=0");
        self.owner.set(owner);
        self.total_supply.set(initial_supply);
        self.initialized.set(true);
    }
}

// Note: Use msg::sender() sparingly during construction;
// if you deploy via a factory, tx_origin may differ from msg::sender()
```

### Verification Failures

#### Problem: "verification failed: bytecode mismatch"
Contract verification requires exact build reproduction.

**Solutions:**
```bash
# ✅ Ensure exact same compiler version
rustc --version  # Note the version
cargo stylus --version

# ✅ Clean build before verification
cargo clean
cargo build --release

# ✅ Verify with same optimization flags
cargo stylus verify \
    --deployment-tx $TX_HASH \
    --endpoint $RPC_URL
```

## Runtime Errors

### Revert Without Reason

#### Problem: Transaction reverts without error message
Unlike Solana's detailed errors, EVM reverts can be opaque.

**Solutions:**
```rust
// ✅ Use descriptive custom errors
use stylus_sdk::alloy_sol_types::sol;

sol! {
    error InsufficientBalance(uint256 required, uint256 available);
    error Unauthorized(address caller);
    error InvalidAmount(uint256 amount);
}

#[derive(SolidityError)]
pub enum TokenError {
    InsufficientBalance(InsufficientBalance),
    Unauthorized(Unauthorized),
    InvalidAmount(InvalidAmount),
}

#[public]
impl Token {
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), TokenError> {
        let balance = self.balances.get(msg::sender());
        
        if balance < amount {
            return Err(TokenError::InsufficientBalance(InsufficientBalance {
                required: amount,
                available: balance,
            }));
        }
        
        // Transfer logic
        Ok(())
    }
}
```

### Out of Gas Errors

#### Problem: "out of gas" during transaction
Gas limits are more restrictive than Solana's compute units.

**Solutions:**
```rust
// ✅ Implement batch processing with limits
use stylus_sdk::evm;

const MAX_BATCH: usize = 50;

#[public]
impl Contract {
    pub fn batch_process(&mut self, items: Vec<ProcessItem>) -> Result<(), Vec<u8>> {
        // Limit batch size
        if items.len() > MAX_BATCH {
            return Err(b"batch too large".to_vec());
        }
        
        // Process with gas checks
        for (i, item) in items.iter().enumerate() {
            // Standardize on evm::gas_left()
            if evm::gas_left() < 50_000 {
                return Err(b"gas exhausted".to_vec());
            }
            
            self.process_single(item)?;
        }
        
        Ok(())
    }
}

// ✅ Optimize storage access
pub fn optimized_sum(&self, users: Vec<Address>) -> U256 {
    let mut total = U256::ZERO;
    
    // Single storage read per user
    for user in users.iter() {
        total += self.balances.get(*user);
    }
    
    total
}
```

### External Call Failures

#### Problem: External contract calls fail unexpectedly
Cross-contract calls in Stylus require careful handling.

**Solutions:**
```rust
sol_interface! {
    interface IExternalContract {
        function doSomething(uint256 value) external returns (bool);
    }
}
use stylus_sdk::call::Call;

#[public]
impl Contract {
    // ✅ External calls must pass a Call config
    pub fn call_external(&mut self, target: Address, value: U256) -> Result<(), Vec<u8>> {
        // Validate target
        if target == Address::ZERO {
            return Err(b"Invalid target address".to_vec());
        }
        
        // Create contract interface
        let external = IExternalContract::new(target);
        
        // Make call with proper Call config
        let ok = external.do_something(Call::new_in(self).gas(100_000), value)
            .map_err(|_| b"external revert".to_vec())?;
            
        if !ok { 
            return Err(b"external returned false".to_vec()); 
        }
        
        Ok(())
    }
}
```

## Performance Issues

### Slow Execution

#### Problem: Functions execute slower than expected
Storage operations in EVM are expensive compared to Solana.

**Solutions:**
```rust
// ❌ Inefficient: Multiple storage reads
pub fn calculate_rewards(&self, user: Address) -> U256 {
    let balance = self.balances.get(user);      // Read 1
    let staked = self.staked.get(user);         // Read 2
    let rate = self.rates.get(user);            // Read 3
    let last_update = self.updates.get(user);   // Read 4
    
    // Calculate rewards
    balance + staked * rate * (block::timestamp() - last_update)
}

// ✅ Efficient: Pack related data
sol_storage! {
    pub struct UserData {
        uint256 balance;
        uint256 staked;
        uint256 rate;
        uint256 last_update;
    }
    
    #[entrypoint]
    pub struct OptimizedContract {
        mapping(address => UserData) users;
    }
}

pub fn calculate_rewards(&self, user: Address) -> U256 {
    let data = self.users.get(user);  // Single read
    
    data.balance + data.staked * data.rate * 
        (U256::from(block::timestamp()) - data.last_update)
}
```

### High Gas Costs

#### Problem: Functions consume excessive gas
Every storage operation costs significant gas.

**Solutions:**
```rust
// ✅ Use events instead of storage for logs
sol! {
    event ActionPerformed(address indexed user, uint256 timestamp, bytes32 data);
}

#[public]
impl Contract {
    pub fn perform_action(&mut self, data: [u8; 32]) -> Result<(), Vec<u8>> {
        // Instead of storing action history
        // self.action_history.push(ActionRecord { ... });
        
        // Emit event (much cheaper)
        evm::log(ActionPerformed {
            user: msg::sender(),
            timestamp: U256::from(block::timestamp()),
            data: data.into(),
        });
        
        Ok(())
    }
}

// ✅ Batch operations to amortize base costs
pub fn batch_transfer(&mut self, transfers: Vec<(Address, U256)>) -> Result<(), Vec<u8>> {
    let sender = msg::sender();
    let mut total = U256::ZERO;
    
    // Validate total amount first
    for (_, amount) in &transfers {
        total += *amount;
    }
    
    let balance = self.balances.get(sender);
    if balance < total {
        return Err(b"Insufficient balance for batch".to_vec());
    }
    
    // Single storage write for sender
    self.balances.setter(sender).set(balance - total);
    
    // Process transfers
    for (recipient, amount) in transfers {
        let recipient_balance = self.balances.get(recipient);
        self.balances.setter(recipient).set(recipient_balance + amount);
    }
    
    Ok(())
}
```

## Debug Strategies

### Using Console Logs

```rust
use stylus_sdk::console;

#[public]
impl Contract {
    pub fn debug_transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        console!("=== Transfer Debug ===");
        console!("From: {}", msg::sender());
        console!("To: {}", to);
        console!("Amount: {}", amount);
        
        let balance = self.balances.get(msg::sender());
        console!("Sender balance: {}", balance);
        
        if balance < amount {
            console!("ERROR: Insufficient balance");
            console!("Required: {}, Available: {}", amount, balance);
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Perform transfer
        self.balances.setter(msg::sender()).set(balance - amount);
        let to_balance = self.balances.get(to);
        self.balances.setter(to).set(to_balance + amount);
        
        console!("Transfer successful!");
        console!("New sender balance: {}", balance - amount);
        console!("New recipient balance: {}", to_balance + amount);
        
        Ok(())
    }
}
```

### State Inspection Functions

```rust
// Add debug view functions
#[public]
impl Contract {
    // Aggregate debug information
    pub fn debug_info(&self) -> DebugInfo {
        DebugInfo {
            total_supply: self.total_supply.get(),
            owner: self.owner.get(),
            paused: self.paused.get(),
            user_count: self.get_user_count(),
            contract_balance: contract::balance(),
        }
    }
    
    // Validate invariants
    pub fn check_invariants(&self) -> Result<(), Vec<u8>> {
        // Check total supply equals sum of balances
        let calculated_supply = self.calculate_total_balances();
        let stored_supply = self.total_supply.get();
        
        if calculated_supply != stored_supply {
            return Err(format!(
                "Supply mismatch: calculated={}, stored={}",
                calculated_supply, stored_supply
            ).into_bytes());
        }
        
        Ok(())
    }
}
```

### Testing Edge Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stylus_sdk::testing::*;

    #[test]
    fn test_overflow_protection() {
        let vm = TestVM::new();
        let mut contract = Contract::from(&vm);
        
        // Test max value transfers
        let user1 = Address::from([1u8; 20]);
        let user2 = Address::from([2u8; 20]);
        
        // Set up max balance
        vm.set_sender(user1);
        contract.balances.setter(user1).set(U256::MAX);
        
        // Try to transfer max value
        let result = contract.transfer(user2, U256::MAX);
        assert!(result.is_ok());
        
        // Try to add to max value (should fail)
        contract.balances.setter(user2).set(U256::MAX);
        let result = contract.transfer(user2, U256::from(1));
        assert!(result.is_err());
    }
}
```

## Common Patterns Reference

### Solana to Stylus Pattern Mapping

| Solana Pattern | Stylus Equivalent |
|----------------|-------------------|
| `msg!()` | `console!()` in tests only; use events for on-chain traces |
| `invoke()` | interface call with `Call::new_in(self)` |
| `invoke_signed()` | No direct EVM equivalent. Use EIP-712/eth_sign/permit style signatures |
| Account validation | Access control modifiers |
| PDA derivation | Deterministic addresses |
| Rent exemption | No rent (gas for storage) |

### Error Message Debugging

```rust
// Enable detailed errors in development
#[cfg(debug_assertions)]
macro_rules! debug_require {
    ($cond:expr, $msg:literal) => {
        if !$cond {
            console!("Requirement failed: {}", $msg);
            console!("At: {}:{}", file!(), line!());
            return Err(format!("Debug assert failed: {}", $msg).into_bytes());
        }
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_require {
    ($cond:expr, $msg:literal) => {
        if !$cond {
            return Err(b"Requirement failed".to_vec());
        }
    };
}
```

## Getting Help

### Resources
- **Arbitrum Discord**: #stylus channel for real-time help
- **Stylus SDK Docs**: https://docs.rs/stylus-sdk
- **Examples repo**: OffchainLabs/stylus-sdk-rs/examples
- **Arbitrum Nitro devnode**: OffchainLabs/nitro-devnode (for local RPC)

### Additional Utility Patterns

```rust
// Deterministic "vault address" (namespacing)
use alloy_primitives::FixedBytes;
use stylus_sdk::{crypto::keccak, contract};

fn pseudo_vault(user: Address) -> Address {
    let salt = keccak(b"vault");
    let mut buf = [0u8; 1 + 20 + 20 + 32];
    buf[0] = 1;
    buf[1..21].copy_from_slice(contract::address().as_slice());
    buf[21..41].copy_from_slice(user.as_slice());
    buf[41..].copy_from_slice(FixedBytes::<32>::from(salt).as_slice());
    let h = keccak(&buf);
    Address::from_slice(&h[12..])
}

// Event test pattern with proper topic handling
use alloy_primitives::{B256, keccak256};

#[cfg(test)]
fn test_events() {
    let vm = TestVM::new();
    let mut contract = MyContract::from(&vm);
    
    contract.transfer(recipient, amount).unwrap();
    
    let logs = vm.get_emitted_logs();
    let sig = B256::from(keccak256("Transfer(address,address,uint256)".as_bytes()));
    assert_eq!(logs[0].0[0], sig); // .0 = topics
}
```

### Diagnostic Commands
```bash
# Check contract validity
cargo stylus check

# Export ABI
cargo stylus export-abi

# Estimate deployment gas
cargo stylus estimate-gas

# Deploy contract
cargo stylus deploy ...

# Verify contract
cargo stylus verify ...

# Get detailed error traces
RUST_LOG=debug cargo test

# Always estimate first, then deploy with a buffer:
cargo stylus estimate-gas --endpoint $RPC --private-key-path $KEY
# add ~20–30% headroom to --gas-limit on deploy
```

Remember: Most migration issues stem from fundamental differences between Solana's account model and EVM's contract model. Take time to understand these differences, and don't hesitate to reach out to the community for help.