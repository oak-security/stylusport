# Gas optimization

Gas optimization is crucial for Stylus contracts to remain cost-effective and competitive. This chapter covers strategies to reduce gas consumption while maintaining functionality, with specific focus on how gas usage differs from Solana's compute unit model.

## Understanding gas in Stylus

### Gas vs compute units

The fundamental difference between Solana and Ethereum fee models:

| Aspect | Solana | Stylus/Ethereum |
|--------|---------|-----------------|
| **Unit** | Compute Units (CU) | Gas |
| **Pricing** | Fixed: ~0.000005 SOL per signature | Variable: Gas price fluctuates with network demand |
| **Limits** | Per-transaction: 1.4M CU max | Per-block gas limit: ~30M gas |
| **Measurement** | Instruction-based | Operation-based |
| **State Access** | Account rent model | Per-operation gas cost |
| **Optimization Focus** | Reduce CU usage and account size | Reduce storage operations and external calls |

**Cost Comparison Examples:**
```rust
// Solana: Cost in compute units
// Simple transfer: ~300 CU
// Account creation: ~5,000 CU  
// Cross-program invocation: ~5,000-10,000 CU
// Total cost: ~0.000005 SOL (fixed)

// Stylus: Cost in gas (relative costs)
// Simple transfer: ~21,000 gas
// Storage write (new): ~20,000 gas
// Storage write (update): ~5,000 gas
// External call: ~2,600+ gas base
// Storage read: ~2,100 gas cold, ~100 gas warm
```

**Note**: On Arbitrum Stylus, absolute fees differ from L1 Ethereum and include L1 data costs. Use `cargo stylus estimate-gas` or Remote Procedure Call (RPC) `eth_estimateGas` for real figures. Gas numbers like "SLOAD ~2,100 cold / ~100 warm" describe relative Ethereum Virtual Machine (EVM) costs and can change with Ethereum Improvement Proposals (EIPs).

### Gas consumption patterns

Understanding what operations cost the most gas:

```rust
use stylus_sdk::prelude::*;
use stylus_sdk::call::{Call, static_call};
use alloy_primitives::{Address, U256, FixedBytes, B256};
use alloy_sol_types::sol;

sol_interface! {
    interface IExternalContract {
        function get_value() external view returns (uint256);
    }
}

#[storage]
#[entrypoint]
pub struct GasAnalysisContract {
    my_value: StorageU256,
    balances: StorageMap<Address, StorageU256>,
    dynamic_array: StorageVec<StorageU256>,
}

#[public]
impl GasAnalysisContract {
    // EXPENSIVE: First-time storage write (~20,000 gas)
    pub fn first_storage_write(&mut self, value: U256) {
        self.my_value.set(value);  // Cold storage slot write
    }
    
    // MODERATE: Update existing storage (~5,000 gas)
    pub fn update_storage(&mut self, value: U256) {
        self.my_value.set(value);  // Warm storage slot update
    }
    
    // CHEAP: Storage read (~100 gas if warm in same tx)
    pub fn storage_read(&self) -> U256 {
        self.my_value.get()  // Warm read after access
    }
    
    // EXPENSIVE: External contract call (2,600+ gas base + called function cost)
    pub fn external_call(&self, target: Address) -> Result<U256, Vec<u8>> {
        let contract = IExternalContract::new(target);
        let config = Call::new_in(self);
        contract.get_value(config)  // Cross-contract call
    }
    
    // VERY EXPENSIVE: Dynamic array operations (multiple storage slots)
    pub fn push_to_array(&mut self, value: U256) {
        // Push touches length and new element slot, and may warm again if accessed later.
        // It is among the most expensive storage patterns. Batch and pre-size when possible.
        self.dynamic_array.push(value);  // Updates length + new slot
    }
    
    // CHEAP: Memory operations (~3 gas per operation)
    pub fn memory_operations(&self, input: U256) -> U256 {
        let mut temp = input;
        temp += U256::from(100);  // Memory arithmetic
        temp *= U256::from(2);    // Memory arithmetic
        temp  // Returns computed value
    }
}
```

