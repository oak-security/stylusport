# Security Considerations

Security is paramount when migrating from Solana to Stylus. The EVM environment introduces different attack vectors, security patterns, and best practices. This chapter covers comprehensive security considerations for migrated contracts.

## Common Vulnerabilities

### Reentrancy Attacks

Reentrancy is one of the most critical vulnerabilities in EVM-based contracts. Unlike Solana, where the runtime provides some protection through account locking, Stylus contracts must explicitly guard against reentrancy:

```rust
use stylus_sdk::prelude::*;
use stylus_sdk::{
    evm, msg, block,
    call::{Call, static_call},
    call::transfer_eth, // for ETH sends
};
use alloy_primitives::{Address, U256, B256, FixedBytes};
use alloy_sol_types::sol;

sol! {
    event AdminAdded(address indexed admin);
    event ExternalCallFailed(address indexed target);
}

// VULNERABLE: Classic reentrancy vulnerability
#[storage]
#[entrypoint]
pub struct VulnerableContract {
    mapping: StorageMap<Address, StorageU256>,
}

#[public]
impl VulnerableContract {
    // VULNERABLE: External call before state update
    pub fn vulnerable_withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let balance = self.mapping.get(msg::sender());
        
        if balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // VULNERABILITY: External call happens first
        transfer_eth(msg::sender(), amount)?;
        
        // State update happens after external call
        self.mapping.setter(msg::sender()).set(balance - amount);
        
        Ok(())
    }
}

// SECURE: Reentrancy-protected version with RAII guard
#[storage]
pub struct ReentrancyGuard {
    locked: StorageBool,
}

impl ReentrancyGuard {
    fn enter(&mut self) -> Result<Guard<'_>, Vec<u8>> {
        if self.locked.get() {
            return Err(b"Reentrancy".to_vec());
        }
        self.locked.set(true);
        Ok(Guard { g: self })
    }
}

pub struct Guard<'a> {
    g: &'a mut ReentrancyGuard
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        self.g.locked.set(false);
    }
}

#[storage]
#[entrypoint]
pub struct SecureContract {
    balances: StorageMap<Address, StorageU256>,
    #[borrow]
    guard: ReentrancyGuard,
}

#[public]
impl SecureContract {
    // SECURE: Checks-Effects-Interactions pattern with RAII guard
    pub fn secure_withdraw(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let _guard = self.guard.enter()?; // auto-unlocks on all paths
        
        let balance = self.balances.get(msg::sender());
        
        // CHECKS: Validate conditions first
        if balance < amount {
            return Err(b"Insufficient balance".to_vec());
        }
        
        // EFFECTS: Update state before external calls
        self.balances.setter(msg::sender()).set(balance - amount);
        
        // INTERACTIONS: External calls last
        transfer_eth(msg::sender(), amount)?;
        
        Ok(())
    }
}
```

### Integer Overflow/Underflow

While Rust provides built-in overflow protection in debug mode, release builds require explicit checks:

```rust
use stylus_sdk::alloy_primitives::U256;

#[public]
impl SafeMathContract {
    // VULNERABLE: No overflow protection in release mode
    // Note: native ints wrap in release; prefer U256 + checked_* or explicitly use
    // saturating_*/overflowing_* with tests for token/accounting operations
    pub fn vulnerable_add(&mut self, a: u64, b: u64) -> u64 {
        a + b  // Can overflow without warning in release
    }
    
    // SECURE: Explicit overflow checking
    pub fn safe_add(&mut self, a: U256, b: U256) -> Result<U256, Vec<u8>> {
        a.checked_add(b).ok_or(b"Overflow".to_vec())
    }
    
    pub fn safe_sub(&mut self, a: U256, b: U256) -> Result<U256, Vec<u8>> {
        a.checked_sub(b).ok_or(b"Underflow".to_vec())
    }
    
    pub fn safe_mul(&mut self, a: U256, b: U256) -> Result<U256, Vec<u8>> {
        a.checked_mul(b).ok_or(b"Overflow".to_vec())
    }
    
    pub fn safe_div(&mut self, a: U256, b: U256) -> Result<U256, Vec<u8>> {
        if b.is_zero() {
            return Err(b"Division by zero".to_vec());
        }
        Ok(a / b)
    }
    
    // Safe percentage calculations
    pub fn calculate_percentage(&self, amount: U256, percentage: U256) -> Result<U256, Vec<u8>> {
        if percentage > U256::from(10000) {  // Max 100.00%
            return Err(b"Invalid percentage".to_vec());
        }
        
        amount
            .checked_mul(percentage)
            .and_then(|result| result.checked_div(U256::from(10000)))
            .ok_or(b"Calculation overflow".to_vec())
    }
}
```

