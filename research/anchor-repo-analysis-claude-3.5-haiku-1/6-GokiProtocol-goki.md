# 6-GokiProtocol-goki - Solana Programs Analysis

## research/anchor-repos/6-GokiProtocol-goki/programs/token-signer/Cargo.toml

Here's a comprehensive report for the programs_token-signer package:

### File Tree Diagram
```
programs_token-signer/
│
├── Cargo.toml                  # Project configuration and dependencies
│
└── src/
    ├── lib.rs                  # Main program logic for token-based signed instructions
    └── account_validators.rs   # Custom account validation for NFT ownership checks
```

### Dependencies
```toml
anchor-lang: ">=0.22, <=0.24"   # Core Solana/Anchor framework for program development
anchor-spl: ">=0.22, <=0.24"    # Solana Program Library utilities for token handling
vipers: "^2.0"                  # Custom validation and error handling library
```

### Package Summary
The `programs_token-signer` is a Solana program that enables dynamic instruction signing and authorization based on NFT token ownership. It allows users to invoke cross-program instructions using a Program Derived Address (PDA) derived from an NFT's mint, effectively creating a token-gated authorization mechanism.

### Notable Features
1. **Token-Based Authorization**
   - Uses NFT ownership as a cryptographic access control mechanism
   - Validates token ownership before allowing instruction execution

2. **Dynamic PDA Signing**
   - Generates a unique PDA from "GokiTokenSigner" and NFT mint
   - Enables programmatic, token-controlled cross-program invocations

3. **Flexible Instruction Invocation**
   - Dynamically constructs and executes instructions across different programs
   - Provides a generic mechanism for token-based delegation

### Implementation Highlights
- Leverages Anchor framework for program development
- Uses `vipers` crate for robust account validation
- Implements strict ownership checks (exactly 1 token in account)
- Supports flexible, programmable access controls

### Security Considerations
- Strict NFT ownership validation
- PDA-based signing prevents unauthorized access
- Validates instruction accounts before execution

The package represents an innovative approach to creating token-gated, programmable authorization in the Solana ecosystem.

---

## research/anchor-repos/6-GokiProtocol-goki/programs/smart-wallet/Cargo.toml

Here's the comprehensive report for the Goki Smart Wallet program:

### File Tree Diagram
```
programs_smart-wallet/
│
├── Cargo.toml                  # Project configuration and dependencies
└── src/
    ├── lib.rs                  # Main program definition and instruction handlers
    ├── state.rs                # Core data structures for smart wallet and transactions
    ├── events.rs               # Event logging for wallet and transaction lifecycle
    ├── validators.rs           # Custom validation logic for account interactions
    └── instructions/
        ├── mod.rs              # Instruction module organization
        ├── approve.rs          # Logic for approving transactions
        └── unapprove.rs        # Logic for unapproving transactions
```

### Dependencies
```toml
anchor-lang: ">=0.22, <=0.24"   # Solana program development framework
vipers: "^2.0"                  # Custom validation and assertion library
```

### Package Summary
A Solana-based multi-signature smart wallet with timelock and governance capabilities, enabling secure, collaborative transaction management through a programmable wallet system with flexible ownership and approval mechanisms.

### Notable Features
1. Multisig Transaction Approval
   - Configurable owner threshold
   - Transaction proposal and approval workflow
   - Owner-based signature tracking

2. Timelock Mechanism
   - Minimum delay before transaction execution
   - Grace period for transaction cancellation
   - Estimated time of execution (ETA)

3. Advanced Security
   - Dynamic owner management
   - Strict signature validation
   - One-time transaction execution
   - Comprehensive error handling

4. Flexible Account Management
   - Subaccount tracking
   - Owner-invoker pattern
   - Derived account creation support

### Implementation Highlights
- Uses Anchor framework for Solana program development
- Implements custom validation through `vipers` library
- Comprehensive event logging for wallet and transaction lifecycle
- Modular instruction design with separate approve/unapprove logic
- Robust state management with `SmartWallet` and `Transaction` structs

The package provides a sophisticated, programmable multi-signature wallet solution for secure, collaborative on-chain asset and instruction management.

---