## Storage optimization

### Packing strategies

Unlike Solana where account size directly affects rent, in Stylus each storage slot (32 bytes) has a fixed cost. Pack data efficiently:

```rust
use stylus_sdk::storage::*;
use alloy_primitives::{U128, U32, U64};

// INEFFICIENT: Each field uses a full 32-byte slot
#[storage]
pub struct InefficientStorage {
    flag1: StorageBool,       // 32 bytes (wasteful!)
    small_num: StorageU8,     // 32 bytes (wasteful!)
    flag2: StorageBool,       // 32 bytes (wasteful!)
    medium_num: StorageU16,   // 32 bytes (wasteful!)
    user: StorageAddress,     // 32 bytes (okay - uses most of slot)
    balance: StorageU256,     // 32 bytes (okay - uses full slot)
}

// EFFICIENT: Pack multiple values into single slots (manual packing)
#[storage]
pub struct EfficientStorage {
    packed0: StorageU256,  // [flag1:1 | flag2:1 | small:8 | med1:16 | med2:16 | ts:64 | ...]
    user: StorageAddress,  // separate slot (20 bytes used)
    large_balance: StorageU256,
}

impl EfficientStorage {
    fn set_packed0(&mut self, flag1: bool, flag2: bool, small: u8, med1: u16, med2: u16, ts: u64) {
        let v = (U256::from(flag1 as u8) << 255)
              | (U256::from(flag2 as u8) << 254)
              | (U256::from(small)      << 246)
              | (U256::from(med1)       << 230)
              | (U256::from(med2)       << 214)
              | (U256::from(ts)         << 150);
        self.packed0.set(v);
    }
    // Add getters similarly
}

// Real-world example: User data packing
sol_storage! {
    pub struct UserData {
        uint128 balance;         // 16 bytes - enough for most use cases
        uint32 last_update;      // 4 bytes - timestamp
        uint32 level;            // 4 bytes - user level
        uint8 status;            // 1 byte - active/inactive/suspended
        bool verified;           // 1 byte
        uint48 referral_code;    // 6 bytes
        // Total: 32 bytes = 1 slot!
    }
}

#[storage]
#[entrypoint]
pub struct OptimizedUserContract {
    users: StorageMap<Address, UserData>,
}

#[public]
impl OptimizedUserContract {
    pub fn update_user_efficiently(&mut self, user: Address, balance: u128, level: u32) {
        // Single SSTORE operation updates all packed data
        let mut user_data = self.users.setter(user);
        user_data.balance.set(U128::from(balance));
        user_data.level.set(U32::from(level));
        user_data.last_update.set(U32::from(block::timestamp() as u32));
    }
}
```

### Access patterns

Optimize how you access storage to reduce gas costs:

```rust
sol_storage! {
    pub struct UserInfo {
        uint256 balance;
        uint256 multiplier;
        uint256 bonus;
    }
}

#[storage]
#[entrypoint]
pub struct StorageOptimizedContract {
    balances: StorageMap<Address, StorageU256>,
    multipliers: StorageMap<Address, StorageU256>,
    bonuses: StorageMap<Address, StorageU256>,
    user_info: StorageMap<Address, UserInfo>,  // Packed struct
    global_multiplier: StorageU256,
    base_bonus: StorageU256,
}

#[public]
impl StorageOptimizedContract {
    // INEFFICIENT: Multiple separate storage reads (each costs gas)
    pub fn inefficient_calculation(&self, user: Address) -> U256 {
        let balance = self.balances.get(user);         // SLOAD 1
        let multiplier = self.multipliers.get(user);   // SLOAD 2  
        let bonus = self.bonuses.get(user);           // SLOAD 3
        
        balance * multiplier + bonus
    }
    
    // EFFICIENT: Single storage read for packed data
    pub fn efficient_calculation(&self, user: Address) -> U256 {
        let user_data = self.user_info.get(user);     // Single SLOAD
        
        user_data.balance.get() * user_data.multiplier.get() + user_data.bonus.get()
    }
    
    // EFFICIENT: Cache repeated reads
    pub fn batch_process_users(&self, users: Vec<Address>) -> Vec<U256> {
        // Cache globals once (not per user)
        let global_mult = self.global_multiplier.get();
        let base_bonus = self.base_bonus.get();
        
        users.into_iter().map(|user| {
            let bal = self.balances.get(user);
            bal * global_mult + base_bonus
        }).collect()
    }
}
```

