# 1-gemworks-gem-farm - Solana Programs Analysis

## research/anchor-repos/1-gemworks-gem-farm/programs/gem_bank/Cargo.toml

Here's the comprehensive report for the programs_gem_bank package:

### File Tree Diagram
```
programs_gem_bank/
│
├── Cargo.toml                  # Project dependencies and configuration
│
└── src/
    ├── lib.rs                  # Main program entry point and instruction declaration
    │
    ├── instructions/           # Instruction handlers for various operations
    │   ├── mod.rs              # Module organization for instructions
    │   ├── add_to_whitelist.rs # Add addresses to whitelist
    │   ├── deposit_gem.rs      # Deposit NFTs/tokens into vault
    │   ├── deposit_gem_pnft.rs # Deposit programmable NFTs
    │   ├── init_bank.rs        # Initialize bank account
    │   ├── init_vault.rs       # Create new vault
    │   ├── record_rarity_points.rs  # Track NFT rarity scores
    │   ├── remove_from_whitelist.rs # Remove addresses from whitelist
    │   ├── set_bank_flags.rs   # Configure bank-level settings
    │   ├── set_vault_lock.rs   # Lock/unlock vault
    │   ├── shared.rs           # Shared utility functions
    │   ├── update_bank_manager.rs  # Change bank management
    │   ├── update_vault_owner.rs   # Transfer vault ownership
    │   ├── withdraw_gem.rs     # Withdraw NFTs from vault
    │   ├── withdraw_gem_pnft.rs    # Withdraw programmable NFTs
    │   └── withdraw_tokens_auth.rs # Withdraw tokens with authorization
    │
    └── state/                  # Account state definitions
        ├── mod.rs              # State module organization
        ├── bank.rs             # Bank account structure
        ├── gem_deposit_receipt.rs  # Track gem deposit details
        ├── rarity.rs           # NFT rarity scoring
        ├── vault.rs            # Vault account management
        └── whitelist_proof.rs  # Whitelist management
```

### Dependencies
```json
{
  "anchor-lang": "Solana program development framework",
  "anchor-spl": "Solana Program Library token utilities",
  "mpl-token-metadata": "Metaplex NFT metadata handling",
  "mpl-token-auth-rules": "Token authorization rule management",
  "bitflags": "Efficient bit manipulation",
  "bytemuck": "Type casting and memory manipulation",
  "thiserror": "Custom error handling",
  "gem_common": "Internal shared library",
  "proc_macros": "Internal procedural macros"
}
```

### Package Summary
The Gem Bank is a sophisticated Solana program for managing NFT (gem) vaults with advanced features like:
- Secure NFT/token deposit and withdrawal
- Whitelist management for creators and mints
- Rarity point tracking
- Vault and bank-level access controls
- Support for both standard and programmable NFTs

### Notable Features
1. Flexible Whitelisting
   - Can whitelist by creator or mint address
   - Granular access control for vault deposits

2. Advanced NFT Management
   - Supports standard and programmable (pNFT) NFTs
   - Rarity point tracking
   - Vault-level and bank-level locks

3. Modular Design
   - Separate modules for instructions and state
   - Extensive use of Program Derived Addresses (PDAs)
   - Robust error handling and account validation

4. Security Mechanisms
   - Multi-level authorization checks
   - Configurable bank and vault flags
   - Prevents unauthorized token movements

5. Extensibility
   - Reserved space in account structures
   - Versioning support
   - Flexible state management

The program appears designed for complex NFT management scenarios, potentially for gaming platforms, collectible systems, or advanced token vaulting applications.

---

## research/anchor-repos/1-gemworks-gem-farm/programs/gem_farm/Cargo.toml

# Gem Farm Solana Program Package Analysis

