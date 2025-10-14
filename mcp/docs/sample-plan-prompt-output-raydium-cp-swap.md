# MIGRATION_PLAN.md

## 1. Overview

This migration plan transforms the Raydium constant product AMM program from Solana's Anchor framework to Arbitrum's Stylus platform. The program is a full-featured AMM supporting Token2022, configurable fees, permission-based pool creation, and multi-tier access control. The migration assumes retention of core AMM functionality while adapting to Stylus's EVM-compatible storage model and access patterns. The selected program crate is `programs/cp-swap` implementing a constant product curve with comprehensive fee management and creator rewards.

## 2. Architecture Mapping

### 2.1 Accounts → Storage Table

| Solana Account/Seed | Data Fields/Layout | Access/Owner | Stylus Storage Mapping | Init/Migration Step | Source |
|---------------------|-------------------|--------------|----------------------|-------------------|---------|
| AmmConfig PDA (`amm_config` + index) | bump, disable_create_pool, index, trade_fee_rate, protocol_fee_rate, fund_fee_rate, create_pool_fee, protocol_owner, fund_owner, creator_fee_rate | Admin-controlled | `StorageMap<U16, AmmConfig>` where AmmConfig is storage struct | Phase 2: Config management | (source: file:///handbook/src/state-storage.md#solana-to-stylus-type-mappings) |
| PoolState PDA (`pool` + token_0_mint + token_1_mint + config) | amm_config, pool_creator, token vaults, LP mint, observation_key, status flags, decimals, LP supply, fee accumulations, timestamps | Program-controlled | `StorageMap<(Address, Address, U16), PoolState>` keyed by (token0, token1, config_index) | Phase 3: Pool initialization | (source: file:///handbook/src/state-storage.md#nested-mappings) |
| Permission PDA (`permission` + pool + user) | Empty marker account for access control | User-specific | `StorageMap<(Address, Address), StorageBool>` for pool permissions | Phase 4: Permission system | (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| Token Vaults (controlled by pool authority) | SPL token accounts holding pool liquidity | Pool authority PDA | ERC20 token balances tracked in external ERC20 contracts | Phase 5: Token integration | (source: file:///handbook/src/external-calls.md#stylus) |
| LP Token Mint (controlled by pool authority) | SPL token mint for LP tokens | Pool authority PDA | Internal `StorageMap<Address, StorageU256>` for LP balances | Phase 5: LP token system | (source: file:///handbook/src/state-storage.md#stylus-storage-model) |
| Oracle/Observation account | Price and timestamp data for TWAP | Pool-controlled | `StorageMap<(Address, Address), PriceObservation>` for price tracking | Phase 6: Oracle implementation | (code: programs/cp-swap/src/states/oracle.rs) |

### 2.2 Instructions → Public Functions Table

| Solana Instruction | Preconditions (signers/ACL/accounts) | State Transitions | Stylus Public Fn + Params | Notes | Source |
|-------------------|-----------------------------------|------------------|--------------------------|-------|---------|
| create_amm_config | Admin signer, unique config index | Creates new AmmConfig PDA with fee parameters | `create_amm_config(index: U16, trade_fee_rate: U256, protocol_fee_rate: U256, fund_fee_rate: U256, create_pool_fee: U256, creator_fee_rate: U256)` | Requires admin access control | (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| update_amm_config | Config owner or admin signer | Updates config parameters or ownership | `update_amm_config(index: U16, param: U8, value: U256)` | Parameter enum for different update types | (code: programs/cp-swap/src/instructions/admin/update_config.rs) |
| initialize | Payer signer, token accounts, initial liquidity | Creates pool, mints initial LP tokens | `initialize(token_0: Address, token_1: Address, config_index: U16, init_amount_0: U256, init_amount_1: U256, open_time: U64)` | Requires ERC20 token approvals | (source: file:///handbook/src/external-calls.md#stylus) |
| initialize_with_permission | Permission PDA present, authority signer | Creates pool with creator fee configuration | `initialize_with_permission(token_0: Address, token_1: Address, config_index: U16, init_amount_0: U256, init_amount_1: U256, open_time: U64, creator_fee_on: U8)` | Enhanced with creator fee model | (code: programs/cp-swap/src/instructions/initialize_with_permission.rs) |
| deposit | Pool exists, token approvals, slippage limits | Increases LP position, transfers tokens | `deposit(token_0: Address, token_1: Address, lp_token_amount: U256, maximum_token_0_amount: U256, maximum_token_1_amount: U256)` | Requires pre-approval of tokens | (source: file:///handbook/src/external-calls.md#stylus) |
| withdraw | LP tokens owned, minimum output limits | Burns LP tokens, returns underlying assets | `withdraw(token_0: Address, token_1: Address, lp_token_amount: U256, minimum_token_0_amount: U256, minimum_token_1_amount: U256)` | Validates LP token ownership | (code: programs/cp-swap/src/instructions/withdraw.rs) |
| swap_base_input | Pool open, input approval, slippage protection | Trades input for output tokens, collects fees | `swap_base_input(token_in: Address, token_out: Address, amount_in: U256, minimum_amount_out: U256)` | Implements constant product formula | (source: file:///handbook/src/external-calls.md#stylus) |
| swap_base_output | Pool open, sufficient input approval | Trades exact output amount, calculates input | `swap_base_output(token_in: Address, token_out: Address, max_amount_in: U256, amount_out: U256)` | Reverse calculation from output | (code: programs/cp-swap/src/instructions/swap_base_output.rs) |
| collect_protocol_fee | Protocol owner signer | Transfers accumulated protocol fees | `collect_protocol_fee(token_0: Address, token_1: Address, amount_0_requested: U256, amount_1_requested: U256)` | Admin-only fee collection | (source: file:///handbook/src/access-control.md#stylus-authentication-model) |
| collect_fund_fee | Fund owner signer | Transfers accumulated fund fees | `collect_fund_fee(token_0: Address, token_1: Address, amount_0_requested: U256, amount_1_requested: U256)` | Separate from protocol fees | (code: programs/cp-swap/src/instructions/admin/collect_fund_fee.rs) |
| collect_creator_fee | Pool creator signer | Transfers creator-specific fees | `collect_creator_fee(token_0: Address, token_1: Address)` | Creator rewards system | (code: programs/cp-swap/src/instructions/collect_creator_fee.rs) |

## 3. Authorities & Access Control

The program implements a three-tier authority model: 1) Global admin with program-wide configuration authority stored as `admin::ID` constant, 2) Config-level owners (protocol_owner, fund_owner) for fee collection rights, and 3) Pool creators with creator fee collection privileges. In Solana, authority is verified through PDA derivation and signer checks against stored pubkeys.

The Stylus migration replaces this with storage-based access control using `msg_sender()` validation. The contract maintains `admin_address: StorageAddress` for global admin functions, while config structs store `protocol_owner: StorageAddress` and `fund_owner: StorageAddress`. Pool creator authority is tracked in `pool_creator: StorageAddress` within each pool's state. Access control modifiers will validate `self.vm().msg_sender()` against the appropriate stored address for each function category.

(source: file:///handbook/src/access-control.md#stylus-authentication-model)

## 4. CPI Dependency Audit

| External Program | Guarantees Relied Upon | Stylus Replacement Strategy | Source |
|-----------------|----------------------|---------------------------|---------|
| SPL Token Program | Token transfers, mint/burn operations, account creation | Native ERC20 interface calls via external contracts | (source: file:///handbook/src/external-calls.md#stylus) |
| SPL Token-2022 Program | Extended token functionality, transfer fees, mint extensions | ERC20 with custom extension handling or adapter pattern | (source: file:///handbook/src/external-calls.md#stylus) |
| System Program | Account creation, lamport transfers, rent payments | Contract deployment and ETH transfers via native functions | (source: file:///handbook/src/external-calls.md#stylus) |

All external programs are replaced with direct ERC20 contract interactions. The Token programs' transfer, mint, and burn operations map to standard ERC20 `transfer`, `transferFrom`, `mint`, and `burn` functions. Token-2022's transfer fee mechanism requires custom handling in swap calculations. System program dependencies are eliminated as Stylus handles account creation through constructor calls and value transfers through native ETH operations.

## 5. Serialization & Data Layout

The Solana program uses Anchor's built-in serialization with `#[account]` structs implementing `AccountSerialize` and `AccountDeserialize`. Key data types include `u64` for token amounts, `Pubkey` for addresses, and `u8` for flags and enums. Anchor automatically handles discriminators and padding.

Stylus migration adopts EVM-compatible types: `U256` for token amounts (supporting larger values and ERC20 compatibility), `Address` for account references, and `StorageU8` for flags. The `#[storage]` macro replaces Anchor's account serialization, with automatic storage slot management. Data layout migration requires no explicit byte-level compatibility since storage is rebuilt from genesis state.

(source: file:///handbook/src/state-storage.md#solana-to-stylus-type-mappings)

## 6. Errors & Events Mapping

| Solana Error/Event | Stylus Equivalent | Error Namespace | Event Schema | Source |
|-------------------|------------------|----------------|--------------|---------|
| `ErrorCode::NotApproved` | `NotApproved()` | ContractError enum | Standard revert with message | (source: file:///handbook/src/errors-events.md#stylus) |
| `ErrorCode::InvalidOwner` | `InvalidOwner(address account)` | ContractError enum | Include problematic address | (source: file:///handbook/src/errors-events.md#stylus) |
| `ErrorCode::ExceededSlippage` | `ExceededSlippage(uint256 expected, uint256 received)` | ContractError enum | Detailed slippage information | (source: file:///handbook/src/errors-events.md#stylus) |
| `LpChangeEvent` | `LiquidityChanged(address indexed pool, address indexed user, uint256 lp_amount, uint256 token0_amount, uint256 token1_amount, bool is_deposit)` | Contract events | EVM-compatible event with indexed fields | (source: file:///handbook/src/errors-events.md#stylus) |
| `SwapEvent` | `Swap(address indexed pool, address indexed user, address token_in, address token_out, uint256 amount_in, uint256 amount_out, uint256 trade_fee)` | Contract events | Standard DEX swap event format | (source: file:///handbook/src/errors-events.md#stylus) |

All Anchor error codes are mapped to structured Solidity-style errors using the `sol!` macro and `SolidityError` derive. Events transition from Anchor's `emit!` macro to Stylus's `log()` function with typed event structs.

## 7. Risk Register

| Risk | Where it Arises | Mitigation in Stylus | Source |
|------|----------------|---------------------|---------|
| **Unauthorized Access** | Admin functions, fee collection, pool management | Implement `only_admin`, `only_protocol_owner`, `only_pool_creator` modifiers checking `msg_sender()` against stored addresses | (source: file:///handbook/src/security-considerations.md#sender-authorization) |
| **Integer Overflow/Underflow** | Fee calculations, token amount arithmetic, price computations | Use `checked_add()`, `checked_sub()`, `checked_mul()`, `checked_div()` throughout, revert on overflow | (source: file:///handbook/src/security-considerations.md#integer-arithmetic-overflow) |
| **Reentrancy Attacks** | External ERC20 calls during swaps/deposits/withdrawals | Stylus has reentrancy protection by default; ensure CEI pattern for state updates | (source: file:///handbook/src/security-considerations.md#reentrancy) |
| **Price Manipulation** | Large swaps affecting pool price, MEV exploitation | Implement slippage protection, consider time-weighted average pricing (TWAP) | (source: file:///handbook/src/security-considerations.md#sender-authorization) |
| **Token Standard Compatibility** | ERC20 vs SPL token behavior differences, transfer fees | Validate ERC20 standard compliance, handle transfer fee tokens with adjusted calculations | (source: file:///handbook/src/external-calls.md#stylus) |
| **Liquidity Drain** | Precision loss in small liquidity pools, minimum liquidity requirements | Lock minimum liquidity (1000 wei) permanently, use sufficient decimal precision | (source: file:///handbook/src/security-considerations.md#integer-arithmetic-overflow) |
| **Admin Key Compromise** | Single admin controls critical functions | Implement multi-signature requirements, time delays for critical changes, ownership transfer process | (source: file:///handbook/src/access-control.md#standardized-access-control-patterns) |
| **Flash Loan Attacks** | Atomic arbitrage opportunities, price oracle manipulation | Implement per-block limits, consider commit-reveal schemes for large operations | (source: file:///handbook/src/security-considerations.md#reentrancy) |
| **Gas Limit Issues** | Complex calculations exceeding block gas limit | Optimize storage reads, batch operations, use view functions for gas estimation | (source: file:///handbook/src/state-storage.md#cost-considerations) |
| **Storage Collision** | Incorrect storage slot usage, proxy upgrade issues | Use `#[storage]` macro correctly, avoid manual slot management, test storage layout | (source: file:///handbook/src/state-storage.md#stylus-storage-model) |

## 8. Implementation Phases

### Phase 1: Core Infrastructure
**Objectives:** Establish basic contract structure and access control
**Success Criteria:** Contract deploys, admin functions work, basic access control enforced
**Tasks:**
- Create main contract struct with `#[storage]` and `#[entrypoint]` attributes in `src/lib.rs` (source: file:///handbook/src/state-storage.md#stylus-storage-model)
- Implement admin access control using OpenZeppelin's Ownable pattern in `src/access_control.rs` (source: file:///handbook/src/access-control.md#standardized-access-control-patterns)
- Define error types using `sol!` macro and `SolidityError` derive in `src/errors.rs` (source: file:///handbook/src/errors-events.md#stylus)
- Create event definitions for core operations in `src/events.rs` (source: file:///handbook/src/errors-events.md#stylus)
**Exit Conditions:** All admin functions callable by owner, proper error handling, events emitted correctly

### Phase 2: Configuration Management
**Objectives:** Implement AMM configuration system with fee parameters
**Success Criteria:** Config creation/updates work, fee validation enforced, multiple configs supported
**Tasks:**
- Define `AmmConfig` storage struct in `src/config.rs` with all fee parameters (source: file:///handbook/src/state-storage.md#stylus-storage-model)
- Implement `create_amm_config` function with parameter validation in `src/config.rs` (code: programs/cp-swap/src/instructions/admin/create_config.rs)
- Implement `update_amm_config` with parameter-specific updates in `src/config.rs` (code: programs/cp-swap/src/instructions/admin/update_config.rs)
- Add fee rate validation logic ensuring rates sum correctly in `src/config.rs` (code: programs/cp-swap/src/lib.rs:67-70)
**Exit Conditions:** Multiple configs can be created, all parameters validated, owner-only access enforced

### Phase 3: Pool State Management
**Objectives:** Implement core pool data structures and initialization
**Success Criteria:** Pools can be created, state persisted correctly, token pairs managed
**Tasks:**
- Define `PoolState` storage struct with all required fields in `src/pool.rs` (code: programs/cp-swap/src/states/pool.rs)
- Implement pool key generation from token pair and config in `src/pool.rs` (source: file:///handbook/src/state-storage.md#nested-mappings)
- Create `initialize` function for basic pool creation in `src/pool.rs` (code: programs/cp-swap/src/instructions/initialize.rs)
- Create `initialize_with_permission` for advanced pool features in `src/pool.rs` (code: programs/cp-swap/src/instructions/initialize_with_permission.rs)
- Add pool status management (enable/disable operations) in `src/pool.rs` (code: programs/cp-swap/src/states/pool.rs:180-197)
**Exit Conditions:** Pools initialize with correct state, status flags work, creator tracking functional

### Phase 4: Liquidity Operations
**Objectives:** Enable deposit and withdrawal of liquidity
**Success Criteria:** LP tokens minted/burned correctly, proportional asset distribution, slippage protection
**Tasks:**
- Implement LP token balance tracking using storage maps in `src/liquidity.rs` (source: file:///handbook/src/state-storage.md#stylus-storage-model)
- Create `deposit` function with slippage protection in `src/liquidity.rs` (code: programs/cp-swap/src/instructions/deposit.rs)
- Create `withdraw` function with minimum output validation in `src/liquidity.rs` (code: programs/cp-swap/src/instructions/withdraw.rs)
- Add liquidity calculation functions (constant product formula) in `src/math.rs` (code: programs/cp-swap/src/curve/constant_product.rs)
- Implement ERC20 token interaction helpers in `src/token_operations.rs` (source: file:///handbook/src/external-calls.md#stylus)
**Exit Conditions:** Liquidity can be added/removed, LP tokens track correctly, slippage limits enforced

### Phase 5: Swap Functionality
**Objectives:** Implement token swapping with fee collection
**Success Criteria:** Swaps execute correctly, fees calculated properly, price impact accurate
**Tasks:**
- Implement constant product curve calculations in `src/curve.rs` (code: programs/cp-swap/src/curve/constant_product.rs)
- Create `swap_base_input` function with input amount specification in `src/swap.rs` (code: programs/cp-swap/src/instructions/swap_base_input.rs)
- Create `swap_base_output` function with output amount specification in `src/swap.rs` (code: programs/cp-swap/src/instructions/swap_base_output.rs)
- Implement multi-tier fee collection (protocol, fund, creator) in `src/fees.rs` (code: programs/cp-swap/src/curve/fees.rs)
- Add price oracle updates and TWAP calculations in `src/oracle.rs` (code: programs/cp-swap/src/states/oracle.rs)
**Exit Conditions:** All swap types work, fees distributed correctly, price oracle functional

### Phase 6: Fee Collection System
**Objectives:** Enable fee withdrawal by authorized parties
**Success Criteria:** Different fee types collected by correct authorities, accounting accurate
**Tasks:**
- Implement `collect_protocol_fee` for protocol owner in `src/fee_collection.rs` (code: programs/cp-swap/src/instructions/admin/collect_protocol_fee.rs)
- Implement `collect_fund_fee` for fund owner in `src/fee_collection.rs` (code: programs/cp-swap/src/instructions/admin/collect_fund_fee.rs)
- Implement `collect_creator_fee` for pool creators in `src/fee_collection.rs` (code: programs/cp-swap/src/instructions/collect_creator_fee.rs)
- Add fee tracking and accumulation logic in `src/fee_tracking.rs` (code: programs/cp-swap/src/states/pool.rs:326-369)
**Exit Conditions:** All fee types can be collected, proper authorization enforced, accurate accounting

### Phase 7: Permission System & Final Integration
**Objectives:** Complete permission-based pool creation and comprehensive testing
**Success Criteria:** Permission system works, all components integrated, security validated
**Tasks:**
- Implement permission PDA equivalent using storage maps in `src/permissions.rs` (source: file:///handbook/src/access-control.md#stylus-authentication-model)
- Add permission checks to pool creation functions in `src/permissions.rs` (code: programs/cp-swap/src/instructions/initialize_with_permission.rs)
- Implement comprehensive integration tests in `tests/integration.rs` (source: file:///handbook/src/testing-debugging.md#unit-testing)
- Add end-to-end testing with mock ERC20 tokens in `tests/e2e.rs` (source: file:///handbook/src/testing-debugging.md#example)
- Perform security audit and optimization review in `tests/security.rs` (source: file:///handbook/src/security-considerations.md)
**Exit Conditions:** Full functionality working, all tests passing, security review completed

## 9. Boilerplate Artifacts

### Cargo.toml
```toml
[package]
name = "raydium-cp-swap-stylus"
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

### main.rs
```rust
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]

#[cfg(not(any(test, feature = "export-abi")))]
#[no_mangle]
pub extern "C" fn main() {}

#[cfg(feature = "export-abi")]
fn main() {
    raydium_cp_swap_stylus::print_from_args();
}
```

All dependency versions are pinned to ensure reproducible builds. The `export-abi` feature enables ABI generation for contract interaction. The `motsu` test framework provides enhanced testing capabilities for multi-contract scenarios.

## 10. Test Plan

All testing must conform to handbook guidelines in `testing-debugging.md`. The test strategy encompasses three categories:

**Unit Tests:** Test each instruction's pre/post-conditions using `motsu` framework. Create tests for `create_amm_config` validating fee rate limits, `initialize` checking initial liquidity constraints, `deposit/withdraw` verifying LP token calculations, and `swap` functions validating constant product invariants. Each test isolates individual contract functions with mock dependencies. (source: file:///handbook/src/testing-debugging.md#unit-testing)

**Property-Based Tests:** Implement invariant testing for AMM properties using fuzzing. Test constant product invariant `x * y = k` across all swap operations, ensure LP token supply equals square root of product after initialization, verify total fees collected never exceed trade volume, and validate that all operations maintain pool solvency. Use `arbitrary` crate for generating test inputs. (source: file:///handbook/src/testing-debugging.md#unit-testing)

**Negative Tests:** Test authorization failures with unauthorized callers attempting admin functions, account constraint violations with invalid token addresses or zero amounts, and CPI failure surfaces by testing with failing ERC20 contracts. Verify slippage protection triggers correctly and fee collection fails with insufficient accumulated fees. (source: file:///handbook/src/testing-debugging.md#unit-testing)

**Integration Tests:** Deploy complete contract ecosystem with mock ERC20 tokens, test full swap workflows from pool creation through trading, and validate multi-pool scenarios with different configurations. Test permission-based pool creation and comprehensive fee collection flows. (source: file:///handbook/src/testing-debugging.md#example)

## 11. Handbook References

**State Storage:**
- file:///handbook/src/state-storage.md#solana-to-stylus-type-mappings
- file:///handbook/src/state-storage.md#nested-mappings
- file:///handbook/src/state-storage.md#stylus-storage-model
- file:///handbook/src/state-storage.md#cost-considerations

**Access Control:**
- file:///handbook/src/access-control.md#stylus-authentication-model
- file:///handbook/src/access-control.md#standardized-access-control-patterns

**External Calls:**
- file:///handbook/src/external-calls.md#stylus

**Errors & Events:**
- file:///handbook/src/errors-events.md#stylus

**Security Considerations:**
- file:///handbook/src/security-considerations.md#reentrancy
- file:///handbook/src/security-considerations.md#integer-arithmetic-overflow
- file:///handbook/src/security-considerations.md#sender-authorization

**Testing & Debugging:**
- file:///handbook/src/testing-debugging.md#unit-testing
- file:///handbook/src/testing-debugging.md#example