# Fungible Token Handling

SPL Tokens provide fundamental standardized fungible token functionality for Solana applications. This chapter covers migrating SPL Token operations to ERC-20 patterns in Stylus, including minting, burning, transfers, and allowance mechanisms.

## Token Model Comparison

### SPL Token (Solana)
- **Standard**: SPL Token Program (token-2022 for newer features)
- **Accounts**: Separate mint account and token accounts per holder
- **Operations**: Cross-Program Invocations (CPIs) to token program
- **Authority**: Mint authority and freeze authority patterns

### ERC-20 (Stylus)
- **Standard**: ERC-20 interface (with Stylus-specific implementation)
- **Storage**: Mappings for balances and allowances in contract storage
- **Operations**: Direct contract method calls
- **Authority**: Ownership/roles are not part of ERC-20; implement via contract logic (e.g., Ownable/RBAC)

## Basic Token Operations

### Token Creation and Initialization

**Solana Native:**
```rust
use solana_program::*;
use spl_token::{
    instruction as token_instruction,
    state::{Mint, Account as TokenAccount},
};

fn create_token_mint(
    accounts: &[AccountInfo],
    decimals: u8,
    mint_authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> ProgramResult {
    let mint_account = &accounts[0];
    let rent_account = &accounts[1];
    let token_program = &accounts[2];
    
    // Initialize mint account
    let init_instruction = token_instruction::initialize_mint(
        token_program.key,
        mint_account.key,
        mint_authority,
        freeze_authority,
        decimals,
    )?;
    
    invoke(&init_instruction, &[
        mint_account.clone(),
        rent_account.clone(),
        token_program.clone(),
    ])?;
    
    msg!("Token mint created with {} decimals", decimals);
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(
        init,
        payer = authority,
        mint::decimals = 6,
        mint::authority = authority,
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_token(ctx: Context<InitializeToken>) -> Result<()> {
    // Mint account is automatically initialized by Anchor
    msg!("Token mint initialized");
    Ok(())
}
```

**Stylus:**
```rust
use stylus_sdk::prelude::*;
use stylus_sdk::storage::*;
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;

#[storage]
#[entrypoint]
pub struct ERC20Token {
    balances: StorageMap<Address, StorageU256>,
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    total_supply: StorageU256,
    name: StorageString,
    symbol: StorageString,
    decimals: StorageU8,
    owner: StorageAddress,
}

#[public]
impl ERC20Token {
    #[constructor]
    pub fn constructor(
        &mut self,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: U256,
    ) {
        self.name.set_str(&name);
        self.symbol.set_str(&symbol);
        self.decimals.set(decimals);
        self.owner.set(msg::sender());
        
        // Mint initial supply to creator
        if initial_supply > U256::ZERO {
            self.balances.setter(msg::sender()).set(initial_supply);
            self.total_supply.set(initial_supply);
            evm::log(Transfer { from: Address::ZERO, to: msg::sender(), value: initial_supply });
        }
    }
    
    // Standard ERC-20 view functions
    pub fn name(&self) -> String {
        self.name.get_string()
    }
    
    pub fn symbol(&self) -> String {
        self.symbol.get_string()
    }
    
    pub fn decimals(&self) -> u8 {
        self.decimals.get()
    }
    
    #[selector(name = "totalSupply")]
    pub fn total_supply(&self) -> U256 {
        self.total_supply.get()
    }
    
    #[selector(name = "balanceOf")]
    pub fn balance_of(&self, account: Address) -> U256 {
        self.balances.get(account)
    }
}
```

### Token Transfers

**Solana Native:**
```rust
fn transfer_tokens(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let source_account = &accounts[0];
    let destination_account = &accounts[1];
    let authority_account = &accounts[2];
    let token_program = &accounts[3];
    
    // Create transfer instruction
    let transfer_instruction = token_instruction::transfer(
        token_program.key,
        source_account.key,
        destination_account.key,
        authority_account.key,
        &[],
        amount,
    )?;
    
    // Execute CPI
    invoke(&transfer_instruction, &[
        source_account.clone(),
        destination_account.clone(),
        authority_account.clone(),
        token_program.clone(),
    ])?;
    
    msg!("Transferred {} tokens", amount);
    Ok(())
}
```

