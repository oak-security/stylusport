# 7-coral-xyz-xnft - Solana Programs Analysis

## research/anchor-repos/7-coral-xyz-xnft/programs/xnft/Cargo.toml

Here's a comprehensive report for the programs_xnft package:

### File Tree Diagram
```
programs_xnft/
│
├── Cargo.toml                 # Project dependencies and configuration
└── src/
    ├── lib.rs                 # Main program entrypoint and instruction registration
    │
    ├── events.rs              # Defines blockchain events for xNFT actions
    │
    ├── instructions/          # Instruction handlers for various xNFT operations
    │   ├── mod.rs             # Instruction module organizer
    │   ├── create_app_xnft.rs # Handler for creating app-type xNFTs
    │   ├── create_collectible_xnft.rs  # Handler for creating collectible xNFTs
    │   ├── create_install.rs  # Handler for installing xNFTs
    │   ├── create_review.rs   # Handler for creating xNFT reviews
    │   ├── delete_install.rs  # Handler for deleting xNFT installations
    │   ├── delete_review.rs   # Handler for deleting xNFT reviews
    │   ├── delete_xnft.rs     # Handler for deleting xNFTs
    │   ├── donate.rs          # Handler for donating to xNFT creators
    │   ├── grant_access.rs    # Handler for granting access to xNFTs
    │   ├── revoke_access.rs   # Handler for revoking xNFT access
    │   ├── set_curator.rs     # Handler for setting xNFT curators
    │   ├── set_curator_verification.rs  # Handler for curator verification
    │   ├── set_suspended.rs   # Handler for suspending xNFTs
    │   ├── transfer.rs        # Handler for transferring xNFTs
    │   └── update_xnft.rs     # Handler for updating xNFT metadata
    │
    └── state/                 # Data structures for xNFT state management
        ├── mod.rs             # State module organizer
        ├── access.rs          # Access control account structure
        ├── install.rs         # xNFT installation account structure
        ├── parameters.rs      # Parameter structures for xNFT creation/update
        ├── review.rs          # Review account structure
        ├── serialization.rs   # Custom serialization for xNFT structures
        └── xnft.rs            # Core xNFT account structure
```

### Dependency List
```toml
"anchor-lang": "0.28.0"         # Solana program development framework
"anchor-spl": "0.28.0"          # Solana Program Library token utilities
"mpl-token-metadata": "1.12.0"  # Metaplex NFT metadata standard
"serde": "1.0"                  # Serialization/deserialization library
"solana-security-txt": "1.1"    # Security contact information for the program
```

### Package Summary
The `programs_xnft` is a comprehensive Solana blockchain program for creating and managing extended NFTs (xNFTs) with advanced features beyond traditional NFTs. It provides a flexible framework for creating app-type and collectible NFTs with granular access control, installation management, reviews, and creator monetization.

### Notable Features
1. **Advanced NFT Types**
   - Support for both app-type and collectible xNFTs
   - Flexible metadata management
   - Customizable creator royalties

2. **Access Control**
   - Granular access granting/revoking
   - Permissioned installations
   - Curator management

3. **Monetization Mechanisms**
   - Installation pricing
   - Creator donation system
   - Seller fees

4. **Governance Features**
   - Suspension capabilities
   - Review and rating system
   - Transfer controls

5. **Security Considerations**
   - Strict account validation
   - PDA (Program Derived Address) usage
   - Comprehensive error handling

The package represents a sophisticated NFT ecosystem that goes beyond simple token ownership, enabling complex interactions and programmable digital assets.

---

