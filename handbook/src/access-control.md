# Access Control

This chapter demonstrates how to translate Solana's signer checks and PDA patterns to Stylus. Learn how to verify callers with `msg::sender()`, model roles in storage, and replace PDAs with mappings and contract-controlled logic.

## Authentication Model Comparison

### Solana Authentication
- **Signer Verification**: Check `account.is_signer` field or use `Signer<'info>` type
- **Program Derived Addresses**: Use PDAs for programmatic authority
- **Account Ownership**: Verify account owner matches expected program
- **Manual Validation**: Explicit checks in instruction handlers

### Stylus Authentication
- **Message Sender**: Use `msg::sender()` for caller identification
- **Storage-Based Roles**: Create role systems in contract storage
- **Address-Based Permissions**: Direct address comparison for access control
- **Security Note**: Use only `msg::sender()` for authorization, never `tx::origin()`
- **Context Functions**: Access `msg::sender()` and `block::timestamp()` as functions

## Basic Authentication Patterns

### Signer Verification Migration

**Solana Native:**

```rust
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Data {
    pub authority: Pubkey,
    pub value: u64,
}

pub fn process_update(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let data_account = next_account_info(accounts_iter)?;
    let authority_account = next_account_info(accounts_iter)?;
    
    // Verify signer
    if !authority_account.is_signer {
        msg!("Authority must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    // Verify account ownership
    if data_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Load and verify authority
    let mut data = Data::try_from_slice(&data_account.data.borrow())?;
    if data.authority != *authority_account.key {
        msg!("Wrong authority");
        return Err(ProgramError::InvalidAccountData);
    }
    
    // Update data
    data.value = new_value;
    data.serialize(&mut *data_account.data.borrow_mut())?;
    
    Ok(())
}
```

**Anchor:**
```rust
use anchor_lang::prelude::*;

#[account]
pub struct DataAccount {
    pub authority: Pubkey,
    pub value: u64,
}

#[derive(Accounts)]
pub struct UpdateData<'info> {
    #[account(
        mut,
        has_one = authority @ ErrorCode::InvalidAuthority
    )]
    pub data_account: Account<'info, DataAccount>,
    pub authority: Signer<'info>,
}

#[program]
pub mod my_program {
    use super::*;
    
    pub fn update_data(ctx: Context<UpdateData>, new_value: u64) -> Result<()> {
        // Authority is automatically verified by Anchor's has_one constraint
        ctx.accounts.data_account.value = new_value;
        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid authority")]
    InvalidAuthority,
}
```

**Stylus:**
```rust
use stylus_sdk::{
    alloy_primitives::{Address, U256},
    msg, prelude::*, evm,
};
use alloy_sol_types::sol;

#[storage]
#[entrypoint]
pub struct DataContract {
    authority: StorageAddress,
    value: StorageU256,
}

sol! {
    error Unauthorized(address caller);
    event AuthorityUpdated(address indexed oldAuth, address indexed newAuth);
    event ValueUpdated(uint256 newValue);
}

#[public]
impl DataContract {
    #[constructor]
    pub fn constructor(&mut self, initial_value: U256) {
        self.authority.set(msg::sender());
        self.value.set(initial_value);
        evm::log(ValueUpdated { newValue: initial_value });
    }

    pub fn update_data(&mut self, new_value: U256) -> Result<(), Unauthorized> {
        if msg::sender() != self.authority.get() {
            return Err(Unauthorized { caller: msg::sender() });
        }
        self.value.set(new_value);
        evm::log(ValueUpdated { newValue: new_value });
        Ok(())
    }

    pub fn transfer_authority(&mut self, new_auth: Address) -> Result<(), Unauthorized> {
        if msg::sender() != self.authority.get() {
            return Err(Unauthorized { caller: msg::sender() });
        }
        let old = self.authority.get();
        self.authority.set(new_auth);
        evm::log(AuthorityUpdated { oldAuth: old, newAuth: new_auth });
        Ok(())
    }

    pub fn get_authority(&self) -> Address { 
        self.authority.get() 
    }
    
    pub fn get_value(&self) -> U256 { 
        self.value.get() 
    }
}
```

