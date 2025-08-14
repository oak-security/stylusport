# Non-fungible token handling

Solana's NFT ecosystem primarily uses the Metaplex Token Metadata standard for non-fungible tokens. This chapter covers migrating Metaplex NFT operations to ERC-721 patterns in Stylus, including minting, transfers, approvals, and metadata management.

## NFT model comparison

### Metaplex NFTs (Solana)
- **Standard**: Metaplex Token Metadata Program
- **Structure**: SPL Token with supply of 1 + metadata account
- **Metadata**: Separate account with JSON URI reference
- **Collections**: Master Edition and Collection NFTs
- **Operations**: CPIs to Token Metadata Program

### ERC-721 NFTs (Stylus)
- **Standard**: ERC-721 Non-Fungible Token Standard
- **Structure**: Contract storage with token mappings
- **Metadata**: Built-in tokenURI function with metadata JSON
- **Collections**: Single contract can represent entire collection
- **Operations**: Direct contract method calls

## Basic NFT operations

### NFT collection creation

**Solana Native (with Metaplex):**
```rust
use solana_program::*;
use mpl_token_metadata::{
    instruction::{create_metadata_accounts_v3, create_master_edition_v3},
    state::{DataV2, Creator},
};

fn create_nft(
    accounts: &[AccountInfo],
    name: String,
    symbol: String,
    uri: String,
) -> ProgramResult {
    let mint_account = &accounts[0];
    let metadata_account = &accounts[1];
    let master_edition_account = &accounts[2];
    let mint_authority = &accounts[3];
    let update_authority = &accounts[4];
    let token_metadata_program = &accounts[5];
    
    // Create metadata account
    let creators = vec![Creator {
        address: *mint_authority.key,
        verified: true,
        share: 100,
    }];
    
    let data = DataV2 {
        name,
        symbol,
        uri,
        seller_fee_basis_points: 500, // 5%
        creators: Some(creators),
        collection: None,
        uses: None,
    };
    
    let create_metadata_ix = create_metadata_accounts_v3(
        *token_metadata_program.key,
        *metadata_account.key,
        *mint_account.key,
        *mint_authority.key,
        *mint_authority.key,
        *update_authority.key,
        data,
        true, // is_mutable
        true, // update_authority_is_signer
        None, // collection_details
    );
    
    invoke(&create_metadata_ix, accounts)?;
    
    // Create master edition (makes it an NFT)
    let create_master_edition_ix = create_master_edition_v3(
        *token_metadata_program.key,
        *master_edition_account.key,
        *mint_account.key,
        *update_authority.key,
        *mint_authority.key,
        *metadata_account.key,
        *mint_authority.key,
        Some(0), // max_supply (0 = unlimited prints)
    );
    
    invoke(&create_master_edition_ix, accounts)?;
    
    msg!("NFT created: {}", name);
    Ok(())
}
```

**Anchor (with Metaplex):**
```rust
use anchor_lang::prelude::*;
use anchor_spl::{
    metadata::{
        CreateMetadataAccountsV3, CreateMasterEditionV3,
        Metadata, MetadataAccount, MasterEdition,
    },
    token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
#[instruction(name: String, symbol: String, uri: String)]
pub struct CreateNFT<'info> {
    #[account(
        init,
        payer = creator,
        mint::decimals = 0,
        mint::authority = creator,
        mint::freeze_authority = creator,
    )]
    pub mint: Account<'info, Mint>,
    
    /// CHECK: This account will be created by the metadata program
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    
    /// CHECK: This account will be created by the metadata program  
    #[account(mut)]
    pub master_edition: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub creator: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_nft(
    ctx: Context<CreateNFT>,
    name: String,
    symbol: String, 
    uri: String,
) -> Result<()> {
    // Create metadata
    let metadata_ctx = CpiContext::new(
        ctx.accounts.metadata_program.to_account_info(),
        CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            mint_authority: ctx.accounts.creator.to_account_info(),
            update_authority: ctx.accounts.creator.to_account_info(),
            payer: ctx.accounts.creator.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
    );
    
    let creators = vec![mpl_token_metadata::state::Creator {
        address: ctx.accounts.creator.key(),
        verified: false,
        share: 100,
    }];
    
    create_metadata_accounts_v3(
        metadata_ctx,
        DataV2 {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            seller_fee_basis_points: 500,
            creators: Some(creators),
            collection: None,
            uses: None,
        },
        true,
        true,
        None,
    )?;
    
    // Create master edition
    let master_edition_ctx = CpiContext::new(
        ctx.accounts.metadata_program.to_account_info(),
        CreateMasterEditionV3 {
            edition: ctx.accounts.master_edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            update_authority: ctx.accounts.creator.to_account_info(),
            mint_authority: ctx.accounts.creator.to_account_info(),
            payer: ctx.accounts.creator.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        },
    );
    
    create_master_edition_v3(master_edition_ctx, Some(0))?;
    
    msg!("NFT created: {}", name);
    Ok(())
}
```

