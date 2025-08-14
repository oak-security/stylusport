# Native Tokens

This chapter maps SOL flows to ETH flows in Stylus: payable functions, internal balance accounting, safe withdrawals, and the key behavioral differences between lamports and wei.

## Token Model Comparison

### Solana Native Token (SOL)
- **Unit**: Lamports (1 SOL = 1,000,000,000 lamports)
- **Transfers**: Via System Program CPIs 
- **Balance Checks**: Account lamports field access
- **Program Payments**: Transfer lamports to program accounts

### Stylus Native Token (ETH)
- **Unit**: Wei (1 ETH = 1,000,000,000,000,000,000 Wei)
- **Transfers**: `call::transfer_eth` (forwards all gas); cap gas with a low-level call when needed
- **Balance Checks**: Use `contract::balance()` for the contract's ETH balance; track per-user balances in storage
- **Program Payments**: Payable functions with `msg::value()`; Non-payable functions revert if Ether is sent

## Basic Token Operations

### Balance Checking

**Solana Native:**
```rust
use solana_program::*;

fn check_balance(account: &AccountInfo) -> u64 {
    account.lamports()
}

fn process_payment(accounts: &[AccountInfo]) -> ProgramResult {
    let payer = &accounts[0];
    let recipient = &accounts[1];
    
    // Check if payer has enough SOL
    let payer_balance = payer.lamports();
    if payer_balance < required_amount {
        return Err(ProgramError::InsufficientFunds);
    }
    
    msg!("Payer balance: {} lamports", payer_balance);
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CheckBalance<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: This account's balance will be checked
    #[account(mut)]
    pub recipient: AccountInfo<'info>,
}

#[program]
pub mod payment_program {
    pub fn check_payment(ctx: Context<CheckBalance>, amount: u64) -> Result<()> {
        let payer_balance = ctx.accounts.payer.lamports();
        
        if payer_balance < amount {
            return Err(ErrorCode::InsufficientFunds.into());
        }
        
        msg!("Balance check passed: {} lamports available", payer_balance);
        Ok(())
    }
}
```

**Stylus:**
```rust
use stylus_sdk::{prelude::*, alloy_primitives::{Address, U256}, contract, msg};

#[storage]
#[entrypoint]
pub struct PaymentContract {
    mapping(address => uint256) user_balances;
    mapping(address => uint256) refunds;
    address owner;
}

#[public]
impl PaymentContract {
    pub fn check_balance(&self, user: Address) -> U256 {
        // Check contract's internal balance for user
        self.user_balances.get(user)
    }
    
    pub fn get_contract_balance(&self) -> U256 {
        // Check actual ETH balance of contract
        contract::balance()
    }
    
    pub fn get_msg_value(&self) -> U256 {
        // Check ETH sent with current transaction
        msg::value()
    }
}
```

### SOL Transfers to ETH Transfers

**Solana Native:**
```rust
use solana_program::{
    system_instruction,
    program::invoke,
};

fn transfer_sol(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let from_account = &accounts[0];
    let to_account = &accounts[1];
    let system_program = &accounts[2];
    
    // Create transfer instruction
    let transfer_instruction = system_instruction::transfer(
        from_account.key,
        to_account.key,
        amount,
    );
    
    // Execute CPI to System Program
    invoke(
        &transfer_instruction,
        &[
            from_account.clone(),
            to_account.clone(),
            system_program.clone(),
        ],
    )?;
    
    msg!("Transferred {} lamports", amount);
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::system_program;

#[derive(Accounts)]
pub struct TransferSol<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    /// CHECK: Recipient account
    #[account(mut)]
    pub to: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn transfer_sol(ctx: Context<TransferSol>, amount: u64) -> Result<()> {
    // Use Anchor's transfer helper
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
            },
        ),
        amount,
    )?;
    
    msg!("Transferred {} lamports via Anchor", amount);
    Ok(())
}
```

