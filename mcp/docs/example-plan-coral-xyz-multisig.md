# Coral Multisig: Solana to Stylus Migration Plan

## 1. Overview

This plan migrates the **coral-multisig** Anchor program (version 0.9.0) to Stylus. The program implements an M-of-N multisig wallet that can execute arbitrary transactions when enough owners approve. It stores multisig configuration (owners, threshold) and pending transactions as separate accounts, using a PDA-derived signer for transaction execution.

**Selected program crate**: `programs/multisig` (Anchor-based, anchor-lang 0.25.0)

**Assumptions**:
- The Stylus contract will use a single contract instance per multisig (no factory pattern initially)
- Transactions are stored in contract storage using mapping by ID rather than separate accounts
- The multisig PDA signer concept translates to the contract address acting as the signer for external calls
- Dynamic owner lists and transaction data will use `StorageVec` and `StorageMap`

---

## 2. Architecture Mapping

### 2.1 Accounts to Storage Table

| Solana Account/Seed | Data Fields/Layout | Access/Owner | Stylus Storage Mapping | Init/Migration Step | Source |
|---------------------|-------------------|--------------|------------------------|---------------------|--------|
| `Multisig` (program-owned, signer constraint) | `owners: Vec<Pubkey>`, `threshold: u64`, `nonce: u8`, `owner_set_seqno: u32` | Program-owned; initialized via `create_multisig`; mutable by `set_owners`, `change_threshold` | Top-level `#[storage]` struct fields: `owners: StorageVec<StorageAddress>`, `threshold: StorageU64`, `owner_set_seqno: StorageU32` | Initialize in `constructor`; nonce removed (not needed for EVM) | (code: programs/multisig/src/lib.rs:246-251), (source: file:///handbook/src/state-storage.md#stylus-storage-model) |
| `Transaction` (program-owned, zeroed + signer on init) | `multisig: Pubkey`, `program_id: Pubkey`, `accounts: Vec<TransactionAccount>`, `data: Vec<u8>`, `signers: Vec<bool>`, `did_execute: bool`, `owner_set_seqno: u32` | Created by owner; mutable for approvals; read during execution | `StorageMap<U256, Transaction>` where `Transaction` is a nested `#[storage]` struct with: `target_address: StorageAddress`, `call_value: StorageU256`, `call_data: StorageBytes`, `signers: StorageVec<StorageBool>`, `did_execute: StorageBool`, `owner_set_seqno: StorageU32` | Create via `create_transaction`; use incrementing `tx_count: StorageU256` as ID | (code: programs/multisig/src/lib.rs:254-269), (source: file:///handbook/src/state-storage.md#nested-mappings) |
| `TransactionAccount` (embedded struct) | `pubkey: Pubkey`, `is_signer: bool`, `is_writable: bool` | N/A - embedded in Transaction | Not directly needed; Stylus uses raw calldata bytes for external calls | Remove; EVM external calls use `Address + calldata + value` | (code: programs/multisig/src/lib.rs:281-286), (source: file:///handbook/src/external-calls.md#stylus) |
| PDA Signer (`multisig_signer`) | Derived from `[multisig.key(), nonce]` | Signs for CPI execution | Contract address (`address(this)`) acts as msg.sender for external calls | Built-in; no migration needed | (code: programs/multisig/src/lib.rs:236-240), (source: file:///handbook/src/external-calls.md#stylus) |

### 2.2 Instructions to Public Functions Table

| Solana Instruction | Preconditions (signers/ACL/accounts) | State Transitions | Stylus Public Fn + Params | Notes | Source |
|-------------------|--------------------------------------|-------------------|---------------------------|-------|--------|
| `create_multisig(owners, threshold, nonce)` | None (new account, signer constraint) | Initializes `Multisig` with owners, threshold, nonce=0, owner_set_seqno=0 | `#[constructor] fn constructor(&mut self, owners: Vec<Address>, threshold: U64)` | Nonce removed; validate unique owners, threshold bounds | (code: programs/multisig/src/lib.rs:33-52), (source: file:///handbook/src/program-structure.md#stylus-idioms) |
| `create_transaction(pid, accs, data)` | Proposer must be in `multisig.owners`; Signer on proposer | Creates `Transaction`, auto-signs for proposer | `fn create_transaction(&mut self, target: Address, value: U256, data: Bytes) -> Result<U256, ContractError>` | Returns tx_id; auto-approve for caller; validate caller is owner | (code: programs/multisig/src/lib.rs:56-84), (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| `approve(ctx)` | Owner must be signer; transaction.multisig == multisig.key; owner_set_seqno match | Sets `signers[owner_index] = true` | `fn approve(&mut self, tx_id: U256) -> Result<(), ContractError>` | Validate caller is owner; tx exists; seqno match | (code: programs/multisig/src/lib.rs:87-98), (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| `set_owners(owners)` | Signer must be multisig_signer (self-call via execute) | Updates owners; adjusts threshold if needed; increments seqno | `fn set_owners(&mut self, owners: Vec<Address>) -> Result<(), ContractError>` | Only callable by contract itself (via execute); validate unique owners | (code: programs/multisig/src/lib.rs:119-135), (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| `change_threshold(threshold)` | Signer must be multisig_signer (self-call via execute) | Updates threshold | `fn change_threshold(&mut self, threshold: U64) -> Result<(), ContractError>` | Only callable by contract itself; validate bounds | (code: programs/multisig/src/lib.rs:140-148), (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| `set_owners_and_change_threshold(owners, threshold)` | Signer must be multisig_signer | Combines both operations atomically | `fn set_owners_and_change_threshold(&mut self, owners: Vec<Address>, threshold: U64) -> Result<(), ContractError>` | Only callable by contract itself | (code: programs/multisig/src/lib.rs:102-117), (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| `execute_transaction(ctx)` | Enough signers (>= threshold); not already executed; seqno match | Executes CPI; sets did_execute=true | `fn execute_transaction(&mut self, tx_id: U256) -> Result<Bytes, ContractError>` | Use low-level `call` for external execution; return call result | (code: programs/multisig/src/lib.rs:151-192), (source: file:///handbook/src/external-calls.md#stylus) |
| N/A | N/A | N/A | `fn get_owners(&self) -> Vec<Address>` | View function to read owners | (source: file:///handbook/src/program-structure.md#stylus-idioms) |
| N/A | N/A | N/A | `fn get_threshold(&self) -> U64` | View function to read threshold | (source: file:///handbook/src/program-structure.md#stylus-idioms) |
| N/A | N/A | N/A | `fn get_transaction(&self, tx_id: U256) -> (Address, U256, Bytes, Vec<bool>, bool, U32)` | View function to read transaction details | (source: file:///handbook/src/program-structure.md#stylus-idioms) |
| N/A | N/A | N/A | `fn is_owner(&self, addr: Address) -> bool` | View function to check ownership | (source: file:///handbook/src/program-structure.md#stylus-idioms) |
| N/A | N/A | N/A | `fn tx_count(&self) -> U256` | View function to read transaction count | (source: file:///handbook/src/program-structure.md#stylus-idioms) |

---

## 3. Authorities & Access Control

### Key Roles

| Role | Solana Implementation | Stylus Implementation | Source |
|------|----------------------|----------------------|--------|
| **Owners** | `Vec<Pubkey>` stored in Multisig account; validated via `is_signer` check | `StorageVec<StorageAddress>` in contract storage; validated via `msg_sender()` comparison | (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| **Threshold** | `u64` requiring N-of-M approvals before execution | `StorageU64`; same logic applies | (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| **Multisig Signer (PDA)** | Derived PDA that signs for CPI; used to authorize self-modifications | Contract address is `msg_sender()` when called via `execute_transaction`; check `msg_sender() == address(this)` | (source: file:///handbook/src/access-control.md#stylus-authentication-model) |

### Enforcement Strategy

1. **Owner validation**: Create helper `fn owner_index(&self, addr: Address) -> Option<usize>` that iterates `owners` storage vector
2. **Self-call authorization**: For `set_owners`, `change_threshold`, and `set_owners_and_change_threshold`, require `self.vm().msg_sender() == contract_address()` where `contract_address()` is obtained via `address(this)` equivalent
3. **Signature tracking**: Use `StorageVec<StorageBool>` per transaction, indexed by owner position; position determined at approval time using current owner list
4. **Seqno validation**: Store `owner_set_seqno` on each transaction; reject approvals/executions if transaction seqno != current multisig seqno

(source: file:///handbook/src/access-control.md#stylus-authentication-model)

---

## 4. CPI Dependency Audit

| External Program Called | Guarantees Relied Upon | Replacement Strategy in Stylus | Source |
|------------------------|------------------------|------------------------------|--------|
| **Arbitrary programs via `invoke_signed`** | The multisig executes arbitrary Solana instructions with PDA signer authority | Use `stylus_sdk::call::call` with `Call::new().value(tx.value)` context for external contract calls; contract address acts as msg.sender | (code: programs/multisig/src/lib.rs:186), (source: file:///handbook/src/external-calls.md#stylus) |
| **System Program** (implicit for account creation) | Account rent and creation | Not needed; Stylus storage is implicit | (source: file:///handbook/src/state-storage.md#cost-considerations) |

### CPI Translation

The Solana program builds an `Instruction` from stored `program_id`, `accounts`, and `data`, then calls `invoke_signed`. In Stylus:

```rust
// Low-level call pattern
let result = self.vm().call(
    &calls::context::Call::new().value(tx_value),
    tx_target,
    &tx_calldata,
);
```

This approach:
- Sends ETH value if specified
- Executes arbitrary calldata against target contract
- Returns success/failure and return data

(source: file:///handbook/src/external-calls.md#stylus)

---

## 5. Serialization & Data Layout

### Solana Serialization

| Data Type | Solana Encoding | Notes |
|-----------|-----------------|-------|
| Account state | Borsh (little-endian) | Anchor adds 8-byte discriminator prefix |
| `Vec<T>` | Length-prefixed (u32 LE) + elements | Variable size |
| `Pubkey` | 32 bytes, raw | Direct copy |
| `bool` | 1 byte | 0 or 1 |

### Stylus Serialization

| Solana Type | Stylus Type | ABI Encoding | Source |
|-------------|-------------|--------------|--------|
| `Pubkey` (32 bytes) | `Address` (20 bytes) | EVM ABI, left-padded to 32 bytes | (source: file:///handbook/src/state-storage.md#primitive-types) |
| `Vec<Pubkey>` | `Vec<Address>` | Dynamic array encoding | (source: file:///handbook/src/state-storage.md#primitive-types) |
| `Vec<u8>` | `Bytes` or `Vec<u8>` | Dynamic bytes encoding | (source: file:///handbook/src/state-storage.md#primitive-types) |
| `u64` | `U64` | Fixed 8 bytes, big-endian in ABI | (source: file:///handbook/src/state-storage.md#primitive-types) |
| `u32` | `U32` | Fixed 4 bytes | (source: file:///handbook/src/state-storage.md#primitive-types) |
| `u8` | `U8` | Fixed 1 byte | (source: file:///handbook/src/state-storage.md#primitive-types) |
| `bool` | `bool` | Fixed 1 byte | (source: file:///handbook/src/state-storage.md#primitive-types) |

### Layout Migration

- **No backward compatibility required**: Fresh deployment on Arbitrum/Stylus
- **Endianness**: EVM uses big-endian for ABI; storage is implementation-defined but abstracted by SDK
- **Address size reduction**: Solana Pubkey (32 bytes) → EVM Address (20 bytes); no data loss for EOA/contract addresses

(source: file:///handbook/src/state-storage.md#solana-to-stylus-type-mappings)

---

## 6. Errors & Events Mapping

### Error Mapping

| Solana Error (ErrorCode) | Code | Stylus Error Type | Source |
|-------------------------|------|-------------------|--------|
| `InvalidOwner` | 0 | `error InvalidOwner(address caller)` | (code: programs/multisig/src/lib.rs:319-320), (source: file:///handbook/src/errors-events.md#stylus) |
| `InvalidOwnersLen` | 1 | `error InvalidOwnersLen()` | (code: programs/multisig/src/lib.rs:321-322), (source: file:///handbook/src/errors-events.md#stylus) |
| `NotEnoughSigners` | 2 | `error NotEnoughSigners(uint64 required, uint64 provided)` | (code: programs/multisig/src/lib.rs:323-324), (source: file:///handbook/src/errors-events.md#stylus) |
| `TransactionAlreadySigned` | 3 | `error TransactionAlreadySigned(uint256 tx_id, address signer)` | (code: programs/multisig/src/lib.rs:325-326), (source: file:///handbook/src/errors-events.md#stylus) |
| `Overflow` | 4 | Use checked arithmetic; panic on overflow | (code: programs/multisig/src/lib.rs:327-328), (source: file:///handbook/src/security-considerations.md#integer-arithmetic-overflow) |
| `UnableToDelete` | 5 | Not applicable (no delete instruction in original) | (code: programs/multisig/src/lib.rs:329-330) |
| `AlreadyExecuted` | 6 | `error AlreadyExecuted(uint256 tx_id)` | (code: programs/multisig/src/lib.rs:331-332), (source: file:///handbook/src/errors-events.md#stylus) |
| `InvalidThreshold` | 7 | `error InvalidThreshold(uint64 threshold, uint64 owners_len)` | (code: programs/multisig/src/lib.rs:333-334), (source: file:///handbook/src/errors-events.md#stylus) |
| `UniqueOwners` | 8 | `error DuplicateOwner(address owner)` | (code: programs/multisig/src/lib.rs:335-336), (source: file:///handbook/src/errors-events.md#stylus) |
| N/A | N/A | `error TransactionNotFound(uint256 tx_id)` | New error for invalid tx_id lookups |
| N/A | N/A | `error OwnerSetSeqnoMismatch(uint32 expected, uint32 actual)` | Explicit seqno mismatch error |
| N/A | N/A | `error ExecutionFailed(uint256 tx_id, bytes return_data)` | External call failure |
| N/A | N/A | `error Unauthorized()` | For self-call-only functions |

### Event Mapping

| Event Name | Fields | When Emitted | Source |
|------------|--------|--------------|--------|
| `MultisigCreated` | `owners: address[]`, `threshold: uint64` | Constructor completion | (source: file:///handbook/src/errors-events.md#stylus) |
| `TransactionCreated` | `tx_id: uint256`, `proposer: address`, `target: address`, `value: uint256` | `create_transaction` success | (source: file:///handbook/src/errors-events.md#stylus) |
| `TransactionApproved` | `tx_id: uint256`, `approver: address`, `approvals: uint64` | `approve` success | (source: file:///handbook/src/errors-events.md#stylus) |
| `TransactionExecuted` | `tx_id: uint256`, `executor: address`, `return_data: bytes` | `execute_transaction` success | (source: file:///handbook/src/errors-events.md#stylus) |
| `OwnersUpdated` | `old_owners: address[]`, `new_owners: address[]`, `new_seqno: uint32` | `set_owners` success | (source: file:///handbook/src/errors-events.md#stylus) |
| `ThresholdUpdated` | `old_threshold: uint64`, `new_threshold: uint64` | `change_threshold` success | (source: file:///handbook/src/errors-events.md#stylus) |

---

## 7. Risk Register

| # | Risk | Where It Arises | Mitigation in Stylus | Source |
|---|------|-----------------|---------------------|--------|
| 1 | **Authorization bypass** | `set_owners`, `change_threshold` must only be callable by the multisig itself | Require `msg_sender() == contract_address`; implement via low-level self-call during `execute_transaction` | (source: file:///handbook/src/security-considerations.md#sender-authorization) |
| 2 | **Integer overflow in threshold/owner calculations** | Threshold validation, signer counting | Use `checked_add`, `checked_sub`; return errors on overflow rather than wrapping | (source: file:///handbook/src/security-considerations.md#integer-arithmetic-overflow) |
| 3 | **Reentrancy during external call execution** | `execute_transaction` calls arbitrary external contracts | Stylus disables reentrancy by default; mark `did_execute = true` BEFORE external call as defense-in-depth | (source: file:///handbook/src/security-considerations.md#contract-reentrancy) |
| 4 | **Owner index drift after owner set change** | Approvals reference owner index; if owners change, indices become invalid | Use `owner_set_seqno` to invalidate pending transactions when owners change; reject operations with mismatched seqno | (code: programs/multisig/src/lib.rs:212), (source: file:///handbook/src/access-control.md) |
| 5 | **Duplicate owners allowing threshold bypass** | If same address appears twice, one signer could count as multiple approvals | Implement `assert_unique_owners` helper; check during `constructor` and `set_owners` | (code: programs/multisig/src/lib.rs:307-315), (source: file:///handbook/src/security-considerations.md) |
| 6 | **Zero threshold or threshold > owners** | Invalid configuration allows immediate execution or impossible execution | Validate `0 < threshold <= owners.len()` in constructor, `change_threshold`, and `set_owners` (adjust if needed) | (code: programs/multisig/src/lib.rs:40-44), (source: file:///handbook/src/security-considerations.md) |
| 7 | **Empty owners list** | No valid signers; contract becomes unusable | Require `owners.len() > 0` in constructor and `set_owners` | (code: programs/multisig/src/lib.rs:44), (source: file:///handbook/src/security-considerations.md) |
| 8 | **External call return data handling** | Large return data could cause issues; failed calls need proper error propagation | Capture and return call result; emit event with return data; allow caller to handle success/failure | (source: file:///handbook/src/external-calls.md#stylus) |
| 9 | **Constructor authority (tx_origin vs msg_sender)** | Constructor `msg_sender` returns factory address in production | Use `tx_origin` to get deployer address if needed for initial owner, or pass owner addresses as constructor params | (source: file:///handbook/src/security-considerations.md#sender-authorization) |
| 10 | **Storage slot collision in nested mappings** | Transaction storage with dynamic fields | Use SDK storage types which handle slot derivation; avoid manual slot calculation | (source: file:///handbook/src/state-storage.md#nested-mappings) |

---

## 8. Implementation Phases

### Phase 1: Project Setup and Boilerplate

**Objectives**: Create project structure with correct dependencies and build configuration.

**Success Criteria**:
- `cargo build --target wasm32-unknown-unknown` completes without errors
- Project structure matches Stylus conventions

**Tasks**:

1. Create `/Users/chris/Desktop/coral-xyz-multisig-stylus/Cargo.toml` with pinned dependencies from Section 9 (source: file:///handbook/src/testing-debugging.md#setup)
2. Create `/Users/chris/Desktop/coral-xyz-multisig-stylus/src/main.rs` with boilerplate from Section 9 (source: file:///handbook/src/program-structure.md#stylus-contract-model)
3. Create `/Users/chris/Desktop/coral-xyz-multisig-stylus/src/lib.rs` with empty entrypoint struct (source: file:///handbook/src/program-structure.md#stylus-contract-model)
4. Run `cargo build --target wasm32-unknown-unknown` to verify setup (source: file:///handbook/src/testing-debugging.md)

**Exit Conditions**: WASM build succeeds with no errors.

---

### Phase 2: Storage and Error Definitions

**Objectives**: Define all storage types, error types, and events.

**Success Criteria**:
- All storage structs compile
- Error and event types are ABI-compatible
- `cargo build --target wasm32-unknown-unknown` passes

**Tasks**:

1. Define error types in `/Users/chris/Desktop/coral-xyz-multisig-stylus/src/lib.rs` using `sol!` macro:
   - `InvalidOwner`, `InvalidOwnersLen`, `NotEnoughSigners`, `AlreadyExecuted`, `InvalidThreshold`, `DuplicateOwner`, `TransactionNotFound`, `OwnerSetSeqnoMismatch`, `ExecutionFailed`, `Unauthorized`
   (source: file:///handbook/src/errors-events.md#stylus)

2. Define event types using `sol!` macro:
   - `MultisigCreated`, `TransactionCreated`, `TransactionApproved`, `TransactionExecuted`, `OwnersUpdated`, `ThresholdUpdated`
   (source: file:///handbook/src/errors-events.md#stylus)

3. Create `ContractError` enum with `#[derive(SolidityError)]` (source: file:///handbook/src/errors-events.md#stylus)

4. Define `Transaction` storage struct with `#[storage]` attribute containing:
   - `target: StorageAddress`
   - `value: StorageU256`
   - `data: StorageBytes`
   - `signers: StorageVec<StorageBool>`
   - `did_execute: StorageBool`
   - `owner_set_seqno: StorageU32`
   (source: file:///handbook/src/state-storage.md#stylus-storage-model)

5. Define main `Multisig` storage struct with `#[storage]` and `#[entrypoint]` containing:
   - `owners: StorageVec<StorageAddress>`
   - `threshold: StorageU64`
   - `owner_set_seqno: StorageU32`
   - `tx_count: StorageU256`
   - `transactions: StorageMap<U256, Transaction>`
   (source: file:///handbook/src/state-storage.md#stylus-storage-model)

6. Run `cargo build --target wasm32-unknown-unknown` to verify (source: file:///handbook/src/testing-debugging.md)

**Exit Conditions**: All types defined and WASM build passes.

---

### Phase 3: Core Logic Implementation

**Objectives**: Implement all public functions and helper methods.

**Success Criteria**:
- All functions from Section 2.2 implemented
- Internal helpers for validation work correctly
- `cargo build --target wasm32-unknown-unknown` passes

**Tasks**:

1. Implement helper `fn assert_unique_owners(&self, owners: &[Address]) -> Result<(), ContractError>` in `/Users/chris/Desktop/coral-xyz-multisig-stylus/src/lib.rs` (source: file:///handbook/src/access-control.md#stylus-authentication-model)

2. Implement helper `fn owner_index(&self, addr: Address) -> Option<usize>` (source: file:///handbook/src/access-control.md#stylus-authentication-model)

3. Implement helper `fn count_approvals(&self, tx_id: U256) -> u64` (source: file:///handbook/src/state-storage.md#stylus-storage-model)

4. Implement `#[constructor] fn constructor(&mut self, owners: Vec<Address>, threshold: U64)`:
   - Validate unique owners
   - Validate threshold bounds
   - Store owners, threshold, seqno=0
   - Emit `MultisigCreated`
   (source: file:///handbook/src/program-structure.md#stylus-idioms)

5. Implement `fn create_transaction(&mut self, target: Address, value: U256, data: Bytes) -> Result<U256, ContractError>`:
   - Validate caller is owner
   - Create transaction with current seqno
   - Auto-approve for caller
   - Increment tx_count
   - Emit `TransactionCreated`
   (source: file:///handbook/src/access-control.md#stylus-authentication-model)

6. Implement `fn approve(&mut self, tx_id: U256) -> Result<(), ContractError>`:
   - Validate tx exists
   - Validate caller is owner
   - Validate seqno match
   - Validate not already signed by caller
   - Set signer flag
   - Emit `TransactionApproved`
   (source: file:///handbook/src/access-control.md#stylus-authentication-model)

7. Implement `fn execute_transaction(&mut self, tx_id: U256) -> Result<Bytes, ContractError>`:
   - Validate tx exists
   - Validate not already executed
   - Validate seqno match
   - Validate enough signers
   - Set did_execute = true (before call, defense-in-depth)
   - Execute external call
   - Emit `TransactionExecuted`
   - Return call result
   (source: file:///handbook/src/external-calls.md#stylus)

8. Implement `fn set_owners(&mut self, owners: Vec<Address>) -> Result<(), ContractError>`:
   - Require `msg_sender() == address(this)`
   - Validate unique owners
   - Validate non-empty
   - Adjust threshold if needed
   - Update owners and increment seqno
   - Emit `OwnersUpdated`
   (source: file:///handbook/src/access-control.md#stylus-authentication-model)

9. Implement `fn change_threshold(&mut self, threshold: U64) -> Result<(), ContractError>`:
   - Require `msg_sender() == address(this)`
   - Validate threshold bounds
   - Update threshold
   - Emit `ThresholdUpdated`
   (source: file:///handbook/src/access-control.md#stylus-authentication-model)

10. Implement `fn set_owners_and_change_threshold(&mut self, owners: Vec<Address>, threshold: U64) -> Result<(), ContractError>`:
    - Require `msg_sender() == address(this)`
    - Call `set_owners` logic
    - Call `change_threshold` logic
    (source: file:///handbook/src/access-control.md#stylus-authentication-model)

11. Implement view functions: `get_owners`, `get_threshold`, `get_transaction`, `is_owner`, `tx_count`, `get_owner_set_seqno` (source: file:///handbook/src/program-structure.md#stylus-idioms)

12. Run `cargo build --target wasm32-unknown-unknown` to verify (source: file:///handbook/src/testing-debugging.md)

**Exit Conditions**: All functions implemented and WASM build passes.

---

### Phase 4: Unit Testing

**Objectives**: Comprehensive unit test coverage using motsu.

**Success Criteria**:
- All positive paths tested
- All error conditions tested
- All security invariants verified
- `cargo test` passes

**Tasks**:

1. Create `/Users/chris/Desktop/coral-xyz-multisig-stylus/src/lib.rs` test module with motsu imports (source: file:///handbook/src/testing-debugging.md#setup)

2. Implement constructor tests:
   - Valid initialization with multiple owners
   - Reject empty owners
   - Reject threshold = 0
   - Reject threshold > owners.len()
   - Reject duplicate owners
   (source: file:///handbook/src/testing-debugging.md#example)

3. Implement create_transaction tests:
   - Owner can create transaction
   - Non-owner rejected
   - Creator auto-approves
   - Transaction ID increments
   (source: file:///handbook/src/testing-debugging.md#example)

4. Implement approve tests:
   - Owner can approve
   - Non-owner rejected
   - Double approval rejected
   - Invalid tx_id rejected
   - Seqno mismatch rejected
   (source: file:///handbook/src/testing-debugging.md#example)

5. Implement execute_transaction tests:
   - Execute with exact threshold
   - Execute with more than threshold
   - Reject with fewer than threshold
   - Reject already executed
   - Reject seqno mismatch
   - Verify external call is made
   (source: file:///handbook/src/testing-debugging.md#example)

6. Implement owner management tests:
   - set_owners only callable by self
   - change_threshold only callable by self
   - Owners update invalidates pending transactions (seqno)
   - Threshold auto-adjusts when owners reduced
   (source: file:///handbook/src/testing-debugging.md#example)

7. Implement property-based tests:
   - Approval count never exceeds owner count
   - Executed transactions stay executed
   - Seqno only increases
   (source: file:///handbook/src/testing-debugging.md#example)

8. Run `cargo test` and verify all pass (source: file:///handbook/src/testing-debugging.md)

**Exit Conditions**: All tests pass; code coverage on all public functions and error paths.

---

### Phase 5: Integration Testing and Optimization

**Objectives**: End-to-end testing and WASM size optimization.

**Success Criteria**:
- Full workflow tests pass
- WASM binary size acceptable for deployment
- Gas estimates reasonable

**Tasks**:

1. Create integration test simulating full workflow:
   - Deploy multisig
   - Create transaction targeting a test contract
   - Collect approvals from multiple owners
   - Execute and verify target contract state changed
   (source: file:///handbook/src/testing-debugging.md#example)

2. Optimize WASM binary size:
   - Enable LTO in release profile
   - Use `wasm-opt` if needed
   - Target < 24KB compressed for L2 deployment
   (source: file:///handbook/src/testing-debugging.md)

3. Document gas estimates for each function (source: file:///handbook/src/state-storage.md#cost-considerations)

4. Final `cargo build --target wasm32-unknown-unknown --release` verification (source: file:///handbook/src/testing-debugging.md)

**Exit Conditions**: Integration tests pass; binary size acceptable; ready for testnet deployment.

---

## 9. Boilerplate Artifacts

### Cargo.toml

```toml
[package]
name = "coral-multisig-stylus"
version = "0.1.0"
edition = "2021"

[features]
export-abi = ["stylus-sdk/export-abi", "openzeppelin-stylus/export-abi"]

[dependencies]
alloy-primitives = "=0.8.20"
alloy-sol-types = "=0.8.20"
openzeppelin-stylus = "0.3.0"
stylus-sdk = "=0.9.0"

[dev-dependencies]
alloy-primitives = { version = "=0.8.20", features = [ "tiny-keccak" ] }
arbitrary = { version = "=1.4.2", features = [ "derive" ] }
motsu = "0.10.0"
```

### src/main.rs

```rust
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]

#[cfg(not(any(test, feature = "export-abi")))]
#[no_mangle]
pub extern "C" fn main() {}

#[cfg(feature = "export-abi")]
fn main() {
    coral_multisig_stylus::print_from_args();
}
```

### Pinned Versions

| Dependency | Version | Notes |
|------------|---------|-------|
| `alloy-primitives` | 0.8.20 | Exact pin required |
| `alloy-sol-types` | 0.8.20 | Exact pin required |
| `stylus-sdk` | 0.9.0 | Exact pin required |
| `openzeppelin-stylus` | 0.3.0 | Latest stable |
| `motsu` | 0.10.0 | Test harness |
| `arbitrary` | 1.4.2 | Required by motsu |

---

## 10. Test Plan

All tests MUST follow the patterns and guidelines established in the handbook's testing chapter. (source: file:///handbook/src/testing-debugging.md)

### Unit Tests

| Test Category | Test Cases | Source |
|--------------|------------|--------|
| **Constructor validation** | Valid init; empty owners rejected; zero threshold rejected; threshold > owners rejected; duplicate owners rejected | (source: file:///handbook/src/testing-debugging.md#example) |
| **Transaction creation** | Owner creates successfully; non-owner rejected; auto-approval verified; tx_id increments | (source: file:///handbook/src/testing-debugging.md#example) |
| **Approval flow** | Owner approves; non-owner rejected; double approval rejected; invalid tx rejected; seqno mismatch rejected | (source: file:///handbook/src/testing-debugging.md#example) |
| **Execution** | Threshold met executes; below threshold rejected; already executed rejected; external call made; return data captured | (source: file:///handbook/src/testing-debugging.md#example) |
| **Owner management** | Only self-call allowed; unique owners enforced; threshold auto-adjustment; seqno increments | (source: file:///handbook/src/testing-debugging.md#example) |

### Property-Based Tests

| Invariant | Description | Source |
|-----------|-------------|--------|
| Approval bound | `count_approvals(tx_id) <= owners.len()` always | (source: file:///handbook/src/testing-debugging.md#example) |
| Monotonic seqno | `owner_set_seqno` only increases | (source: file:///handbook/src/testing-debugging.md#example) |
| Execution idempotence | Once `did_execute == true`, subsequent execute calls fail | (source: file:///handbook/src/testing-debugging.md#example) |
| Threshold invariant | `0 < threshold <= owners.len()` always | (source: file:///handbook/src/testing-debugging.md#example) |

### Negative Tests

| Scenario | Expected Error | Source |
|----------|---------------|--------|
| Non-owner calls approve | `InvalidOwner` | (source: file:///handbook/src/testing-debugging.md#example) |
| Non-owner calls create_transaction | `InvalidOwner` | (source: file:///handbook/src/testing-debugging.md#example) |
| External caller calls set_owners | `Unauthorized` | (source: file:///handbook/src/testing-debugging.md#example) |
| Execute with insufficient approvals | `NotEnoughSigners` | (source: file:///handbook/src/testing-debugging.md#example) |
| Execute already executed tx | `AlreadyExecuted` | (source: file:///handbook/src/testing-debugging.md#example) |
| Approve with stale seqno | `OwnerSetSeqnoMismatch` | (source: file:///handbook/src/testing-debugging.md#example) |
| Set threshold to 0 | `InvalidThreshold` | (source: file:///handbook/src/testing-debugging.md#example) |
| Set threshold > new owners count | `InvalidThreshold` | (source: file:///handbook/src/testing-debugging.md#example) |

### Fixture Strategy

- Use motsu's `Contract<T>` for contract instantiation (source: file:///handbook/src/testing-debugging.md#example)
- Use deterministic addresses via motsu test parameters (source: file:///handbook/src/testing-debugging.md#example)
- Create mock target contract for execute tests (source: file:///handbook/src/testing-debugging.md#example)

---

## 11. Handbook References

### Chapters Referenced

| Chapter | URI | Topics Used |
|---------|-----|-------------|
| State Storage | file:///handbook/src/state-storage.md | Storage types, mappings, nested storage, type mappings, cost considerations |
| Access Control | file:///handbook/src/access-control.md | msg_sender, authorization patterns, owner validation |
| External Calls | file:///handbook/src/external-calls.md | Low-level calls, call context, return data handling |
| Errors and Events | file:///handbook/src/errors-events.md | sol! macro, SolidityError derive, event emission |
| Security Considerations | file:///handbook/src/security-considerations.md | Reentrancy, integer overflow, sender authorization |
| Testing and Debugging | file:///handbook/src/testing-debugging.md | motsu setup, unit testing patterns, dbg! macro |
| Program Structure | file:///handbook/src/program-structure.md | Constructor pattern, public functions, view functions |

### Specific Sections

- file:///handbook/src/state-storage.md#stylus-storage-model
- file:///handbook/src/state-storage.md#solana-to-stylus-type-mappings
- file:///handbook/src/state-storage.md#nested-mappings
- file:///handbook/src/state-storage.md#primitive-types
- file:///handbook/src/state-storage.md#cost-considerations
- file:///handbook/src/access-control.md#stylus-authentication-model
- file:///handbook/src/external-calls.md#stylus
- file:///handbook/src/errors-events.md#stylus
- file:///handbook/src/security-considerations.md#contract-reentrancy
- file:///handbook/src/security-considerations.md#integer-arithmetic-overflow
- file:///handbook/src/security-considerations.md#sender-authorization
- file:///handbook/src/testing-debugging.md#setup
- file:///handbook/src/testing-debugging.md#example
- file:///handbook/src/program-structure.md#stylus-contract-model
- file:///handbook/src/program-structure.md#stylus-idioms
