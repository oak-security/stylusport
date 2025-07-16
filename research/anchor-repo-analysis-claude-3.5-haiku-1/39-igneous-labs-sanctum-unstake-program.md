# 39-igneous-labs-sanctum-unstake-program - Solana Programs Analysis

## research/anchor-repos/39-igneous-labs-sanctum-unstake-program/programs/unstake/Cargo.toml

Here's the comprehensive report for the programs_unstake package:

### File Tree Diagram
```
programs_unstake/
│
├── Cargo.toml                  # Project dependencies and configuration
└── src/
    ├── lib.rs                  # Main program entry point and instruction definitions
    ├── anchor_len.rs           # Utility trait for calculating Anchor account space
    ├── consts.rs               # Global constants for the program
    ├── errors.rs               # Custom error definitions for the program
    ├── rational.rs             # Rational number implementation for precise calculations
    ├── utils.rs                # Utility functions for PDA and lamport management
    │
    ├── instructions/           # Program instruction implementations
    │   ├── mod.rs
    │   ├── add_liquidity.rs
    │   ├── create_pool.rs
    │   ├── remove_liquidity.rs
    │   ├── set_fee.rs
    │   └── ...
    │
    ├── state/                  # Program state account definitions
    │   ├── mod.rs
    │   ├── pool.rs
    │   ├── fee.rs
    │   ├── flash_loan_fee.rs
    │   └── ...
    │
    └── instructions/unstake_instructions/  # Unstaking-specific instructions
        ├── mod.rs
        ├── unstake.rs
        └── unstake_wsol.rs
```

### Dependency List
```json
{
  "spl-math": "Provides mathematical utilities for Solana programs",
  "anchor-lang": "Solana program development framework",
  "anchor-spl": "Solana Program Library (SPL) token and stake program integrations",
  "mpl-token-metadata": "Metaplex token metadata standard support",
  "serde": "Serialization and deserialization framework",
  "spl-associated-token-account": "Solana Associated Token Account utilities"
}
```

### Package Summary
The `programs_unstake` is a sophisticated Solana program implementing a liquid staking protocol with advanced features like:
- Liquidity pool management for staked SOL
- Flash loan functionality
- Flexible fee mechanisms
- Stake account management
- Wrapped SOL (wSOL) unstaking support

### Notable Features
1. **Modular Design**: Separates concerns into instructions, state, and utility modules
2. **Advanced Fee Management**: 
   - Supports flat and liquidity-based fee models
   - Configurable protocol and referrer fees
3. **Flash Loan Mechanism**: 
   - Atomic borrowing and repayment
   - Fee calculation and distribution
4. **Precise Calculations**: 
   - Uses `Rational` type for exact fee and liquidity calculations
   - Prevents arithmetic overflows
5. **Comprehensive Error Handling**: 
   - Detailed custom error codes
   - Strict validation for all operations
6. **Flexible Unstaking**: 
   - Supports both native SOL and wrapped SOL unstaking
   - Handles stake account deactivation and reclamation

The program represents a complex DeFi protocol focused on providing liquidity and flexible unstaking options for Solana stake accounts.

---

## research/anchor-repos/39-igneous-labs-sanctum-unstake-program/cli-rust/Cargo.toml

Here's the comprehensive report for the cli-rust package:

### File Tree Diagram
```
cli-rust/
├── Cargo.toml                 # Project dependencies and configuration
└── src/
    ├── main.rs                # CLI application entry point
    ├── argparse.rs             # Solana CLI configuration parsing utilities
    ├── tx_utils.rs             # Transaction handling utilities
    ├── subcmd/                 # Subcommand implementations
    │   ├── mod.rs              # Subcommand routing and management
    │   ├── add_liquidity.rs    # Add liquidity to unstaking pool
    │   ├── create_pool.rs      # Create new liquidity pool
    │   ├── deactivate_all.rs   # Deactivate all stake accounts
    │   ├── deactivate_stake_account.rs  # Deactivate specific stake account
    │   ├── fetch_protocol_fee.rs  # Retrieve protocol fee information
    │   ├── init_protocol_fee.rs   # Initialize protocol fee account
    │   ├── reclaim_all.rs      # Reclaim SOL from all inactive stake accounts
    │   ├── reclaim_stake_account.rs  # Reclaim SOL from specific stake account
    │   ├── remove_liquidity.rs # Remove liquidity from pool
    │   ├── set_fee.rs          # Configure pool fees
    │   ├── set_fee_authority.rs  # Change fee authority
    │   ├── set_flash_loan_fee.rs  # Set flash loan fees
    │   ├── set_lp_token_metadata.rs  # Configure LP token metadata
    │   └── view_pool.rs        # Inspect pool details
    └── utils/                  # Utility modules
        ├── mod.rs              # Utility module organization
        ├── fee.rs              # Fee configuration parsing
        └── state.rs            # Stake account state management
```

### Dependency List
```json
{
  "anchor-lang": "0.28.0",           # Solana program development framework
  "solana-sdk": "~1.14",             # Solana blockchain SDK
  "solana-client": "~1.14",          # RPC client for Solana interactions
  "clap": "^4.0",                    # CLI argument parsing
  "serde": "1.0.171",                # Serialization/deserialization
  "spl-token": "^3.0",               # Solana token program utilities
  "unstake": { "path": "../programs/unstake" },  # Local unstake program
  "unstake_interface": { "path": "../unstake_interface" }  # Unstake program interface
}
```

### Package Summary
A comprehensive CLI tool for managing a Solana-based liquid unstaking protocol. The package provides a flexible command-line interface for interacting with an unstaking liquidity pool, allowing users to perform various operations like adding/removing liquidity, managing stake accounts, configuring fees, and inspecting pool states.

### Notable Features
1. **Flexible Subcommand Architecture**: Modular design with multiple subcommands for different protocol interactions
2. **Transaction Simulation**: Built-in support for dry-run transaction simulation
3. **Dynamic Fee Configuration**: Supports complex fee structures (flat and liquidity-based)
4. **Stake Account Management**: Comprehensive tools for handling stake accounts
5. **PDA (Program Derived Address) Utilities**: Robust address derivation for protocol accounts
6. **JSON Configuration Support**: Allows configuration via JSON files
7. **Metaplex Token Metadata Integration**: Supports setting LP token metadata

The CLI provides a user-friendly interface for interacting with a sophisticated liquid staking protocol, abstracting away complex blockchain interactions into simple, composable commands.

---