## Role-Based Access Control

### Simple Owner Pattern

**Stylus Implementation:**
```rust
use stylus_sdk::{
    alloy_primitives::{Address, U256},
    msg,
    prelude::*,
};
use alloy_sol_types::sol;

sol! { 
    error OnlyOwner(address caller); 
}

#[storage]
#[entrypoint]
pub struct Owned {
    owner: StorageAddress,
    data: StorageU256,
}

#[public]
impl Owned {
    #[constructor]
    pub fn constructor(&mut self) { 
        self.owner.set(msg::sender()); 
    }

    pub fn update_data(&mut self, new_data: U256) -> Result<(), OnlyOwner> {
        self.only_owner()?;
        self.data.set(new_data);
        Ok(())
    }

    pub fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), OnlyOwner> {
        self.only_owner()?;
        if new_owner == Address::ZERO { 
            revert(b"zero address") 
        }
        self.owner.set(new_owner);
        Ok(())
    }
}

impl Owned {
    fn only_owner(&self) -> Result<(), OnlyOwner> {
        if msg::sender() != self.owner.get() {
            return Err(OnlyOwner { caller: msg::sender() });
        }
        Ok(())
    }
}
```

### Multi-Role System

**Advanced Role Management:**
```rust
use stylus_sdk::{
    alloy_primitives::{Address, U256},
    block, evm, msg, prelude::*,
};
use alloy_sol_types::sol;

sol! {
    event RoleGranted(address indexed account, uint256 roleMask);
    event RoleRevoked(address indexed account, uint256 roleMask);
    error NotOwner(address caller);
    error InvalidRole();
}

#[storage]
#[entrypoint]
pub struct RoleManaged {
    owner: StorageAddress,
    roles: StorageMap<Address, StorageU256>,        // bitmask
    granted_at: StorageMap<Address, StorageU256>,   // when any role last changed
    data: StorageU256,
}

impl RoleManaged {
    const ADMIN: U256    = U256::from_limbs([1, 0, 0, 0]);       // 1 << 0
    const OPERATOR: U256 = U256::from_limbs([2, 0, 0, 0]);       // 1 << 1
    const USER: U256     = U256::from_limbs([4, 0, 0, 0]);       // 1 << 2
}

#[public]
impl RoleManaged {
    #[constructor]
    pub fn constructor(&mut self) {
        let s = msg::sender();
        self.owner.set(s);
        self.roles.setter(s).set(Self::ADMIN);
        self.granted_at.setter(s).set(U256::from(block::timestamp()));
    }

    pub fn grant_role(&mut self, user: Address, role_mask: U256) -> Result<(), NotOwner> {
        self.only_owner()?;
        if role_mask == U256::ZERO { 
            revert(InvalidRole()); 
        }
        let current = self.roles.get(user);
        self.roles.setter(user).set(current | role_mask);
        self.granted_at.setter(user).set(U256::from(block::timestamp()));
        evm::log(RoleGranted { account: user, roleMask: role_mask });
        Ok(())
    }

    pub fn revoke_role(&mut self, user: Address, role_mask: U256) -> Result<(), NotOwner> {
        self.only_owner()?;
        let current = self.roles.get(user);
        self.roles.setter(user).set(current & !role_mask);
        evm::log(RoleRevoked { account: user, roleMask: role_mask });
        Ok(())
    }

    pub fn admin_function(&mut self, v: U256) -> Result<(), NotOwner> {
        self.require(Self::ADMIN)?;
        self.data.set(v);
        Ok(())
    }

    pub fn operator_function(&mut self, inc: U256) -> Result<(), NotOwner> {
        self.require(Self::OPERATOR)?;
        self.data.set(self.data.get() + inc);
        Ok(())
    }

    pub fn user_function(&self) -> Result<U256, NotOwner> {
        self.require(Self::USER)?;
        Ok(self.data.get())
    }

    pub fn has_role(&self, user: Address, role_mask: U256) -> bool {
        (self.roles.get(user) & role_mask) != U256::ZERO
    }

    fn only_owner(&self) -> Result<(), NotOwner> {
        if msg::sender() != self.owner.get() { 
            return Err(NotOwner { caller: msg::sender() }); 
        }
        Ok(())
    }

    fn require(&self, mask: U256) -> Result<(), NotOwner> {
        if !self.has_role(msg::sender(), mask) { 
            return Err(NotOwner { caller: msg::sender() }); 
        }
        Ok(())
    }
}
```

