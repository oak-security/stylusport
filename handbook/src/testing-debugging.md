# Testing and Debugging

## Introduction

Testing in Stylus requires a different approach than Solana development. While Solana uses `solana-program-test` for integration testing and relies on custom test frameworks, Stylus provides a powerful `TestVM` that creates a fully-mocked host environment entirely in Rust—no actual blockchain needed.

The key differences:
- **Solana**: Tests use `ProgramTest` to simulate the Solana runtime, with accounts and programs loaded into a test environment
- **Stylus**: Tests run entirely in Rust with `TestVM`, providing instant feedback and complete control over the test environment

This chapter will guide you through migrating your Solana test patterns to effective Stylus testing strategies.

## Unit Testing with TestVM

### Basic Test Setup

The `TestVM` provides a controlled environment for testing your Stylus contracts without deploying to any network:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stylus_sdk::testing::*;
    use alloy_primitives::{Address, U256};

    #[test]
    fn test_basic_functionality() {
        // Create a new test VM
        let vm = TestVM::default();
        
        // Create contract instance connected to the VM
        let mut contract = Counter::from(&vm);
        
        // Test initial state
        assert_eq!(contract.number(), U256::ZERO);
        
        // Call methods and assert results
        contract.increment();
        assert_eq!(contract.number(), U256::ONE);
    }
}
```

**Migration from Solana:**
```rust
// Solana test pattern
#[tokio::test]
async fn test_program() {
    let mut test = ProgramTest::new("my_program", id(), processor!(process_instruction));
    let (mut banks_client, payer, recent_blockhash) = test.start().await;
    // ... setup accounts and call instruction
}

// Stylus equivalent - no async, no blockchain simulation needed
#[test]
fn test_contract() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    // ... call contract methods directly
}
```

### State Testing

Testing storage reads and writes in Stylus:

```rust
#[test]
fn test_storage_operations() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    
    // Test initial state
    assert_eq!(contract.get_value(), U256::ZERO);
    
    // Test state change
    contract.set_value(U256::from(42));
    assert_eq!(contract.get_value(), U256::from(42));
    
    // Test complex state transitions
    contract.increment();
    assert_eq!(contract.get_value(), U256::from(43));
}
```

**Migration Pattern:**
```rust
// Solana: Testing account data changes
let account = banks_client.get_account(account_pubkey).await?;
let account_data = MyAccountData::try_from_slice(&account.data)?;
assert_eq!(account_data.value, expected_value);

// Stylus: Direct storage access
let stored_value = contract.get_stored_value();
assert_eq!(stored_value, expected_value);
```

### Mocking Context and External Calls

The `TestVM` allows you to mock blockchain context and external calls:

```rust
#[test]
fn test_with_mocked_context() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    
    // Mock msg.sender
    let sender = Address::from([1u8; 20]);
    vm.set_sender(sender);
    
    // Mock msg.value
    vm.set_value(U256::from(1_000_000));
    
    // Mock block.timestamp
    vm.set_block_timestamp(1234567890);
    
    // Call payable function
    contract.deposit();
    assert_eq!(contract.get_balance(sender), U256::from(1_000_000));
}

