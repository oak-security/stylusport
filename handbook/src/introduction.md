# Introduction

This handbook provides a comprehensive guide for migrating Solana programs to Arbitrum Stylus smart contracts. This guide helps you understand the key differences, set up your development environment, and provides practical patterns for successful migration.

## What Arbitrum Stylus offers

Arbitrum Stylus provides a smart contract platform that supports developers writing contracts in Rust, C, and C++. The platform maintains full Ethereum Virtual Machine (EVM) compatibility. Unlike traditional EVM chains that require Solidity, Stylus enables the use of languages that compile to WebAssembly (WASM), offering:

- **Performance**: 10-100x faster execution than Solidity
- **Memory efficiency**: More efficient memory usage and lower gas costs
- **Familiar languages**: Use Rust, C, or C++ instead of learning Solidity
- **EVM compatibility**: Full interoperability with existing Ethereum tooling
- **Rust safety**: Memory safety and thread safety guarantees

## Why migrate from Solana to Stylus

### Technical advantages

**Shared Language**: Both Solana and Stylus support Rust, reducing the learning curve and enabling code reuse of business logic, data structures, and algorithms.

**Enhanced Interoperability**: Stylus contracts can interact seamlessly with the broader Ethereum ecosystem, including DeFi protocols, bridges, and tooling.

**Simplified Architecture**: The EVM account model reduces complexity compared to Solana's account model in state management and cross-contract interactions.

### Business benefits

**Market Access**: Tap into Ethereum's larger user base and liquidity pools
**Tooling Ecosystem**: Leverage mature development tools and infrastructure
**EVM Compatibility**: Easy integration with existing Ethereum protocols and services

## Key differences overview

| Aspect | Solana | Stylus |
|--------|--------|---------|
| **Language** | Rust (native/Anchor) | Rust + EVM compatibility |
| **Account Model** | Explicit accounts | EVM account model |
| **State Storage** | Account data | Contract storage |
| **Function Calls** | Instructions | Direct method calls |
| **Gas Model** | Compute units | Wei/Gas |
| **Concurrency** | High (parallel execution) | Sequential (EVM) |

## Development environment setup

Before starting your migration, set up your development environment and understand the foundational concepts.

### Required tools

**1. Rust toolchain**
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Ensure you have the latest stable version
rustup update stable
rustup default stable

# Add WebAssembly target
rustup target add wasm32-unknown-unknown
```

**2. Stylus CLI**
```bash
# Install Stylus CLI
cargo install --force cargo-stylus

# Verify installation
cargo stylus --version
```

**3. Additional tools**
```bash
# For local development and testing
docker pull offchainlabs/nitro-node:latest
```

### Optional but recommended

**Solana CLI** (if migrating existing projects):
```bash
# Install Solana CLI for reference/testing
sh -c "$(curl -sSfL https://release.solana.com/v1.18.0/install)"
```

**Anchor CLI** (if working with Anchor projects):
```bash
# Install Anchor CLI
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest
```

## Project structure comparison

Understanding the typical project structures helps you organize your migration:

### Solana native project
```
solana-program/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Program entrypoint
│   ├── processor.rs    # Instruction processing
│   ├── instruction.rs  # Instruction definitions
│   ├── state.rs        # Account state structures
│   └── error.rs        # Program errors
└── tests/
    └── integration.rs
```

### Anchor project
```
anchor-program/
├── Anchor.toml
├── programs/
│   └── my-program/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs   # All-in-one program file
├── tests/
└── migrations/
```

### Stylus project
```
stylus-contract/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Contract implementation
│   └── main.rs         # ABI export entry point
└── tests/
    └── integration.rs
```

## Key concept mappings

### Architecture differences

| Concept | Solana | Stylus |
|---------|--------|--------|
| **Execution Model** | Program processes instructions | Contract methods called directly |
| **State Storage** | Separate account data | Contract storage variables |
| **Entry Point** | `process_instruction()` | `#[public]` methods |
| **Initialization** | Via instruction | Constructor or initialization method |
| **Access Control** | Signer checks | `msg::sender()` checks |

### Data type mappings

| Solana Type | Stylus Type | Notes |
|-------------|-------------------|-------|
| `Pubkey` | `Address` | 20 bytes vs 32 bytes |
| `u64` | `U256` | Use U256 for consistency |
| `Vec<u8>` | `Vec<u8>` or `Bytes` | Direct mapping |
| `String` | `String` | Direct mapping |
| `Account<T>` | Storage struct fields | Different access patterns |

### Function patterns