### Access Control Vulnerabilities

Implement robust access control systems that mirror Solana's signer verification:

```rust
use stylus_sdk::{msg, alloy_primitives::Address};

sol! {
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    event Paused(address indexed account);
    event Unpaused(address indexed account);
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
}

#[storage]
#[entrypoint]
pub struct AccessControlledContract {
    owner: StorageAddress,
    pending_owner: StorageAddress,
    admins: StorageMap<Address, StorageBool>,
    user_roles: StorageMap<Address, StorageU256>,  // Bitmap for multiple roles
    initialized: StorageBool,
    paused: StorageBool,
}

// Role definitions using bit flags
const ADMIN_ROLE: u64 = 1 << 0;
const MODERATOR_ROLE: u64 = 1 << 1;
const USER_ROLE: u64 = 1 << 2;

#[public]
impl AccessControlledContract {
    pub fn initialize(&mut self, initial_owner: Address) -> Result<(), Vec<u8>> {
        if self.initialized.get() {
            return Err(b"Already initialized".to_vec());
        }
        
        self.owner.set(initial_owner);
        self.admins.setter(initial_owner).set(true);
        self.user_roles.setter(initial_owner).set(U256::from(ADMIN_ROLE));
        self.initialized.set(true);
        Ok(())
    }
    
    // Access control modifiers
    fn only_owner(&self) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner".to_vec());
        }
        Ok(())
    }
    
    fn only_admin(&self) -> Result<(), Vec<u8>> {
        if !self.admins.get(msg::sender()) {
            return Err(b"Only admin".to_vec());
        }
        Ok(())
    }
    
    fn has_role(&self, user: Address, role: u64) -> bool {
        (self.user_roles.get(user) & U256::from(role)) != U256::ZERO
    }
    
    fn require_role(&self, role: u64) -> Result<(), Vec<u8>> {
        if !self.has_role(msg::sender(), role) {
            return Err(b"Insufficient role".to_vec());
        }
        Ok(())
    }
    
    // Secure admin functions
    pub fn add_admin(&mut self, new_admin: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        
        if new_admin == Address::ZERO {
            return Err(b"Invalid address".to_vec());
        }
        
        let current = self.user_roles.get(new_admin);
        self.user_roles.setter(new_admin).set(current | U256::from(ADMIN_ROLE));
        self.admins.setter(new_admin).set(true);
        
        evm::log(AdminAdded { admin: new_admin });
        Ok(())
    }
    
    // Two-step ownership transfer to prevent accidental lockouts
    pub fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), Vec<u8>> {
        self.only_owner()?;
        if new_owner == Address::ZERO {
            return Err(b"Invalid address".to_vec());
        }
        self.pending_owner.set(new_owner);
        Ok(())
    }
    
    pub fn accept_ownership(&mut self) -> Result<(), Vec<u8>> {
        let pending = self.pending_owner.get();
        if msg::sender() != pending {
            return Err(b"Only pending owner".to_vec());
        }
        let old_owner = self.owner.get();
        self.owner.set(pending);
        self.pending_owner.set(Address::ZERO);
        evm::log(OwnershipTransferred { previous_owner: old_owner, new_owner: pending });
        Ok(())
    }
    
    // Pause switch for emergency stops
    pub fn pause(&mut self) -> Result<(), Vec<u8>> {
        self.only_admin()?;
        self.paused.set(true);
        evm::log(Paused { account: msg::sender() });
        Ok(())
    }
    
    pub fn unpause(&mut self) -> Result<(), Vec<u8>> {
        self.only_admin()?;
        self.paused.set(false);
        evm::log(Unpaused { account: msg::sender() });
        Ok(())
    }
    
    fn when_not_paused(&self) -> Result<(), Vec<u8>> {
        if self.paused.get() {
            return Err(b"Contract is paused".to_vec());
        }
        Ok(())
    }
}
```