**Stylus:**
```rust
use stylus_sdk::{call::transfer_eth, prelude::*, msg};

// transfer_eth and call(...).value(...) are equivalent under the hood; transfer_eth supplies all remaining gas

#[public]
impl PaymentContract {
    #[payable]
    pub fn deposit(&mut self) -> Result<(), Vec<u8>> {
        let sender = msg::sender();
        let amount = msg::value();
        
        if amount == U256::ZERO {
            return Err(b"Must send ETH to deposit".to_vec());
        }
        
        // Update internal balance tracking
        let current_balance = self.user_balances.get(sender);
        self.user_balances.setter(sender).set(current_balance + amount);
        
        Ok(())
    }
    
    pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let sender = msg::sender();
        let balance = self.user_balances.get(sender);
        
        if balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update balance first (checks-effects-interactions)
        self.user_balances.setter(sender).set(balance - amount);
        
        // Transfer ETH back to user
        transfer_eth(sender, amount)?;
        // Using `?` ensures a revert on failure, so prior storage writes are rolled back
            
        Ok(())
    }
    
    pub fn transfer_to(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        let sender = msg::sender();
        let sender_balance = self.user_balances.get(sender);
        
        if sender_balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update balances
        self.user_balances.setter(sender).set(sender_balance - amount);
        let recipient_balance = self.user_balances.get(to);
        self.user_balances.setter(to).set(recipient_balance + amount);
        
        Ok(())
    }
}
```

## Payable Function Patterns

To accept plain ETH with no calldata, implement `#[receive]` (and optionally `#[fallback]`).

### Accepting Payments

**Solana Pattern**: Manual account balance manipulation
```rust
// Solana: Transfer SOL to program-owned account
let program_account = &accounts[2]; // Program's SOL holding account
invoke(&transfer_instruction, accounts)?;
```

**Stylus Pattern**: Payable functions
```rust
use stylus_sdk::{alloy_primitives::U256, block, call::{call, Call}};

// Cap gas when sending to untrusted contracts
// call(Call::new_in(self).value(amount).gas(50_000), recipient, &[])?;

#[public]
impl PaymentContract {
    #[payable]
    pub fn pay_for_service(&mut self, service_id: U256) -> Result<(), Vec<u8>> {
        let payment_amount = msg::value();
        let required_amount = self.get_service_cost(service_id);
        
        if payment_amount < required_amount {
            return Err(b"Insufficient payment".to_vec());
        }
        
        // Process service
        self.activate_service(msg::sender(), service_id)?;
        
        // Handle overpayment - prefer pull refunds over auto-refunds
        if payment_amount > required_amount {
            let refund = payment_amount - required_amount;
            // Instead of immediate refund:
            // transfer_eth(msg::sender(), refund)?;
            // Record refundable credit:
            self.refunds.setter(msg::sender()).set(self.refunds.get(msg::sender()) + refund);
        }
        
        Ok(())
    }
    
    fn get_service_cost(&self, service_id: U256) -> U256 {
        // Example service pricing
        match service_id {
            x if x == U256::from(1) => U256::from(100_000_000_000_000_000u64), // 0.1 ETH
            x if x == U256::from(2) => U256::from(200_000_000_000_000_000u64), // 0.2 ETH
            _ => U256::ZERO,
        }
    }
    
    pub fn claim_refund(&mut self) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        let amt = self.refunds.get(user);
        if amt == U256::ZERO { 
            return Ok(()); 
        }
        self.refunds.setter(user).set(U256::ZERO); // effects before interaction
        transfer_eth(user, amt)?;
        Ok(())
    }
    
    #[receive]
    #[payable]
    pub fn receive_eth(&mut self) -> Result<(), Vec<u8>> { 
        Ok(()) 
    }

    #[fallback]
    #[payable]
    pub fn fallback_eth(&mut self, _calldata: &[u8]) -> Result<Vec<u8>, Vec<u8>> {
        Ok(Vec::new())
    }
    
    fn activate_service(&mut self, user: Address, service_id: U256) -> Result<(), Vec<u8>> {
        // Service activation logic
        Ok(())
    }
}
```

