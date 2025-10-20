# Non-Fungible Token Handling

Solana's NFT ecosystem primarily uses the Metaplex Token Metadata standard for non-fungible tokens. This chapter covers migrating Metaplex NFT operations to ERC-721 patterns in Stylus, including minting, transfers, approvals, and metadata management.

To illustrate NFT operations comprehensively, we will implement a contract that creates a complete NFT collection with minting, metadata, and transfer capabilities.

## Solana

Solana NFTs use the Metaplex Token Metadata Program built on top of SPL Tokens. Each NFT requires three accounts: a mint account (SPL Token with supply of 1), a metadata account (storing name, symbol, URI), and optionally a master edition account (marking it as an NFT). Programs interact with NFTs through CPIs to both the SPL Token and Metaplex programs. Collections use a collection NFT that individual NFTs reference. Creators and royalties are stored on-chain in the metadata. Token Metadata v2 adds programmable NFTs with rule sets for transfer restrictions and utility uses.

#### Native 

```rust
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum Instruction {
    CreateNameCollection,
    MintNameNft { name: String },
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let instruction = Instruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        Instruction::CreateNameCollection => process_create_name_collection(program_id, accounts),
        Instruction::MintNameNft { name } => process_mint_name_nft(program_id, accounts, name),
    }
}

fn process_create_name_collection(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Create Name Collection");

    let [authority, collection_mint, collection_metadata, collection_master_edition, collection_token, system_program, token_program, associated_token_program, token_metadata_program, rent_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify program IDs
    if *system_program.key != system_program::id()
        || *token_program.key != spl_token_2022::id()
        || *associated_token_program.key != spl_associated_token_account::id()
        || *token_metadata_program.key != mpl_token_metadata::ID
        || *rent_sysvar.key != rent::sysvar::id()
    {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Derive and verify collection mint PDA
    let (collection_mint_key, collection_bump) =
        Pubkey::find_program_address(&[COLLECTION_SEED], program_id);

    if collection_mint_key != *collection_mint.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if collection mint already exists
    if !collection_mint.data_is_empty() || *collection_mint.owner == spl_token_2022::id() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Verify collection token account
    let expected_collection_token =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            authority.key,
            &collection_mint_key,
            &spl_token_2022::id(),
        );

    if expected_collection_token != *collection_token.key {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify metadata account
    let (expected_metadata, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            &mpl_token_metadata::ID.to_bytes(),
            &collection_mint_key.to_bytes(),
        ],
        &mpl_token_metadata::ID,
    );

    if expected_metadata != *collection_metadata.key {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify master edition account
    let (expected_edition, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            &mpl_token_metadata::ID.to_bytes(),
            &collection_mint_key.to_bytes(),
            b"edition",
        ],
        &mpl_token_metadata::ID,
    );

    if expected_edition != *collection_master_edition.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let signer_seeds = &[COLLECTION_SEED, &[collection_bump]];

    // Create mint account
    let mint_space = Mint::get_packed_len();
    let mint_lamports = Rent::get()?.minimum_balance(mint_space);

    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            collection_mint.key,
            mint_lamports,
            mint_space as u64,
            &spl_token_2022::id(),
        ),
        &[
            authority.clone(),
            collection_mint.clone(),
            system_program.clone(),
        ],
        &[signer_seeds],
    )?;

    msg!("Created Name Collection Mint Account");

    // Initialize mint with 0 decimals for NFT
    invoke_signed(
        &token_instruction::initialize_mint(
            &spl_token_2022::id(),
            collection_mint.key,
            collection_mint.key,
            Some(collection_mint.key),
            0,
        )?,
        &[collection_mint.clone(), rent_sysvar.clone()],
        &[signer_seeds],
    )?;

    msg!("Intitialized Name Collection Mint");

    // Create associated token account
    invoke(
        &associated_token_instruction::create_associated_token_account(
            authority.key,
            authority.key,
            collection_mint.key,
            &spl_token_2022::id(),
        ),
        &[
            authority.clone(),
            collection_token.clone(),
            authority.clone(),
            collection_mint.clone(),
            system_program.clone(),
            token_program.clone(),
            associated_token_program.clone(),
        ],
    )?;

    msg!("Created Name Collection ATA");

    // Mint 1 token to the collection token account
    invoke_signed(
        &token_instruction::mint_to(
            &spl_token_2022::id(),
            collection_mint.key,
            collection_token.key,
            collection_mint.key,
            &[],
            1,
        )?,
        &[
            collection_mint.clone(),
            collection_token.clone(),
            collection_mint.clone(),
        ],
        &[signer_seeds],
    )?;

    msg!("Minted Collection to ATA");

    // Create metadata account
    let creators = vec![Creator {
        address: *collection_mint.key,
        verified: true,
        share: 100,
    }];

    let create_metadata_ix = CreateMetadataAccountV3Builder::new()
        .metadata(*collection_metadata.key)
        .mint(*collection_mint.key)
        .mint_authority(*collection_mint.key)
        .payer(*authority.key)
        .update_authority(*collection_mint.key, true)
        .system_program(*system_program.key)
        .data(DataV2 {
            name: "Mock Name Service".to_string(),
            symbol: "MNS".to_string(),
            uri: String::new(),
            seller_fee_basis_points: 0,
            creators: Some(creators),
            collection: None,
            uses: None,
        })
        .is_mutable(true)
        .collection_details(CollectionDetails::V1 { size: 0 })
        .instruction();

    invoke_signed(
        &create_metadata_ix,
        &[
            collection_metadata.clone(),
            collection_mint.clone(),
            collection_mint.clone(),
            authority.clone(),
            collection_mint.clone(),
            system_program.clone(),
        ],
        &[signer_seeds],
    )?;

    msg!("Created Name Collection Metadata");

    // Create master edition
    let create_edition_ix = CreateMasterEditionV3Builder::new()
        .edition(*collection_master_edition.key)
        .update_authority(*collection_mint.key)
        .mint_authority(*collection_mint.key)
        .mint(*collection_mint.key)
        .payer(*authority.key)
        .metadata(*collection_metadata.key)
        .token_program(*token_program.key)
        .system_program(*system_program.key)
        .max_supply(0)
        .instruction();

    invoke_signed(
        &create_edition_ix,
        &[
            collection_master_edition.clone(),
            collection_mint.clone(),
            collection_mint.clone(),
            collection_mint.clone(),
            authority.clone(),
            collection_metadata.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
        &[signer_seeds],
    )?;

    msg!("Created Name Collection Master Edition");

    Ok(())
}

fn process_mint_name_nft(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
) -> ProgramResult {
    let [owner, name_mint, name_token, name_metadata, name_master_edition, collection_mint, collection_metadata, collection_master_edition, system_program, token_program, associated_token_program, token_metadata_program, sysvar_instruction, rent_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !owner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Validate name
    if name.is_empty() || name.len() > MAX_NAME_LENGTH {
        return Err(ProgramError::InvalidArgument);
    }

    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ProgramError::InvalidArgument);
    }

    // Verify program IDs
    if *system_program.key != system_program::id()
        || *token_program.key != spl_token_2022::id()
        || *associated_token_program.key != spl_associated_token_account::id()
        || *token_metadata_program.key != mpl_token_metadata::ID
        || *sysvar_instruction.key != solana_sdk_ids::sysvar::instructions::id()
    {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Derive and verify name mint PDA
    let (name_mint_key, name_bump) =
        Pubkey::find_program_address(&[MINT_SEED, name.as_bytes()], program_id);

    if name_mint_key != *name_mint.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if name mint already exists
    if !name_mint.data_is_empty() || *name_mint.owner == spl_token_2022::id() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Verify collection mint PDA
    let (collection_mint_key, collection_bump) =
        Pubkey::find_program_address(&[COLLECTION_SEED], program_id);

    if collection_mint_key != *collection_mint.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify name token account
    let expected_name_token =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            owner.key,
            &name_mint_key,
            &spl_token_2022::id(),
        );

    if expected_name_token != *name_token.key {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify metadata accounts
    let (expected_name_metadata, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            &mpl_token_metadata::ID.to_bytes(),
            &name_mint_key.to_bytes(),
        ],
        &mpl_token_metadata::ID,
    );

    if expected_name_metadata != *name_metadata.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let (expected_name_edition, _) = Pubkey::find_program_address(
        &[
            b"metadata",
            &mpl_token_metadata::ID.to_bytes(),
            &name_mint_key.to_bytes(),
            b"edition",
        ],
        &mpl_token_metadata::ID,
    );

    if expected_name_edition != *name_master_edition.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let collection_signer_seeds = &[COLLECTION_SEED, &[collection_bump]];

    // Create name mint account
    let mint_space = Mint::get_packed_len();
    let mint_lamports = Rent::get()?.minimum_balance(mint_space);

    invoke_signed(
        &system_instruction::create_account(
            owner.key,
            name_mint.key,
            mint_lamports,
            mint_space as u64,
            &spl_token_2022::id(),
        ),
        &[owner.clone(), name_mint.clone(), system_program.clone()],
        &[&[MINT_SEED, &[name_bump]]],
    )?;

    // Initialize name mint with collection mint as authority
    invoke_signed(
        &token_instruction::initialize_mint(
            &spl_token_2022::id(),
            name_mint.key,
            collection_mint.key,
            Some(collection_mint.key),
            0,
        )?,
        &[name_mint.clone(), rent_sysvar.clone()],
        &[collection_signer_seeds],
    )?;

    // Create associated token account for name NFT
    invoke(
        &associated_token_instruction::create_associated_token_account(
            owner.key,
            owner.key,
            name_mint.key,
            &spl_token_2022::id(),
        ),
        &[
            owner.clone(),
            name_token.clone(),
            owner.clone(),
            name_mint.clone(),
            system_program.clone(),
            token_program.clone(),
            associated_token_program.clone(),
        ],
    )?;

    // Mint 1 token
    invoke_signed(
        &token_instruction::mint_to(
            &spl_token_2022::id(),
            name_mint.key,
            name_token.key,
            collection_mint.key,
            &[],
            1,
        )?,
        &[
            name_mint.clone(),
            name_token.clone(),
            collection_mint.clone(),
        ],
        &[collection_signer_seeds],
    )?;

    // Create metadata for name NFT
    let creators = vec![Creator {
        address: *collection_mint.key,
        verified: true,
        share: 100,
    }];

    let create_metadata_ix = CreateMetadataAccountV3Builder::new()
        .metadata(*name_metadata.key)
        .mint(*name_mint.key)
        .mint_authority(*collection_mint.key)
        .payer(*owner.key)
        .update_authority(*collection_mint.key, true)
        .system_program(*system_program.key)
        .data(DataV2 {
            name: name.clone(),
            symbol: "MSN".to_owned(),
            uri: String::new(),
            seller_fee_basis_points: 0,
            creators: Some(creators),
            collection: Some(Collection {
                verified: false,
                key: *collection_mint.key,
            }),
            uses: None,
        })
        .is_mutable(true)
        .instruction();

    invoke_signed(
        &create_metadata_ix,
        &[
            name_metadata.clone(),
            name_mint.clone(),
            collection_mint.clone(),
            owner.clone(),
            collection_mint.clone(),
            system_program.clone(),
        ],
        &[collection_signer_seeds],
    )?;

    // Create master edition for name NFT
    let create_edition_ix = CreateMasterEditionV3Builder::new()
        .edition(*name_master_edition.key)
        .update_authority(*collection_mint.key)
        .mint_authority(*collection_mint.key)
        .mint(*name_mint.key)
        .payer(*owner.key)
        .metadata(*name_metadata.key)
        .token_program(*token_program.key)
        .system_program(*system_program.key)
        .max_supply(1)
        .instruction();

    invoke_signed(
        &create_edition_ix,
        &[
            name_master_edition.clone(),
            collection_mint.clone(),
            collection_mint.clone(),
            name_mint.clone(),
            owner.clone(),
            name_metadata.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
        &[collection_signer_seeds],
    )?;

    // Verify collection membership
    let verify_collection_ix = VerifyCollectionV1Builder::new()
        .authority(*collection_mint.key)
        .metadata(*name_metadata.key)
        .collection_mint(*collection_mint.key)
        .collection_metadata(Some(*collection_metadata.key))
        .collection_master_edition(Some(*collection_master_edition.key))
        .system_program(*system_program.key)
        .sysvar_instructions(*sysvar_instruction.key)
        .instruction();

    invoke_signed(
        &verify_collection_ix,
        &[
            collection_mint.clone(),
            name_metadata.clone(),
            collection_mint.clone(),
            collection_metadata.clone(),
            collection_master_edition.clone(),
            system_program.clone(),
            sysvar_instruction.clone(),
        ],
        &[collection_signer_seeds],
    )?;

    Ok(())
}
```

