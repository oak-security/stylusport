# Program Structure & Instructions

This chapter explains how to translate Solana's instruction-dispatch model to Stylus contracts. The transformation involves converting instruction handlers into direct methods, mapping parameter and return types to ABI-encodable forms, and preserving behavior with simpler, explicit entry points.

## Solana Program Model

### Native

Solana programs follow an instruction-processing model:
- Single entry point (`process_instruction`)
- Instruction data deserialized from bytes
- Manual instruction dispatch

```rust
use solana_program::*;

// Instruction enum
#[derive(BorshDeserialize)]
pub enum Instruction {
    Initialize { value: u64 },
    Increment,
    SetValue { new_value: u64 },
}

// Main entry point
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(instruction_data)?;
    
    match instruction {
        Instruction::Initialize { value } => process_initialize(accounts, value),
        Instruction::Increment => process_increment(accounts),
        Instruction::SetValue { new_value } => process_set_value(accounts, new_value),
    }
}

fn process_initialize(accounts: &[AccountInfo], initial_value: u64) -> ProgramResult {
    // validate accounts and perform business logic
}

fn process_increment(accounts: &[AccountInfo]) -> ProgramResult {
    // validate accounts and perform business logic
}

fn process_set_value(accounts: &[AccountInfo], new_value: u64) -> ProgramResult {
    // validate accounts and perform business logic
}
```

### Anchor

The Anchor framework abstracts the boilerplate for deserializing instruction data and function routing behind macros and conventions.

```rust
use anchor_lang::prelude::*;

#[program]
pub mod counter {
    use super::*;
    
    pub fn initialize(ctx: Context<Initialize>, value: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value = value;
        counter.authority = ctx.accounts.authority.key();
        Ok(())
    }
    
    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value += 1;
        Ok(())
    }
    
    pub fn set_value(ctx: Context<SetValue>, new_value: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value = new_value;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init_if_needed, payer = authority, space = 8 + 8 + 32)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut)]
    pub counter: Account<'info, Counter>,
}

#[derive(Accounts)]
pub struct SetValue<'info> {
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub value: u64,
    pub authority: Pubkey,
}
```

## Stylus Contract Model

Stylus contracts use direct method calls:
- Multiple public methods as entry points
- Type-safe parameters via Solidity-ABI decoding
- Automatic storage management
- EVM-compatible function dispatch

### Stylus

```rust
use stylus_sdk::prelude::*;
use stylus_sdk::{alloy_primitives::U256, msg};
use stylus_sdk::storage::{StorageU256, StorageAddress};

#[storage]
#[entrypoint]
pub struct Counter {
    value: StorageU256,
    authority: StorageAddress,
}

#[public]
impl Counter {
    pub fn initialize(&mut self, initial_value: U256) -> Result<(), Vec<u8>> {
        self.value.set(initial_value);
        self.authority.set(msg::sender());
        Ok(())
    }
    
    pub fn increment(&mut self) -> Result<(), Vec<u8>> {
        let current = self.value.get();
        self.value.set(current + U256::from(1));
        Ok(())
    }
    
    pub fn set_value(&mut self, new_value: U256) -> Result<(), Vec<u8>> {
        // Check authority
        if msg::sender() != self.authority.get() {
            return Err(b"Unauthorized".to_vec());
        }
        self.value.set(new_value);
        Ok(())
    }
    
    pub fn get_value(&self) -> U256 {
        self.value.get()
    }
}
```

## Key Transformations

### 1. Entry Points

| Aspect | Solana | Stylus |
|--------|--------|--------|
| Entry Point | Single `process_instruction` | Multiple `#[public]` methods |
| Dispatch | Manual instruction matching | Automatic method dispatch |
| Return Type | `ProgramResult` | `Result<T, E>` (where E implements SolidityError) or infallible |
| Parameters | Deserialized from bytes | Direct ABI-decoded parameters |

### 2. Initialization

Solana uses a dedicated instruction:
```rust
Instruction::Initialize { value } => {
    // Check account is uninitialized
    // Allocate space
    // Set initial data
}
```

Stylus can use either a regular method or constructor (starting in SDK 0.6.0):
```rust
// Option 1: Regular initialization method
pub fn initialize(&mut self, initial_value: U256) -> Result<(), Vec<u8>> {
    // Set initial values directly
}

// Option 2: Constructor (SDK 0.6.0+)
#[constructor]
pub fn constructor(&mut self, initial_value: U256) {
    // State automatically allocated
    // Set initial values directly
}
```

### 3. Parameter Handling