**Anchor:**
```rust
#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
    )?;
    
    msg!("Transferred {} tokens", amount);
    Ok(())
}
```

**Stylus:**
```rust
#[public]
impl ERC20Token {
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, Vec<u8>> {
        self.do_transfer(msg::sender(), to, amount)?;
        Ok(true)
    }
    
    pub fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<bool, Vec<u8>> {
        let sender = msg::sender();
        
        // Check allowance if not self-transfer
        if sender != from {
            let current_allowance = self.allowances.getter(from).get(sender);
            if current_allowance < amount {
                return Err(b"ERC20: transfer amount exceeds allowance".to_vec());
            }
            
            // Update allowance
            self.allowances.setter(from).insert(sender, current_allowance - amount);
        }
        
        self.do_transfer(from, to, amount)?;
        Ok(true)
    }
}

impl ERC20Token {
    fn do_transfer(&mut self, from: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        if from == Address::ZERO {
            return Err(b"ERC20: transfer from zero address".to_vec());
        }
        if to == Address::ZERO {
            return Err(b"ERC20: transfer to zero address".to_vec());
        }
        
        let from_balance = self.balances.get(from);
        if from_balance < amount {
            return Err(b"ERC20: transfer amount exceeds balance".to_vec());
        }
        
        // Update balances
        self.balances.setter(from).set(from_balance - amount);
        let to_balance = self.balances.get(to);
        self.balances.setter(to).set(to_balance + amount);
        
        // Emit Transfer event
        evm::log(Transfer {
            from,
            to,
            value: amount,
        });
        
        Ok(())
    }
}

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
}
```

### Minting and Burning

**Solana Native:**
```rust
fn mint_tokens(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let mint_account = &accounts[0];
    let destination_account = &accounts[1];
    let mint_authority = &accounts[2];
    let token_program = &accounts[3];
    
    // Create mint instruction
    let mint_instruction = token_instruction::mint_to(
        token_program.key,
        mint_account.key,
        destination_account.key,
        mint_authority.key,
        &[],
        amount,
    )?;
    
    invoke(&mint_instruction, &[
        mint_account.clone(),
        destination_account.clone(),
        mint_authority.clone(),
        token_program.clone(),
    ])?;
    
    msg!("Minted {} tokens", amount);
    Ok(())
}

fn burn_tokens(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let token_account = &accounts[0];
    let mint_account = &accounts[1];
    let authority = &accounts[2];
    let token_program = &accounts[3];
    
    let burn_instruction = token_instruction::burn(
        token_program.key,
        token_account.key,
        mint_account.key,
        authority.key,
        &[],
        amount,
    )?;
    
    invoke(&burn_instruction, &[
        token_account.clone(),
        mint_account.clone(),
        authority.clone(),
        token_program.clone(),
    ])?;
    
    msg!("Burned {} tokens", amount);
    Ok(())
}
```

**Anchor:**
```rust
#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    pub mint_authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    token::mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.mint_authority.to_account_info(),
            },
        ),
        amount,
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.from.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
    )?;
    Ok(())
}
```