**Stylus:**
```rust
use stylus_sdk::prelude::*;
use stylus_sdk::{alloy_primitives::{Address, U256, FixedBytes}, msg, evm};
use stylus_sdk::storage::*;
use stylus_sdk::call::transfer_eth;

#[storage]
#[entrypoint]
pub struct NFTCollection {
    // ERC-721 standard storage
    owners: StorageMap<U256, StorageAddress>,
    balances: StorageMap<Address, StorageU256>,
    token_approvals: StorageMap<U256, StorageAddress>,
    operator_approvals: StorageMap<Address, StorageMap<Address, StorageBool>>,
    
    // Collection metadata
    name: StorageString,
    symbol: StorageString,
    base_uri: StorageString,
    
    // NFT-specific storage
    token_uris: StorageMap<U256, StorageString>,
    next_token_id: StorageU256,
    max_supply: StorageU256,
    owner: StorageAddress,
    
    // Minting configuration
    mint_price: StorageU256,
    public_mint_active: StorageBool,
}

sol! {
    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
    event Approval(address indexed owner, address indexed approved, uint256 indexed tokenId);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);
}

sol_interface! {
    interface IERC721Receiver {
        function onERC721Received(address operator, address from, uint256 tokenId, bytes data) external returns (bytes4);
    }
}

const ON_ERC721_RECEIVED: [u8;4] = [0x15, 0x0b, 0x7a, 0x02]; // 0x150b7a02

#[public]
impl NFTCollection {
    #[selector(name = "supportsInterface")]
    pub fn supports_interface(&self, interface_id: [u8; 4]) -> bool {
        let id = FixedBytes::<4>::from(interface_id);
        // 0x80ac58cd ERC721, 0x5b5e139f ERC721Metadata
        id == FixedBytes::from_hex("0x80ac58cd").unwrap()
        || id == FixedBytes::from_hex("0x5b5e139f").unwrap()
    }
    
    pub fn initialize(
        &mut self,
        name: String,
        symbol: String,
        base_uri: String,
        max_supply: U256,
        mint_price: U256,
    ) -> Result<(), Vec<u8>> {
        // Ensure not already initialized
        if self.owner.get() != Address::ZERO {
            return Err(b"Already initialized".to_vec());
        }
        
        self.name.set_str(&name);
        self.symbol.set_str(&symbol);
        self.base_uri.set_str(&base_uri);
        self.max_supply.set(max_supply);
        self.mint_price.set(mint_price);
        self.owner.set(msg::sender());
        self.next_token_id.set(U256::from(1)); // Start from token ID 1
        
        Ok(())
    }
    
    // ERC-721 standard view functions
    pub fn name(&self) -> String {
        self.name.get_string()
    }
    
    pub fn symbol(&self) -> String {
        self.symbol.get_string()
    }
    
    #[selector(name = "totalSupply")]
    pub fn total_supply(&self) -> U256 {
        self.next_token_id.get() - U256::from(1)
    }
    
    #[selector(name = "balanceOf")]
    pub fn balance_of(&self, owner: Address) -> U256 {
        if owner == Address::ZERO {
            panic!("ERC721: balance query for the zero address");
        }
        self.balances.get(owner)
    }
    
    #[selector(name = "ownerOf")]
    pub fn owner_of(&self, token_id: U256) -> Result<Address, Vec<u8>> {
        let owner = self.owners.get(token_id);
        if owner == Address::ZERO {
            return Err(b"ERC721: owner query for nonexistent token".to_vec());
        }
        Ok(owner)
    }
    
    #[selector(name = "tokenURI")]
    pub fn token_uri(&self, token_id: U256) -> Result<String, Vec<u8>> {
        if !self.exists(token_id) {
            return Err(b"ERC721: URI query for nonexistent token".to_vec());
        }
        
        // Check for individual token URI first
        let individual_uri = self.token_uris.getter(token_id).get_string();
        if !individual_uri.is_empty() {
            return Ok(individual_uri);
        }
        
        // Fall back to base URI + token ID
        let base = self.base_uri.get_string();
        if base.is_empty() {
            return Ok(String::new());
        }
        
        Ok(format!("{}{}", base, token_id))
    }
}
```