### Reducing storage operations

Reduce storage writes by batching updates:

```rust
use stylus_sdk::msg;

sol_storage! {
    pub struct AccountData {
        uint256 balance;
        uint256 rewards_earned;
        uint256 last_claim_time;
        uint256 total_staked;
    }
}

#[storage]
#[entrypoint]
pub struct BatchOptimizedContract {
    balances: StorageMap<Address, StorageU256>,
    accounts: StorageMap<Address, AccountData>,
}

#[public]
impl BatchOptimizedContract {
    // INEFFICIENT: Multiple storage writes
    pub fn inefficient_update(&mut self, user: Address) -> Result<(), Vec<u8>> {
        let mut account = self.accounts.setter(user);
        
        // Each line is a separate SSTORE!
        account.balance.set(account.balance.get() + U256::from(100));
        account.rewards_earned.set(account.rewards_earned.get() + U256::from(10));
        account.last_claim_time.set(U256::from(block::timestamp()));
        account.total_staked.set(account.total_staked.get() + U256::from(50));
        
        Ok(())
    }
    
    // EFFICIENT: Batch all updates
    pub fn efficient_update(&mut self, user: Address) -> Result<(), Vec<u8>> {
        // Read once
        let mut account = self.accounts.setter(user);
        let current_balance = account.balance.get();
        let current_rewards = account.rewards_earned.get();
        let current_staked = account.total_staked.get();
        
        // Compute all updates in memory
        let new_balance = current_balance + U256::from(100);
        let new_rewards = current_rewards + U256::from(10);
        let new_staked = current_staked + U256::from(50);
        
        // Write adjacent fields once each (fewer SLOAD/SSTOREs overall). 
        // For a true single SSTORE, pack multiple values into one uint256 and write that single slot.
        account.balance.set(new_balance);
        account.rewards_earned.set(new_rewards);
        account.last_claim_time.set(U256::from(block::timestamp()));
        account.total_staked.set(new_staked);
        
        Ok(())
    }
}
```

## Computation optimization

### Algorithm efficiency

Choose algorithms that reduce gas consumption:

```rust
use stylus_sdk::storage::StorageVec;

#[storage]
#[entrypoint]
pub struct AlgorithmOptimizedContract {
    items: StorageVec<StorageU256>,
    item_exists: StorageMap<U256, StorageBool>,
    item_index: StorageMap<U256, StorageU256>,
    sorted_items: StorageVec<StorageU256>,
}

#[public]
impl AlgorithmOptimizedContract {
    // INEFFICIENT: O(n) search through storage
    pub fn inefficient_contains(&self, target: U256) -> bool {
        let len = self.items.len();
        for i in 0..len {
            if self.items.get(i).unwrap().get() == target {
                return true;
            }
        }
        false
    }
    
    // EFFICIENT: O(1) lookup using mapping
    pub fn efficient_contains(&self, target: U256) -> bool {
        self.item_exists.get(target)
    }
    
    // EFFICIENT: Use indices for direct access
    pub fn get_item_by_value(&self, target: U256) -> Option<U256> {
        if self.item_exists.get(target) {
            let index = self.item_index.get(target);
            Some(self.items.get(index).unwrap().get())
        } else {
            None
        }
    }
    
    // EFFICIENT: Binary search on sorted data
    pub fn binary_search(&self, target: U256) -> bool {
        let mut left = 0usize;
        let mut right = self.sorted_items.len();
        
        while left < right {
            let mid = left + ((right - left) / 2);
            let mid_value = self.sorted_items.get(mid).unwrap().get();
            
            if mid_value == target {
                return true;
            } else if mid_value < target {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        false
    }
}
```

