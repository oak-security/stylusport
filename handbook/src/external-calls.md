# External Calls

This chapter demonstrates how to translate Solana CPIs into Stylus external calls: define interfaces, invoke methods via sol_interface!, control gas and value with Call, and implement protection against reentrancy.

## Call Model Comparison

### Solana Cross-Program Invocations (CPIs)
- **Instruction-Based**: Call other programs via instruction construction
- **Account Passing**: Must pass all required accounts explicitly
- **Signed Invocations**: Use `invoke_signed` for PDA authority
- **Manual Setup**: Construct instructions and account metas manually

### Stylus External Calls
- **Interface-Based**: Call contracts via defined interfaces
- **Direct Method Calls**: Call contract methods directly
- **Configurable Context**: Control gas and value via Call; defaults supply all remaining gas and zero value unless set
- **Type Safety**: Compile-time interface verification
- **Two levels**: High-level sol_interface! calls for typed safety; low-level call/static_call/RawCall for bytes-in/bytes-out

## Basic Call Pattern Migration

### From Solana CPIs

**Solana Native:**
```rust
use solana_program::*;
use spl_token::instruction as token_instruction;

fn transfer_tokens(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let token_program = next_account_info(accounts_iter)?;
    let source_account = next_account_info(accounts_iter)?;
    let destination_account = next_account_info(accounts_iter)?;
    let authority_account = next_account_info(accounts_iter)?;
    
    // Construct CPI instruction
    let transfer_instruction = token_instruction::transfer(
        token_program.key,
        source_account.key,
        destination_account.key,
        authority_account.key,
        &[],
        amount,
    )?;
    
    // Execute CPI
    invoke(
        &transfer_instruction,
        &[
            source_account.clone(),
            destination_account.clone(),
            authority_account.clone(),
            token_program.clone(),
        ],
    )?;
    
    msg!("Transferred {} tokens", amount);
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub source: Account<'info, TokenAccount>,
    #[account(mut)]  
    pub destination: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[program]
pub mod my_program {
    pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
        // Anchor CPI helper
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;
        
        msg!("Transferred {} tokens", amount);
        Ok(())
    }
}
```

### To Stylus External Calls

**Stylus:**
```rust
use stylus_sdk::{prelude::*, call::Call, evm, msg, alloy_primitives::{Address, U256}};
use alloy_sol_types::sol;

sol_interface! {
    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }
}

sol! {
    error ExternalCallFailed();
    event TransferExecuted(address indexed to, uint256 amount);
}

#[storage]
#[entrypoint]
pub struct TokenManager {
    address token_contract;
    mapping(address => uint256) user_balances;
}

#[public]
impl TokenManager {
    pub fn transfer_tokens(&mut self, to: Address, amount: U256) -> Result<(), ExternalCallFailed> {
        let token = IERC20::new(self.token_contract.get());
        let ok = token.transfer(self, to, amount).map_err(|_| ExternalCallFailed {})?;
        if !ok { 
            return Err(ExternalCallFailed {}); 
        }
        evm::log(TransferExecuted { to, amount });
        Ok(())
    }

    pub fn transfer_from_user(
        &mut self,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<(), ExternalCallFailed> {
        let token = IERC20::new(self.token_contract.get());
        // snake_case method generated for transferFrom
        let ok = token.transfer_from(self, from, to, amount).map_err(|_| ExternalCallFailed {})?;
        if !ok { 
            return Err(ExternalCallFailed {}); 
        }
        Ok(())
    }
}
```

## Interface Definition Patterns

### Defining External Contract Interfaces

**Simple Interface:**
```rust
sol_interface! {
    interface ISimpleStorage {
        function setValue(uint256 value) external;
        function getValue() external view returns (uint256);
    }
}
```

**Complex Interface with Events:**
```rust
sol_interface! {
    interface IAdvancedContract {
        function processPayment(address user, uint256 amount) external payable returns (bool);
        function getAccountInfo(address user) external view returns (string memory, uint256, bool);
        function batchProcess(address[] memory users, uint256[] memory amounts) external;
    }
}
```