**Stylus:**
```rust
#[public]
impl ERC20Token {
    pub fn mint(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        // Only owner can mint
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can mint".to_vec());
        }
        
        if to == Address::ZERO {
            return Err(b"ERC20: mint to zero address".to_vec());
        }
        
        // Update total supply
        self.total_supply.set(self.total_supply.get() + amount);
        
        // Update balance
        let balance = self.balances.get(to);
        self.balances.setter(to).set(balance + amount);
        
        // Emit Transfer event (from zero address indicates minting)
        evm::log(Transfer {
            from: Address::ZERO,
            to,
            value: amount,
        });
        
        Ok(())
    }
    
    pub fn burn(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        self.burn_from(msg::sender(), amount)
    }
    
    pub fn burn_from(&mut self, from: Address, amount: U256) -> Result<(), Vec<u8>> {
        let sender = msg::sender();
        
        // Check allowance if not self-burn
        if sender != from {
            let current_allowance = self.allowances.getter(from).get(sender);
            if current_allowance < amount {
                return Err(b"ERC20: burn amount exceeds allowance".to_vec());
            }
            
            self.allowances.setter(from).insert(sender, current_allowance - amount);
        }
        
        let balance = self.balances.get(from);
        if balance < amount {
            return Err(b"ERC20: burn amount exceeds balance".to_vec());
        }
        
        // Update balance and total supply
        self.balances.setter(from).set(balance - amount);
        self.total_supply.set(self.total_supply.get() - amount);
        
        // Emit Transfer event (to zero address indicates burning)
        evm::log(Transfer {
            from,
            to: Address::ZERO,
            value: amount,
        });
        
        Ok(())
    }
}
```

### Allowance System

**Solana**: Uses delegate authority mechanism
```rust
// SPL Token uses account delegation
let approve_instruction = token_instruction::approve(
    token_program.key,
    source_account.key,
    delegate_account.key,
    owner_account.key,
    &[],
    amount,
)?;
```

**Anchor**: Simplified delegation
```rust
#[derive(Accounts)]
pub struct ApproveDelegate<'info> {
    #[account(mut)]
    pub source: Account<'info, TokenAccount>,
    pub delegate: SystemAccount<'info>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn approve_delegate(ctx: Context<ApproveDelegate>, amount: u64) -> Result<()> {
    token::approve(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Approve {
                to: ctx.accounts.source.to_account_info(),
                delegate: ctx.accounts.delegate.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        amount,
    )?;
    Ok(())
}
```

**Stylus**: ERC-20 allowance pattern
```rust
#[public]
impl ERC20Token {
    pub fn approve(&mut self, spender: Address, amount: U256) -> Result<bool, Vec<u8>> {
        let owner = msg::sender();
        
        if spender == Address::ZERO {
            return Err(b"ERC20: approve to zero address".to_vec());
        }
        
        self.allowances.setter(owner).insert(spender, amount);
        
        evm::log(Approval {
            owner,
            spender,
            value: amount,
        });
        
        Ok(true)
    }
    
    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.getter(owner).get(spender)
    }
    
    #[selector(name = "increaseAllowance")]
    pub fn increase_allowance(&mut self, spender: Address, added_value: U256) -> Result<bool, Vec<u8>> {
        let owner = msg::sender();
        let current_allowance = self.allowances.getter(owner).get(spender);
        self.allowances.setter(owner).insert(spender, current_allowance + added_value);
        
        evm::log(Approval {
            owner,
            spender,
            value: current_allowance + added_value,
        });
        
        Ok(true)
    }
    
    #[selector(name = "decreaseAllowance")]
    pub fn decrease_allowance(&mut self, spender: Address, subtracted_value: U256) -> Result<bool, Vec<u8>> {
        let owner = msg::sender();
        let current_allowance = self.allowances.getter(owner).get(spender);
        
        if current_allowance < subtracted_value {
            return Err(b"ERC20: decreased allowance below zero".to_vec());
        }
        
        let new_allowance = current_allowance - subtracted_value;
        self.allowances.setter(owner).insert(spender, new_allowance);
        
        evm::log(Approval {
            owner,
            spender,
            value: new_allowance,
        });
        
        Ok(true)
    }
}
```

## Working Example: Complete Migration

The `fungible-tokens` example demonstrates the full transformation:

### Running the Example

```bash
cd examples/concepts/fungible-tokens

# Compare all implementations
ls -la anchor/src/lib.rs native/src/lib.rs stylus/src/lib.rs

# Test Stylus ERC-20 implementation
cd stylus && cargo test

# Check generated ABI
cargo stylus export-abi
```

### Key Transformations

