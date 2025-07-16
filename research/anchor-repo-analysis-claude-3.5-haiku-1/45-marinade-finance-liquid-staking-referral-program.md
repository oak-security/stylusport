# 45-marinade-finance-liquid-staking-referral-program - Solana Programs Analysis

## research/anchor-repos/45-marinade-finance-liquid-staking-referral-program/programs/marinade-referral/Cargo.toml

# Marinade Referral Program Analysis

## File Tree Diagram
```
programs_marinade-referral/
│
├── Cargo.toml                 # Project dependencies and configuration
│
├── src/
│   ├── lib.rs                 # Main program entry point and instruction definitions
│   ├── constant.rs            # Predefined constants for fees and stake limits
│   ├── error.rs               # Custom error definitions for the referral program
│   ├── states.rs              # Account state structures for global and referral tracking
│   │
│   └── instructions/
│       ├── mod.rs             # Module organization for different instruction types
│       ├── admin.rs           # Administrative functions and controls
│       ├── common.rs          # Shared utility functions for token operations
│       ├── deposit_sol.rs     # SOL deposit instruction handling
│       ├── deposit_stake_account.rs  # Stake account deposit instruction
│       └── liquid_unstake.rs  # Liquid unstaking instruction implementation
│
└── tests/
    ├── src/
    │   ├── lib.rs             # Test utility functions
    │   ├── initialize.rs      # Initialization helpers for testing
    │   │
    │   └── integration_test/
    │       ├── test_admin.rs              # Admin functionality tests
    │       ├── test_add_remove_liquidity.rs  # Liquidity management tests
    │       ├── test_delayed_unstake.rs    # Delayed unstaking tests
    │       ├── test_deposit_sol_liquid_unstake.rs  # SOL deposit and unstake tests
    │       └── test_deposit_stake_account.rs  # Stake account deposit tests
    │
    └── integration_test.rs    # Main integration test environment setup
```

## Dependency List
```json
{
  "anchor-lang": "0.14.0",         // Solana program development framework
  "anchor-spl": "0.14.0",          // Solana Program Library token utilities
  "solana-program": "1.7.11",      // Core Solana blockchain programming
  "spl-token": "3.2.0",            // Token program interactions
  "marinade-finance": {            // Liquid staking protocol integration
    "git": "https://github.com/marinade-finance/liquid-staking-program"
  },
  "marinade-onchain-helper": {     // On-chain helper utilities
    "git": "https://github.com/marinade-finance/marinade-onchain-helper"
  }
}
```

## Package Summary
The Marinade Referral Program is a sophisticated Solana blockchain program designed to manage liquid staking referrals and partnerships within the Marinade Finance ecosystem. It provides a flexible system for:
- SOL and stake account deposits
- Liquid unstaking
- Referral fee management
- Administrative controls

## Notable Features
1. Granular Fee Management
- Configurable referral and operation fees
- Dynamic fee calculation based on stake amounts
- Precise fee tracking and distribution

2. Comprehensive Access Controls
- Multi-tier authority system (admin, foreman)
- Pause/unpause mechanisms for referral accounts
- Strict account validation

3. Advanced Staking Interactions
- Cross-Program Invocations (CPI) with Marinade Finance
- Support for various deposit types (SOL, stake accounts)
- Liquid unstaking with fee calculations

4. Robust Error Handling
- Detailed custom error codes
- Comprehensive input validation
- Preventative checks for invalid operations

5. Flexible Referral State Tracking
- Accumulate deposit and unstake metrics
- Track partner-specific parameters
- Support for different partner modes

Program ID: `MR2LqxoSbw831bNy68utpu5n4YqBH3AzDmddkgk9LQv`

---