### Memory management

Optimize memory usage to reduce gas costs:

```rust
use stylus_sdk::abi::Bytes;
use alloc::vec::Vec;
use alloc::string::String;

#[public]
impl MemoryOptimizedContract {
    // INEFFICIENT: Multiple allocations and copies
    pub fn inefficient_string_concat(&self, parts: Vec<String>) -> String {
        let mut result = String::new();
        for part in parts {
            result.push_str(&part);  // Reallocation on each push
        }
        result
    }
    
    // EFFICIENT: Pre-calculate size and allocate once
    pub fn efficient_string_concat(&self, parts: Vec<String>) -> String {
        let total_len: usize = parts.iter().map(|s| s.len()).sum();
        let mut result = String::with_capacity(total_len);
        
        for part in parts {
            result.push_str(&part);  // No reallocation needed
        }
        result
    }
    
    // EFFICIENT: Use fixed-size arrays when possible
    pub fn process_fixed_data(&self) -> U256 {
        // Stack allocation is cheaper than heap
        let values: [U256; 10] = [U256::from(1); 10];
        let mut sum = U256::ZERO;
        
        for value in values.iter() {
            sum += value;
        }
        sum
    }
    
    // EFFICIENT: Reuse buffers
    pub fn process_chunks(&self, data: Bytes) -> Bytes {
        let chunk_size = 32;
        let mut result = Vec::with_capacity(data.len());
        let mut temp_buffer = [0u8; 32];  // Reusable buffer
        
        for chunk in data.chunks(chunk_size) {
            // Process into reusable buffer
            temp_buffer[..chunk.len()].copy_from_slice(chunk);
            self.process_buffer(&mut temp_buffer);
            result.extend_from_slice(&temp_buffer[..chunk.len()]);
            // Note: Zero the reused buffer slice if downstream consumers expect zeroed tails.
            // Otherwise tail bytes may contain previous data but are truncated on extend_from_slice.
        }
        
        result.into()
    }
}
```

### Avoiding redundant calculations

Cache expensive computations:

```rust
use stylus_sdk::crypto::keccak;

#[storage]
#[entrypoint]
pub struct CacheOptimizedContract {
    computation_cache: StorageMap<U256, StorageU256>,
    hash_cache: StorageMap<FixedBytes<32>, StorageU256>,  // key = B256
    last_update: StorageU256,
    cached_global: StorageU256,
}

#[public]
impl CacheOptimizedContract {
    // Cache expensive calculations
    pub fn get_computed_value(&mut self, input: U256) -> U256 {
        // Check cache first
        let cached = self.computation_cache.get(input);
        if cached != U256::ZERO {
            return cached;
        }
        
        // Expensive computation only if not cached
        let result = self.expensive_computation(input);
        self.computation_cache.setter(input).set(result);
        
        result
    }
    
    // Cache with expiration
    pub fn get_time_sensitive_value(&mut self) -> U256 {
        let current_time = U256::from(block::timestamp());
        let last_update = self.last_update.get();
        
        // Cache valid for 1 hour
        if current_time - last_update < U256::from(3600) {
            return self.cached_global.get();
        }
        
        // Recompute and cache
        let new_value = self.compute_global_value();
        self.cached_global.set(new_value);
        self.last_update.set(current_time);
        
        new_value
    }
    
    // Memoize hash computations
    pub fn get_hash_result(&mut self, data: Bytes) -> U256 {
        let hash: B256 = stylus_sdk::crypto::keccak(&data).into();
        
        let cached = self.hash_cache.get(hash.into());
        if cached != U256::ZERO {
            return cached;
        }
        
        let result = self.process_hashed_data(hash);
        self.hash_cache.setter(hash.into()).set(result);
        
        result
    }
}
```