## Advanced Payment Patterns

### Escrow and Time-Locked Payments

```rust
use stylus_sdk::{alloy_primitives::{Address, U256}, block};

#[storage]
#[entrypoint]
pub struct EscrowContract {
    mapping(uint256 => Escrow) escrows;
    uint256 next_escrow_id;
}

#[storage]
pub struct Escrow {
    address payer;
    address recipient;
    uint256 amount;
    uint256 unlock_time;
    bool completed;
}

#[public]
impl EscrowContract {
    #[payable]
    pub fn create_escrow(
        &mut self,
        recipient: Address,
        unlock_time: U256,
    ) -> Result<U256, Vec<u8>> {
        let escrow_id = self.next_escrow_id.get();
        let amount = msg::value();
        
        if amount == U256::ZERO {
            return Err(b"Must send ETH for escrow".to_vec());
        }
        
        if unlock_time <= U256::from(block::timestamp()) {
            return Err(b"Unlock time must be in future".to_vec());
        }
        
        // Create escrow
        let mut escrow = self.escrows.setter(escrow_id);
        escrow.payer.set(msg::sender());
        escrow.recipient.set(recipient);
        escrow.amount.set(amount);
        escrow.unlock_time.set(unlock_time);
        escrow.completed.set(false);
        
        self.next_escrow_id.set(escrow_id + U256::from(1));
        
        Ok(escrow_id)
    }
    
    pub fn release_escrow(&mut self, escrow_id: U256) -> Result<(), Vec<u8>> {
        let escrow = self.escrows.getter(escrow_id);
        
        if escrow.completed.get() {
            return Err(b"Escrow already completed".to_vec());
        }
        
        if U256::from(block::timestamp()) < escrow.unlock_time.get() {
            return Err(b"Escrow still locked".to_vec());
        }
        
        let recipient = escrow.recipient.get();
        let amount = escrow.amount.get();
        
        // Mark as completed
        self.escrows.setter(escrow_id).completed.set(true);
        
        // Transfer ETH to recipient
        transfer_eth(recipient, amount)?;
            
        Ok(())
    }
}
```

### Multi-Signature Payments

```rust
#[storage]
#[entrypoint]
pub struct MultiSigWallet {
    mapping(address => bool) is_owner;
    uint256 required_confirmations;
    mapping(uint256 => Transaction) transactions;
    mapping(uint256 => mapping(address => bool)) confirmations;
    uint256 transaction_count;
}

#[storage]
pub struct Transaction {
    address to;
    uint256 value;
    bool executed;
    uint256 confirmation_count;
}

#[public]
impl MultiSigWallet {
    #[payable]
    pub fn deposit(&mut self) -> Result<(), Vec<u8>> {
        // ETH automatically received by contract
        // Could emit an event here
        Ok(())
    }
    
    pub fn submit_transaction(
        &mut self,
        to: Address,
        value: U256,
    ) -> Result<U256, Vec<u8>> {
        if !self.is_owner(msg::sender()) {
            return Err(b"Only owners can submit transactions".to_vec());
        }
        
        let tx_id = self.transaction_count.get();
        
        let mut transaction = self.transactions.setter(tx_id);
        transaction.to.set(to);
        transaction.value.set(value);
        transaction.executed.set(false);
        transaction.confirmation_count.set(U256::from(1));
        
        self.transaction_count.set(tx_id + U256::from(1));
        
        // Auto-confirm for submitter
        self.confirmations.setter(tx_id).setter(msg::sender()).set(true);
        
        Ok(tx_id)
    }
    
    pub fn confirm_transaction(&mut self, tx_id: U256) -> Result<(), Vec<u8>> {
        if !self.is_owner(msg::sender()) {
            return Err(b"Only owners can confirm".to_vec());
        }
        
        if self.confirmations.getter(tx_id).get(msg::sender()) {
            return Err(b"Already confirmed".to_vec());
        }
        
        self.confirmations.setter(tx_id).setter(msg::sender()).set(true);
        
        let mut transaction = self.transactions.setter(tx_id);
        let count = transaction.confirmation_count.get();
        transaction.confirmation_count.set(count + U256::from(1));
        
        Ok(())
    }
    
    pub fn execute_transaction(&mut self, tx_id: U256) -> Result<(), Vec<u8>> {
        let mut transaction = self.transactions.setter(tx_id);
        
        if transaction.executed.get() {
            return Err(b"Transaction already executed".to_vec());
        }
        
        if transaction.confirmation_count.get() < self.required_confirmations.get() {
            return Err(b"Not enough confirmations".to_vec());
        }
        
        // Mark as executed
        transaction.executed.set(true);
        
        // Execute transfer
        transfer_eth(transaction.to.get(), transaction.value.get())?;
            
        Ok(())
    }
    
    fn is_owner(&self, addr: Address) -> bool { 
        self.is_owner.get(addr) 
    }
}
```