**Solana native:**
```rust
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Match instruction type
    // Validate accounts
    // Process instruction
}
```

**Anchor:**
```rust
#[program]
pub mod my_program {
    pub fn my_instruction(ctx: Context<MyAccounts>, data: MyData) -> Result<()> {
        // Direct processing with type safety
    }
}
```

**Stylus:**
```rust
#[public]
impl MyContract {
    pub fn my_method(&mut self, data: MyData) -> Result<(), Vec<u8>> {
        // Direct method call with EVM semantics
    }
}
```

## Quick start verification

Create a basic Stylus project to verify your setup:

```bash
# Create new Stylus project
cargo stylus new my-migration-test
cd my-migration-test

# Check that everything compiles
cargo stylus check

# Export ABI to verify setup
cargo stylus export-abi
```

**Expected output:**
```solidity
interface IMyMigrationTest {
    // Your contract interface will appear here
}
```

## Understanding gas vs compute units

One key difference to understand early:

### Solana compute units
- Fixed compute budget per transaction (1.4M compute units default)
- Deterministic costs based on instruction complexity
- Parallel processing possible
- Compute units can be increased per instruction

### Stylus gas
- Dynamic gas costs based on execution
- Gas limit set per transaction
- Sequential execution (EVM model)
- Much more efficient than Solidity (10-100x)
- Ink pricing model for compute efficiency

## Migration strategy

This handbook follows a systematic approach:

1. **Program Structure**: Convert entry points and instruction dispatch
2. **State Storage**: Transform account-based storage to contract storage
3. **Access Control**: Migrate signer checks and PDA patterns
4. **External Calls**: Convert CPIs to contract interactions
5. **Token Operations**: Handle native and custom tokens
6. **Error Handling**: Map program errors to EVM reverts and events

## Code examples structure

Throughout this handbook, three types of code examples appear:

**Solana native**: Raw Solana program implementation
```rust
// Native Solana program example
use solana_program::*;
```

**Anchor**: Anchor framework implementation  
```rust
// Anchor program example
use anchor_lang::prelude::*;
```

**Stylus**: Target Stylus implementation
```rust
// Stylus contract example
use stylus_sdk::prelude::*;
```

Each chapter includes working examples that you can run, test, and modify.

## Migration checklist

Before starting the migration process, confirm you have:

### Environment setup
- [ ] Rust toolchain installed and updated
- [ ] WebAssembly target added (`wasm32-unknown-unknown`)
- [ ] Stylus CLI installed and working
- [ ] Test project compiles with `cargo stylus check`
- [ ] Familiar with `cargo stylus export-abi` command

### Knowledge prerequisites
- [ ] Basic Rust programming
- [ ] Understanding of your current Solana program structure
- [ ] Familiarity with blockchain concepts (accounts, transactions, etc.)
- [ ] Basic understanding of EVM concepts (gas, reverts, events)

### Migration planning
- [ ] Identified all program instructions/methods
- [ ] Mapped account structures to storage needs
- [ ] Listed external program dependencies
- [ ] Documented current error handling approach
- [ ] Planned testing strategy

## When not to migrate

Consider staying on Solana if:
- Your application requires parallel transaction execution
- You need Solana-specific features (rent economics, native programs)
- Your business model depends on Solana's fee structure
- Deep integration with Solana-native protocols exists

## Troubleshooting

### Common setup issues

**"cargo stylus not found"**
```bash
# Make Cargo's bin directory available in PATH
export PATH="$HOME/.cargo/bin:$PATH"
source ~/.bashrc  # or ~/.zshrc
```

**"WebAssembly target not found"**
```bash
# Add WebAssembly target
rustup target add wasm32-unknown-unknown
```

**Compilation errors**
- Use compatible versions of dependencies
- Check that your Rust version is up to date
- Verify all required features are enabled in Cargo.toml
- Ensure you're using the correct Stylus SDK version

For more detailed troubleshooting, refer to the [official Stylus documentation](https://docs.arbitrum.io/stylus/).

## Next steps

With your environment set up and understanding of the key differences, you can begin the migration process. The next chapter covers [Program Structure & Instructions](./program-structure.md), where your Solana program's entry points and instruction handling transform into Stylus contract methods.

## Need help

- **Examples Repository**: All code examples from this handbook are available with full tests
- **Community**: Join the Arbitrum Discord for developer support
- **Documentation**: Refer to official Stylus documentation for detailed API references

---

*This handbook assumes familiarity with Rust and basic blockchain concepts. If you are new to Solana or Arbitrum, review their respective documentation first.*