#### Anchor 

```rust
#[program]
pub mod non_fungible_tokens {
    use super::*;

    pub fn create_name_collection(ctx: Context<CreateNameCollection>) -> Result<()> {
        // Mint the collection NFT
        let seeds = &[COLLECTION_SEED, &[ctx.bumps.collection_mint]];
        let signer_seeds = &[&seeds[..]];

        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.collection_mint.to_account_info(),
                    to: ctx.accounts.collection_token.to_account_info(),
                    authority: ctx.accounts.collection_mint.to_account_info(),
                },
                signer_seeds,
            ),
            1,
        )?;
        msg!("Name Collection NFT minted!");

        // Create metadata account for the collection
        let creator = vec![Creator {
            address: ctx.accounts.collection_mint.key(),
            verified: true,
            share: 100,
        }];

        CreateMetadataAccountV3Cpi::new(
            &ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountV3CpiAccounts {
                metadata: &ctx.accounts.collection_metadata.to_account_info(),
                mint: &ctx.accounts.collection_mint.to_account_info(),
                mint_authority: &ctx.accounts.collection_mint.to_account_info(),
                payer: &ctx.accounts.authority.to_account_info(),
                update_authority: (&ctx.accounts.collection_mint.to_account_info(), true),
                system_program: &ctx.accounts.system_program.to_account_info(),
                rent: None,
            },
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name: "Mock Name Service".to_owned(),
                    symbol: "MNS".to_owned(),
                    uri: String::new(),
                    seller_fee_basis_points: 0,
                    creators: Some(creator),
                    collection: None,
                    uses: None,
                },
                is_mutable: true,
                collection_details: Some(CollectionDetails::V1 { size: 0 }),
            },
        )
        .invoke_signed(signer_seeds)?;

        // Create master edition for collection
        CreateMasterEditionV3Cpi::new(
            &ctx.accounts.token_metadata_program.to_account_info(),
            CreateMasterEditionV3CpiAccounts {
                edition: &ctx.accounts.collection_master_edition.to_account_info(),
                update_authority: &ctx.accounts.collection_mint.to_account_info(),
                mint_authority: &ctx.accounts.collection_mint.to_account_info(),
                mint: &ctx.accounts.collection_mint.to_account_info(),
                payer: &ctx.accounts.authority.to_account_info(),
                metadata: &ctx.accounts.collection_metadata.to_account_info(),
                token_program: &ctx.accounts.token_program.to_account_info(),
                system_program: &ctx.accounts.system_program.to_account_info(),
                rent: None,
            },
            CreateMasterEditionV3InstructionArgs {
                max_supply: Some(0),
            },
        )
        .invoke_signed(signer_seeds)?;

        Ok(())
    }

    pub fn mint_name_nft(ctx: Context<MintNameNFT>, name: String) -> Result<()> {
        require!(
            !name.is_empty() && name.len() <= MAX_NAME_LENGTH,
            ErrorCode::InvalidNameLength
        );
        require!(
            name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            ErrorCode::InvalidNameCharacters
        );

        let collection_seeds = &[COLLECTION_SEED, &[ctx.bumps.collection_mint]];
        let collection_signer_seeds = &[&collection_seeds[..]];

        // Mint the Name NFT
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.name_mint.to_account_info(),
                    to: ctx.accounts.name_token.to_account_info(),
                    authority: ctx.accounts.collection_mint.to_account_info(),
                },
                collection_signer_seeds,
            ),
            1,
        )?;

        // Create metadata with the name
        let creator = vec![Creator {
            address: ctx.accounts.collection_mint.key(),
            verified: true,
            share: 100,
        }];

        CreateMetadataAccountV3Cpi::new(
            &ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountV3CpiAccounts {
                metadata: &ctx.accounts.name_metadata.to_account_info(),
                mint: &ctx.accounts.name_mint.to_account_info(),
                mint_authority: &ctx.accounts.collection_mint.to_account_info(),
                payer: &ctx.accounts.owner.to_account_info(),
                update_authority: (&ctx.accounts.collection_mint.to_account_info(), true),
                system_program: &ctx.accounts.system_program.to_account_info(),
                rent: None,
            },
            CreateMetadataAccountV3InstructionArgs {
                data: DataV2 {
                    name,
                    symbol: "MSN".to_owned(),
                    uri: String::new(),
                    seller_fee_basis_points: 0,
                    creators: Some(creator),
                    collection: Some(Collection {
                        verified: false,
                        key: ctx.accounts.collection_mint.key(),
                    }),
                    uses: None,
                },
                is_mutable: true,
                collection_details: None,
            },
        )
        .invoke_signed(collection_signer_seeds)?;

        // Create master edition for the name NFT
        CreateMasterEditionV3Cpi::new(
            &ctx.accounts.token_metadata_program.to_account_info(),
            CreateMasterEditionV3CpiAccounts {
                edition: &ctx.accounts.name_master_edition.to_account_info(),
                update_authority: &ctx.accounts.collection_mint.to_account_info(),
                mint_authority: &ctx.accounts.collection_mint.to_account_info(),
                mint: &ctx.accounts.name_mint.to_account_info(),
                payer: &ctx.accounts.owner.to_account_info(),
                metadata: &ctx.accounts.name_metadata.to_account_info(),
                token_program: &ctx.accounts.token_program.to_account_info(),
                system_program: &ctx.accounts.system_program.to_account_info(),
                rent: None,
            },
            CreateMasterEditionV3InstructionArgs {
                max_supply: Some(0),
            },
        )
        .invoke_signed(collection_signer_seeds)?;

        // Verify collection membership
        VerifyCollectionV1Cpi::new(
            &ctx.accounts.token_metadata_program.to_account_info(),
            VerifyCollectionV1CpiAccounts {
                authority: &ctx.accounts.collection_mint.to_account_info(),
                delegate_record: None,
                metadata: &ctx.accounts.name_metadata.to_account_info(),
                collection_mint: &ctx.accounts.collection_mint.to_account_info(),
                collection_metadata: Some(&ctx.accounts.collection_metadata.to_account_info()),
                collection_master_edition: Some(
                    &ctx.accounts.collection_master_edition.to_account_info(),
                ),
                system_program: &ctx.accounts.system_program.to_account_info(),
                sysvar_instructions: &ctx.accounts.sysvar_instruction.to_account_info(),
            },
        )
        .invoke_signed(collection_signer_seeds)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateNameCollection<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = collection_mint.key(),
        mint::freeze_authority = collection_mint.key(),
        seeds = [COLLECTION_SEED],
        bump,
    )]
    pub collection_mint: Account<'info, Mint>,

    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub collection_metadata: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub collection_master_edition: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = collection_mint,
        associated_token::authority = authority
    )]
    pub collection_token: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,
}

#[derive(Accounts)]
#[instruction(name: String)]
pub struct MintNameNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        mint::decimals = 0,
        mint::authority = collection_mint,
        mint::freeze_authority = collection_mint,
        seeds = [MINT_SEED, name.as_bytes()],
        bump,
    )]
    pub name_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = owner,
        associated_token::mint = name_mint,
        associated_token::authority = owner
    )]
    pub name_token: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub name_metadata: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: This account will be initialized by the metaplex program
    pub name_master_edition: UncheckedAccount<'info>,

    // Collection accounts for verification
    #[account(
        mut,
        seeds = [COLLECTION_SEED],
        bump,
    )]
    pub collection_mint: Account<'info, Mint>,

    #[account(mut)]
    pub collection_metadata: Account<'info, MetadataAccount>,

    pub collection_master_edition: Account<'info, MasterEditionAccount>,

    // System accounts
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,

    #[account(address = solana_sdk_ids::sysvar::instructions::ID)]
    /// CHECK: Sysvar instruction account that is being checked with an address constraint
    pub sysvar_instruction: UncheckedAccount<'info>,
}
```

