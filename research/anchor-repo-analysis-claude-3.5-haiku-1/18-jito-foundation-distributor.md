# 18-jito-foundation-distributor - Solana Programs Analysis

## research/anchor-repos/18-jito-foundation-distributor/cli/Cargo.toml

Here's a comprehensive report for the Jito Foundation Distributor CLI package:

### File Tree Diagram
```
cli/
├── Cargo.toml                  # Project dependency and configuration manifest
└── src/
    └── bin/
        └── cli.rs              # Main CLI application entry point and command handlers
```

### Dependencies List
```
- anchor-lang           # Solana program development framework
- anchor-spl            # Solana Program Library token utilities
- clap                  # Command-line argument parsing
- jito-merkle-tree      # Custom Merkle tree implementation
- merkle-distributor    # Local Merkle distribution program
- solana-program        # Core Solana blockchain programming utilities
- solana-rpc-client     # Solana RPC client for blockchain interactions
- solana-sdk            # Solana SDK for key management and transactions
- spl-associated-token-account  # SPL Associated Token Account utilities
```

### Package Summary
A Solana-based CLI tool for managing token distributions using Merkle tree proofs, specifically designed for the Jito Foundation's token airdrop and distribution mechanism. The application enables administrators to create, manage, and execute token distribution events with advanced features like vesting, claiming, and clawback.

### Notable Features
1. Merkle Tree-based Distribution
   - Secure, gas-efficient token distribution
   - Supports large-scale airdrops
   - Cryptographically verifiable claim proofs

2. Flexible Distribution Mechanisms
   - Vesting schedules
   - Partial and full token claims
   - Admin-controlled clawback functionality

3. Command-line Interface
   - Create new distributors
   - Generate Merkle trees from CSV
   - Claim tokens
   - Manage admin permissions
   - Recover unclaimed tokens

### Implementation Highlights
- Uses Anchor framework for Solana program development
- Leverages Merkle tree cryptography for efficient distribution
- Supports complex token distribution scenarios
- Provides a user-friendly CLI for interaction

The package represents a sophisticated, secure approach to managing token distributions on the Solana blockchain.

---

## research/anchor-repos/18-jito-foundation-distributor/programs/merkle-distributor/Cargo.toml

# Merkle Distributor Program Analysis

## File Tree Diagram
```
programs_merkle-distributor/
│
├── Cargo.toml                 # Project dependencies and configuration
└── src/
    ├── lib.rs                 # Main program entrypoint and instruction definitions
    │
    ├── error.rs               # Custom error codes for the distribution mechanism
    │
    ├── instructions/          # Instruction handlers for various program actions
    │   ├── mod.rs             # Module organization for instructions
    │   ├── new_distributor.rs # Create new token distribution merkle tree
    │   ├── new_claim.rs       # Handle initial token claim with merkle proof
    │   ├── claim_locked.rs    # Manage time-locked token claims
    │   ├── clawback.rs        # Recover unclaimed tokens after lockup
    │   ├── set_admin.rs       # Change program administrator
    │   └── set_clawback_receiver.rs  # Update clawback token receiver
    │
    └── state/                 # Data structures for program state
        ├── mod.rs             # State module organization
        ├── claim_status.rs    # Track individual claim statuses
        ├── claimed_event.rs   # Define claim-related events
        └── merkle_distributor.rs  # Main distributor account structure
```

## Dependencies
```json
{
  "anchor-lang": "Solana program development framework",
  "anchor-spl": "Solana token program interactions",
  "bytemuck": "Byte-level memory manipulation",
  "jito-merkle-verify": "Merkle proof verification utility",
  "solana-program": "Core Solana programming primitives",
  "solana-security-txt": "Security contact information"
}
```

## Package Summary
A Merkle tree-based token distribution program that enables efficient, gas-optimized token airdrops and vesting mechanisms. The program allows administrators to create token distributions with time-locked claims, supporting complex vesting schedules and providing a clawback mechanism for unclaimed tokens.

## Notable Features
1. Merkle Proof Verification
   - Enables distributing tokens to multiple recipients cost-effectively
   - Proof-based claim mechanism reduces on-chain storage

2. Flexible Vesting Schedules
   - Time-based token unlocking
   - Partial claim capabilities
   - Configurable start and end timestamps

3. Admin Controls
   - Set/change program administrator
   - Clawback mechanism for recovering unclaimed tokens
   - Configurable clawback receiver

4. Security Mechanisms
   - Prevents double-claiming
   - Enforces claim window restrictions
   - Comprehensive error handling
   - Timestamp-based access controls

5. Event Logging
   - Emits events for claims and distributions
   - Provides transparency and indexing capabilities

The program is particularly useful for projects needing to distribute tokens to many wallets while minimizing upfront gas costs and maintaining fine-grained control over token release.

---

## research/anchor-repos/18-jito-foundation-distributor/api/Cargo.toml

Here's a comprehensive report on the Jito Foundation Distributor API package:

### File Tree Diagram
```
api/
├── Cargo.toml                  # Project dependencies and configuration
└── src/
    ├── main.rs                 # Entry point for CLI-configurable airdrop distribution server
    ├── router.rs               # Defines web API routes for token distribution and claims
    └── error.rs                # Custom error handling and API error response mechanisms
```

### Dependency List
```
Dependencies Purpose:
- anchor-lang             # Solana program development framework
- axum                    # Lightweight web server framework
- solana-rpc-client       # Interact with Solana blockchain
- jito-merkle-tree        # Custom Merkle tree implementation for token distribution
- merkle-distributor      # On-chain Merkle tree token distribution program
- serde                   # Serialization/deserialization of data structures
- tokio                   # Asynchronous runtime for web server
- tracing                 # Logging and instrumentation
- tower                   # HTTP middleware and service composition
```

### Package Summary
The Jito Foundation Distributor API is a sophisticated web service designed to manage token airdrops using Merkle tree-based distribution. It provides a flexible, configurable backend for verifying and processing token claims across different user types (stakers, searchers, validators) with support for vesting schedules.

### Notable Features
1. CLI-Configurable Server
   - Dynamically configure RPC endpoints
   - Flexible Merkle tree loading
   - Customizable server binding

2. Merkle Tree Claim Verification
   - On-chain proof validation
   - Supports complex vesting schedules
   - Tracks locked and unlocked tokens

3. Robust Error Handling
   - Comprehensive custom error types
   - HTTP response mapping
   - Detailed logging and tracing

4. Blockchain Integration
   - Direct Solana RPC client interactions
   - Account data deserialization
   - Token claim status checking

5. Web API Endpoints
   - User proof retrieval
   - Claim status checking
   - Distributor information
   - Rate-limited and traced requests

### Implementation Highlights
- Uses Axum for lightweight, performant web routing
- Leverages Tokio for asynchronous request handling
- Implements Tower middleware for request processing
- Provides type-safe error handling with `thiserror`
- Supports flexible configuration through CLI arguments

The package represents a sophisticated, production-ready solution for managing complex token distribution mechanisms with strong blockchain integration.

---