## PDA to Storage Pattern Migration

Program Derived Addresses (PDAs) in Solana serve as both unique identifiers and program-controlled authorities. In Stylus, we achieve similar functionality through storage mappings and contract-controlled logic.

### Solana PDA Pattern

**Anchor with PDAs:**
```rust
use anchor_lang::prelude::*;

#[account]
pub struct UserVault {
    pub owner: Pubkey,
    pub balance: u64,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct CreateUserVault<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + 8 + 1,
        seeds = [b"user-vault", owner.key().as_ref()],
        bump
    )]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"user-vault", owner.key().as_ref()],
        bump = user_vault.bump,
        has_one = owner
    )]
    pub user_vault: Account<'info, UserVault>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: This is the program's vault authority
    #[account(
        seeds = [b"vault-authority"],
        bump
    )]
    pub vault_authority: AccountInfo<'info>,
}

#[program]
pub mod vault_program {
    use super::*;
    
    pub fn create_user_vault(ctx: Context<CreateUserVault>) -> Result<()> {
        let user_vault = &mut ctx.accounts.user_vault;
        user_vault.owner = ctx.accounts.owner.key();
        user_vault.balance = 0;
        user_vault.bump = ctx.bumps.user_vault;
        Ok(())
    }
    
    pub fn deposit(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.user_vault.balance = ctx.accounts.user_vault.balance
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        Ok(())
    }
}
```

