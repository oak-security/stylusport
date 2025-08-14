# Appendix C: Resources and Glossary

## Glossary

### A

**ABI (Application Binary Interface)**: The standard interface between smart contracts in Ethereum. Defines how functions are called, parameters are encoded, and return values are decoded. In Stylus, the SDK automatically generates ABI-compatible interfaces from your Rust code.

**Account**: In Solana, accounts are the fundamental storage unit that hold both programs and data. In Ethereum/Stylus, this concept maps to either EOAs (user wallets) or contract accounts (deployed contracts with code and storage).

**Address**: A 20-byte identifier in Ethereum that identifies accounts and contracts. Shorter than Solana's 32-byte Pubkeys. Stylus uses the `alloy_primitives::Address` type.

**Alloy**: Modern Rust library for Ethereum development, providing primitives and utilities. Stylus SDK builds on Alloy for types like `Address`, `U256`, and `FixedBytes`.

**Anchor**: Popular framework for Solana development that provides macros and abstractions. Many patterns in this book show how to migrate from Anchor to Stylus equivalents.

**Arbitrum**: Layer 2 scaling solution for Ethereum that maintains EVM compatibility while reducing costs. Stylus is Arbitrum's WASM-based smart contract platform.

### B

**Block**: Collection of transactions processed together. Arbitrum produces blocks more frequently than Ethereum L1, but block timestamps are estimates derived from L1.

**Borsh**: Binary Object Representation Serializer for Hashing - the primary serialization format in Solana. Stylus uses ABI encoding instead, which is automatically handled by the SDK.

**Bump**: In Solana PDAs, the bump is a value that ensures the derived address is off the ed25519 curve. No equivalent concept in Ethereum/Stylus.

### C

**Calldata**: Input data sent with a transaction to a contract. In Stylus, function parameters are automatically encoded/decoded from calldata.

**Canonical Bump**: The highest valid bump value for a PDA in Solana. Not applicable in Ethereum's addressing scheme.

**Compute Budget**: Solana's system for limiting computational resources per transaction. Equivalent to gas limits in Ethereum, but with different pricing models.

**Compute Units**: Solana's measure of computational work. In Ethereum/Stylus, this maps to gas consumption.

**Constructor**: Special initialization function in Stylus contracts, marked with `#[constructor]`. Called once during deployment, similar to Anchor's `initialize` but part of the deployment transaction. Typically does not return Result. Any panic or assert failure aborts deployment.

**Contract Storage**: Persistent state storage in Ethereum contracts. Organized in 256-bit slots, unlike Solana's account-based storage model.

**CPI (Cross-Program Invocation)**: Solana's mechanism for inter-program calls. In Stylus, this is replaced by interface calls via `sol_interface!` or low-level `call`, `static_call`, or `delegate_call` methods.

### D

**Data Account**: In Solana, accounts that store program state. In Stylus, this concept maps to the contract's storage variables.

**Delegate Call**: EVM call type that executes another contract's code in the caller's storage context. Used for proxy patterns and libraries.

**Deployment**: Process of uploading contract code to the blockchain. The `cargo stylus deploy` command uploads the WASM and deploys the contract. If a constructor is present, it runs atomically as part of deployment.

### E

**Entrypoint**: In Stylus, the main contract struct marked with `#[entrypoint]`. Similar to Anchor's `#[program]` module but for the entire contract interface.

**ERC-20**: Ethereum's fungible token standard. Replaces Solana's SPL Token program with a standardized interface pattern.

**ERC-721**: Ethereum's NFT standard. Replaces Solana's Metaplex token standard with contract-based NFT logic.

**Error**: In Stylus, custom errors are defined with `sol!` macro and derive `SolidityError`. More structured than Solana's ProgramError.

**Event**: Logging mechanism in Ethereum for communicating state changes. Replaces Solana's msg! logs with structured, indexed event data.

**EVM (Ethereum Virtual Machine)**: Execution environment for Ethereum contracts. Stylus compiles to WASM but maintains EVM compatibility through the SDK.

### F

**Fallback Function**: Special function in Stylus marked with `#[fallback]` that handles calls with unknown function selectors or non-empty calldata that does not match any function. Can be marked as `#[payable]`.