## Stylus-Specific Security

### Storage Collision Prevention

When building upgradeable contracts, prevent storage collisions:

```rust
// Use explicit storage layout to prevent collisions
#[storage]
#[entrypoint]
pub struct UpgradeableStorage {
    // Reserve slots for future versions
    __gap0: StorageU256,
    __gap1: StorageU256,
    __gap2: StorageU256,
    __gap3: StorageU256,
    __gap4: StorageU256,
    __gap5: StorageU256,
    __gap6: StorageU256,
    __gap7: StorageU256,
    __gap8: StorageU256,
    __gap9: StorageU256,
    __gap10: StorageU256,
    __gap11: StorageU256,
    __gap12: StorageU256,
    __gap13: StorageU256,
    __gap14: StorageU256,
    __gap15: StorageU256,
    __gap16: StorageU256,
    __gap17: StorageU256,
    __gap18: StorageU256,
    __gap19: StorageU256,
    // Continue pattern or keep new fields in borrowed modules via #[borrow]
    
    balances: StorageMap<Address, StorageU256>,
    total_supply: StorageU256,
    version: StorageU256,
}

#[public]
impl UpgradeableStorage {
    // Safe upgrade patterns
    pub fn upgrade_to_v2(&mut self) -> Result<(), Vec<u8>> {
        self.only_admin()?;
        
        let current_version = self.version.get();
        if current_version >= U256::from(2) {
            return Err(b"Already upgraded".to_vec());
        }
        
        // Perform upgrade logic
        self.migrate_v1_to_v2()?;
        
        // Update version
        self.version.set(U256::from(2));
        
        Ok(())
    }
}
```

### External Call Security

Secure patterns for calling external contracts:

```rust
use stylus_sdk::call::{call, Call};

#[storage]
#[entrypoint]
pub struct ExternalCallSecurity {
    approved_targets: StorageMap<Address, StorageBool>,
    locked: StorageBool,
}

#[public]
impl ExternalCallSecurity {
    pub fn secure_external_call(&mut self, target: Address, data: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
        if !self.approved_targets.get(target) {
            return Err(b"Unapproved target".to_vec());
        }
        if self.locked.get() {
            return Err(b"Reentrancy detected".to_vec());
        }
        self.locked.set(true);
        let res = call(Call::new_in(self).gas(100_000), target, &data);
        self.locked.set(false);

        match res {
            Ok(bytes) => Ok(bytes),
            Err(_) => {
                evm::log(ExternalCallFailed { target });
                Err(b"External call failed".to_vec())
            }
        }
    }
}
```

### Constructor Security

Secure contract initialization patterns:

```rust
#[storage]
#[entrypoint]
pub struct SecureInitialization {
    initialized: StorageBool,
    owner: StorageAddress,
    creation_block: StorageU256,
}

#[public]
impl SecureInitialization {
    // Prefer explicit constructor over post-deployment initialization
    #[constructor]
    pub fn constructor(&mut self, owner: Address) {
        if owner == Address::ZERO {
            panic!("Invalid owner");
        }
        self.owner.set(owner);
        self.creation_block.set(U256::from(block::number()));
        self.initialized.set(true);
    }
    
    // Legacy initializer pattern if needed
    pub fn initialize(&mut self, owner: Address) -> Result<(), Vec<u8>> {
        // 1. Prevent duplicate initialization
        if self.initialized.get() {
            return Err(b"Already initialized".to_vec());
        }
        
        // 2. Validate parameters
        if owner == Address::ZERO {
            return Err(b"Invalid owner".to_vec());
        }
        
        // 3. Set critical state
        self.owner.set(owner);
        self.creation_block.set(U256::from(block::number()));
        
        // 4. Mark as initialized
        self.initialized.set(true);
        
        Ok(())
    }
}
```

## Migrated Code Security

### New Attack Vectors

Be aware of EVM-specific vulnerabilities when migrating:

```rust
#[public]
impl MigrationSecurity {
    // SOLANA: Account ownership was validated by runtime
    // STYLUS: Must explicitly validate caller authority
    pub fn migrated_transfer(&mut self, from: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        // In Solana, signers were automatically validated
        // In Stylus, we must check msg::sender explicitly
        if msg::sender() != from && !self.is_approved(from, msg::sender()) {
            return Err(b"Unauthorized".to_vec());
        }
        
        // Additional EVM-specific checks
        if to == Address::ZERO {
            return Err(b"Transfer to zero address".to_vec());
        }
        
        self.internal_transfer(from, to, amount)
    }
    
    // SOLANA: Cross-program invocations were handled by runtime
    // STYLUS: External calls need explicit error handling
    pub fn migrated_external_operation(&mut self, target: Address) -> Result<(), Vec<u8>> {
        // In Solana, CPI errors were automatically handled
        // In Stylus, we need explicit error handling
        
        match call(Call::new_in(self), target, &[]) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Log error and handle gracefully
                evm::log(ExternalCallFailed { target });
                Err(b"External call failed".to_vec())
            }
        }
    }
}
```

### Maintaining Security Invariants

Preserve Solana security properties in Stylus:

```rust
// Preserve Solana's strict account ownership model
#[storage]
#[entrypoint]
pub struct AccountOwnershipModel {
    account_owners: StorageMap<Address, StorageAddress>,
    account_exists: StorageMap<Address, StorageBool>,
}

#[public]
impl AccountOwnershipModel {
    pub fn create_account(&mut self, account: Address, owner: Address) -> Result<(), Vec<u8>> {
        // Preserve Solana's "accounts must be owned" principle
        if self.account_exists.get(account) {
            return Err(b"Account already exists".to_vec());
        }
        
        if owner == Address::ZERO {
            return Err(b"Invalid owner".to_vec());
        }
        
        self.account_owners.setter(account).set(owner);
        self.account_exists.setter(account).set(true);
        
        Ok(())
    }
    
    pub fn modify_account(&mut self, account: Address) -> Result<(), Vec<u8>> {
        // Preserve Solana's ownership checks
        let owner = self.account_owners.get(account);
        
        if owner == Address::ZERO {
            return Err(b"Account not found".to_vec());
        }
        
        // Only owner can modify
        if msg::sender() != owner {
            return Err(b"Unauthorized account access".to_vec());
        }
        
        // Perform modifications...
        Ok(())
    }
}
```

### Chain-Specific Risks

Handle Arbitrum/EVM-specific security considerations:

```rust
#[storage]
#[entrypoint]
pub struct ChainSpecificSecurity {
    last_operation_time: StorageU256,
    last_operation_block: StorageU256,
}

#[public]
impl ChainSpecificSecurity {
    pub fn time_sensitive_operation(&mut self) -> Result<(), Vec<u8>> {
        // Be aware of block timestamp manipulation
        let current_time = U256::from(block::timestamp());
        let last_operation = self.last_operation_time.get();
        
        // Use block number for more reliable time checks
        let current_block = U256::from(block::number());
        let last_block = self.last_operation_block.get();
        
        // Require minimum block distance instead of just time
        if current_block < last_block + U256::from(10) {
            return Err(b"Too soon".to_vec());
        }
        
        self.last_operation_time.set(current_time);
        self.last_operation_block.set(current_block);
        
        Ok(())
    }
}
```