### NFT minting

**Stylus Minting Implementation:**
```rust
#[public]
impl NFTCollection {
    #[payable]
    pub fn mint(&mut self, to: Address) -> Result<U256, Vec<u8>> {
        if !self.public_mint_active.get() {
            return Err(b"Public minting not active".to_vec());
        }
        
        let price = self.mint_price.get();
        if msg::value() < price {
            return Err(b"Insufficient payment".to_vec());
        }
        
        let token_id = self.next_token_id.get();
        if token_id > self.max_supply.get() {
            return Err(b"Max supply reached".to_vec());
        }
        
        // Effects first
        self.safe_mint(to, token_id)?;
        
        // Record credit for pull-based refund (avoid forced sends)
        // self.credits.setter(msg::sender()).set(self.credits.get(msg::sender()) + (msg::value() - price));
        
        Ok(token_id)
    }
    
    pub fn owner_mint(&mut self, to: Address, token_id: U256) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can mint".to_vec());
        }
        if token_id == U256::ZERO || token_id > self.max_supply.get() {
            return Err(b"ERC721: tokenId out of range".to_vec());
        }
        self.safe_mint(to, token_id)
    }
    
    pub fn batch_mint(&mut self, to: Address, quantity: U256) -> Result<Vec<U256>, Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can batch mint".to_vec());
        }
        
        let mut minted_tokens = Vec::new();
        let start_token_id = self.next_token_id.get();
        
        for i in 0..quantity.as_u32() {
            let token_id = start_token_id + U256::from(i);
            
            if token_id > self.max_supply.get() {
                return Err(b"Exceeds max supply".to_vec());
            }
            
            self.safe_mint(to, token_id)?;
            minted_tokens.push(token_id);
        }
        
        Ok(minted_tokens)
    }
    
    pub fn toggle_public_mint(&mut self) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can toggle minting".to_vec());
        }
        
        let current = self.public_mint_active.get();
        self.public_mint_active.set(!current);
        
        Ok(())
    }
    
    pub fn set_mint_price(&mut self, new_price: U256) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can set price".to_vec());
        }
        
        self.mint_price.set(new_price);
        
        Ok(())
    }
}

impl NFTCollection {
    fn safe_mint(&mut self, to: Address, token_id: U256) -> Result<(), Vec<u8>> {
        if to == Address::ZERO {
            return Err(b"ERC721: mint to zero address".to_vec());
        }
        
        if self.exists(token_id) {
            return Err(b"ERC721: token already minted".to_vec());
        }
        
        // Update balances and ownership
        let balance = self.balances.get(to);
        self.balances.setter(to).set(balance + U256::from(1));
        self.owners.setter(token_id).set(to);
        
        // Update next token ID if sequential
        if token_id >= self.next_token_id.get() {
            self.next_token_id.set(token_id + U256::from(1));
        }
        
        // Emit Transfer event
        evm::log(Transfer {
            from: Address::ZERO,
            to,
            tokenId: token_id,
        });
        
        Ok(())
    }
    
    fn exists(&self, token_id: U256) -> bool {
        self.owners.get(token_id) != Address::ZERO
    }
}
```

### NFT transfers and approvals

