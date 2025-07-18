# Solana & Anchor to Stylus Concept Mapping

## Core Architecture & Concepts

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **Programs** | **Smart Contracts** | Solana programs are stateless and store data in separate accounts; Stylus contracts combine code and state |
| **Accounts Model** | **Storage Variables** | Solana uses separate accounts for data storage; Stylus uses contract storage slots |
| **Program Derived Addresses (PDAs)** | **Storage Slot Indexes** | PDAs are generally used to namespace program-controlled state; Stylus Stylus uses contract storage slots |
| **Cross Program Invocation (CPI)** | **External Contract Calls** | Both allow inter-contract communication |
| **Rent** | **Gas for Storage** | Solana requires rent for account storage; Stylus uses gas fees |
| **Lamports** | **Wei** | Smallest unit: 1 SOL = 10^9 lamports; 1 ETH = 10^18 wei |

## Account Types & Storage

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **System Program** | **Precompiled Contracts** | System-level operations |
| **Token Program (SPL)** | **ERC-20/ERC-721 Standards** | Token standards implementation |
| **Associated Token Accounts (ATA)** | **Token Balance Mapping** | ATAs are deterministic token accounts; Stylus uses mappings |
| **Account Ownership** | **Contract Ownership Pattern** | Solana has program ownership of accounts; Stylus uses access control |
| **Sysvar Accounts** | **Block/Transaction Context** | Network state access |
| **StorageAccount Types** | **Storage Types** | Mapping of storage types below |

### Storage Type Mappings

| Anchor Storage Type | Stylus Storage Type | Notes |
|--------------------|---------------------|-------|
| `StorageU256` | `StorageU256` | Direct equivalent |
| `StorageAddress` | `StorageAddress` | Direct equivalent |
| `StorageBool` | `StorageBool` | Direct equivalent |
| `StorageMap` | `mapping` | Key-value storage |
| `StorageVec` | `StorageVec` | Dynamic arrays |
| `StorageString` | `StorageString` | String storage |
| `StorageBytes` | `StorageBytes` | Byte array storage |

## Development Patterns

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **#[program] macro** | **#[public] impl** | Defines external methods |
| **#[derive(Accounts)]** | **Function Parameters** | Account validation in Anchor vs parameter passing in Stylus |
| **Context<T>** | **&self/&mut self** | Anchor uses Context wrapper; Stylus uses Rust self |
| **AccountInfo** | **Address Parameter** | Raw account access |
| **Signer Validation** | **msg::sender() checks** | Signature verification |
| **#[account] macro** | **#[storage] struct** | Defines data structures |

## Initialization & Deployment

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **anchor init** | **cargo stylus new** | Project initialization |
| **anchor build** | **cargo stylus build** | Compilation process |
| **anchor deploy** | **cargo stylus deploy** | Deployment command |
| **Program Keypair** | **Contract Address** | Solana uses keypairs; Stylus uses addresses |
| **Upgrade Authority** | **Contract Upgradeability** | Different upgrade mechanisms |

## Token Operations

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **SPL Token Mint** | **ERC-20 Contract** | Token creation |
| **Token Extensions (Token-2022)** | **Extended ERC Standards** | Enhanced token functionality |
| **Mint Authority** | **Minter Role** | Permission to create tokens |
| **Freeze Authority** | **Pausable Pattern** | Token freezing capability |
| **Transfer Hooks** | **Transfer Hooks/Callbacks** | Custom transfer logic |

## Error Handling

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **#[error_code] enum** | **#[derive(SolidityError)]** | Custom error definitions |
| **err!() macro** | **Err() return** | Error returns |
| **require!() macros** | **require statements/assertions** | Validation patterns |
| **ProgramError** | **Contract Errors** | Error types |

## Events & Logging

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **emit!() macro** | **evm::log()** | Event emission |
| **Program logs** | **Event logs** | On-chain logging |
| **Log truncation concerns** | **Gas limits for logs** | Different constraints |

