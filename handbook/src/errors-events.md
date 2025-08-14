# Chapter 8: Errors and Events

Proper error handling and event emission provide crucial functionality for robust smart contracts and user experience. This chapter covers migrating from Solana's `msg!()` logging and `ProgramError` returns to Stylus's structured events and custom error types.

## Error and Event Model Comparison

### Solana Error/Event Model
- **Errors**: Return `ProgramError` variants or custom errors
- **Logging**: Use `msg!()` macro for simple string messages
- **Program Logs**: All outputs captured in transaction logs
- **Structured Data**: Limited - primarily string messages

### Stylus Error/Event Model  
- **Errors**: Return `Result<T, CustomError>` with structured error types
- **Events**: Emit typed events with indexed parameters using `evm::log()`
- **ABI Integration**: Events become part of contract ABI for frontend integration
- **Rich Data**: Full support for complex event data structures

## Basic Error Handling Migration

### From Solana Errors

**Solana Native:**
```rust
use solana_program::*;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum CustomError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Invalid authority")]
    InvalidAuthority,
    #[error("Account not initialized")]
    NotInitialized,
}

impl From<CustomError> for ProgramError {
    fn from(e: CustomError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

fn process_transfer(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let user_account = &accounts[0];
    let authority = &accounts[1];
    
    if !authority.is_signer {
        msg!("Authority must sign the transaction");
        return Err(CustomError::InvalidAuthority.into());
    }
    
    let user_data = UserAccount::try_from_slice(&user_account.data.borrow())?;
    
    if user_data.balance < amount {
        msg!("Insufficient funds: {} < {}", user_data.balance, amount);
        return Err(CustomError::InsufficientFunds.into());
    }
    
    // Process transfer
    msg!("Transfer successful: {} tokens", amount);
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Insufficient funds for this operation")]
    InsufficientFunds,
    #[msg("Invalid authority for this operation")]
    InvalidAuthority,
    #[msg("Account has not been initialized")]
    NotInitialized,
}

#[program]
pub mod my_program {
    pub fn process_transfer(ctx: Context<TransferAccounts>, amount: u64) -> Result<()> {
        let user_account = &ctx.accounts.user_account;
        
        if user_account.balance < amount {
            msg!("Insufficient funds: {} < {}", user_account.balance, amount);
            return Err(error!(CustomError::InsufficientFunds));
        }
        
        // Process transfer
        user_account.balance -= amount;
        
        msg!("Transfer successful: {} tokens", amount);
        Ok(())
    }
}
```

### To Stylus Structured Errors

**Stylus:**
```rust
use stylus_sdk::prelude::*;
use stylus_sdk::{evm, msg, block};
use alloy_primitives::{Address, U256};

// Define custom error types with the sol! macro
sol! {
    error InsufficientFunds(uint256 requested, uint256 available);
    error InvalidAuthority(address provided, address expected);
    error NotInitialized();
    error TransferFailed(address from, address to, uint256 amount);
    error InvalidAmount(uint256 amount);
    error Unauthorized(address caller);
}

// Define the SolidityError enum that derives from the errors above
#[derive(SolidityError)]
pub enum CustomError {
    InsufficientFunds(InsufficientFunds),
    InvalidAuthority(InvalidAuthority),
    NotInitialized(NotInitialized),
    TransferFailed(TransferFailed),
    InvalidAmount(InvalidAmount),
    Unauthorized(Unauthorized),
}

sol! {
    event TransferCompleted(address indexed from, address indexed to, uint256 amount);
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    event FunctionCalled(address indexed caller, string function_name, uint256 timestamp);
}

#[storage]
#[entrypoint]
pub struct TokenContract {
    balances: StorageMap<Address, StorageU256>,
    owner: StorageAddress,
}

#[public]
impl TokenContract {
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), CustomError> {
        let sender = msg::sender();
        let sender_balance = self.balances.get(sender);
        
        // Structured error with data
        if sender_balance < amount {
            return Err(CustomError::InsufficientFunds(InsufficientFunds {
                requested: amount,
                available: sender_balance,
            }));
        }
        
        if to == Address::ZERO {
            return Err(CustomError::TransferFailed(TransferFailed {
                from: sender,
                to,
                amount,
            }));
        }
        
        // Update balances
        self.balances.setter(sender).set(sender_balance - amount);
        let to_balance = self.balances.get(to);
        self.balances.setter(to).set(to_balance + amount);
        
        evm::log(TransferCompleted {
            from: sender,
            to,
            amount,
        });
        
        Ok(())
    }
    
    pub fn restricted_function(&mut self) -> Result<(), CustomError> {
        let sender = msg::sender();
        let expected_owner = self.owner.get();
        
        if sender != expected_owner {
            return Err(CustomError::InvalidAuthority(InvalidAuthority {
                provided: sender,
                expected: expected_owner,
            }));
        }
        
        // Function implementation
        Ok(())
    }
}
```