**Multiple Interfaces:**
```rust
sol_interface! {
    interface ITokenContract {
        function mint(address to, uint256 amount) external;
        function burn(uint256 amount) external;
    }
    
    interface IGovernance {
        function propose(string memory description) external returns (uint256);
        function vote(uint256 proposalId, bool support) external;
    }
}
```

## Call Configuration and Gas Management

### Basic Call Configuration

```rust
use stylus_sdk::call::Call;

sol_interface! {
    interface IExternalContract {
        function externalMethod(uint256 data) external;
    }
}

#[public]
impl MyContract {
    pub fn call_with_config(&mut self, target: Address, data: U256) -> Result<(), Vec<u8>> {
        let external_contract = IExternalContract::new(target);
        
        // Configure the call
        let config = Call::new_in(self)
            .gas(evm::gas_left() / 2)  // Use half the remaining gas
            .value(msg::value());       // Forward ETH if desired (default is zero)
            
        // Make the call with configuration
        external_contract.external_method(config, data)
            .map_err(|_| b"External call failed".to_vec())?;
            
        Ok(())
    }
}
```

### Advanced Call Patterns

**Conditional Calls:**
```rust
pub fn conditional_external_call(
    &mut self,
    should_call_a: bool,
    param: U256,
) -> Result<(), Vec<u8>> {
    if should_call_a {
        let contract_a = IContractA::new(self.contract_a_address.get());
        contract_a.method_a(self, param)?;
    } else {
        let contract_b = IContractB::new(self.contract_b_address.get());
        contract_b.method_b(self, param)?;
    }
    
    Ok(())
}
```

**Batch External Calls:**
```rust
pub fn batch_external_calls(
    &mut self,
    targets: Vec<Address>,
    amounts: Vec<U256>,
) -> Result<(), Vec<u8>> {
    if targets.len() != amounts.len() {
        return Err(b"Array length mismatch".to_vec());
    }
    
    for (i, target) in targets.iter().enumerate() {
        let contract = ITargetContract::new(*target);
        let amount = amounts[i];
        
        // Each call is independent
        contract.process_amount(self, amount)
            .map_err(|_| format!("Call {} failed", i).into_bytes())?;
    }
    
    Ok(())
}
```

## Working Example: Complete CPI Migration

The following example demonstrates migrating a Solana Oracle CPI to Stylus:

### Solana Oracle CPI Example

**Anchor:**
```rust
use anchor_lang::prelude::*;

// Oracle interface
#[derive(Clone)]
pub struct OracleProgram;

impl anchor_lang::Id for OracleProgram {
    fn id() -> Pubkey {
        oracle_program::ID
    }
}

#[derive(Accounts)]
pub struct GetPrice<'info> {
    /// Oracle program
    pub oracle_program: Program<'info, OracleProgram>,
    /// Oracle price feed account
    pub price_feed: AccountInfo<'info>,
    /// System program for any fees
    pub system_program: Program<'info, System>,
}

pub fn get_oracle_price(ctx: Context<GetPrice>) -> Result<u64> {
    // Prepare CPI accounts
    let cpi_accounts = oracle_program::cpi::accounts::GetPrice {
        price_feed: ctx.accounts.price_feed.to_account_info(),
    };
    
    // Create CPI context
    let cpi_program = ctx.accounts.oracle_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
    // Make the CPI call
    let price_result = oracle_program::cpi::get_latest_price(cpi_ctx)?;
    
    Ok(price_result.price)
}
```

### Stylus Oracle Call Example

**Stylus:**
```rust
use stylus_sdk::prelude::*;
use stylus_sdk::storage::{StorageAddress, StorageU256};

sol! { 
    event PriceUpdated(uint256 price, uint256 timestamp); 
}

sol_interface! {
    interface IOracle {
        function getLatestPrice() external view returns (uint256);
        function getLatestPriceWithTimestamp() external view returns (uint256 price, uint256 timestamp);
        function updatePrice(uint256 newPrice) external;
    }
}

#[storage]
#[entrypoint]
pub struct PriceConsumer {
    oracle_address: StorageAddress,
    last_price: StorageU256,
    last_update: StorageU256,
}

#[public]
impl PriceConsumer {
    pub fn update_price_from_oracle(&mut self) -> Result<(), Vec<u8>> {
        let oracle = IOracle::new(self.oracle_address.get());
        
        // View-style call via interface; default uses all remaining gas and zero value
        let (price, timestamp) = oracle.get_latest_price_with_timestamp(self)
            .map_err(|_| b"Oracle call failed".to_vec())?;
        
        // Update local state
        self.last_price.set(price);
        self.last_update.set(timestamp);
        
        evm::log(PriceUpdated { price, timestamp });
        Ok(())
    }
    
    pub fn get_current_price(&self) -> Result<U256, Vec<u8>> {
        let oracle = IOracle::new(self.oracle_address.get());
        
        // Direct return from external call
        oracle.get_latest_price(self)
            .map_err(|_| b"Failed to get price".to_vec())
    }
}
```