## Unit Conversions

### Decimals and Display Scaling (Lamports vs Wei)

SOL and ETH are different assets. There is no canonical conversion between lamports and wei; any mapping is application-defined and should be treated as display scaling only.

**Scale Differences:**
- 1 SOL = 1,000,000,000 lamports (9 decimals)
- 1 ETH = 1,000,000,000,000,000,000 Wei (18 decimals)

**Conversion Helper:**
```rust
impl PaymentContract {
    // Display-only scaling: maps 9-decimal lamports to 18-decimal units
    fn scale_lamports_to_18_decimals(&self, lamports: U256) -> U256 {
        lamports * U256::from(1_000_000_000u64) // 10^9
    }
}
```

## Security Considerations

### Reentrancy Protection

```rust
#[storage]
#[entrypoint]
pub struct SecurePayment {
    mapping(address => uint256) balances;
    bool locked;
}

#[public]
impl SecurePayment {
    pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        // Reentrancy guard
        if self.locked.get() {
            return Err(b"Reentrant call".to_vec());
        }
        self.locked.set(true);
        
        let sender = msg::sender();
        let balance = self.balances.get(sender);
        
        if balance < amount {
            self.locked.set(false);
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update state before external call
        self.balances.setter(sender).set(balance - amount);
        
        // External call
        let result = transfer_eth(sender, amount);
        
        self.locked.set(false);
        
        result
    }
}
```

### Overflow Protection

```rust
#[public]
impl SafePayment {
    pub fn safe_add_balance(&mut self, user: Address, amount: U256) -> Result<(), Vec<u8>> {
        let current = self.balances.get(user);
        
        // Check for overflow
        let new_balance = current.checked_add(amount)
            .ok_or(b"Balance overflow".to_vec())?;
            
        self.balances.setter(user).set(new_balance);
        Ok(())
    }
    
    pub fn safe_subtract_balance(&mut self, user: Address, amount: U256) -> Result<(), Vec<u8>> {
        let current = self.balances.get(user);
        
        // Check for underflow
        let new_balance = current.checked_sub(amount)
            .ok_or(b"Balance underflow".to_vec())?;
            
        self.balances.setter(user).set(new_balance);
        Ok(())
    }
}
```

## Best Practices

### 1. Always Use Checks-Effects-Interactions Pattern
```rust
pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
    // 1. CHECKS
    if amount == U256::ZERO { 
        return Err(b"Invalid amount".to_vec()); 
    }
    let balance = self.balances.get(msg::sender());
    if balance < amount { 
        return Err(b"Insufficient funds".to_vec()); 
    }
    
    // 2. EFFECTS
    self.balances.setter(msg::sender()).set(balance - amount);
    
    // 3. INTERACTIONS
    transfer_eth(msg::sender(), amount)?;
    Ok(())
}
```