**Function Selector**: First 4 bytes of keccak256 hash of a function signature. Used by EVM to route calls to the correct function.

### G

**Gas**: Unit measuring computational work in Ethereum. Every operation has a gas cost, paid in ETH. Replaces Solana's compute units with a different pricing model.

**Gas Limit**: Maximum gas a transaction can consume. Similar to Solana's compute budget but measured and priced differently.

**Gas Price**: Amount of ETH per gas unit. Unlike Solana's fixed compute unit pricing, gas prices fluctuate with network demand.

### I

**IDL (Interface Definition Language)**: In Anchor, the JSON representation of a program's interface. Stylus exports ABI via `cargo stylus export-abi`. For interface detection, EIP-165 applies to Solidity contracts. In Stylus, you can implement EIP-165 patterns if interoperating with contracts that expect it.

**Inheritance**: Pattern in Stylus using `#[inherit]` to compose contracts. Different from Solana's program composition through CPIs.

**Ink**: Stylus' internal metering unit that maps 1:1 to EVM gas for users. This is an internal implementation detail - developers still reason in terms of gas.

**Instruction**: In Solana, the basic unit of program execution. In Stylus, instructions map to contract methods/functions.

**Interface**: In Stylus, defined with `sol_interface!` macro to call other contracts. Replaces Solana's CPI account structures.

### K

**Keccak256**: Cryptographic hash function used throughout Ethereum. Replaces Solana's use of SHA256 for most hashing needs.

### L

**Lamport**: Smallest unit of SOL (10^-9 SOL). This compares to wei, which is 10^-18 ETH. The difference is about unit precision, not asset price.

**Low-level Call**: Direct contract invocation using `call`, `static_call`, or `delegate_call`. More flexible but less safe than interface calls.

### M

**Mapping**: Solidity/Stylus storage type similar to hash maps. In Stylus, use `StorageMap<K, V>` to replace Solana's PDA-based lookups.

**Memory**: Temporary data storage during execution. Stylus manages memory automatically, unlike Solana's explicit account data management.

**msg**: Global context in Stylus providing `sender()`, `value()`, etc. Replaces Solana's instruction context and account infos.

**Multicall**: Pattern for batching multiple operations in one transaction. No direct Solana equivalent due to different execution models.

### N

**Native Token**: ETH in Ethereum/Arbitrum, SOL in Solana. Both chains handle native tokens differently from user-defined tokens.

**Nonce**: In Ethereum, transaction counter preventing replay attacks. In Solana PDAs, part of the derivation seeds.

### O

**Oracle**: External data provider for smart contracts. Similar patterns exist in both ecosystems but with different trust models.

**Owner**: Common pattern for access control. In Stylus, typically stored as `StorageAddress`, replacing Solana's signer checks.

### P

**Payable**: Function modifier in Stylus (`#[payable]`) allowing functions to receive ETH. No Solana equivalent as SOL transfers are explicit.

**PDA (Program Derived Address)**: Solana's deterministic addresses without private keys. No exact equivalent in Ethereum. Use mappings for similar functionality, or CREATE2 if you need deterministic contract addresses.

**Precompile**: Built-in contracts in Ethereum for expensive operations. Stylus can call precompiles like ecrecover at fixed addresses.

**Program**: Solana's term for smart contracts. In Stylus context, "program" and "contract" are often used interchangeably.

**Program ID**: In Solana, the public key identifying a program. Maps to contract address in Ethereum/Stylus.

**Proxy**: Contract pattern for upgradability. Replaces Solana's native program upgrade authority with contract-based patterns.

### R

**Receive Function**: Special function in Stylus marked with `#[receive]` that triggers on plain ETH transfers with empty calldata. Can be marked as `#[payable]`. No Solana equivalent.

**Reentrancy**: Vulnerability where external calls can re-enter the contract. Different attack vectors than Solana due to execution model.

**Rent**: In Solana, fee for data storage. Ethereum uses gas for storage operations instead of ongoing rent.

**Revert**: Transaction failure in EVM that undoes all state changes. Similar to Solana's Error returns but with automatic state rollback.

**RPC**: Remote Procedure Call endpoints for interacting with the blockchain. Both ecosystems use JSON-RPC but with different method sets.

### S

**Seeds**: In Solana, inputs for PDA derivation. In Stylus, might be used as storage keys or for deterministic computations.