Solana requires manual deserialization from bytes:
```rust
let instruction = Instruction::try_from_slice(instruction_data)?;
match instruction {
    Instruction::Transfer { amount, recipient } => {
        // Use deserialized values
    }
}
```

Stylus provides automatic ABI decoding:
```rust
pub fn transfer(&mut self, recipient: Address, amount: U256) -> Result<(), Vec<u8>> {
    // Parameters are automatically decoded and available
}
```

## Complex Parameter Examples

### Handling Structured Data

Consider a comprehensive example that handles different parameter types:

**Solana Native:**
```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    CreateOrder {
        order_id: u64,
        items: Vec<OrderItem>,
        metadata: OrderMetadata,
    },
    UpdateStatus {
        order_id: u64,
        new_status: OrderStatus,
    },
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct OrderItem {
    pub product_id: u64,
    pub quantity: u32,
    pub price: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct OrderMetadata {
    pub customer: Pubkey,
    pub notes: String,
    pub priority: bool,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum OrderStatus {
    Pending,
    Processing,
    Shipped,
    Delivered,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(instruction_data)?;
    
    match instruction {
        Instruction::CreateOrder { order_id, items, metadata } => {
            process_create_order(accounts, order_id, items, metadata)
        }
        Instruction::UpdateStatus { order_id, new_status } => {
            process_update_status(accounts, order_id, new_status)
        }
    }
}
```

**Stylus:**
```rust
use stylus_sdk::prelude::*;
use stylus_sdk::evm;
use stylus_sdk::alloy_primitives::{Address, U256};
use stylus_sdk::storage::{StorageMap, StorageAddress, StorageString, StorageBool, StorageU8, StorageU256};
use alloy_sol_types::sol;

// Solidity-ABI types for parameters and events
sol! {
    struct OrderItem {
        uint256 productId;
        uint256 quantity;
        uint256 price;
    }

    struct OrderMetadata {
        address customer;
        string notes;
        bool priority;
    }

    enum OrderStatus {
        Pending,
        Processing,
        Shipped,
        Delivered
    }
}

// Persistent storage schema
#[storage]
#[entrypoint]
pub struct OrderContract {
    // orderId => Order
    orders: StorageMap<U256, Order>,
    // orderId => itemIndex => StoredItem
    items: StorageMap<U256, StorageMap<U256, StoredItem>>,
    // orderId => count
    item_counts: StorageMap<U256, StorageU256>,
}

#[storage]
struct Order {
    customer: StorageAddress,
    notes: StorageString,
    priority: StorageBool,
    status: StorageU8, // compact enum code
}

#[storage]
struct StoredItem {
    product_id: StorageU256,
    quantity: StorageU256,
    price: StorageU256,
}

#[public]
impl OrderContract {
    pub fn create_order(
        &mut self,
        order_id: U256,
        items: Vec<OrderItem>,
        metadata: OrderMetadata,
    ) -> Result<(), Vec<u8>> {
        let order_ro = self.orders.getter(order_id);
        if order_ro.customer.get() != Address::ZERO {
            return Err(b"Order already exists".to_vec());
        }

        let mut order = self.orders.setter(order_id);
        order.customer.set(metadata.customer);
        order.notes.set_str(&metadata.notes);
        order.priority.set(metadata.priority);
        order.status.set(0u8); // Pending

        let mut count: u64 = 0;
        for (i, item) in items.into_iter().enumerate() {
            let mut slot = self.items.setter(order_id).setter(U256::from(i));
            slot.product_id.set(item.productId);
            slot.quantity.set(item.quantity);
            slot.price.set(item.price);
            count += 1;
        }
        self.item_counts.setter(order_id).set(U256::from(count));
        Ok(())
    }

    pub fn update_status(
        &mut self,
        order_id: U256,
        new_status: OrderStatus,
    ) -> Result<(), Vec<u8>> {
        if self.orders.getter(order_id).customer.get() == Address::ZERO {
            return Err(b"Order not found".to_vec());
        }

        let mut order = self.orders.setter(order_id);
        let code = match new_status {
            OrderStatus::Pending => 0u8,
            OrderStatus::Processing => 1u8,
            OrderStatus::Shipped => 2u8,
            OrderStatus::Delivered => 3u8,
        };
        order.status.set(code);
        Ok(())
    }
}
```

### Benefits of Stylus Approach

**Type Safety**: Parameters are ABI-decoded into strongly typed Rust/Alloy types.
```rust
// This will revert if parameters don't match expected types
pub fn process_payment(&mut self, 
    payer: Address, 
    amount: U256, 
    payment_id: FixedBytes<32>
) -> Result<(), Vec<u8>>
```