## Call optimization

### Batching operations

Reduce transaction overhead by batching:

```rust
#[storage]
#[entrypoint]
pub struct BatchContract {
    balances: StorageMap<Address, StorageU256>,
    total_supply: StorageU256,
}

#[public]
impl BatchContract {
    // INEFFICIENT: Separate transaction for each transfer
    pub fn single_transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        let sender = msg::sender();
        let sender_balance = self.balances.get(sender);
        
        if sender_balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        self.balances.setter(sender).set(sender_balance - amount);
        let recipient_balance = self.balances.get(to);
        self.balances.setter(to).set(recipient_balance + amount);
        
        Ok(())
    }
    
    // EFFICIENT: Batch multiple transfers in one transaction
    pub fn batch_transfer(&mut self, recipients: Vec<Address>, amounts: Vec<U256>) -> Result<(), Vec<u8>> {
        if recipients.len() != amounts.len() {
            return Err(b"Mismatched arrays".to_vec());
        }
        
        let sender = msg::sender();
        let mut sender_balance = self.balances.get(sender);
        let total_amount = amounts.iter().fold(U256::ZERO, |acc, x| acc + *x);
        
        // Check total balance once
        if sender_balance < total_amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update sender once
        sender_balance -= total_amount;
        self.balances.setter(sender).set(sender_balance);
        
        // Batch recipient updates
        for (recipient, amount) in recipients.iter().zip(amounts.iter()) {
            let current = self.balances.get(*recipient);
            self.balances.setter(*recipient).set(current + amount);
        }
        
        Ok(())
    }
}
```

### Multicall patterns

Create flexible multicall functionality:

```rust
use stylus_sdk::abi::{AbiEncode, AbiDecode};
use stylus_sdk::contract;

#[derive(AbiEncode, AbiDecode)]
pub struct CallData {
    pub target: Address,
    pub value: U256,
    pub data: Bytes,
}

#[storage]
#[entrypoint]
pub struct MulticallContract {
    authorized: StorageMap<Address, StorageBool>,
}

#[public]
impl MulticallContract {
    // Execute multiple calls in one transaction
    pub fn multicall(&mut self, calls: Vec<CallData>) -> Result<Vec<Bytes>, Vec<u8>> {
        let mut results = Vec::with_capacity(calls.len());
        
        for call in calls {
            // Security: Only allow calls to this contract.
            // Limiting target to contract::address() prevents arbitrary external calls
            // and keeps the aggregate atomic without expanding the attack surface.
            // If you need external targets, add allowlists and per-call gas caps.
            if call.target != contract::address() {
                return Err(b"External calls not allowed".to_vec());
            }
            
            // Execute internal call
            match self.execute_call(call.data) {
                Ok(result) => results.push(result),
                Err(e) => return Err(e),
            }
        }
        
        Ok(results)
    }
    
    // Aggregate multiple view calls
    pub fn aggregate_static(&self, calls: Vec<CallData>) -> Result<Vec<Bytes>, Vec<u8>> {
        let mut results = Vec::with_capacity(calls.len());
        
        for call in calls {
            // Execute static call
            match static_call(
                Call::new(),
                call.target,
                &call.data
            ) {
                Ok(result) => results.push(result.into()),
                Err(_) => results.push(Bytes::new()), // Non-reverting aggregator
            }
        }
        
        Ok(results)
    }
}
```

### Transaction bundling

Design for efficient multi-step operations:

```rust
#[derive(AbiEncode, AbiDecode)]
pub struct SwapParams {
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub min_amount_out: U256,
}

#[storage]
#[entrypoint]
pub struct EfficientDeFiContract {
    balances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    staked: StorageMap<Address, StorageU256>,
    rewards: StorageMap<Address, StorageU256>,
}

#[public]
impl EfficientDeFiContract {
    // Bundle swap + stake in one transaction
    pub fn swap_and_stake(&mut self, swap: SwapParams, stake_duration: U256) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        
        // 1. Execute swap (simplified)
        let amount_out = self.execute_swap(user, swap)?;
        require!(amount_out >= swap.min_amount_out, "Slippage exceeded");
        
        // 2. Immediately stake the output tokens
        self.stake_internal(user, swap.token_out, amount_out, stake_duration)?;
        
        // 3. Calculate and store initial rewards
        let reward = self.calculate_stake_reward(amount_out, stake_duration);
        let current_rewards = self.rewards.get(user);
        self.rewards.setter(user).set(current_rewards + reward);
        
        // Single transaction for entire flow!
        Ok(())
    }
    
    // Compound operations
    pub fn claim_and_restake(&mut self) -> Result<U256, Vec<u8>> {
        let user = msg::sender();
        
        // Get current rewards
        let rewards = self.rewards.get(user);
        if rewards == U256::ZERO {
            return Err(b"No rewards to claim".to_vec());
        }
        
        // Reset rewards
        self.rewards.setter(user).set(U256::ZERO);
        
        // Add rewards to stake in same transaction
        let current_stake = self.staked.get(user);
        self.staked.setter(user).set(current_stake + rewards);
        
        Ok(rewards)
    }
}
```

## Measuring and profiling

### Gas usage analysis

Track gas consumption in tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stylus_sdk::testing::*;
    
    #[test]
    fn test_gas_optimization() {
        let vm = TestVM::new();
        let mut contract = GasAnalysisContract::from(&vm);
        
        // Use event-based gas profiling instead of vm.gas_left()
        // which may not be stable across VM implementations
        contract.efficient_operation(U256::from(100));
        
        let logs = vm.get_emitted_logs();
        // Assert on logs for gas profiling events
        
        // For comparison testing, instrument both versions with events
        contract.inefficient_operation(U256::from(100));
        let logs2 = vm.get_emitted_logs();
        // Compare gas usage from logged events
        
        assert!(gas_used < gas_used_2, "Optimized version should use less gas");
    }
}
```

### Profiling tools

Use specialized tools for gas analysis:

```rust
use stylus_sdk::evm;

pub fn profile_gas<F: FnOnce() -> R, R>(label: &str, f: F) -> R {
    let start = evm::gas_left();
    let result = f();
    let used = start - evm::gas_left();
    #[cfg(debug_assertions)]
    stylus_sdk::console!("{} used {} gas", label, used);
    result
}

// Note: TestVM does not guarantee a stable vm.gas_left() API.
// Prefer instrumenting the contract by emitting a GasProfiled event
// and asserting on logs in tests.

sol! {
    event GasProfiled(string label, uint256 used);
}

#[public]
impl GasAnalysisContract {
    pub fn efficient_operation(&mut self, x: U256) {
        let start = evm::gas_left();
        // Perform work
        evm::log(GasProfiled {
            label: "efficient".into(),
            used: start - evm::gas_left()
        });
    }
}

#[public]
impl ProfiledContract {
    pub fn complex_operation(&mut self, input: U256) -> U256 {
        let step1 = profile_gas("Step 1: Load data", || {
            self.load_user_data(msg::sender())
        });
        
        let step2 = profile_gas("Step 2: Calculate", || {
            self.perform_calculations(step1, input)
        });
        
        profile_gas("Step 3: Store result", || {
            self.store_result(msg::sender(), step2);
            step2
        })
    }
}
```

## Migration-specific optimizations

### Leveraging EVM features

Take advantage of EVM-specific optimizations:

```rust
#[storage]
#[entrypoint]
pub struct EVMOptimizedContract {
    data: StorageMap<U256, StorageU256>,
    cache: StorageMap<U256, StorageU256>,
    cache_version: StorageU256,
}