**Selector**: 4-byte function identifier in Ethereum. Computed as `keccak256(signature)[:4]`. No Solana equivalent.

**Signer**: In Solana, account that signed the transaction. In Stylus, the immediate caller is accessed via `msg::sender()`. Both EOAs and contracts can appear as senders, so be careful with proxy patterns.

**Slot**: 32-byte storage unit in Ethereum contracts. Each storage variable occupies one or more slots.

**Solidity**: Primary language for Ethereum contracts. Stylus provides compatibility while using Rust.

**Static Call**: Read-only contract call that cannot modify state. Similar to Solana's view of account data without mutation.

**Storage**: Persistent contract data in Ethereum. Replaces Solana's account data with slot-based storage model.

**Stylus**: Arbitrum's WASM-based smart contract platform enabling Rust, C, and C++ development with EVM compatibility.

**System Program**: Solana's built-in program for account management. No direct equivalent in Ethereum; similar operations are native or via CREATE2.

### T

**TestVM**: Stylus testing framework providing mocked host environment. Similar to Solana's ProgramTest but with different capabilities.

**Token Account**: In Solana, separate accounts holding token balances. In ERC-20, balances are stored in the token contract's storage.

**Token Program**: Solana's built-in program for tokens. Replaced by ERC-20/ERC-721 contract standards in Ethereum.

**Transaction**: Signed operation modifying blockchain state. Similar concept but different structures between Solana and Ethereum.

### U

**Upgradeable**: In Solana, programs can be upgraded by authority. In Stylus, requires proxy patterns as contracts are immutable by default.

### V

**View Function**: Read-only function that doesn't modify state. Free to call externally, similar to Solana's account data queries.

### W

**WASM (WebAssembly)**: Compilation target for Stylus contracts. Provides near-native performance compared to EVM bytecode.

**Wei**: Smallest unit of ETH (10^-18 ETH). Wei has 18 decimal places compared to lamports' 9 decimals. The difference is about unit precision, not asset price.

### Z

**Zero-Copy**: Deserialization pattern in Stylus for large data structures. Different from Solana's zero-copy which avoids account data copying.

## Conceptual Mappings

### Solana to Stylus Equivalents

| Solana Concept | Stylus Equivalent | Notes |
|----------------|-------------------|-------|
| Program | Contract | Both are executable code on-chain |
| Account | Storage Variable | State storage mechanism |
| Instruction | Function/Method | Entry points for execution |
| PDA | Deterministic Address | Different derivation methods |
| CPI | Contract Call | Inter-contract communication |
| Signer | msg::sender() | Transaction authorization |
| Lamports | Wei | Native token units |
| Compute Units | Gas | Resource metering |
| Rent | Storage Gas Cost | One-time vs ongoing cost |
| Borsh | ABI Encoding | Serialization format |

### Common Patterns Translation

| Pattern | Solana | Stylus |
|---------|---------|---------|
| Access Control | Signer checks | Owner modifiers |
| Token Transfers | SPL Token CPI | ERC-20 interface |
| Data Storage | Account data | Contract storage |
| Initialization | init instruction | Constructor |
| Upgrades | Program authority | Proxy contracts |
| Batching | Multiple instructions | Multicall |
| Randomness | Recent blockhashes | Block properties (caution: manipulable) |
| Time | Clock sysvar | block::timestamp() |

## Code Examples

### Function Selector Example

```rust
use alloy_primitives::{keccak256, FixedBytes};

fn selector(sig: &str) -> [u8; 4] {
    let h = keccak256(sig.as_bytes());
    [h[0], h[1], h[2], h[3]]
}
// e.g., "transfer(address,uint256)"
```

### Interface Calls in Stylus

```rust
use stylus_sdk::{prelude::*, call::Call};
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;

sol_interface! {
    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

pub fn erc20_transfer(&mut self, token: Address, to: Address, amount: U256) -> Result<(), Vec<u8>> {
    let erc20 = IERC20::new(token);
    let ok = erc20.transfer(Call::new_in(self).gas(100_000), to, amount)
        .map_err(|_| b"ERC20 call reverted".to_vec())?;
    if !ok { return Err(b"ERC20 returned false".to_vec()); }
    Ok(())
}
```

