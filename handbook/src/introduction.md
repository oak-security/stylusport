# Introduction

This handbook provides a comprehensive guide for migrating Solana programs to Arbitrum Stylus smart contracts. This guide helps you understand the key differences and provides practical patterns for successful migration.

## What Arbitrum Stylus offers

Arbitrum Stylus provides a smart contract platform that supports developers writing contracts in Rust, C, and C++. The platform maintains full Ethereum Virtual Machine (EVM) compatibility. Unlike most EVM chains that require programming contracts with DSLs like Solidity or Vyper, Stylus enables the use of languages that compile to WebAssembly (WASM), offering:

- **Performance**: 10-100x faster execution than Solidity
- **Memory efficiency**: More efficient memory usage and lower gas costs
- **Familiar languages**: Use Rust, C, C++ and other mainstream languages and tooling instead of coping with Solidity
- **EVM compatibility**: Full interoperability with existing Ethereum tooling

## Why migrate from Solana to Stylus

### Technical advantages

- **Shared Language**: Both Solana and Stylus support Rust, reducing the learning curve and enabling code reuse of business logic, data structures, and algorithms.
- **Enhanced Interoperability**: Stylus contracts can interact seamlessly with the broader Ethereum ecosystem, including DeFi protocols, bridges, and tooling.
- **Simplified Architecture**: The EVM account model reduces complexity compared to Solana's account model in state management and cross-contract interactions.

### Business benefits

- **Market Access**: Tap into Ethereum's large user base and liquidity pools
- **Tooling Ecosystem**: Leverage mature development tools and infrastructure
- **EVM Compatibility**: Easy integration with existing Ethereum protocols and services

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

Before starting your migration, set up your development environment according to the [official documentation](https://docs.arbitrum.io/stylus/quickstart).

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

## Migration strategy

This handbook follows a systematic approach:

1. [Program Structure Migration](./program-structure.md): Convert entry points and instruction dispatch
1. [State Storage Patterns](./state-storage.md): Transform account-based storage to contract storage
1. [Access Control Migration](./access-control.md): Migrate signer checks and PDA patterns
1. [External Program Calls](./external-calls.md): Convert CPIs to contract interactions
1. [Native Token Operations](./native-tokens.md): Handle receiving, holdings and transferring native tokens
1. [Fungible Token Handling](./fungible-tokens.md): Convert SPL tokens usage to ERC20 contracts, extensions and interfaces
1. [Non-Fungible Token Handling](./non-fungible-tokens.md): Convert Metaplex NFT metadata to the ERC721 standard
1. [Errors and Events](./errors-events.md): Map program errors to EVM reverts and events

Each chapter includes working examples that you can run, test, and build on.

*This handbook assumes familiarity with Rust and basic blockchain concepts. If you are new to Solana or Arbitrum, review their respective documentation first.*

## Next steps

With your environment set up and understanding of the handbook structure, you can begin the migration process. The next chapter covers [Program Structure Migration](./program-structure.md), where your Solana program's entry points and instruction handling transform into Stylus contract methods.