**Stylus with Storage Mappings:**
```rust
use stylus_sdk::{
    alloy_primitives::{Address, U256},
    block, evm, msg,
    prelude::*,
    call,
};
use alloy_sol_types::sol;

sol! {
    event VaultCreated(address indexed owner, uint256 timestamp);
    event Deposit(address indexed owner, uint256 amount);
    event Withdrawal(address indexed owner, uint256 amount);
}

#[storage]
#[entrypoint]
pub struct VaultManager {
    // Direct user mapping - replaces PDA lookup
    vaults: StorageMap<Address, UserVault>,
    // Contract authority for vault operations
    vault_authority: StorageAddress,
    // Track vault existence
    vault_exists: StorageMap<Address, StorageBool>,
    // Global statistics
    total_deposits: StorageU256,
    total_vaults: StorageU256,
}

#[storage]
pub struct UserVault {
    balance: StorageU256,
    created_at: StorageU256,
    last_activity: StorageU256,
    locked: StorageBool,
}

#[public]
impl VaultManager {
    #[constructor]
    pub fn constructor(&mut self) {
        if self.vault_authority.get() != Address::ZERO { 
            revert(b"initialized"); 
        }
        // Set the deployer as the authority
        self.vault_authority.set(msg::sender());
    }
    
    /// Create a new user vault
    pub fn create_user_vault(&mut self) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        
        // Check if vault already exists
        if self.vault_exists.get(user) {
            return Err(b"Vault already exists".to_vec());
        }
        
        // Create vault directly in mapping
        let mut vault = self.vaults.setter(user);
        vault.balance.set(U256::ZERO);
        vault.created_at.set(U256::from(block::timestamp()));
        vault.last_activity.set(U256::from(block::timestamp()));
        vault.locked.set(false);
        
        // Mark as existing
        self.vault_exists.setter(user).set(true);
        
        // Update global stats
        let current_total = self.total_vaults.get();
        self.total_vaults.set(current_total + U256::from(1));
        
        // Emit event
        evm::log(VaultCreated {
            owner: user,
            timestamp: U256::from(block::timestamp()),
        });
        
        Ok(())
    }
    
    /// Deposit funds into vault
    #[payable]
    pub fn deposit(&mut self) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        let amount = msg::value();
        
        // Verify vault exists
        if !self.vault_exists.get(user) {
            return Err(b"Vault does not exist".to_vec());
        }
        
        // Check vault not locked
        if self.vaults.getter(user).locked.get() {
            return Err(b"Vault is locked".to_vec());
        }
        
        // Update balance
        let mut vault = self.vaults.setter(user);
        let current_balance = vault.balance.get();
        vault.balance.set(current_balance + amount);
        vault.last_activity.set(U256::from(block::timestamp()));
        
        // Update global deposits
        let total = self.total_deposits.get();
        self.total_deposits.set(total + amount);
        
        evm::log(Deposit {
            owner: user,
            amount,
        });
        
        Ok(())
    }
    
    /// Withdraw funds from vault
    pub fn withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let user = msg::sender();
        
        // Verify vault exists
        if !self.vault_exists.get(user) {
            return Err(b"Vault does not exist".to_vec());
        }
        
        let mut vault = self.vaults.setter(user);
        
        // Check vault not locked
        if vault.locked.get() {
            return Err(b"Vault is locked".to_vec());
        }
        
        // Check sufficient balance
        let current_balance = vault.balance.get();
        if current_balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // Update balance
        vault.balance.set(current_balance - amount);
        vault.last_activity.set(U256::from(block::timestamp()));
        
        // Transfer ETH to user
        call::transfer_eth(user, amount)?;
        
        // Update global deposits
        let total = self.total_deposits.get();
        self.total_deposits.set(total - amount);
        
        evm::log(Withdrawal {
            owner: user,
            amount,
        });
        
        Ok(())
    }
    
    /// Admin function to lock/unlock a vault
    pub fn set_vault_lock(&mut self, user: Address, locked: bool) -> Result<(), Vec<u8>> {
        // Only authority can lock vaults
        if msg::sender() != self.vault_authority.get() {
            return Err(b"Not authorized".to_vec());
        }
        
        if !self.vault_exists.get(user) {
            return Err(b"Vault does not exist".to_vec());
        }
        
        self.vaults.setter(user).locked.set(locked);
        Ok(())
    }
    
    /// Get vault info
    pub fn get_vault_info(&self, user: Address) -> Result<(U256, U256, bool), Vec<u8>> {
        if !self.vault_exists.get(user) {
            return Err(b"Vault does not exist".to_vec());
        }
        
        let vault = self.vaults.getter(user);
        Ok((
            vault.balance.get(),
            vault.created_at.get(),
            vault.locked.get(),
        ))
    }
}
```

Note: Stylus forwards all gas on `transfer_eth`. Consider using low-level call with a gas cap or implementing a pull-withdraw pattern for defense-in-depth.

## Advanced Access Control Patterns

### Time-Locked Operations