## Reentrancy Protection

Reentrancy is a critical security concern when making external calls. The following sections explore protection mechanisms:

### Understanding Reentrancy Risk

**Vulnerable Pattern:**
```rust
// DANGEROUS - State change after external call
pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
    let balance = self.balances.get(msg::sender());
    
    if balance < amount {
        return Err(b"Insufficient balance".to_vec());
    }
    
    // External call before state change - DANGEROUS!
    let token = IERC20::new(self.token_address.get());
    token.transfer(self, msg::sender(), amount)?;
    
    // Attacker could reenter here before this line
    self.balances.setter(msg::sender()).set(balance - amount);
    
    Ok(())
}
```

**Secure Pattern - Checks-Effects-Interactions:**
```rust
// SECURE - State change before external call
pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
    let sender = msg::sender();
    let balance = self.balances.get(sender);
    
    // 1. Checks
    if balance < amount {
        return Err(b"Insufficient balance".to_vec());
    }
    
    // 2. Effects - Update state BEFORE external call
    self.balances.setter(sender).set(balance - amount);
    
    // 3. Interactions - External call after state change
    let token = IERC20::new(self.token_address.get());
    match token.transfer(self, sender, amount) {
        Ok(success) if success => Ok(()),
        Ok(_) => {
            // Revert state on failure
            self.balances.setter(sender).set(balance);
            Err(b"Transfer returned false".to_vec())
        }
        Err(_) => {
            // Revert state on failure
            self.balances.setter(sender).set(balance);
            Err(b"Transfer failed".to_vec())
        }
    }
}
```

### Reentrancy Guards

**Manual Reentrancy Guard:**
```rust
use stylus_sdk::storage::StorageBool;

#[storage]
pub struct GuardedContract {
    locked: StorageBool,
    mapping(address => uint256) balances;
    address token_address;
}

impl GuardedContract {
    fn enter(&self) -> bool { 
        !self.locked.get() 
    }
}

#[public]
impl GuardedContract {
    pub fn protected_withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        if !self.enter() { 
            return Err(b"Reentrant call".to_vec()); 
        }
        self.locked.set(true);
        let result = self.perform_withdrawal(amount);
        self.locked.set(false);
        result
    }
    
    fn perform_withdrawal(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let sender = msg::sender();
        let balance = self.balances.get(sender);
        
        if balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update state first
        self.balances.setter(sender).set(balance - amount);
        
        // Then make external call
        let token = IERC20::new(self.token_address.get());
        token.transfer(self, sender, amount)?;
        
        Ok(())
    }
}
```

## Low-Level Call Patterns

For advanced use cases, Stylus provides low-level call mechanisms:

### Raw Calls

```rust
use stylus_sdk::call::{call, static_call, RawCall};

#[public]
impl AdvancedCaller {
    pub fn low_level_call(
        &mut self,
        target: Address,
        calldata: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>> {
        // Low-level call with custom data
        let result = call(
            Call::new_in(self).gas(100_000),
            target,
            &calldata,
        )?;
        
        Ok(result)
    }
    
    pub fn static_call_example(
        &self,
        target: Address,
        calldata: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>> {
        // Read-only call that doesn't modify state
        static_call(
            Call::new(),
            target,
            &calldata,
        )
    }
    
    pub fn delegate_call_example(
        &mut self, 
        target: Address, 
        calldata: Vec<u8>
    ) -> Result<Vec<u8>, Vec<u8>> {
        unsafe {
            RawCall::new_delegate()
                .gas(50_000)
                .limit_return_data(0, 1024)
                .call(target, &calldata)
        }
    }
}
```