**Stylus Transfer Implementation:**
```rust
#[public]
impl NFTCollection {
    pub fn approve(&mut self, to: Address, token_id: U256) -> Result<(), Vec<u8>> {
        let owner = self.owner_of(token_id)?;
        
        if to == owner {
            return Err(b"ERC721: approval to current owner".to_vec());
        }
        
        let sender = msg::sender();
        if sender != owner && !self.is_approved_for_all(owner, sender) {
            return Err(b"ERC721: approve caller is not owner nor approved for all".to_vec());
        }
        
        self.approve_internal(owner, to, token_id);
        Ok(())
    }
    
    pub fn get_approved(&self, token_id: U256) -> Result<Address, Vec<u8>> {
        if !self.exists(token_id) {
            return Err(b"ERC721: approved query for nonexistent token".to_vec());
        }
        
        Ok(self.token_approvals.get(token_id))
    }
    
    #[selector(name = "setApprovalForAll")]
    pub fn set_approval_for_all(&mut self, operator: Address, approved: bool) -> Result<(), Vec<u8>> {
        let owner = msg::sender();
        if owner == operator { 
            return Err(b"ERC721: approve to caller".to_vec()); 
        }
        self.operator_approvals.setter(owner).setter(operator).set(approved);
        evm::log(ApprovalForAll { owner, operator, approved });
        Ok(())
    }
    
    pub fn is_approved_for_all(&self, owner: Address, operator: Address) -> bool {
        self.operator_approvals.getter(owner).get(operator)
    }
    
    pub fn transfer_from(&mut self, from: Address, to: Address, token_id: U256) -> Result<(), Vec<u8>> {
        if !self.is_approved_or_owner(msg::sender(), token_id)? {
            return Err(b"ERC721: transfer caller is not owner nor approved".to_vec());
        }
        
        self.transfer_internal(from, to, token_id)?;
        Ok(())
    }
    
    #[selector(name = "safeTransferFrom")]
    pub fn safe_transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256
    ) -> Result<(), Vec<u8>> {
        self.safe_transfer_from_with_data(from, to, token_id, Bytes::new())
    }

    #[selector(name = "safeTransferFrom")]
    pub fn safe_transfer_from_with_data(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
        data: Bytes,
    ) -> Result<(), Vec<u8>> {
        if !self.is_approved_or_owner(msg::sender(), token_id)? {
            return Err(b"ERC721: transfer caller is not owner nor approved".to_vec());
        }
        self.safe_transfer_internal(from, to, token_id, data)?;
        Ok(())
    }
}

impl NFTCollection {
    fn approve_internal(&mut self, owner: Address, to: Address, token_id: U256) {
        self.token_approvals.setter(token_id).set(to);
        
        evm::log(Approval {
            owner,
            approved: to,
            tokenId: token_id,
        });
    }
    
    fn is_approved_or_owner(&self, spender: Address, token_id: U256) -> Result<bool, Vec<u8>> {
        let owner = self.owner_of(token_id)?;
        
        Ok(spender == owner
            || self.get_approved(token_id)? == spender
            || self.is_approved_for_all(owner, spender))
    }
    
    fn transfer_internal(&mut self, from: Address, to: Address, token_id: U256) -> Result<(), Vec<u8>> {
        if self.owner_of(token_id)? != from {
            return Err(b"ERC721: transfer from incorrect owner".to_vec());
        }
        
        if to == Address::ZERO {
            return Err(b"ERC721: transfer to zero address".to_vec());
        }
        
        // Clear approval
        self.token_approvals.setter(token_id).set(Address::ZERO);
        evm::log(Approval { owner: from, approved: Address::ZERO, tokenId: token_id });
        
        // Update balances
        let from_balance = self.balances.get(from);
        self.balances.setter(from).set(from_balance - U256::from(1));
        
        let to_balance = self.balances.get(to);
        self.balances.setter(to).set(to_balance + U256::from(1));
        
        // Update ownership
        self.owners.setter(token_id).set(to);
        
        evm::log(Transfer {
            from,
            to,
            tokenId: token_id,
        });
        
        Ok(())
    }
    
    fn safe_transfer_internal(&mut self, from: Address, to: Address, token_id: U256, data: Bytes) -> Result<(), Vec<u8>> {
        self.transfer_internal(from, to, token_id)?;
        // External call to `to.onERC721Received` happens AFTER state updates (CEI).
        // Be aware: receiver can reenter other functions; guard critical paths if needed.
        // If `to` is a contract, it must return the magic value
        if evm::code_size(to) > 0 {
            let rcpt = IERC721Receiver::new(to);
            let ret = rcpt.on_erc721_received(self, msg::sender(), from, token_id, data)
                .map_err(|_| b"ERC721: onERC721Received reverted".to_vec())?;
            if ret.0 != ON_ERC721_RECEIVED { 
                return Err(b"ERC721: receiver rejected tokens".to_vec()); 
            }
        }
        Ok(())
    }
}
```