#[test]
fn test_external_call_handling() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    
    // Mock successful external call
    let external_addr = Address::from([5u8; 20]);
    let return_data = vec![1, 2, 3, 4];
    vm.mock_call(external_addr, vec![], Ok(return_data.clone()));
    
    // Test that our contract handles the call correctly
    let result = contract.call_external(external_addr, vec![]);
    assert_eq!(result, Ok(return_data));
    
    // Mock failed external call
    vm.mock_call(external_addr, vec![], Err(vec![9, 9, 9]));
    let result = contract.call_external(external_addr, vec![]);
    assert!(result.is_err());
}
```

### Event Testing

Capturing and verifying emitted events using `get_emitted_logs()`:

```rust
#[test]
fn test_event_emission() {
    let vm = TestVM::default();
    let mut contract = Counter::from(&vm);
    
    // Trigger events
    contract.increment();
    
    // Get emitted logs
    let logs = vm.get_emitted_logs();
    assert_eq!(logs.len(), 2); // increment() emits two events in our example
    
    // Verify event signature (topic[0])
    let event_sig = B256::from(keccak256(
        "CounterUpdated(address,uint256,uint256)".as_bytes()
    ));
    assert_eq!(logs[0].0[0], event_sig);
    
    // Verify indexed parameters
    let mut sender_bytes = [0u8; 20];
    sender_bytes.copy_from_slice(&logs[0].0[1].into()[12..]);
    assert_eq!(Address::from(sender_bytes), vm.msg_sender());
}
```

## Integration Testing

### Multi-Contract Testing

Testing interactions between multiple contracts:

```rust
#[test]
fn test_contract_interactions() {
    let vm = TestVM::default();
    
    // Deploy multiple contracts
    let mut factory = Factory::from(&vm);
    let mut token = Token::from(&vm);
    
    // Initialize token with factory as minter
    let factory_addr = Address::from([1u8; 20]); // Mock address
    token.initialize("Test Token", "TT", factory_addr);
    
    // Mock the factory's call to mint tokens
    vm.mock_call(
        factory_addr,
        vec![],  // Calldata
        Ok(vec![]) // Success
    );
    
    // Test cross-contract interaction
    let result = factory.create_and_mint(token_addr, recipient, amount);
    assert!(result.is_ok());
}
```

### Raw Storage Testing

Direct storage slot manipulation for advanced testing:

```rust
#[test]
fn test_storage_layout() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    
    // Set value through contract method
    contract.set_number(U256::from(42));
    
    // Read raw storage slot
    let slot = U256::ZERO; // First storage slot
    let raw_value = vm.storage_load_bytes32(slot);
    assert_eq!(
        raw_value,
        B256::from_slice(&U256::from(42).to_be_bytes::<32>())
    );
    
    // Directly manipulate storage
    let new_value = U256::from(100);
    unsafe { 
        vm.storage_cache_bytes32(
            slot, 
            B256::from_slice(&new_value.to_be_bytes::<32>())
        );
    }
    vm.flush_cache(false);
    
    // Verify through contract method
    assert_eq!(contract.number(), new_value);
}
```

## Debugging Techniques

### Using console! macro

Debug output in tests (requires `debug` feature):

```rust
// In Cargo.toml
[features]
debug = ["stylus-sdk/debug"]

// In your contract
use stylus_sdk::console;

#[public]
impl MyContract {
    pub fn debug_method(&mut self, value: U256) {
        console!("Input value: {}", value);
        
        let result = self.complex_calculation(value);
        console!("Calculated result: {}", result);
        
        if result > U256::from(1000) {
            console!("Large result detected");
        }
    }
}

#[test]
fn test_with_debugging() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    
    // Debug output will appear in test output when run with debug feature
    contract.debug_method(U256::from(500));
}
```

### Common Migration Issues and Solutions

**Access Control Testing:**
```rust
// Solana: Testing signer validation
let ix = Instruction {
    accounts: vec![AccountMeta::new(account, true)], // is_signer: true
    // ...
};

// Stylus equivalent
#[test]
fn test_access_control() {
    let vm = TestVM::default();
    let mut contract = MyContract::from(&vm);
    
    // Set up owner
    let owner = vm.msg_sender();
    contract.transfer_ownership(owner).unwrap();
    
    // Test non-owner access
    let non_owner = Address::from([2u8; 20]);
    vm.set_sender(non_owner);
    
    let result = contract.owner_only_function();
    assert!(result.is_err());
    
    // Test owner access
    vm.set_sender(owner);
    let result = contract.owner_only_function();
    assert!(result.is_ok());
}
```

**PDA Migration Testing:**
```rust
// Test PDA-equivalent functionality
#[test]
fn test_deterministic_addresses() {
    let vm = TestVM::default();
    let mut contract = MigratedContract::from(&vm);
    
    // In Solana, we had PDAs
    // In Stylus, we use mappings or compute addresses differently
    let user = Address::from([1u8; 20]);
    let nonce = U256::from(1);
    
    // Test that our migration maintains deterministic behavior
    let account1 = contract.get_user_account(user, nonce);
    let account2 = contract.get_user_account(user, nonce);
    assert_eq!(account1, account2); // Same inputs = same output
}
```

## Gas and Performance Testing

### Measuring Gas Consumption

```rust
#[test]
fn test_gas_consumption() {
    let vm = TestVM::default();
    let mut contract = OptimizedContract::from(&vm);
    
    // Complex operation that we want to optimize
    contract.complex_operation(U256::from(100));
    
    // Note: Gas measurement in TestVM is simulated
    // For accurate gas measurements, deploy to testnet
    
    // Compare implementations
    let mut baseline = BaselineContract::from(&vm);
    baseline.complex_operation(U256::from(100));
    
    // Use local testnet for accurate gas comparisons
}
```

### Block Context Testing

Testing time-dependent logic:

```rust
#[test]
fn test_time_dependent_logic() {
    let vm = TestVM::default();
    let mut contract = TimeLock::from(&vm);
    
    // Set initial timestamp
    vm.set_block_timestamp(1000);
    contract.create_timelock(U256::from(100), 3600); // 1 hour lock
    
    // Try to withdraw immediately (should fail)
    let result = contract.withdraw(0);
    assert!(result.is_err());
    
    // Advance time
    vm.set_block_timestamp(5000); // Past the 1 hour lock
    
    // Now withdrawal should succeed
    let result = contract.withdraw(0);
    assert!(result.is_ok());
}
```

## Advanced Testing Patterns

### Custom Test VM Extensions

For specialized testing needs:

```rust
#[cfg(test)]
mod custom_vm {
    use super::*;
    use stylus_sdk::testing::TestVM;