### 2. Validate Payable Function Inputs
```rust
#[payable]
pub fn purchase_item(&mut self, item_id: U256) -> Result<(), Vec<u8>> {
    let payment = msg::value();
    let required = self.item_prices.get(item_id);
    
    if required == U256::ZERO {
        return Err(b"Item not found".to_vec());
    }
    
    if payment < required {
        return Err(b"Insufficient payment".to_vec());
    }
    
    // Process purchase...
    Ok(())
}
```

### 3. Prefer Pull Refunds Over Auto-refunds
Prefer pull refunds over auto-refunds in payable flows. If you must refund, use the checks-effects-interactions pattern and consider gas-capped calls to untrusted contracts.

### 4. Handle Failed Transfers Gracefully
```rust
use stylus_sdk::evm;
use alloy_sol_types::sol;

sol! {
    event TransferFailed(address indexed recipient, uint256 amount);
}

pub fn batch_transfer(&mut self, recipients: Vec<Address>, amounts: Vec<U256>) -> Result<(), Vec<u8>> {
    if recipients.len() != amounts.len() {
        return Err(b"Array length mismatch".to_vec());
    }
    
    for (i, (recipient, amount)) in recipients.iter().zip(amounts.iter()).enumerate() {
        match transfer_eth(*recipient, *amount) {
            Ok(_) => {
                // Success - continue
            },
            Err(_) => {
                // Log failure but don't stop entire batch
                evm::log(TransferFailed { 
                    recipient: *recipient, 
                    amount: *amount 
                });
            }
        }
    }
    Ok(())
}
```

## Migration Checklist

### Analysis Phase
- [ ] Identify all SOL transfer operations
- [ ] Map lamport amounts to appropriate Wei values
- [ ] Document payment flows and escrow patterns
- [ ] Identify balance checking logic

### Implementation Phase
- [ ] Replace System Program calls with `transfer_eth()`
- [ ] Convert balance checks to use storage or `msg::value()`
- [ ] Add `#[payable]` attribute to payment-receiving functions
- [ ] Implement reentrancy protection
- [ ] Add overflow/underflow protection

### Testing Phase
- [ ] Test payable functions with different ETH amounts
- [ ] Test withdrawal and transfer functions
- [ ] Verify reentrancy protection works
- [ ] Test edge cases (zero amounts, insufficient funds)
- [ ] Check gas costs remain reasonable

## Common Pitfalls

### Forgetting Payable Attribute
```rust
// Won't receive ETH
pub fn deposit(&mut self) -> Result<(), Vec<u8>> { ... }

// Correctly receives ETH  
#[payable]
pub fn deposit(&mut self) -> Result<(), Vec<u8>> { ... }
```

### Different Gas Behavior Than Solidity
Stylus `transfer_eth` forwards all gas, unlike Solidity's `.transfer` and `.send` which have a 2300 gas stipend. Consider using gas-capped calls when transferring to untrusted contracts.

### External Calls Before State Updates
```rust
// Vulnerable to reentrancy
pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
    transfer_eth(msg::sender(), amount)?; // External call first
    self.balances.setter(msg::sender()).set(balance - amount); // State change after
    Ok(())
}
```

### Not Handling Transfer Failures
```rust
// Doesn't handle failure
transfer_eth(recipient, amount); // If this fails, execution continues

// Properly handle failure
transfer_eth(recipient, amount)?; // Propagates error and reverts
```

## Next Steps

With native token handling covered, the next chapter explores [Fungible Token Handling](./fungible-tokens.md) - migrating SPL Token operations to ERC-20 patterns in Stylus contracts.

## Reference

- [Example Code: native-token-handling](/examples/concepts/native-token-handling/)
- [Stylus SDK: Payable Functions](https://docs.rs/stylus-sdk/latest/stylus_sdk/attr.payable.html)
- [Stylus SDK: transfer_eth](https://docs.rs/stylus-sdk/latest/stylus_sdk/call/fn.transfer_eth.html)
- [Ethereum Units and Denominations](https://ethereum.org/en/developers/docs/intro-to-ether/)