1. **Mint Account → Contract Storage**
   ```rust
   // Solana: Separate mint account with metadata
   pub struct Mint {
       pub mint_authority: COption<Pubkey>,
       pub supply: u64,
       pub decimals: u8,
       // ...
   }
   
   // Stylus: Contract storage fields
   total_supply: StorageU256,
   name: StorageString,
   symbol: StorageString,
   decimals: StorageU8,
   ```

2. **Token Accounts → Balance Mapping**
   ```rust
   // Solana: Separate account per holder
   pub struct Account {
       pub mint: Pubkey,
       pub owner: Pubkey,
       pub amount: u64,
       // ...
   }
   
   // Stylus: Simple mapping
   balances: StorageMap<Address, StorageU256>,
   ```

3. **CPI Instructions → Direct Methods**
   ```rust
   // Solana: CPI to token program
   invoke(&token_instruction::transfer(...), accounts)?;
   
   // Stylus: Direct method call
   token_contract.transfer(to, amount)?;
   ```

## Advanced Token Features

### Pausable Token

```rust
#[storage]
#[entrypoint]
pub struct PausableToken {
    balances: StorageMap<Address, StorageU256>,
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    total_supply: StorageU256,
    name: StorageString,
    symbol: StorageString,
    decimals: StorageU8,
    owner: StorageAddress,
    paused: StorageBool,
}

#[public]
impl PausableToken {
    pub fn pause(&mut self) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can pause".to_vec());
        }
        
        self.paused.set(true);
        evm::log(Paused {});
        Ok(())
    }
    
    pub fn unpause(&mut self) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can unpause".to_vec());
        }
        
        self.paused.set(false);
        evm::log(Unpaused {});
        Ok(())
    }
    
    pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, Vec<u8>> {
        if self.paused.get() {
            return Err(b"Token transfers are paused".to_vec());
        }
        
        self.do_transfer(msg::sender(), to, amount)?;
        Ok(true)
    }
}

sol! {
    event Paused();
    event Unpaused();
}
```

### Capped Token Supply

```rust
#[storage]
pub struct CappedToken {
    // ... standard ERC-20 fields
    cap: StorageU256,
}

#[public]
impl CappedToken {
    #[constructor]
    pub fn constructor(&mut self, name: String, symbol: String, cap: U256) {
        // ... standard initialization
        self.cap.set(cap);
    }
    
    pub fn mint(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can mint".to_vec());
        }
        
        let new_total = self.total_supply.get() + amount;
        if new_total > self.cap.get() {
            return Err(b"ERC20Capped: cap exceeded".to_vec());
        }
        
        // ... standard minting logic
        self.total_supply.set(new_total);
        // ... update balance and emit event
        
        Ok(())
    }
    
    pub fn cap(&self) -> U256 {
        self.cap.get()
    }
}
```

### Token with Fees

```rust
#[storage]
pub struct FeesToken {
    // ... standard ERC-20 fields
    transfer_fee_bps: StorageU256, // Basis points (100 = 1%)
    fee_collector: StorageAddress,
}

#[public]
impl FeesToken {
    pub fn set_transfer_fee(&mut self, fee_bps: U256) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can set fees".to_vec());
        }
        
        if fee_bps > U256::from(1000) { // Max 10%
            return Err(b"Fee too high".to_vec());
        }
        
        self.transfer_fee_bps.set(fee_bps);
        Ok(())
    }
    
    fn do_transfer(&mut self, from: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        // Calculate fee
        let fee = amount * self.transfer_fee_bps.get() / U256::from(10000);
        let transfer_amount = amount - fee;
        
        let from_balance = self.balances.get(from);
        if from_balance < amount {
            return Err(b"ERC20: transfer amount exceeds balance".to_vec());
        }
        
        // Update balances
        self.balances.setter(from).set(from_balance - amount);
        
        let to_balance = self.balances.get(to);
        self.balances.setter(to).set(to_balance + transfer_amount);
        
        // Collect fee
        if fee > U256::ZERO {
            let fee_collector = self.fee_collector.get();
            let collector_balance = self.balances.get(fee_collector);
            self.balances.setter(fee_collector).set(collector_balance + fee);
        }
        
        evm::log(Transfer { from, to, value: transfer_amount });
        
        Ok(())
    }
}
```