```rust
use stylus_sdk::{alloy_primitives::FixedBytes, abi::Bytes, call::Call};

#[storage]
#[entrypoint]
pub struct TimeLocked {
    owner: StorageAddress,
    requests: StorageMap<FixedBytes<32>, TimeLockRequest>,
    delay_period: StorageU256,
}

#[storage]
pub struct TimeLockRequest {
    target: StorageAddress,
    selector: StorageFixedBytes<4>,
    data: StorageBytes,
    execute_after: StorageU256,
    executed: StorageBool,
}

#[public]
impl TimeLocked {
    pub fn queue_operation(
        &mut self,
        target: Address,
        selector: [u8; 4],
        data: Bytes,
    ) -> Result<[u8; 32], Vec<u8>> {
        self.only_owner()?;
        
        // Create operation hash
        let operation_id = self.hash_operation(target, selector, &data);
        
        // Check not already queued
        if self.requests.getter(FixedBytes::from(operation_id)).execute_after.get() != U256::ZERO {
            return Err(b"Operation already queued".to_vec());
        }
        
        // Queue operation
        let mut request = self.requests.setter(FixedBytes::from(operation_id));
        request.target.set(target);
        request.selector.set(FixedBytes::from(selector));
        request.data.set_bytes(data);
        request.execute_after.set(U256::from(block::timestamp()) + self.delay_period.get());
        request.executed.set(false);
        
        Ok(operation_id)
    }
    
    pub fn execute_operation(&mut self, operation_id: [u8; 32]) -> Result<Vec<u8>, Vec<u8>> {
        let key = FixedBytes::from(operation_id);
        let mut request = self.requests.setter(key);
        
        // Verify time lock passed
        if U256::from(block::timestamp()) < request.execute_after.get() {
            return Err(b"Time lock not expired".to_vec());
        }
        
        // Verify not already executed
        if request.executed.get() {
            return Err(b"Already executed".to_vec());
        }
        
        // Mark as executed
        request.executed.set(true);
        
        // Execute the call
        let target = request.target.get();
        let selector = request.selector.get();
        let data = request.data.get_bytes();
        
        // Prepare calldata
        let mut calldata = Vec::with_capacity(4 + data.len());
        calldata.extend_from_slice(&selector.0);
        calldata.extend_from_slice(&data);
        
        // Make the call
        let result = call::call(Call::new_in(self), target, &calldata)?;
        
        Ok(result)
    }
}

use alloy_primitives::utils::keccak256;

impl TimeLocked {
    fn hash_operation(&self, target: Address, selector: [u8; 4], data: &Bytes) -> [u8; 32] {
        // abi.encodePacked(target, selector, data)
        let mut buf = Vec::with_capacity(20 + 4 + data.len());
        buf.extend_from_slice(target.as_slice());
        buf.extend_from_slice(&selector);
        buf.extend_from_slice(data);
        keccak256(buf).0
    }
}
```

### Hierarchical Permissions

```rust
#[storage]
#[entrypoint]
pub struct HierarchicalAccess {
    super_admin: StorageAddress,
    admins: StorageMap<Address, StorageBool>,
    function_permissions: StorageMap<Address, StorageMap<FixedBytes<4>, StorageBool>>,
    permission_expiry: StorageMap<Address, StorageU256>,
}

#[public]
impl HierarchicalAccess {
    /// Grant temporary permission to call specific function
    pub fn grant_temporary_permission(
        &mut self,
        user: Address,
        selector: [u8; 4],
        duration: U256,
    ) -> Result<(), Vec<u8>> {
        // Only admins can grant permissions
        if !self.is_admin(msg::sender()) {
            return Err(b"Not an admin".to_vec());
        }
        
        // Set permission
        let selector_key = FixedBytes::from(selector);
        self.function_permissions
            .setter(user)
            .setter(selector_key)
            .set(true);
        
        // Set expiry
        let expiry = U256::from(block::timestamp()) + duration;
        self.permission_expiry.setter(user).set(expiry);
        
        Ok(())
    }
    
    /// Check if user can call function
    pub fn can_call(&self, user: Address, selector: [u8; 4]) -> bool {
        // Super admin can always call
        if user == self.super_admin.get() {
            return true;
        }
        
        // Regular admins can call
        if self.admins.get(user) {
            return true;
        }
        
        // Check specific permission
        let selector_key = FixedBytes::from(selector);
        if self.function_permissions.getter(user).get(selector_key) {
            // Check if not expired
            let expiry = self.permission_expiry.get(user);
            if expiry != U256::ZERO && U256::from(block::timestamp()) > expiry {
                return false;
            }
            return true;
        }
        
        false
    }
}

impl HierarchicalAccess {
    fn is_admin(&self, user: Address) -> bool {
        user == self.super_admin.get() || self.admins.get(user)
    }
}
```

## Security Considerations

### Common Vulnerabilities and Mitigations

Stylus reverts on reentrancy by default unless compiled with the reentrant feature. Do not assume this protection if you enable that feature.

For signature-based authentication, use ecrecover semantics via Alloy primitives to verify off-chain signatures. Do not attempt to infer signatures from the transaction.