## Testing

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **anchor test** | **cargo test with TestVM** | Test execution |
| **solana-test-validator** | **TestVM** | Local testing environment |
| **BankClient** | **Mock contract calls** | Test utilities |

## VM & Runtime

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **BPF/SBF VM** | **WASM VM** | Execution environment |
| **Compute Units** | **Gas** | Computational metering |
| **Stack/Heap limits** | **Memory limits** | Resource constraints |
| **Syscalls** | **Host functions** | System interactions |

## Special Patterns

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **PDA Signing** | **Contract as Signer** | Programs signing for PDAs vs contracts executing |
| **CPI with Signer Seeds** | **Delegate Calls** | Advanced call patterns |
| **Discriminator (8 bytes)** | **Function Selectors (4 bytes)** | Method identification |
| **Zero-Copy Deserialization** | **Direct Memory Access** | Efficient data handling |

## Network & Environment

| Solana/Anchor Concept | Stylus Equivalent | Notes |
|----------------------|-------------------|-------|
| **Devnet/Testnet/Mainnet** | **Arbitrum Networks** | Network environments |
| **Solana Explorer** | **Arbiscan** | Blockchain explorers |
| **RPC Providers** | **RPC Providers** | Network access |

## Concepts Without Direct Stylus Equivalents

### Solana/Anchor Specific Concepts

1. **Rent-Exempt Accounts**: Stylus doesn't have rent; uses gas for storage operations
2. **Account Ownership Model**: Stylus doesn't separate program and data accounts
3. **Program Derived Addresses (PDAs)**: While CREATE2 exists, the off-curve key derivation is unique to Solana
4. **Sysvar Accounts**: Stylus accesses system state through global functions
5. **Associated Token Program**: Stylus uses simpler token balance mappings
6. **CPI Depth Limits (4)**: Stylus has different call depth limitations
7. **Transaction Size Limits (1232 bytes)**: Different transaction constraints
8. **Account Size Limits (10MB)**: Different storage limitations
9. **Parallel Transaction Processing**: Solana's Sealevel runtime feature
10. **Native Program Loaders**: Stylus uses different deployment mechanisms
11. **BPF/SBF Bytecode**: Stylus uses WASM
12. **Anchor IDL**: Stylus uses Solidity ABI
13. **Anchor Workspace**: Different project organization
14. **Seeds-based Account Derivation**: Closest analog is CREATE2 contract addresses which work differently
15. **Close Account Pattern**: Different cleanup mechanisms
16. **Realloc Constraint**: Dynamic storage reallocation works differently
17. **Token Account Delegates**: Different delegation patterns
18. **Native SOL Wrapping**: Different native token handling
19. **Program Logs Data Encoding**: Different event encoding
20. **Executable Accounts**: Programs aren't stored as accounts in Stylus

### Stylus Specific Concepts (Not in Solana/Anchor)

1. **Fallback/Receive Functions**: Ethereum-specific patterns
2. **Solidity ABI Compatibility**: EVM-specific
4. **Inheritance**: Stylus supports contract inheritance with function overriding
5. **Modifiers**: Function modifiers pattern
7. **CREATE/CREATE2 Opcodes**: EVM-specific deployment
8. **Delegate Calls**: Different from CPIs
9. **Storage Slots**: Different storage model
10. **Constructor Functions**: Different initialization pattern
11. **Payable Functions**: Different from Solana's lamport transfers
12. **View/Pure Functions**: Explicit state mutability
13. **Storage Packing**: EVM storage optimization
14. **Block.timestamp precision**: Different time handling
15. **Multicall Pattern**: Batch operations work differently

## Key Architectural Differences

1. **Account Model vs Storage Model**: Solana separates programs and data into different accounts, while Stylus combines them in contracts
2. **Stateless vs Stateful**: Solana programs are stateless, Stylus contracts maintain state
3. **Explicit vs Implicit Fees**: Solana uses rent, Stylus uses gas
4. **Parallel vs Sequential**: Solana designed for parallel execution, Stylus follows EVM's sequential model
5. **Program Composition**: CPIs vs external calls have different semantics and limitations