## Event Emission Patterns

### From Simple Logging

**Solana `msg!()` Calls:**
```rust
msg!("User {} deposited {} tokens", user_key, amount);
msg!("Withdrawal processed for account {}", account_key);
msg!("Price updated from {} to {}", old_price, new_price);
```

**Stylus Structured Events:**
```rust
// Define events with typed parameters
sol! {
    event Deposit(address indexed user, uint256 amount, uint256 timestamp);
    event Withdrawal(address indexed account, uint256 amount, uint256 new_balance);
    event PriceUpdated(uint256 old_price, uint256 new_price, address indexed updater);
}

#[public]
impl MyContract {
    pub fn deposit(&mut self, amount: U256) -> Result<(), CustomError> {
        let user = msg::sender();
        if amount == U256::ZERO {
            return Err(CustomError::InvalidAmount(InvalidAmount { amount }));
        }
        
        // ... deposit logic
        
        // Emit structured event
        evm::log(Deposit {
            user,
            amount,
            timestamp: U256::from(block::timestamp()),
        });
        
        Ok(())
    }
    
    pub fn withdraw(&mut self, amount: U256) -> Result<(), CustomError> {
        let account = msg::sender();
        
        // ... withdrawal logic
        let new_balance = self.balances.get(account);
        
        // Emit event with computed data
        evm::log(Withdrawal {
            account,
            amount,
            new_balance,
        });
        
        Ok(())
    }
    
    pub fn update_price(&mut self, new_price: U256) -> Result<(), CustomError> {
        let old_price = self.price.get();
        self.price.set(new_price);
        
        // Event with indexed parameter for efficient filtering
        evm::log(PriceUpdated {
            old_price,
            new_price,
            updater: msg::sender(),
        });
        
        Ok(())
    }
}
```

### Advanced Event Patterns

**Complex Event Data:**
```rust
use alloc::vec::Vec;
use alloc::string::{String, ToString};

sol! {
    struct TradeInfo {
        address token_in;
        address token_out;
        uint256 amount_in;
        uint256 amount_out;
        uint256 fee;
    }
    
    event TradeExecuted(
        address indexed trader,
        TradeInfo trade,
        uint256 timestamp
    );
    
    event BatchOperation(
        address indexed operator,
        string operation_type,
        uint256[] token_ids,
        address[] recipients
    );
}

#[public]
impl DEXContract {
    pub fn execute_trade(
        &mut self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<U256, Vec<u8>> {
        let trader = msg::sender();
        
        // ... trade execution logic
        let amount_out = self.calculate_output(token_in, token_out, amount_in)?;
        let fee = amount_out * U256::from(3) / U256::from(1000); // 0.3% fee
        
        // Emit complex structured event
        evm::log(TradeExecuted {
            trader,
            trade: TradeInfo {
                token_in,
                token_out,
                amount_in,
                amount_out,
                fee,
            },
            timestamp: U256::from(block::timestamp()),
        });
        
        Ok(amount_out - fee)
    }
    
    pub fn batch_mint(&mut self, recipients: Vec<Address>, amounts: Vec<U256>) -> Result<(), Vec<u8>> {
        // ... minting logic
        
        // Convert amounts to token_ids for the event
        let mut token_ids = Vec::new();
        for (i, _) in amounts.iter().enumerate() {
            token_ids.push(U256::from(self.next_token_id.get() + U256::from(i)));
        }
        
        evm::log(BatchOperation {
            operator: msg::sender(),
            operation_type: "batch_mint".to_string(),
            token_ids,
            recipients,
        });
        
        Ok(())
    }
}
```