## Token Interaction Patterns

### Multi-Token Contracts

```rust
#[storage]
#[entrypoint]
pub struct MultiTokenVault {
    token_balances: StorageMap<Address, StorageMap<Address, StorageU256>>, // user -> token -> balance
    supported_tokens: StorageVec<StorageAddress>,
    owner: StorageAddress,
}

#[public]
impl MultiTokenVault {
    pub fn deposit_token(&mut self, token: Address, amount: U256) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        
        // Interface for ERC-20 token
        let erc20 = IERC20::new(token);
        
        // Transfer tokens from user to contract
        let config = Call::new_in(self);
        let success = erc20.transfer_from(config, user, self.vm().contract_address(), amount)
            .map_err(|_| b"Token transfer failed".to_vec())?;
            
        if !success {
            return Err(b"Transfer returned false".to_vec());
        }
        
        // Update internal balance
        let current_balance = self.token_balances.getter(user).get(token);
        self.token_balances.setter(user).insert(token, current_balance + amount);
        
        Ok(())
    }
    
    pub fn withdraw_token(&mut self, token: Address, amount: U256) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        let balance = self.token_balances.getter(user).get(token);
        
        if balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update internal balance first
        self.token_balances.setter(user).insert(token, balance - amount);
        
        // Transfer tokens back to user
        let erc20 = IERC20::new(token);
        let config = Call::new_in(self);
        let success = erc20.transfer(config, user, amount)
            .map_err(|_| b"Token transfer failed".to_vec())?;
            
        if !success {
            // Revert internal balance change
            self.token_balances.setter(user).insert(token, balance);
            return Err(b"Transfer failed".to_vec());
        }
        
        Ok(())
    }
}

// Standard ERC-20 interface
sol_interface! {
    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
        function balanceOf(address account) external view returns (uint256);
    }
}
```

### Token Wrapper/Bridge Pattern

```rust
#[storage]
#[entrypoint]
pub struct WrappedToken {
    balances: StorageMap<Address, StorageU256>,
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    total_supply: StorageU256,
    name: StorageString,
    symbol: StorageString,
    decimals: StorageU8,
    underlying_token: StorageAddress,
    processed_deposits: StorageMap<FixedBytes<32>, StorageBool>,
}

#[public]
impl WrappedToken {
    #[constructor]
    pub fn constructor(
        &mut self,
        underlying: Address,
        name: String,
        symbol: String,
    ) {
        self.underlying_token.set(underlying);
        self.name.set_str(&name);
        self.symbol.set_str(&symbol);
        
        // Get decimals from underlying token
        let erc20 = IERC20Metadata::new(underlying);
        let decimals = erc20.decimals(self).unwrap_or(18);
        self.decimals.set(decimals);
    }
    
    pub fn deposit(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        
        // Transfer underlying tokens to this contract
        let underlying = IERC20::new(self.underlying_token.get());
        let config = Call::new_in(self);
        let success = underlying.transfer_from(config, user, self.vm().contract_address(), amount)
            .map_err(|_| b"Underlying transfer failed".to_vec())?;
            
        if !success {
            return Err(b"Transfer failed".to_vec());
        }
        
        // Mint wrapped tokens
        self.total_supply.set(self.total_supply.get() + amount);
        let balance = self.balances.get(user);
        self.balances.setter(user).set(balance + amount);
        
        evm::log(Transfer {
            from: Address::ZERO,
            to: user,
            value: amount,
        });
        
        Ok(())
    }
    
    pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        let balance = self.balances.get(user);
        
        if balance < amount {
            return Err(b"Insufficient wrapped balance".to_vec());
        }
        
        // Burn wrapped tokens
        self.balances.setter(user).set(balance - amount);
        self.total_supply.set(self.total_supply.get() - amount);
        
        // Transfer underlying tokens back
        let underlying = IERC20::new(self.underlying_token.get());
        let config = Call::new_in(self);
        let success = underlying.transfer(config, user, amount)
            .map_err(|_| b"Underlying transfer failed".to_vec())?;
            
        if !success {
            // Revert burn
            self.balances.setter(user).set(balance);
            self.total_supply.set(self.total_supply.get() + amount);
            return Err(b"Transfer failed".to_vec());
        }
        
        evm::log(Transfer {
            from: user,
            to: Address::ZERO,
            value: amount,
        });
        
        Ok(())
    }
}

sol_interface! {
    interface IERC20Metadata {
        function decimals() external view returns (uint8);
    }
}
```