## Stylus

Stylus NFTs follow the ERC-721 standard: each collection is a single contract managing all tokens through internal mappings. Token ownership, approvals, and metadata are stored directly in contract storage. The standard interface - `ownerOf`, `approve`, `transferFrom` - ensures marketplace compatibility. Contracts extend base functionality through modular patterns. OpenZeppelin's Stylus implementations provide components for enumeration, metadata URIs, and royalties. The single-contract model simplifies collection management compared to Solana's multi-account approach.

```rust
sol! {
    #[derive(Debug)]
    error InvalidNameLength();

    #[derive(Debug)]
    error InvalidNameCharacters();

    #[derive(Debug)]
    error NameAlreadyMinted();
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    InvalidNameLength(InvalidNameLength),
    InvalidNameCharacters(InvalidNameCharacters),
    NameAlreadyMinted(NameAlreadyMinted),
    Erc721(erc721::Error),
}

#[storage]
#[entrypoint]
pub struct NameCollectionContract {
    erc721: Erc721,
    metadata: Erc721Metadata,
    // Map names to token ID
    minted_names: StorageMap<String, StorageU256>,
    // Map token ID to name
    token_names: StorageMap<U256, StorageString>,
    // track supply
    next_token_id: StorageU256,
}

#[public]
#[implements(IErc721<Error = erc721::Error>, IErc721Metadata<Error = erc721::Error>, IErc165)]
impl NameCollectionContract {
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), ContractError> {
        // Initialize the collection metadata
        self.metadata
            .constructor("Mock Name Service".into(), "MNS".into());
        self.next_token_id.set(U256::ONE);
        Ok(())
    }

    pub fn mint_name_nft(&mut self, to: Address, name: String) -> Result<U256, ContractError> {
        // Validate name length
        if name.is_empty() || name.len() > MAX_NAME_LENGTH {
            return Err(ContractError::InvalidNameLength(InvalidNameLength {}));
        }

        // Validate name characters (alphanumeric and underscore only)
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ContractError::InvalidNameCharacters(
                InvalidNameCharacters {},
            ));
        }

        // Check if name is already minted
        if self.is_name_minted(name.clone()) {
            return Err(NameAlreadyMinted {}.into());
        }

        // Get next token ID
        let token_id = self.next_token_id.get();

        // Mint the NFT
        self.erc721._mint(to, token_id)?;

        // Set the bi-directional name mapping
        self.minted_names.setter(name.clone()).set(token_id);
        self.token_names.setter(token_id).set_str(&name);

        // Increment token ID for next mint
        self.next_token_id.set(token_id + U256::from(1));

        Ok(token_id)
    }

    pub fn get_token_id_by_name(&self, name: String) -> U256 {
        self.minted_names.get(name)
    }

    pub fn get_name_by_token_id(&self, token_id: U256) -> String {
        self.token_names.getter(token_id).get_string()
    }

    pub fn is_name_minted(&self, name: String) -> bool {
        self.minted_names.get(name) > U256::ZERO
    }

    pub fn total_minted(&self) -> U256 {
        self.next_token_id.get() - U256::ONE
    }
}

#[public]
impl IErc721 for NameCollectionContract {
    type Error = erc721::Error;

    fn balance_of(&self, owner: Address) -> Result<U256, Self::Error> {
        self.erc721.balance_of(owner)
    }

    fn owner_of(&self, token_id: U256) -> Result<Address, Self::Error> {
        self.erc721.owner_of(token_id)
    }

    fn safe_transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), Self::Error> {
        self.erc721.safe_transfer_from(from, to, token_id)
    }

    fn safe_transfer_from_with_data(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
        data: Bytes,
    ) -> Result<(), Self::Error> {
        self.erc721
            .safe_transfer_from_with_data(from, to, token_id, data)
    }

    fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), Self::Error> {
        self.erc721.transfer_from(from, to, token_id)
    }

    fn approve(&mut self, to: Address, token_id: U256) -> Result<(), Self::Error> {
        self.erc721.approve(to, token_id)
    }

    fn set_approval_for_all(&mut self, to: Address, approved: bool) -> Result<(), Self::Error> {
        self.erc721.set_approval_for_all(to, approved)
    }

    fn get_approved(&self, token_id: U256) -> Result<Address, Self::Error> {
        self.erc721.get_approved(token_id)
    }

    fn is_approved_for_all(&self, owner: Address, operator: Address) -> bool {
        self.erc721.is_approved_for_all(owner, operator)
    }
}

#[public]
impl IErc721Metadata for NameCollectionContract {
    type Error = erc721::Error;

    fn name(&self) -> String {
        self.metadata.name()
    }

    fn symbol(&self) -> String {
        self.metadata.symbol()
    }

    /// unused
    fn token_uri(&self, _token_id: U256) -> Result<String, Self::Error> {
        Ok(String::new())
    }
}

#[public]
impl IErc165 for NameCollectionContract {
    fn supports_interface(&self, interface_id: B32) -> bool {
        self.erc721.supports_interface(interface_id)
            || <Self as IErc721Metadata>::interface_id() == interface_id
    }
}
```

## Next Steps

With non-fungible tokens covered, the next chapter explores [Errors and Events](./errors-events.md) - migrating Solana's logging and error patterns to Stylus structured events and custom errors.