## File Tree
```
programs_gem_farm/
│
├── Cargo.toml                 # Project configuration and dependencies
│
└── src/
    ├── lib.rs                 # Main program entry point and instruction registration
    ├── number128.rs           # Custom high-precision decimal number implementation
    │
    ├── instructions/          # Instruction handlers for various farm operations
    │   ├── mod.rs             # Centralized instruction module exports
    │   ├── add_rarities_to_bank.rs     # Add rarity points to gem bank
    │   ├── add_to_bank_whitelist.rs    # Whitelist addresses in bank
    │   ├── authorize_funder.rs         # Authorize new reward funders
    │   ├── cancel_reward.rs            # Cancel and withdraw farm rewards
    │   ├── claim.rs                    # Claim staking rewards
    │   ├── deauthorize_funder.rs       # Remove authorized funders
    │   ├── flash_deposit.rs            # Quick NFT deposit into farm
    │   ├── flash_deposit_pnft.rs       # Programmable NFT deposit
    │   ├── fund_reward.rs              # Add rewards to farm pot
    │   ├── init_farm.rs                # Initialize new farming pool
    │   ├── init_farmer.rs              # Create farmer account
    │   ├── lock_reward.rs              # Lock specific reward tokens
    │   ├── refresh_farmer.rs           # Update farmer reward status
    │   ├── refresh_farmer_signed.rs    # Signed farmer status refresh
    │   ├── remove_from_bank_whitelist.rs # Remove bank whitelist entry
    │   ├── stake.rs                    # Stake NFTs in farm
    │   ├── treasury_payout.rs          # Transfer funds from farm treasury
    │   ├── unstake.rs                  # Unstake NFTs from farm
    │   └── update_farm.rs              # Modify farm configuration
    │
    └── state/                 # Program state definitions
        ├── mod.rs             # State module exports
        ├── authorization_proof.rs  # Funder authorization tracking
        ├── farm.rs            # Farm configuration and metrics
        ├── farmer.rs          # Farmer staking and reward state
        ├── fixed_rewards.rs   # Fixed-rate reward scheduling
        └── variable_rewards.rs # Dynamic reward calculation
```

## Dependencies
```json
{
  "anchor-lang": "0.26.0",       // Solana program framework
  "anchor-spl": "0.26.0",        // Solana token program utilities
  "bitflags": "1.3.2",           // Bitwise flag manipulation
  "bytemuck": "1.7.2",           // Byte-level type conversions
  "thiserror": "1.0.30",         // Error handling utilities
  "gem_bank": { "path": "../gem_bank" },  // Custom NFT vault management
  "gem_common": { "path": "../../lib/gem_common" }  // Shared utilities
}
```

## Package Summary
Gem Farm is a sophisticated NFT staking and reward distribution platform on Solana, enabling users to stake NFTs (called "gems") and earn rewards based on rarity, staking duration, and configurable reward mechanisms.

## Notable Features
1. Dual Reward Mechanisms
   - Fixed-rate rewards with progressive tiers
   - Variable-rate rewards with dynamic calculation

2. Advanced Rarity Tracking
   - Assign rarity points to NFTs
   - Calculate rewards based on NFT characteristics

3. Flexible Staking Options
   - Support for standard and programmable NFTs
   - Configurable staking periods and cooldown
   - Flash deposit mechanisms

4. Robust Access Control
   - Farm manager authorization
   - Funder whitelisting
   - Secure PDA-based account management

5. Comprehensive Reward Calculations
   - Precise decimal handling
   - Multiple reward token support
   - Detailed reward accrual tracking

The program provides a flexible, secure framework for creating gamified staking experiences with nuanced reward distribution logic.

---

## research/anchor-repos/1-gemworks-gem-farm/lib/gem_common/Cargo.toml

# lib_gem_common Package Analysis

## File Tree Diagram
```
lib_gem_common/
│
├── Cargo.toml                # Package configuration and dependencies
│
└── src/
    ├── lib.rs                # Module organization and re-exports
    ├── account.rs             # Account closure utility functions
    ├── errors.rs              # Centralized error handling enum
    ├── try_math.rs            # Safe mathematical operations with error handling
    └── util.rs                # Blockchain timestamp utility
```

## Dependencies
```toml
anchor-lang@0.26.0     # Solana program development framework
anchor-spl@0.26.0      # Solana Program Library token utilities
thiserror@1.0.30       # Ergonomic error handling for Rust
static_assertions@1.1.0# Compile-time type and constant assertions
spl-math@0.1.0         # Solana-specific mathematical utilities
```

## Package Summary
`lib_gem_common` is a utility library for Solana programs, providing a collection of common helper functions and error handling mechanisms. It appears to be part of a blockchain-based application (likely related to NFT or gaming) that requires safe mathematical operations, account management, and standardized error reporting.

## Notable Features
1. **Safe Mathematical Operations**
   - Checked arithmetic to prevent overflow
   - Error-handled mathematical transformations
   - Supports multiple integer types

2. **Robust Error Handling**
   - Comprehensive error enum with categorized error codes
   - Descriptive error messages
   - Supports various blockchain-specific error scenarios

3. **Account Management**
   - Safe account closure utility
   - Handles lamport transfer and account cleanup

4. **Timestamp Utility**
   - Standardized blockchain timestamp retrieval
   - Error-handled type conversion

## Implementation Highlights
- Uses Anchor framework for Solana program development
- Modular design with separate concern-specific modules
- Emphasizes type safety and error prevention
- Provides reusable utilities across Solana programs

The library seems designed to be a shared resource for complex Solana programs, offering foundational utilities that can be imported and used across different blockchain applications.

---