**No Manual Matching**: Method dispatch happens automatically.
```rust
// No need for match statements - each method is a direct entry point
#[public]
impl TokenContract {
    pub fn mint(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>>
    pub fn burn(&mut self, from: Address, amount: U256) -> Result<(), Vec<u8>>
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>>
}
```

**Cleaner Error Handling**: Return structured errors.
```rust
sol! {
    error InsufficientBalance(uint256 available, uint256 required);
    error Unauthorized(address caller);
}

#[derive(SolidityError)]
pub enum TokenError {
    InsufficientBalance(InsufficientBalance),
    Unauthorized(Unauthorized),
}

pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), TokenError> {
    let balance = self.balances.get(msg::sender());
    if balance < amount {
        return Err(TokenError::InsufficientBalance(InsufficientBalance {
            available: balance,
            required: amount,
        }));
    }
    // Transfer logic
    Ok(())
}
```

## Working Example

Let's examine a complete program-structure example that demonstrates these concepts:

### Counter Program Migration

**Solana Implementation:**
```rust
// programs/counter/src/lib.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    Initialize { initial_value: u64 },
    Increment,
    Decrement,
    Reset,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CounterAccount {
    pub count: u64,
    pub authority: Pubkey,
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = CounterInstruction::try_from_slice(instruction_data)?;
    let accounts_iter = &mut accounts.iter();
    
    match instruction {
        CounterInstruction::Initialize { initial_value } => {
            msg!("Instruction: Initialize");
            let counter_account = next_account_info(accounts_iter)?;
            let authority = next_account_info(accounts_iter)?;
            
            // Verify account ownership
            if counter_account.owner != program_id {
                return Err(ProgramError::IncorrectProgramId);
            }
            
            // Initialize the counter
            let mut counter_data = CounterAccount {
                count: initial_value,
                authority: *authority.key,
            };
            
            counter_data.serialize(&mut *counter_account.data.borrow_mut())?;
            msg!("Counter initialized with value: {}", initial_value);
            Ok(())
        }
        CounterInstruction::Increment => {
            msg!("Instruction: Increment");
            let counter_account = next_account_info(accounts_iter)?;
            
            let mut counter_data = CounterAccount::try_from_slice(&counter_account.data.borrow())?;
            counter_data.count += 1;
            counter_data.serialize(&mut *counter_account.data.borrow_mut())?;
            
            msg!("Counter incremented to: {}", counter_data.count);
            Ok(())
        }
        CounterInstruction::Decrement => {
            msg!("Instruction: Decrement");
            let counter_account = next_account_info(accounts_iter)?;
            
            let mut counter_data = CounterAccount::try_from_slice(&counter_account.data.borrow())?;
            counter_data.count = counter_data.count.saturating_sub(1);
            counter_data.serialize(&mut *counter_account.data.borrow_mut())?;
            
            msg!("Counter decremented to: {}", counter_data.count);
            Ok(())
        }
        CounterInstruction::Reset => {
            msg!("Instruction: Reset");
            let counter_account = next_account_info(accounts_iter)?;
            let authority = next_account_info(accounts_iter)?;
            
            let mut counter_data = CounterAccount::try_from_slice(&counter_account.data.borrow())?;
            
            // Check authority
            if counter_data.authority != *authority.key || !authority.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            
            counter_data.count = 0;
            counter_data.serialize(&mut *counter_account.data.borrow_mut())?;
            
            msg!("Counter reset to 0");
            Ok(())
        }
    }
}
```

**Stylus Implementation:**
```rust
// src/lib.rs
#![cfg_attr(not(feature = "export-abi"), no_main)]
extern crate alloc;

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    alloy_sol_types::sol,
    msg,
    prelude::*,
    storage::{StorageU256, StorageAddress},
};

#[storage]
#[entrypoint]
pub struct Counter {
    count: StorageU256,
    authority: StorageAddress,
}

sol! {
    event CounterInitialized(uint256 initial_value);
    event CounterIncremented(uint256 new_value);
    event CounterDecremented(uint256 new_value);
    event CounterReset();
    
    error Unauthorized();
    error Underflow();
}

#[derive(SolidityError)]
pub enum CounterError {
    Unauthorized(Unauthorized),
    Underflow(Underflow),
}

#[public]
impl Counter {
    #[constructor]
    pub fn constructor(&mut self, initial_value: U256) {
        self.count.set(initial_value);
        self.authority.set(msg::sender());
        
        evm::log(CounterInitialized { initial_value });
    }
    
    pub fn increment(&mut self) -> Result<U256, Vec<u8>> {
        let new_value = self.count.get() + U256::from(1);
        self.count.set(new_value);
        
        evm::log(CounterIncremented { new_value });
        Ok(new_value)
    }
    
    pub fn decrement(&mut self) -> Result<U256, CounterError> {
        let current = self.count.get();
        if current == U256::ZERO {
            return Err(CounterError::Underflow(Underflow {}));
        }
        
        let new_value = current - U256::from(1);
        self.count.set(new_value);
        
        evm::log(CounterDecremented { new_value });
        Ok(new_value)
    }
    
    pub fn reset(&mut self) -> Result<(), CounterError> {
        if msg::sender() != self.authority.get() {
            return Err(CounterError::Unauthorized(Unauthorized {}));
        }
        
        self.count.set(U256::ZERO);
        evm::log(CounterReset {});
        Ok(())
    }
    
    // View functions (don't modify state)
    pub fn get_count(&self) -> U256 {
        self.count.get()
    }
    
    pub fn get_authority(&self) -> Address {
        self.authority.get()
    }
}
```