#[public]
impl EVMOptimizedContract {
    // Use CREATE2 for deterministic addresses (not available in Solana)
    pub fn deploy_child_contract(&mut self, salt: FixedBytes<32>, init_code: Bytes) -> Address {
        // Note: Stylus SDK may not expose create2 directly. Deploy via factory or low-level raw call.
        // Use a factory contract that exposes CREATE2, or
        // use a Stylus low-level helper if/when available.
        let deployed = self.deploy_via_factory(salt, init_code);
        
        // Can interact with it immediately
        self.register_child(deployed);
        deployed
    }
    
    // Leverage warm storage access in same transaction
    pub fn batch_operations(&mut self, keys: Vec<U256>) -> Vec<U256> {
        // First pass: warm up the storage slots
        for &key in &keys {
            let _ = self.data.get(key); // Warm the slot
        }
        
        // Second pass: operations are now cheaper
        keys.iter().map(|&key| {
            let value = self.data.get(key); // Warm access ~100 gas
            self.process_value(value)
        }).collect()
    }
    
    // Use EVM's automatic storage refunds
    pub fn cleanup_old_data(&mut self, keys: Vec<U256>) -> U256 {
        let mut refund_count = U256::ZERO;
        
        for key in keys {
            let value = self.data.get(key);
            if value != U256::ZERO {
                // Setting to zero triggers gas refund
                self.data.setter(key).set(U256::ZERO);
                refund_count += U256::ONE;
            }
        }
        
        // Refunds are capped at 20% of gas used (EIP-3529). Many refunds were reduced.
        refund_count
    }
}
```

### State rent vs gas trade-offs

Consider the different cost models:

```rust
#[public]
impl MigrationOptimizedContract {
    // In Solana: Would need separate accounts for each user
    // In Stylus: Can use efficient mappings
    pub fn efficient_user_storage(&mut self, users: Vec<Address>, values: Vec<U256>) -> Result<(), Vec<u8>> {
        // Single transaction to update multiple users
        for (user, value) in users.iter().zip(values.iter()) {
            self.user_data.setter(*user).set(*value);
        }
        Ok(())
    }
    
    // Compute vs Storage trade-off
    pub fn get_derived_value(&mut self, base: U256, multiplier: U256) -> U256 {
        // In Solana: Might store this to avoid recomputation
        // In Stylus: Often cheaper to recompute than store
        
        let result = base * multiplier / U256::from(100);
        
        // Only cache if computation is very expensive
        if multiplier > U256::from(1000) {
            self.cache.setter(base).set(result);
        }
        
        result
    }
}
```

## Reentrancy guard pattern

For external calls and ETH transfers, add Reentrancy protection:

```rust
#[storage]
pub struct ReentGuard {
    locked: StorageBool,
}

impl ReentGuard {
    fn enter(&mut self) -> Result<(), Vec<u8>> {
        if self.locked.get() {
            return Err(b"Reentrancy".to_vec());
        }
        self.locked.set(true);
        Ok(())
    }
    
    fn exit(&mut self) {
        self.locked.set(false);
    }
}
```

## Best practices summary

1. **Pack Storage Efficiently**: Group related small values into single storage slots
2. **Reduce Storage Operations**: Cache values in memory, batch updates
3. **Optimize Algorithms**: Use O(1) lookups instead of O(n) searches
4. **Batch Transactions**: Combine several operations into single transactions
5. **Profile and Measure**: Always test gas usage of optimizations
6. **Consider Trade-offs**: Balance computation vs storage costs

The biggest gas optimization wins come from:
1. Fewer SSTORE/SLOADs via packing and caching.
2. Batch operations and avoiding per-item external calls.
3. Checks-Effects-Interactions pattern with Reentrancy guards.
4. Measuring everything through estimate-gas and log-based profiling.

## Next steps

Gas optimization requires ongoing refinement. Use the techniques in this chapter as a starting point, but always measure the actual gas usage of your contracts. The next chapter covers security considerations specific to Stylus contracts, ensuring your optimized contracts remain secure.