## Best Practices

### 1. Follow ERC-20 Standard Exactly
```rust
// Always return bool from transfer functions
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, Vec<u8>> {
    self.do_transfer(msg::sender(), to, amount)?;
    Ok(true) // Must return true on success
}
```

The ERC-20 standard requires Transfer and Approval events and boolean return values on transfer, transferFrom, and approve methods. ABI names must match exactly; use `#[selector(name = "...")]` if you keep Rust snake_case naming conventions.

### 2. Emit Events for All State Changes
```rust
fn do_transfer(&mut self, from: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
    // ... update balances
    
    evm::log(Transfer { from, to, value: amount });
    Ok(())
}
```

### 3. Check All Inputs
```rust
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, Vec<u8>> {
    if to == Address::ZERO {
        return Err(b"ERC20: transfer to zero address".to_vec());
    }
    if amount == U256::ZERO {
        // Optional: allow zero transfers for compatibility
        // return Err(b"Cannot transfer zero amount".to_vec());
    }
    
    // ... rest of logic
}
```

### 4. Use Safe Math Operations
```rust
// EVM arithmetic wraps modulo 2²⁵⁶. Use explicit checks or checked_*/saturating_* helpers when needed.

// Add with an explicit overflow check
let new_balance = from_balance.checked_sub(amount)
    .ok_or_else(|| b"ERC20: transfer amount exceeds balance".to_vec())?;
self.balances.setter(from).set(new_balance);

// Example for addition:
let new_to = self.balances.get(to).checked_add(amount)
    .ok_or_else(|| b"ERC20: balance overflow".to_vec())?;
self.balances.setter(to).set(new_to);
```

### 5. Implement Standard Optional Extensions
```rust
// ERC-20 Metadata (already standard)
pub fn name(&self) -> String { /* ... */ }
pub fn symbol(&self) -> String { /* ... */ }
pub fn decimals(&self) -> u8 { /* ... */ }

// ERC-20 Permit (EIP-2612) - for gasless approvals
pub fn permit(
    &mut self,
    owner: Address,
    spender: Address,
    value: U256,
    deadline: U256,
    v: u8,
    r: B256,
    s: B256,
) -> Result<(), Vec<u8>> {
    // Implement EIP-712 signature verification
    // Update allowance without requiring transaction from owner
}
```

## Migration Checklist

### Analysis Phase
- [ ] Identify all SPL Token operations
- [ ] Map token mint configurations to ERC-20 parameters
- [ ] Document transfer patterns and authorities
- [ ] List any custom token features (fees, pausing, etc.)
- [ ] Note any token-2022 extensions used

### Implementation Phase  
- [ ] Create ERC-20 contract with appropriate storage
- [ ] Implement standard ERC-20 methods
- [ ] Add minting/burning functionality if needed
- [ ] Migrate custom features to contract methods
- [ ] Add proper event emission
- [ ] Implement any required access control

### Testing Phase
- [ ] Test all ERC-20 standard methods
- [ ] Verify transfer and allowance mechanisms
- [ ] Test minting and burning operations
- [ ] Check custom features work correctly
- [ ] Verify events are emitted properly
- [ ] Test edge cases (zero transfers, self-transfers)
- [ ] Check gas costs remain reasonable