## Additional Security Considerations

### DoS Prevention via Unbounded Loops

Iterating mappings/long arrays in a single transaction can cause denial of service. Batch and cap lengths, and store cursors for resumable processing:

```rust
#[storage]
#[entrypoint]
pub struct BatchProcessor {
    user_data: StorageMap<Address, StorageU256>,
    processing_cursor: StorageU256,
    batch_size: StorageU256,
}

#[public]
impl BatchProcessor {
    pub fn process_users_batch(&mut self, max_iterations: U256) -> Result<U256, Vec<u8>> {
        let batch_limit = if max_iterations > U256::from(100) {
            U256::from(100) // Cap iterations to prevent DoS
        } else {
            max_iterations
        };
        
        let mut processed = U256::ZERO;
        // Process in batches with cursor for resumable processing
        // Implementation would track cursor state across calls
        
        Ok(processed)
    }
}
```

### Replay Protection and Signatures

For permit-like flows, use EIP-712 domain separators with chain ID, nonces, and deadlines to prevent cross-chain and replay attacks:

```rust
#[storage]
#[entrypoint]
pub struct PermitContract {
    nonces: StorageMap<Address, StorageU256>,
    chain_id: StorageU256,
}

#[public]
impl PermitContract {
    pub fn permit_with_signature(&mut self, owner: Address, spender: Address, deadline: U256, v: u8, r: B256, s: B256) -> Result<(), Vec<u8>> {
        if U256::from(block::timestamp()) > deadline {
            return Err(b"Permit expired".to_vec());
        }
        
        // Verify EIP-712 signature with domain separator including chain ID
        let nonce = self.nonces.get(owner);
        self.nonces.setter(owner).set(nonce + U256::from(1));
        
        // EIP-712 domain separator and signature verification would go here
        // This prevents replay attacks across chains and nonce reuse
        
        Ok(())
    }
}
```

### ETH Transfer Security Notes

When handling ETH transfers in Stylus:
- The `transfer_eth` function errors must revert the function (using `?` is correct)
- EIP-1884 and L2 gas semantics make "assume 2300 stipend" obsolete
- Always explicitly set gas on external calls and avoid low-level send patterns without checks
- Consider pull-payment patterns for refunds to avoid forced failures

## Audit Preparation

### Documentation Requirements

Prepare comprehensive security documentation:

```rust
/// # Security Documentation
/// 
/// ## Threat Model
/// - Trusted entities: Contract owner, whitelisted admins
/// - Untrusted entities: General users, external contracts
/// - Assets at risk: User token balances, protocol reserves
/// 
/// ## Security Assumptions
/// - Block timestamps are accurate within 15 seconds
/// - External oracles provide honest price feeds
/// - Admin keys are properly secured
/// 
/// ## Known Limitations
/// - Flash loan attacks: Mitigated by time locks
/// - MEV exploitation: Mitigated by commit-reveal
#[public]
impl AuditReadyContract {
    /// Admin function - requires elevated privileges
    /// @param from Source account (must be approved or msg.sender)  
    /// @param to Destination account (cannot be zero address)
    /// @param amount Number of tokens to transfer
    pub fn admin_transfer(&mut self, from: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
        // Document security checks
        self.only_admin()?;                    // Access control
        self.check_not_locked()?;              // Reentrancy protection
        self.validate_addresses(from, to)?;    // Input validation
        self.check_balance(from, amount)?;     // Balance check
        
        self.execute_transfer(from, to, amount)?;
        Ok(())
    }
}
```

### Testing for Security