## Working example: complete migration

The `non-fungible-tokens` example demonstrates the full transformation:

### Running the example

```bash
cd examples/concepts/non-fungible-tokens

# Compare implementations
ls -la anchor/src/lib.rs native/src/lib.rs stylus/src/lib.rs

# Test Stylus ERC-721 implementation
cd stylus && cargo test

# Check generated ABI
cargo stylus export-abi
```

### Key transformations

1. **Metaplex Metadata to ERC-721 Storage**
   ```rust
   // Solana: Separate metadata account
   pub struct Metadata {
       pub key: Key,
       pub update_authority: Pubkey,
       pub mint: Pubkey,
       pub data: Data,
       // ...
   }
   
   // Stylus: Built-in contract storage
   token_uris: StorageMap<U256, StorageString>,
   base_uri: StorageString,
   ```

2. **SPL Token + Metadata to Single Contract**
   ```rust
   // Solana: Multiple accounts per NFT
   // - Mint account (SPL Token)
   // - Metadata account (Metaplex)
   // - Master Edition account
   
   // Stylus: Single contract manages all NFTs
   owners: StorageMap<U256, StorageAddress>,
   balances: StorageMap<Address, StorageU256>,
   ```

3. **CPI Complexity to Direct Methods**
   ```rust
   // Solana: Complex CPI setup
   invoke(&create_metadata_accounts_v3(...), accounts)?;
   
   // Stylus: Simple method call
   nft_contract.mint(to_address)?;
   ```

## Advanced NFT features

### Royalties and Marketplace Integration

```rust
#[storage]
pub struct NFTWithRoyalties {
    // ... standard ERC-721 storage
    
    // Royalty information
    royalty_recipient: StorageAddress,
    royalty_percentage: StorageU256, // In basis points (500 = 5%)
    token_royalties: StorageMap<U256, RoyaltyInfo>,
}

#[storage]
pub struct RoyaltyInfo {
    recipient: StorageAddress,
    percentage: StorageU256,
}

#[public]
impl NFTWithRoyalties {
    #[selector(name = "supportsInterface")]
    pub fn supports_interface(&self, interface_id: [u8; 4]) -> bool {
        let id = FixedBytes::<4>::from(interface_id);
        id == FixedBytes::from_hex("0x80ac58cd").unwrap() // ERC721
        || id == FixedBytes::from_hex("0x5b5e139f").unwrap() // Metadata
        || id == FixedBytes::from_hex("0x2a55205a").unwrap() // ERC-2981
    }
    
    pub fn set_default_royalty(&mut self, recipient: Address, percentage: U256) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can set royalties".to_vec());
        }
        
        if percentage > U256::from(1000) { // Max 10%
            return Err(b"Royalty too high".to_vec());
        }
        
        self.royalty_recipient.set(recipient);
        self.royalty_percentage.set(percentage);
        
        Ok(())
    }
    
    pub fn set_token_royalty(&mut self, token_id: U256, recipient: Address, percentage: U256) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can set royalties".to_vec());
        }
        
        if !self.exists(token_id) {
            return Err(b"Token does not exist".to_vec());
        }
        
        let mut royalty_info = self.token_royalties.setter(token_id);
        royalty_info.recipient.set(recipient);
        royalty_info.percentage.set(percentage);
        
        Ok(())
    }
    
    // EIP-2981 Royalty Standard
    pub fn royalty_info(&self, token_id: U256, sale_price: U256) -> (Address, U256) {
        let token_royalty = self.token_royalties.getter(token_id);
        let recipient = token_royalty.recipient.get();
        
        if recipient != Address::ZERO {
            let percentage = token_royalty.percentage.get();
            let royalty_amount = sale_price * percentage / U256::from(10000);
            return (recipient, royalty_amount);
        }
        
        // Fall back to default royalty
        let default_recipient = self.royalty_recipient.get();
        let default_percentage = self.royalty_percentage.get();
        let royalty_amount = sale_price * default_percentage / U256::from(10000);
        
        (default_recipient, royalty_amount)
    }
}
```