## Working Example: Complete Migration

The `errors-events` example demonstrates the full transformation:

### Running the Example

```bash
cd examples/concepts/errors-events

# Compare implementations
ls -la anchor/src/lib.rs native/src/lib.rs stylus/src/lib.rs

# Test Stylus error and event handling
cd stylus && cargo test

# Check generated ABI for events and errors
cargo stylus export-abi
```

### Key Transformations

1. **Error Codes to Structured Errors**
   ```rust
   // Solana: Numeric error codes
   return Err(ProgramError::Custom(101));
   
   // Stylus: Rich error data
   return Err(CustomError::InsufficientFunds(InsufficientFunds { 
       requested, 
       available 
   }));
   ```

2. **msg!() to Typed Events**
   ```rust
   // Solana: String logging
   msg!("Transfer of {} tokens completed", amount);
   
   // Stylus: Structured events
   evm::log(TransferCompleted { from, to, amount });
   ```

3. **Limited Data to Rich Event Parameters**
   ```rust
   // Solana: Basic information
   msg!("Trade executed");
   
   // Stylus: Comprehensive trade data
   evm::log(TradeExecuted { trader, trade_info, timestamp });
   ```

## Error Recovery Patterns

### Graceful Failure Handling

```rust
use alloc::string::String;

sol! {
    error BatchOperationFailed(uint256 failed_index, string reason);
    error PartialSuccess(uint256 successful, uint256 failed);
}

#[derive(SolidityError)]
pub enum BatchError {
    BatchOperationFailed(BatchOperationFailed),
    PartialSuccess(PartialSuccess),
}

#[public]
impl BatchProcessor {
    pub fn process_batch(&mut self, operations: Vec<Operation>) -> Result<(), BatchError> {
        let mut successful = 0u32;
        let mut failed = 0u32;
        
        for (index, operation) in operations.iter().enumerate() {
            match self.process_single_operation(operation.clone()) {
                Ok(_) => {
                    successful += 1;
                    evm::log(OperationSuccess {
                        index: U256::from(index),
                        operation_type: operation.operation_type.clone(),
                    });
                }
                Err(error_msg) => {
                    failed += 1;
                    evm::log(OperationFailed {
                        index: U256::from(index),
                        reason: String::from_utf8_lossy(&error_msg).to_string(),
                    });
                }
            }
        }
        
        if failed > 0 {
            if successful > 0 {
                return Err(BatchError::PartialSuccess(PartialSuccess {
                    successful: U256::from(successful),
                    failed: U256::from(failed),
                }));
            } else {
                return Err(BatchError::BatchOperationFailed(BatchOperationFailed {
                    failed_index: U256::from(0),
                    reason: "All operations failed".to_string(),
                }));
            }
        }
        
        evm::log(BatchCompleted {
            total_operations: U256::from(operations.len()),
            successful_operations: U256::from(successful),
        });
        
        Ok(())
    }
}

sol! {
    event OperationSuccess(uint256 indexed index, string operation_type);
    event OperationFailed(uint256 indexed index, string reason);
    event BatchCompleted(uint256 total_operations, uint256 successful_operations);
}
```

### State Recovery Mechanisms