    pub struct CustomVM {
        inner: TestVM,
        pub mock_count: usize,
    }

    impl CustomVM {
        pub fn new() -> Self { 
            Self { 
                inner: TestVM::default(), 
                mock_count: 0 
            } 
        }
        
        pub fn mock_call(&mut self, target: Address, data: Vec<u8>, result: Result<Vec<u8>, Vec<u8>>) {
            self.mock_count += 1;
            self.inner.mock_call(target, data, result);
        }
        
        pub fn inner(&self) -> &TestVM { &self.inner }
    }

    #[test]
    fn test_with_custom_vm() {
        let mut vm = CustomVM::new();
        let contract = MyContract::from(vm.inner());
        
        // Use custom tracking
        vm.mock_call(addr, vec![], Ok(vec![]));
        assert_eq!(vm.mock_count, 1);
    }
}
```

### Testing Migration Equivalence

Ensuring your migrated contract behaves identically to the original:

```rust
#[test]
fn test_migration_equivalence() {
    let vm = TestVM::default();
    let mut contract = MigratedDraffle::from(&vm);
    
    // Test cases based on original Solana behavior
    // These should match the test cases from your Solana program
    
    // Test ticket purchase
    vm.set_value(U256::from(1_000_000)); // 1 USDC equivalent
    contract.buy_tickets(1);
    assert_eq!(contract.get_user_tickets(vm.msg_sender()), 1);
    
    // Test randomness seed generation
    vm.set_block_timestamp(123456);
    vm.set_block_number(789);
    let seed = contract.generate_seed();
    
    // Verify seed generation matches expected pattern
    // (accounting for differences in randomness sources)
    assert_ne!(seed, B256::ZERO);
}
```

## Best Practices

### Test Organization

Structure your tests for clarity and maintainability:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    mod initialization {
        use super::*;
        
        #[test]
        fn test_default_values() { /* ... */ }
        
        #[test]
        fn test_constructor_params() { /* ... */ }
    }
    
    mod access_control {
        use super::*;
        
        #[test]
        fn test_owner_functions() { /* ... */ }
        
        #[test]
        fn test_unauthorized_access() { /* ... */ }
    }
    
    mod edge_cases {
        use super::*;
        
        #[test]
        fn test_zero_amounts() { /* ... */ }
        
        #[test]
        fn test_overflow_protection() { /* ... */ }
    }
}
```

### Migration Testing Checklist

When migrating from Solana to Stylus, ensure your tests cover:

- [ ] All instruction handlers → public methods
- [ ] Account validation → access control
- [ ] PDA derivation → deterministic computations
- [ ] Token operations → ERC-20 interactions
- [ ] Error conditions → custom errors
- [ ] Event emissions → log verification
- [ ] Cross-program invocations → external calls
- [ ] Time-based logic → block timestamp handling

## Next Steps

With comprehensive testing in place, you're ready to move on to deployment and verification. The next chapter covers local deployment with Anvil, testnet deployment to Arbitrum Sepolia, and the contract verification process that ensures your migrated contracts are ready for production.

Remember: Stylus's `TestVM` makes testing fast and deterministic. Unlike Solana's async test environment, you get instant feedback and complete control over the blockchain state, making it easier to write thorough tests for your migrated contracts.