### Function Selector Construction

```rust
use stylus_sdk::function_selector;

pub fn call_specific_function(
    &mut self,
    target: Address,
    value: U256,
) -> Result<(), Vec<u8>> {
    // Construct function selector for setValue(uint256)
    let selector = function_selector!("setValue", U256);
    
    // Encode the parameter
    let mut calldata = selector.to_vec();
    calldata.extend_from_slice(&value.to_be_bytes::<32>());
    
    // Make the low-level call
    call(
        Call::new_in(self),
        target,
        &calldata,
    )?;
    
    Ok(())
}
```

## Error Handling and Recovery

### Comprehensive Error Handling

```rust
sol! {
    event ExternalCallFailed(address indexed target, string reason);
    event FallbackExecuted(address indexed user, uint256 value);
}

#[public]
impl RobustContract {
    pub fn safe_external_operation(
        &mut self,
        primary_target: Address,
        fallback_target: Address,
        amount: U256,
    ) -> Result<(), Vec<u8>> {
        let primary = IPrimaryContract::new(primary_target);
        
        // Try primary target first
        match primary.process(self, amount) {
            Ok(true) => {
                // Success
                return Ok(());
            }
            Ok(false) => {
                // Contract returned false, try fallback
                evm::log(ExternalCallFailed {
                    target: primary_target,
                    reason: "Returned false".into(),
                });
            }
            Err(_) => {
                // Call reverted, try fallback
                evm::log(ExternalCallFailed {
                    target: primary_target,
                    reason: "Reverted".into(),
                });
            }
        }
        
        // Try fallback contract
        let fallback = IFallbackContract::new(fallback_target);
        match fallback.process_fallback(self, amount) {
            Ok(_) => {
                evm::log(FallbackExecuted {
                    user: msg::sender(),
                    value: amount,
                });
                Ok(())
            }
            Err(_) => {
                Err(b"Both primary and fallback failed".to_vec())
            }
        }
    }
}
```

## Best Practices for External Calls

### 1. Always Define Clear Interfaces
```rust
// Good: Clear, well-documented interface
sol_interface! {
    interface ITokenVault {
        /// Deposits tokens and returns vault shares
        /// @param amount The amount of tokens to deposit
        /// @return shares The amount of vault shares minted
        function deposit(uint256 amount) external returns (uint256 shares);
        
        /// Withdraws tokens by burning shares
        /// @param shares The amount of shares to burn
        /// @return amount The amount of tokens withdrawn
        function withdraw(uint256 shares) external returns (uint256 amount);
        
        /// Gets current exchange rate
        /// @return rate The amount of tokens per share (scaled by 1e18)
        function getExchangeRate() external view returns (uint256 rate);
    }
}
```

### 2. Validate Return Values
```rust
pub fn deposit_to_vault(&mut self, vault: Address, amount: U256) -> Result<U256, Vec<u8>> {
    let vault_contract = ITokenVault::new(vault);
    
    // Get shares from deposit
    let shares = vault_contract.deposit(self, amount)
        .map_err(|_| b"Vault deposit failed".to_vec())?;
    
    // Validate the return value
    if shares == U256::ZERO {
        return Err(b"No shares received".to_vec());
    }
    
    // Store user's shares
    let user = msg::sender();
    let current_shares = self.user_shares.getter(user).get(vault);
    self.user_shares.setter(user).insert(vault, current_shares + shares);
    
    Ok(shares)
}
```

### 3. Handle Gas Appropriately
```rust
pub fn complex_multi_call(&mut self, operations: Vec<Operation>) -> Result<(), Vec<u8>> {
    let total_gas = evm::gas_left();
    let operations_count = operations.len();
    if operations_count == 0 { 
        return Ok(()); 
    }
    
    // Reserve gas for post-processing
    let reserved_gas = 50_000;
    let available_gas = total_gas.saturating_sub(reserved_gas);
    let gas_per_operation = available_gas / operations_count as u64;
    
    for (i, operation) in operations.iter().enumerate() {
        let contract = IOperationContract::new(operation.target);
        
        // Configure gas limit per operation
        let config = Call::new_in(self).gas(gas_per_operation);
        
        contract.execute(config, operation.data.clone())
            .map_err(|_| format!("Operation {} failed", i).into_bytes())?;
    }
    
    Ok(())
}
```