```rust
use alloc::format;

sol! {
    error StateCorrupted(string details);
    error RecoveryRequired(address recovery_authority);
}

#[derive(SolidityError)]
pub enum RecoveryError {
    StateCorrupted(StateCorrupted),
    RecoveryRequired(RecoveryRequired),
    InvalidAuthority(InvalidAuthority),
}

#[storage]
#[entrypoint]
pub struct RecoverableContract {
    balances: StorageMap<Address, StorageU256>,
    total_supply: StorageU256,
    recovery_authority: StorageAddress,
    recovery_mode: StorageBool,
    last_checkpoint: StorageU256,
}

#[public]
impl RecoverableContract {
    pub fn verify_state_integrity(&self) -> Result<(), RecoveryError> {
        let calculated_total = self.calculate_total_balances();
        let stored_total = self.total_supply.get();
        
        if calculated_total != stored_total {
            evm::log(StateIntegrityViolation {
                calculated_total,
                stored_total,
                timestamp: U256::from(block::timestamp()),
            });
            
            return Err(RecoveryError::StateCorrupted(StateCorrupted {
                details: format!("Total mismatch: {} vs {}", calculated_total, stored_total),
            }));
        }
        
        evm::log(StateVerificationPassed {
            total_supply: stored_total,
            timestamp: U256::from(block::timestamp()),
        });
        
        Ok(())
    }
    
    pub fn enter_recovery_mode(&mut self) -> Result<(), RecoveryError> {
        if msg::sender() != self.recovery_authority.get() {
            return Err(RecoveryError::InvalidAuthority(InvalidAuthority {
                provided: msg::sender(),
                expected: self.recovery_authority.get(),
            }));
        }
        
        self.recovery_mode.set(true);
        
        evm::log(RecoveryModeActivated {
            authority: msg::sender(),
            timestamp: U256::from(block::timestamp()),
        });
        
        Ok(())
    }
    
    fn calculate_total_balances(&self) -> U256 {
        // Implementation to sum all balances
        // This is a simplified example
        U256::from(0)
    }
}

sol! {
    event StateIntegrityViolation(uint256 calculated_total, uint256 stored_total, uint256 timestamp);
    event StateVerificationPassed(uint256 total_supply, uint256 timestamp);
    event RecoveryModeActivated(address indexed authority, uint256 timestamp);
}
```

## Event Indexing and Filtering

### Efficient Event Design

```rust
use stylus_sdk::abi::Bytes;

sol! {
    // Events with indexed parameters for efficient filtering
    event Transfer(
        address indexed from,
        address indexed to,
        uint256 value
    );
    
    event UserAction(
        address indexed user,
        bytes32 indexed action_type_hash, // Hash of action type for indexing
        uint256 timestamp,
        bytes data // Additional unindexed data
    );
    
    // Events for different contract states
    event ContractStateChanged(
        bytes32 indexed previous_state_hash,
        bytes32 indexed new_state_hash,
        address indexed changer,
        uint256 block_number
    );
}

#[public]
impl EventAwareContract {
    pub fn perform_action(&mut self, action_type: String, data: Bytes) -> Result<(), CustomError> {
        let user = msg::sender();
        
        // Validate action
        if !self.is_valid_action(&action_type) {
            return Err(CustomError::Unauthorized(Unauthorized { caller: user }));
        }
        
        // Process action
        self.execute_action(&action_type, &data)?;
        
        // Hash the action type for indexing
        let action_type_hash = keccak(action_type.as_bytes());
        
        // Emit detailed event for frontend filtering
        evm::log(UserAction {
            user,
            action_type_hash: action_type_hash.into(),
            timestamp: U256::from(block::timestamp()),
            data: data.to_vec().into(),
        });
        
        Ok(())
    }
    
    pub fn change_state(&mut self, new_state: String) -> Result<(), CustomError> {
        let current_state = self.contract_state.get_string();
        
        if current_state == new_state {
            return Err(CustomError::InvalidAmount(InvalidAmount { amount: U256::ZERO }));
        }
        
        self.contract_state.set_str(&new_state);
        
        // Hash states for efficient indexing
        let previous_state_hash = keccak(current_state.as_bytes());
        let new_state_hash = keccak(new_state.as_bytes());
        
        // Emit state change event
        evm::log(ContractStateChanged {
            previous_state_hash: previous_state_hash.into(),
            new_state_hash: new_state_hash.into(),
            changer: msg::sender(),
            block_number: U256::from(block::number()),
        });
        
        Ok(())
    }
    
    fn is_valid_action(&self, action_type: &str) -> bool {
        // Implementation to validate action types
        matches!(action_type, "deposit" | "withdraw" | "transfer")
    }
    
    fn execute_action(&mut self, action_type: &str, data: &Bytes) -> Result<(), CustomError> {
        // Implementation to execute the action
        Ok(())
    }
}

use stylus_sdk::crypto::keccak;
```