### Running the Example

```bash
# Navigate to the example
cd examples/concepts/program-structure/stylus

# Run tests
cargo test

# Export ABI
cargo stylus export-abi

# Deploy to local node
cargo stylus deploy --private-key-path=<KEY_PATH>
```

## Migration Checklist

When migrating from Solana to Stylus:

- [ ] **List all instructions** from your Solana program
- [ ] **Define storage structure** using `#[storage]`
- [ ] **Convert each instruction** to a public method
- [ ] **Map parameter types** appropriately:
  - `Pubkey` → `Address`
  - Choose integer widths deliberately: prefer `uint256`/`U256` for token math and general arithmetic; use narrower widths only when your external ABI specifies them or you have a strong reason to constrain range
  - Custom structs → `sol!` defined structs
- [ ] **Add constructor** for initialization logic (SDK 0.6.0+)
- [ ] **Implement access control** using `msg::sender()`
- [ ] **Convert errors** to Solidity-compatible errors (define with `sol!` and use `#[derive(SolidityError)]`)
- [ ] **Add view methods** for reading state
- [ ] **Test all methods** with appropriate parameters

## Common Pitfalls

### Wrong Parameter Types
```rust
// Don't use Solana-specific types
pub fn bad_method(&mut self, key: Pubkey) -> Result<(), Vec<u8>> // Wrong

// Use EVM-compatible types
pub fn good_method(&mut self, address: Address) -> Result<(), Vec<u8>> // Correct
```

### Missing Constructor
```rust
// Don't forget initialization
#[public]
impl MyContract {
    // Missing constructor - how does state get initialized? Wrong
    pub fn do_something(&mut self) -> Result<(), Vec<u8>> {
        // ...
    }
}

// Always provide a constructor
#[public]
impl MyContract {
    #[constructor]
    pub fn constructor(&mut self, owner: Address) {
        self.owner.set(owner);
    }
    // ... other methods
}
```

### Complex Return Types
```rust
// Avoid return types that don't map cleanly to ABI
pub fn bad_return(&self) -> HashMap<String, Vec<CustomStruct>> // Wrong

// Use simple, ABI-compatible returns
pub fn good_return(&self) -> Vec<Address> // Correct

// Or define proper sol! types
sol! {
    struct UserInfo {
        address wallet;
        uint256 balance;
    }
}
pub fn best_return(&self) -> Vec<UserInfo> // Correct
```

### Forgetting View Semantics
```rust
// Don't mark read-only functions as &mut
pub fn get_balance(&mut self) -> U256 // Wrong

// Use &self for view functions
pub fn get_balance(&self) -> U256 // Correct
```

## Best Practices

### 1. Keep Methods Focused
Each method should handle one logical operation:
```rust
// Good: Single responsibility
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> // Correct

// Bad: Multiple unrelated operations
pub fn transfer_and_update_metadata_and_emit_multiple_events(&mut self, ...) // Wrong
```

### 2. Use Descriptive Names
Method names become part of the ABI:
```rust
// Good: Clear intent
pub fn transfer_ownership(&mut self, new_owner: Address) // Correct
pub fn emergency_withdraw(&mut self) -> Result<U256, Vec<u8>> // Correct

// Bad: Unclear purpose
pub fn do_thing(&mut self, addr: Address) // Wrong
pub fn process(&mut self, val: U256) // Wrong
```

### 3. Validate Early
Check parameters at the beginning of methods:
```rust
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
    // Validate inputs first
    if to == Address::ZERO {
        return Err(b"Cannot transfer to zero address".to_vec());
    }
    if amount == U256::ZERO {
        return Err(b"Amount must be greater than zero".to_vec());
    }
    
    // Then proceed with logic
    let balance = self.balances.get(msg::sender());
    if balance < amount {
        return Err(b"Insufficient balance".to_vec());
    }
    
    // Perform transfer
    // ...
}
```