### Enumerable extension

```rust
#[storage]
pub struct EnumerableNFT {
    // ... standard ERC-721 storage
    
    // Enumeration support
    all_tokens: StorageVec<StorageU256>,
    all_tokens_index: StorageMap<U256, StorageU256>,
    owned_tokens: StorageMap<Address, StorageMap<U256, StorageU256>>,
    owned_tokens_index: StorageMap<U256, StorageU256>,
}

#[public]
impl EnumerableNFT {
    #[selector(name = "totalSupply")]
    pub fn total_supply(&self) -> U256 { 
        U256::from(self.all_tokens.len()) 
    }

    #[selector(name = "tokenByIndex")]
    pub fn token_by_index(&self, index: U256) -> Result<U256, Vec<u8>> {
        if index >= U256::from(self.all_tokens.len()) { 
            return Err(b"ERC721Enumerable: global index out of bounds".to_vec()); 
        }
        Ok(self.all_tokens.get(index).unwrap())
    }

    #[selector(name = "tokenOfOwnerByIndex")]
    pub fn token_of_owner_by_index(&self, owner: Address, index: U256) -> Result<U256, Vec<u8>> {
        let bal = self.balances.get(owner);
        if index >= bal { 
            return Err(b"ERC721Enumerable: owner index out of bounds".to_vec()); 
        }
        Ok(self.owned_tokens.getter(owner).get(index))
    }
}

impl EnumerableNFT {
    fn add_token_to_all_tokens_enumeration(&mut self, token_id: U256) {
        let idx = U256::from(self.all_tokens.len());
        self.all_tokens.push(token_id);
        self.all_tokens_index.setter(token_id).set(idx);
    }

    fn add_token_to_owner_enumeration(&mut self, to: Address, token_id: U256) {
        let idx = self.balances.get(to) - U256::from(1); // already incremented
        self.owned_tokens.setter(to).setter(idx).set(token_id);
        self.owned_tokens_index.setter(token_id).set(idx);
    }

    fn remove_token_from_owner_enumeration(&mut self, from: Address, token_id: U256) {
        let last_index = self.balances.get(from);
        let idx = self.owned_tokens_index.get(token_id);
        let last_token_id = self.owned_tokens.getter(from).get(last_index - U256::from(1));
        self.owned_tokens.setter(from).setter(idx).set(last_token_id);
        self.owned_tokens_index.setter(last_token_id).set(idx);
        // delete last entry
        // (implementation detail depends on SDK helpers; intent: shrink "array")
        self.owned_tokens_index.delete(token_id);
    }
}
```

### Metadata management

```rust
#[storage]
pub struct UpdatableMetadata {
    // ... standard ERC-721 storage
    
    token_metadata: StorageMap<U256, TokenMetadata>,
    metadata_frozen: StorageBool,
}

#[storage]    
pub struct TokenMetadata {
    name: StorageString,
    description: StorageString,
    image: StorageString,
    attributes: StorageVec<StorageString>,
}

sol! {
    event MetadataUpdate(uint256 indexed token_id);
    event MetadataFrozen();
}

#[public]
impl UpdatableMetadata {
    pub fn set_token_metadata(
        &mut self,
        token_id: U256,
        name: String,
        description: String,
        image: String,
        attributes: Vec<String>,
    ) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can update metadata".to_vec());
        }
        
        if self.metadata_frozen.get() {
            return Err(b"Metadata is frozen".to_vec());
        }
        
        if !self.exists(token_id) {
            return Err(b"Token does not exist".to_vec());
        }
        
        let mut metadata = self.token_metadata.setter(token_id);
        metadata.name.set_str(&name);
        metadata.description.set_str(&description);
        metadata.image.set_str(&image);
        
        // Clear existing attributes
        let mut attrs = metadata.attributes;
        while attrs.len() > 0 {
            attrs.pop();
        }
        
        // Set new attributes
        for attr in attributes {
            attrs.push(StorageString::new());
            let last_idx = attrs.len() - 1;
            attrs.setter(last_idx).unwrap().set_str(&attr);
        }
        
        evm::log(MetadataUpdate { token_id });
        
        Ok(())
    }
    
    pub fn freeze_metadata(&mut self) -> Result<(), Vec<u8>> {
        if msg::sender() != self.owner.get() {
            return Err(b"Only owner can freeze metadata".to_vec());
        }
        
        self.metadata_frozen.set(true);
        evm::log(MetadataFrozen {});
        
        Ok(())
    }
    
    pub fn get_token_metadata(&self, token_id: U256) -> Result<(String, String, String, Vec<String>), Vec<u8>> {
        if !self.exists(token_id) {
            return Err(b"Token does not exist".to_vec());
        }
        
        let metadata = self.token_metadata.getter(token_id);
        let name = metadata.name.get_string();
        let description = metadata.description.get_string();
        let image = metadata.image.get_string();
        
        let attrs_storage = metadata.attributes;
        let mut attributes = Vec::new();
        for i in 0..attrs_storage.len() {
            attributes.push(attrs_storage.get(i).unwrap().get_string());
        }
        
        Ok((name, description, image, attributes))
    }
}
```