## Testing Error and Event Patterns

### Stylus Event Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use stylus_sdk::testing::TestVM;
    use alloy_primitives::B256;
    
    #[test]
    fn test_transfer_events() {
        let vm = TestVM::new();
        let mut contract = TokenContract::from(&vm);
        
        // Set up initial state
        vm.set_balance(Address::from([1u8; 20]), U256::from(1_000_000));
        contract.balances.setter(Address::from([1u8; 20])).set(U256::from(1000));
        
        // Set sender for the test
        vm.set_sender(Address::from([1u8; 20]));
        
        // Perform transfer
        let result = contract.transfer(Address::from([2u8; 20]), U256::from(500));
        assert!(result.is_ok());
        
        // Verify event was emitted
        let logs = vm.get_emitted_logs();
        assert_eq!(logs.len(), 1);
        
        // Check the event signature
        let transfer_event_sig = B256::from(keccak(
            "TransferCompleted(address,address,uint256)".as_bytes()
        ));
        assert_eq!(logs[0].0[0], transfer_event_sig);
    }
    
    #[test]
    fn test_error_handling() {
        let vm = TestVM::new();
        let mut contract = TokenContract::from(&vm);
        
        // Set sender
        vm.set_sender(Address::from([1u8; 20]));
        
        // Try to transfer with insufficient funds
        let result = contract.transfer(Address::from([2u8; 20]), U256::from(500));
        assert!(result.is_err());
        
        // Verify error type
        match result {
            Err(CustomError::InsufficientFunds(error)) => {
                assert_eq!(error.requested, U256::from(500));
                assert_eq!(error.available, U256::ZERO);
            }
            _ => panic!("Expected InsufficientFunds error"),
        }
    }
    
    #[test]
    fn test_complex_events() {
        let vm = TestVM::new();
        let mut contract = DEXContract::from(&vm);
        
        // Set up state for trade
        contract.initialize_liquidity(
            Address::from([10u8; 20]), 
            Address::from([11u8; 20]),
            U256::from(1_000_000),
            U256::from(2_000_000)
        ).unwrap();
        
        // Execute trade
        let result = contract.execute_trade(
            Address::from([10u8; 20]),
            Address::from([11u8; 20]),
            U256::from(1000)
        );
        assert!(result.is_ok());
        
        // Check emitted events
        let logs = vm.get_emitted_logs();
        assert!(!logs.is_empty());
        
        // Verify TradeExecuted event was emitted
        let trade_event_sig = B256::from(keccak(
            "TradeExecuted(address,(address,address,uint256,uint256,uint256),uint256)".as_bytes()
        ));
        assert_eq!(logs[logs.len() - 1].0[0], trade_event_sig);
    }
}
```

## Best Practices

### 1. Use Structured Errors
```rust
// Good: Rich error information
return Err(CustomError::InsufficientFunds(InsufficientFunds { 
    requested, 
    available 
}));