### 4. Use Events for Important State Changes
```rust
sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
}

pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
    // ... transfer logic ...
    
    evm::log(Transfer {
        from: msg::sender(),
        to,
        value: amount,
    });
    
    Ok(())
}
```

### 5. Leverage Structured Errors
```rust
sol! {
    error InsufficientBalance(uint256 available, uint256 required);
    error InvalidRecipient(address recipient);
}

#[derive(SolidityError)]
pub enum TransferError {
    InsufficientBalance(InsufficientBalance),
    InvalidRecipient(InvalidRecipient),
}

pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), TransferError> {
    if to == Address::ZERO {
        return Err(TransferError::InvalidRecipient(InvalidRecipient { 
            recipient: to 
        }));
    }
    
    let balance = self.balances.get(msg::sender());
    if balance < amount {
        return Err(TransferError::InsufficientBalance(InsufficientBalance {
            available: balance,
            required: amount,
        }));
    }
    
    // ... transfer logic ...
    Ok(())
}
```

## Advanced Patterns

### Batch Operations
```rust
sol! {
    struct BatchTransfer {
        address recipient;
        uint256 amount;
    }
}

#[public]
impl TokenContract {
    pub fn batch_transfer(
        &mut self, 
        transfers: Vec<BatchTransfer>
    ) -> Result<Vec<bool>, Vec<u8>> {
        let mut results = Vec::new();
        
        for transfer in transfers {
            match self.internal_transfer(msg::sender(), transfer.recipient, transfer.amount) {
                Ok(_) => results.push(true),
                Err(_) => results.push(false),
            }
        }
        
        Ok(results)
    }
}
```

### Factory Pattern
```rust
#[public]
impl Factory {
    pub fn create_token(
        &mut self,
        name: String,
        symbol: String,
        initial_supply: U256,
    ) -> Result<Address, Vec<u8>> {
        // Deploy new token contract
        let token_address = self.deploy_token(name, symbol)?;
        
        // Initialize with supply
        self.initialize_token(token_address, msg::sender(), initial_supply)?;
        
        // Track deployment
        self.deployed_tokens.push(token_address);
        self.token_creators.setter(token_address).set(msg::sender());
        
        Ok(token_address)
    }
}
```

## Performance Considerations

### Gas Efficiency
Stylus methods have different gas costs than Solana instructions:

```rust
// More efficient: Single storage write
pub fn update_multiple(&mut self, a: U256, b: U256, c: U256) {
    self.value_a.set(a);
    self.value_b.set(b);
    self.value_c.set(c);
}

// Less efficient: Multiple method calls
pub fn update_a(&mut self, value: U256) { self.value_a.set(value); }
pub fn update_b(&mut self, value: U256) { self.value_b.set(value); }
pub fn update_c(&mut self, value: U256) { self.value_c.set(value); }
```

### Memory and ABI I/O

Avoid returning unbounded collections. Paginate and track counts explicitly in storage:

```rust
pub fn get_item_count(&self, order_id: U256) -> U256 {
    self.item_counts.get(order_id)
}

pub fn get_items_page(&self, order_id: U256, offset: U256, limit: U256) -> Vec<StoredItem> {
    let mut out = Vec::new();
    let count = self.item_counts.get(order_id).to::<usize>();
    let start = offset.to::<usize>();
    let end = (start + limit.to::<usize>()).min(count);
    for i in start..end {
        let slot = self.items.getter(order_id).getter(U256::from(i));
        out.push(StoredItem {
            product_id: slot.product_id.get(),
            quantity: slot.quantity.get(),
            price: slot.price.get(),
        });
    }
    out
}
```

## Next Steps

Now that you understand program structure transformation, you're ready to explore:
- [State Storage](./state-storage.md) - Converting account-based storage to contract storage
- [Access Control](./access-control.md) - Implementing ownership and permissions
- [External Calls](./external-calls.md) - Making cross-contract calls

## References

- [Example Code: program-structure](/examples/concepts/program-structure/)
- [Stylus SDK Documentation](https://docs.rs/stylus-sdk/latest/stylus_sdk/)
- [Solidity ABI Specification](https://docs.soliditylang.org/en/latest/abi-spec.html)
- [Anchor to Stylus Migration Guide](https://docs.arbitrum.io/stylus/migration)
- [Export ABI with cargo stylus export-abi](https://stylus-by-example.org/basic_examples/export_interface)
- [Using cargo stylus deploy](https://github.com/OffchainLabs/cargo-stylus)