## Common Pitfalls

### Missing Return Values
```rust
// ERC-20 requires bool returns
pub fn transfer(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> // Incorrect

pub fn transfer(&mut self, to: Address, amount: U256) -> Result<bool, Vec<u8>> // Correct
```

### Not Emitting Events
```rust
// Must emit Transfer events
self.balances.setter(from).set(from_balance - amount);
self.balances.setter(to).set(to_balance + amount);
// Missing: evm::log(Transfer { from, to, value: amount });
```

### Integer Overflow/Underflow
```rust
// Dangerous in older Solidity - can overflow
let new_balance = self.balances.get(to) + amount;

// Safe in Stylus - U256 has built-in protection
let new_balance = self.balances.get(to) + amount; // Safe

// For explicit checking:
let new_balance = current_balance.checked_add(amount)
    .ok_or(b"Balance overflow".to_vec())?; // Safe with explicit check
```

### Incorrect Event Parameters
```rust
// Wrong: Using contract address as 'from' for minting
evm::log(Transfer {
    from: self.vm().contract_address(), // Incorrect
    to,
    value: amount,
});

// Correct: Use zero address for minting
evm::log(Transfer {
    from: Address::ZERO, // Correct
    to,
    value: amount,
});
```

### Not Checking External Call Results
```rust
// Dangerous: Not checking return value
let _ = erc20.transfer(config, user, amount);

// Safe: Check return value
let success = erc20.transfer(config, user, amount)
    .map_err(|_| b"Transfer call failed".to_vec())?;
if !success {
    return Err(b"Transfer returned false".to_vec());
}
```

## Migration Patterns

### Pattern: Authority Migration
```rust
// Solana: Separate mint authority
pub struct Mint {
    pub mint_authority: COption<Pubkey>,
    pub freeze_authority: COption<Pubkey>,
}

// Stylus: Role-based access control
#[storage]
pub struct TokenWithRoles {
    balances: StorageMap<Address, StorageU256>,
    owner: StorageAddress,
    minters: StorageMap<Address, StorageBool>,
    pausers: StorageMap<Address, StorageBool>,
}

#[public]
impl TokenWithRoles {
    pub fn add_minter(&mut self, account: Address) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can add minters".to_vec());
        }
        self.minters.setter(account).set(true);
        Ok(())
    }
    
    pub fn mint(&mut self, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        if !self.minters.get(msg::sender()) {
            return Err(b"Caller is not a minter".to_vec());
        }
        // ... minting logic
    }
}
```

### Pattern: Associated Token Accounts → Direct Balances
```rust
// Solana: Create ATA first, then transfer
let ata = get_associated_token_address(&wallet, &mint);
create_associated_token_account_if_needed(&ata, &wallet, &mint)?;
transfer_tokens(&source_ata, &ata, amount)?;

// Stylus: Mappings default to zero; no account creation step is required.
token.transfer(recipient, amount)?; // Automatically handles balance
```

### Pattern: Program-Owned Accounts → Contract Balance
```rust
// Solana: PDA owns token account
let (pda, bump) = Pubkey::find_program_address(&[b"vault"], &program_id);
let pda_ata = get_associated_token_address(&pda, &mint);

// Stylus: Contract itself holds tokens
contract_balance = token.balanceOf(self.vm().contract_address());
```

## Next Steps

With fungible tokens covered, the next chapter explores [Non-Fungible Token Handling](./non-fungible-tokens.md) - migrating from Metaplex NFTs to ERC-721 patterns in Stylus.

## Reference

- [Example Code: fungible-tokens](/examples/concepts/fungible-tokens/)
- [ERC-20 Standard](https://eips.ethereum.org/EIPS/eip-20)
- [OpenZeppelin ERC-20 Implementation](https://docs.openzeppelin.com/contracts/erc20)
- [SPL Token Program](https://spl.solana.com/token)
- [Token-2022 Program](https://spl.solana.com/token-2022)