// Bad: Generic error message
return Err(b"Insufficient funds".to_vec());
```

### 2. Index Important Event Parameters
```rust
// Good: Indexed parameters for efficient filtering
event Transfer(address indexed from, address indexed to, uint256 value);

// Less optimal: No indexed parameters
event Transfer(address from, address to, uint256 value);
```

Note: The EVM supports up to 3 indexed parameters per event. Dynamic types (such as `string` and `bytes`) can be indexed but are hashed rather than stored verbatim in topics.

### 3. Emit Events for All State Changes
```rust
pub fn update_value(&mut self, new_value: U256) -> Result<(), Vec<u8>> {
    let old_value = self.value.get();
    self.value.set(new_value);
    
    // Always emit event for state changes
    evm::log(ValueUpdated { 
        old_value, 
        new_value, 
        updater: msg::sender() 
    });
    
    Ok(())
}
```

### 4. Provide Context in Events
```rust
// Good: Rich context
event TradeExecuted {
    address indexed trader,
    address token_in,
    address token_out,
    uint256 amount_in,
    uint256 amount_out,
    uint256 timestamp
}

// Less useful: Minimal context  
event TradeExecuted(address trader);
```

## Migration Checklist

### Analysis Phase
- [ ] Identify all error types in Solana code
- [ ] Map `msg!()` calls to potential events
- [ ] Document important state changes needing events
- [ ] List data that frontends need to track

### Implementation Phase
- [ ] Define custom error types with `sol!` macro
- [ ] Create SolidityError enum for error handling
- [ ] Replace `ProgramError` returns with structured errors
- [ ] Convert `msg!()` calls to typed events
- [ ] Add event emission to all state changes
- [ ] Include proper indexing for filtering

### Testing Phase
- [ ] Test all error conditions return proper error types
- [ ] Verify events emit correctly
- [ ] Check event data matches expectations
- [ ] Test event filtering and indexing
- [ ] Check gas costs for events and errors

## Common Pitfalls

### Forgetting to Derive SolidityError
```rust
// Bad: Missing derive
pub enum CustomError {
    InsufficientFunds(InsufficientFunds),
}

// Good: Proper derive
#[derive(SolidityError)]
pub enum CustomError {
    InsufficientFunds(InsufficientFunds),
}
```

### Too Many Indexed Parameters
```rust
// Bad: Too many indexed params (max 3 in EVM)
event ComplexEvent(
    address indexed a,
    address indexed b, 
    address indexed c,
    address indexed d  // This won't be indexed!
);

// Good: Use indexed strategically
event ComplexEvent(
    address indexed user,
    address indexed token,
    uint256 indexed action_type,
    uint256 amount
);
```

### Using Raw Vec<u8> Errors
```rust
// Bad: No structured information
return Err(b"Operation failed".to_vec());

// Good: Structured error with context
return Err(CustomError::OperationFailed(OperationFailed {
    reason: "Insufficient balance".to_string(),
    user: msg::sender(),
}));
```

## Summary

In this chapter, we covered:

1. **Error Model Transformation**: Converting from Solana's ProgramError to Stylus's structured errors
2. **Event System Migration**: Replacing msg!() calls with typed EVM events
3. **Rich Error Information**: Using the SolidityError derive macro for comprehensive error handling
4. **Event Indexing**: Strategic use of indexed parameters for efficient filtering
5. **Testing Patterns**: Verifying error handling and event emission in tests

The migration from Solana's simple logging to Stylus's rich error and event system provides better debugging capabilities, improved frontend integration, and more comprehensive error information for users and developers.

## Next Steps

With error handling and events covered, you've completed the core migration concepts. The next chapters will explore:
- Real-world case studies of complete program migrations
- Advanced optimization techniques
- Security considerations specific to EVM

Continue to [Chapter 9: Case Study - Draffle Migration](./09-case-study-draffle.md) to see these concepts applied in a complete program migration.