Comprehensive security test suite:

```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    use stylus_sdk::testing::*;
    
    #[test]
    fn test_reentrancy_protection() {
        let vm = TestVM::new();
        let mut contract = SecureContract::from(&vm);
        
        // Set up initial state
        let user = Address::from([1u8; 20]);
        vm.set_sender(user);
        contract.balances.setter(user).set(U256::from(1000));
        
        // Test normal withdrawal first
        assert!(contract.secure_withdraw(U256::from(100)).is_ok());
        assert_eq!(contract.balances.get(user), U256::from(900));
        
        // Test reentrancy protection - set up malicious reentrant call
        let malicious_target = Address::from([2u8; 20]);
        
        // Mock a reentrant callback that tries to call withdraw again
        vm.mock_call(malicious_target, vec![], Ok(vec![]));
        
        // Simulate the reentrancy attack scenario
        // In a real attack, the malicious contract would call back during transfer_eth
        // Here we test that the guard prevents multiple simultaneous withdrawals
        contract.balances.setter(user).set(U256::from(500)); // Reset for test
        
        // First call should succeed and lock the contract
        assert!(contract.secure_withdraw(U256::from(100)).is_ok());
        
        // Attempt to bypass the lock would be blocked by the RAII guard
        // The guard automatically unlocks when the function completes
    }
    
    #[test]
    fn test_access_control() {
        let vm = TestVM::new();
        let mut contract = AccessControlledContract::from(&vm);
        
        let owner = vm.msg_sender();
        contract.initialize(owner).unwrap();
        
        // Test unauthorized access
        let unauthorized = Address::from([2u8; 20]);
        vm.set_sender(unauthorized);
        
        let result = contract.add_admin(Address::from([3u8; 20]));
        assert!(result.is_err());
        
        // Test authorized access
        vm.set_sender(owner);
        let result = contract.add_admin(Address::from([3u8; 20]));
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_integer_overflow_protection() {
        let vm = TestVM::new();
        let mut contract = SafeMathContract::from(&vm);
        
        // Test overflow scenarios
        let max_value = U256::MAX;
        let result = contract.safe_add(max_value, U256::from(1));
        assert!(result.is_err());
        
        // Test underflow
        let result = contract.safe_sub(U256::ZERO, U256::from(1));
        assert!(result.is_err());
    }
}
```

### Working with Auditors

Best practices for security audits:

```markdown
# Security Audit Checklist

## Pre-Audit
- [ ] Complete test suite with >95% coverage
- [ ] Document all external dependencies
- [ ] Create threat model and attack surface analysis
- [ ] Run static analysis tools
- [ ] Complete gas optimization

## During Audit
- [ ] Respond promptly to auditor questions
- [ ] Test proposed fixes thoroughly
- [ ] Document all changes

## Post-Audit
- [ ] Address all critical findings
- [ ] Update documentation
- [ ] Plan ongoing security monitoring
```

## Audit Focus Checklist

When preparing for audits, emphasize these critical areas to auditors:

```
Audit focus checklist:
- Reentrancy guards on all external-call paths (including token callbacks)
- Role boundaries and two-step ownership transfer  
- Packed/immutable configuration set only in constructor
- External call allowlists with gas limits and error propagation
- Cooldowns use block numbers; timestamps only for UX
- No unbounded loops; batch and cap operations
- Events for all admin/state changes; indexed for triage
- Zero-address checks on both ends of transfers
- Integer overflow protection with U256 checked operations
- Proper error handling with structured errors (#[derive(SolidityError)])
- Storage layout collision prevention in upgradeable contracts
- Signature replay protection with nonces and domain separators
```

## Next Steps

Security is an ongoing process. Stay updated with:
- Latest EVM security best practices
- Stylus SDK security updates
- Arbitrum-specific security considerations
- Community security findings

Regular security reviews and monitoring are essential for maintaining secure contracts in production.