Note that on Arbitrum, end-to-end cost includes L1 calldata posting. Measure costs with appropriate tooling rather than relying on fixed EVM gas constants.

## Migration Checklist

When migrating CPIs to Stylus external calls:

### Analysis Phase
- [ ] Identify all CPIs in existing Solana code
- [ ] Document external program interfaces used
- [ ] Map Solana instructions to EVM method signatures
- [ ] Identify potential reentrancy risks
- [ ] Note any PDA-signed CPIs that need special handling

### Implementation Phase
- [ ] Define Stylus interfaces for all external contracts
- [ ] Replace `invoke()` calls with interface method calls
- [ ] Replace `invoke_signed()` with appropriate patterns
- [ ] Add reentrancy guards where needed
- [ ] Implement proper error handling and recovery
- [ ] Configure gas limits appropriately

### Testing Phase
- [ ] Test all successful call paths
- [ ] Test all failure scenarios
- [ ] Test reentrancy protection
- [ ] Verify gas consumption is reasonable
- [ ] Test with multiple external contracts
- [ ] Test edge cases (zero amounts, empty arrays, etc.)

## Common Pitfalls and Solutions

### Forgetting Return Value Checks
```rust
// Bad: Ignoring return value
pub fn bad_transfer(&mut self, token: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
    let token_contract = IERC20::new(token);
    token_contract.transfer(self, to, amount)?;  // Returns bool!
    Ok(())  // Might succeed even if transfer returned false
}

// Good: Checking return value
pub fn good_transfer(&mut self, token: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
    let token_contract = IERC20::new(token);
    let success = token_contract.transfer(self, to, amount)?;
    if !success {
        return Err(b"Transfer returned false".to_vec());
    }
    Ok(())
}
```

### Not Handling View Function Failures
```rust
// Bad: Assuming view functions always succeed
pub fn bad_price_check(&self, oracle: Address) -> U256 {
    let oracle_contract = IOracle::new(oracle);
    oracle_contract.get_price(self).unwrap()  // Panic on failure!
}

// Good: Proper error handling
pub fn good_price_check(&self, oracle: Address) -> Result<U256, Vec<u8>> {
    let oracle_contract = IOracle::new(oracle);
    oracle_contract.get_price(self)
        .map_err(|_| b"Oracle unavailable".to_vec())
}
```

### Inefficient Multiple Calls
```rust
// Bad: Multiple calls to same contract
pub fn bad_multi_info(&self, token: Address, user: Address) -> Result<(U256, U256, U256), Vec<u8>> {
    let token_contract = IERC20::new(token);
    let balance = token_contract.balance_of(self, user)?;
    let total_supply = token_contract.total_supply(self)?;
    let decimals = token_contract.decimals(self)?;
    Ok((balance, total_supply, decimals))
}

// Good: Batch the calls if possible
sol_interface! {
    interface ITokenInfo {
        function getFullInfo(address user) external view returns (uint256 balance, uint256 totalSupply, uint8 decimals);
    }
}

pub fn good_multi_info(&self, token: Address, user: Address) -> Result<(U256, U256, U8), Vec<u8>> {
    let token_info = ITokenInfo::new(token);
    token_info.get_full_info(self, user)
        .map_err(|_| b"Failed to get token info".to_vec())
}
```

## Next Steps

With external calls mastered, you're ready to explore:
- [Native Tokens](./native-tokens.md) - Converting SOL transfers to ETH operations
- [Fungible Tokens](./fungible-tokens.md) - Migrating SPL tokens to ERC-20
- [Security Patterns](../security-best-practices.md) - Advanced security considerations

## Additional Resources

- Stylus SDK call module (interfaces, Call, low-level calls)
- Stylus by Example: Calls, Sending Ether, Function Selector
- sol_interface! proc-macro docs
- Alloy sol_types (ABI types & interfaces)
- EVM call types reference
- OpenZeppelin reentrancy guidance