## Best practices

### 1. Follow ERC-721 standard
```rust
// Always implement all required functions
pub fn balance_of(&self, owner: Address) -> U256;
pub fn owner_of(&self, token_id: U256) -> Result<Address, Vec<u8>>;
pub fn approve(&mut self, to: Address, token_id: U256) -> Result<(), Vec<u8>>;
// ... etc
```

### 2. Check all token operations
```rust
fn require_minted(&self, token_id: U256) -> Result<(), Vec<u8>> {
    if !self.exists(token_id) {
        return Err(b"ERC721: token does not exist".to_vec());
    }
    Ok(())
}
```

### 3. Emit events for all state changes
```rust
// Always emit Transfer events
evm::log(Transfer { from, to, token_id });
```

### 4. Handle zero address checks
```rust
if to == Address::ZERO {
    return Err(b"ERC721: mint to zero address".to_vec());
}
```

## Migration checklist

### Analysis phase
- [ ] Identify all Metaplex NFT operations
- [ ] Map collection structure and metadata
- [ ] Document minting patterns and authorities
- [ ] List any custom NFT features (royalties, traits, etc.)

### Implementation phase
- [ ] Create ERC-721 contract with appropriate storage
- [ ] Apply standard ERC-721 methods
- [ ] Add minting and burning functionality
- [ ] Migrate metadata management
- [ ] Add custom features (royalties, enumeration, etc.)

### Testing phase
- [ ] Test all ERC-721 standard methods
- [ ] Verify transfer and approval mechanisms
- [ ] Test minting with payment handling
- [ ] Check metadata functionality
- [ ] Check gas costs and limits

## Common pitfalls

### Not checking token existence
```rust
// Always verify token exists
pub fn owner_of(&self, token_id: U256) -> Result<Address, Vec<u8>> {
    let owner = self.owners.get(token_id);
    if owner == Address::ZERO {
        return Err(b"Token does not exist".to_vec());
    }
    Ok(owner)
}
```

### Forgetting to update balances
```rust
// Must update both ownership and balances
self.owners.setter(token_id).set(to);
let balance = self.balances.get(to);
self.balances.setter(to).set(balance + U256::from(1)); // Don't forget this!
```

### Missing approval clearing
```rust
fn transfer_internal(&mut self, from: Address, to: Address, token_id: U256) -> Result<(), Vec<u8>> {
    // Must clear approvals on transfer
    self.token_approvals.setter(token_id).set(Address::ZERO);
    // ... rest of transfer logic
}
```

## Next steps

With non-fungible tokens covered, the final chapter explores [Errors and Events](./errors-events.md) - migrating Solana's logging and error patterns to Stylus structured events and custom errors.

## Reference

- [Example Code: non-fungible-tokens](/examples/concepts/non-fungible-tokens/)
- [ERC-721 Standard](https://eips.ethereum.org/EIPS/eip-721)
- [OpenZeppelin ERC-721 Implementation](https://docs.openzeppelin.com/contracts/erc721)