### CREATE2 Deterministic Address

```rust
// Predictable child address for namespacing and patterns
use alloy_primitives::{Address, Bytes, keccak256, FixedBytes};

fn create2_address(deployer: Address, salt: FixedBytes<32>, init_code: &Bytes) -> Address {
    let data = [&[0xff], deployer.as_slice(), salt.as_slice(), &keccak256(init_code)][..].concat();
    Address::from_slice(&keccak256(&data)[12..])
}
```

### Precompile Addresses

```rust
// Built-in Ethereum precompiles
const ECRECOVER: Address = address!("0000000000000000000000000000000000000001");
const SHA256: Address = address!("0000000000000000000000000000000000000002");
const RIPEMD160: Address = address!("0000000000000000000000000000000000000003");
const IDENTITY: Address = address!("0000000000000000000000000000000000000004");
```

## Network Facts

| Network | Chain ID | Notes |
|---------|----------|---------|
| Arbitrum One | 42161 | Mainnet |
| Arbitrum Sepolia | 421614 | Testnet |

**Note**: Public RPCs can rate-limit. Use your own provider for production applications.

## Additional Resources

### Official Documentation

**Arbitrum Stylus**
- Main Documentation: [docs.arbitrum.io/stylus](https://docs.arbitrum.io/stylus)
- SDK Reference: [docs.rs/stylus-sdk](https://docs.rs/stylus-sdk/latest/stylus_sdk/)
- Stylus by Example: [stylus-by-example.org](https://stylus-by-example.org)

**Development Tools**
- Cargo Stylus: [github.com/OffchainLabs/cargo-stylus](https://github.com/OffchainLabs/cargo-stylus)
- Example Contracts: [github.com/OffchainLabs/stylus-sdk-rs/examples](https://github.com/OffchainLabs/stylus-sdk-rs/tree/main/examples)

### Network Information

**RPC Endpoints**

Arbitrum One (Mainnet):
```
https://arb1.arbitrum.io/rpc
https://arbitrum-mainnet.infura.io/v3/YOUR-PROJECT-ID
```

Arbitrum Sepolia (Testnet):
```
https://sepolia-rollup.arbitrum.io/rpc
https://public.stackup.sh/api/v1/node/arbitrum-sepolia
```

**Block Explorers**
- Mainnet: [arbiscan.io](https://arbiscan.io)
- Sepolia: [sepolia.arbiscan.io](https://sepolia.arbiscan.io)

**Faucets**
- Use reputable Arbitrum Sepolia faucets from major infrastructure providers or bridge Sepolia ETH

### Developer Resources

**Community**
- Discord: [discord.gg/arbitrum](https://discord.gg/arbitrum) (#stylus channel)
- Forum: [forum.arbitrum.io](https://forum.arbitrum.io)
- GitHub Discussions: [github.com/OffchainLabs/stylus-sdk-rs/discussions](https://github.com/OffchainLabs/stylus-sdk-rs/discussions)

**Learning Materials**
- Migration Examples: This book's companion repository
- Video Tutorials: Arbitrum YouTube channel
- Workshop Materials: Regular developer workshops

**Developer Tools**
- Cargo Stylus: Official CLI tool for deployment and management  
- Foundry: Use for scripts and tests against deployed contracts. Compilation and deployment of Stylus code is via `cargo stylus`
- Rust Analyzer: Standard Rust language server with excellent IDE support

### Troubleshooting Resources

**Common Issues**
- Stack too deep: Use `Box<T>` for large storage types
- Out of gas: Check gas limits and optimize storage access
- Serialization errors: Ensure proper type conversions
- Deployment failures: Verify constructor parameters

**Debug Tools**
- `stylus-sdk` debug feature for local testing
- Arbiscan contract verification
- Tenderly for transaction debugging

**Migration Checklist**
1. Map account structure to storage variables
2. Convert instructions to functions
3. Replace CPIs with contract calls
4. Adapt error handling patterns
5. Implement events for logging
6. Test with TestVM framework
7. Deploy to testnet first
8. Verify on block explorer

This glossary and resource guide serves as a comprehensive reference for developers migrating from Solana to Arbitrum Stylus. Keep it handy as you work through migration projects and contribute back to the community as you gain experience.