**1. Missing Access Control**
```rust
// Vulnerable
pub fn critical_function(&mut self) -> Result<(), Vec<u8>> {
    // No access control!
    self.perform_critical_action()
}

// Secure
pub fn critical_function(&mut self) -> Result<(), Vec<u8>> {
    self.only_owner()?;  // Always check permissions first
    self.perform_critical_action()
}
```

**2. Centralization Risks**
```rust
// Implement decentralized control
#[storage]
pub struct DecentralizedControl {
    guardians: StorageVec<StorageAddress>,
    required_confirmations: StorageU256,
    confirmations: StorageMap<FixedBytes<32>, StorageMap<Address, StorageBool>>,
}

impl DecentralizedControl {
    pub fn execute_with_consensus(&mut self, action_id: [u8; 32]) -> Result<(), Vec<u8>> {
        let confirmations = self.count_confirmations(action_id);
        if confirmations < self.required_confirmations.get() {
            return Err(b"Insufficient confirmations".to_vec());
        }
        
        // Execute action
        Ok(())
    }
}
```

**3. Privilege Escalation**
```rust
// Vulnerable to privilege escalation
pub fn grant_admin(&mut self, user: Address) -> Result<(), Vec<u8>> {
    // Anyone can become admin!
    self.admins.setter(user).set(true);
    Ok(())
}

// Secure
pub fn grant_admin(&mut self, user: Address) -> Result<(), Vec<u8>> {
    // Only existing admin can grant admin
    if !self.is_admin(msg::sender()) {
        return Err(b"Not authorized".to_vec());
    }
    
    // Additional checks
    if user == Address::ZERO {
        return Err(b"Invalid address".to_vec());
    }
    
    self.admins.setter(user).set(true);
    Ok(())
}
```

## Migration Checklist

### Analysis Phase
- [ ] Map all Solana signer checks to Stylus patterns
- [ ] Identify PDA usage patterns
- [ ] Document authority relationships
- [ ] Plan permission hierarchy
- [ ] List all privileged operations

### Implementation Phase
- [ ] Replace `is_signer` with `msg::sender()` checks
- [ ] Convert PDAs to storage mappings
- [ ] Implement role-based access control
- [ ] Add ownership transfer mechanisms
- [ ] Create permission helper functions
- [ ] Add time-lock for critical operations

### Testing Phase
- [ ] Test unauthorized access attempts
- [ ] Verify role inheritance works correctly
- [ ] Test ownership transfers
- [ ] Check time-locked operations
- [ ] Simulate privilege escalation attempts
- [ ] Audit all access control paths

### Security Review
- [ ] Ensure fail-secure defaults
- [ ] Check for centralization risks
- [ ] Verify no backdoors exist
- [ ] Test emergency pause mechanisms
- [ ] Review permission granularity

## Key Takeaways

1. **Direct Mapping**: Solana's signer checks map directly to `msg::sender()` verification
2. **Storage-Based Roles**: Replace PDAs with storage mappings for role management
3. **Hierarchical Permissions**: Implement flexible permission systems using mappings
4. **Time Locks**: Add delays for critical operations to enhance security
5. **Fail Secure**: Always default to denying access when in doubt

## Next Steps

With access control patterns established, the next chapter covers [External Calls](./external-calls.md) - converting Solana's Cross-Program Invocations (CPIs) to Stylus contract calls.

## References

- [Example Code: Access Control](/examples/concepts/access-control/)
- [Stylus SDK Docs - Message Context](https://docs.rs/stylus-sdk/latest/stylus_sdk/msg/index.html)
- [Stylus SDK Docs - Storage](https://docs.rs/stylus-sdk/latest/stylus_sdk/storage/index.html)
- [Stylus SDK Docs - Events](https://docs.rs/stylus-sdk/latest/stylus_sdk/evm/fn.log.html)
- [Solana Documentation - Signer Verification](https://docs.solana.com/developing/programming-model/accounts#signers)
- [OpenZeppelin Access Control Patterns](https://docs.openzeppelin.com/contracts/access-control)
- [Alloy Primitives - Keccak256](https://docs.rs/alloy-primitives/latest/alloy_primitives/utils/fn.keccak256.html)
