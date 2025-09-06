#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use mpl_token_metadata::{
    instructions::{
        CreateMasterEditionV3Builder, CreateMetadataAccountV3Builder, VerifyCollectionV1Builder,
    },
    types::{Collection, CollectionDetails, Creator, DataV2},
};
use solana_program::{
    account_info::AccountInfo,
    declare_id, entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::{self, Rent},
    sysvar::Sysvar,
};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;
use spl_associated_token_account::instruction as associated_token_instruction;
use spl_token_2022::{instruction as token_instruction, state::Mint};

declare_id!("3EMcczaGi9ivdLxvvFwRbGYeEUEHpGwabXegARw4jLxa");

pub static COLLECTION_SEED: &[u8] = b"collection";
pub static MINT_SEED: &[u8] = b"mint";
pub const MAX_NAME_LENGTH: usize = 10;

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

entrypoint!(process_instruction);

#[cfg(test)]
mod tests {
    use super::{Instruction, COLLECTION_SEED, ID as PROGRAM_ID, MINT_SEED};

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::{AccountMeta, Instruction as SolanaInstruction};
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    static MPL_TOKEN_METADATA_ELF: &[u8] = include_bytes!("../../elf/mpl-token-metadata.so");

    // TODO: Fix program so test passes. Directionally correct but account debugging required.
    #[test]
    fn test_program() {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        // Add required programs
        mollusk_svm_programs_token::token2022::add_program(&mut mollusk);
        mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);
        mollusk.add_program_with_elf_and_loader(
            &mpl_token_metadata::ID,
            MPL_TOKEN_METADATA_ELF,
            &mollusk_svm::program::loader_keys::LOADER_V2,
        );

        let authority_key = Pubkey::new_unique();
        let authority_lamports = 1_000_000_000;
        let authority_account = Account::new(authority_lamports, 0, &system_program::id());

        // Collection setup
        let (collection_mint_key, _) =
            Pubkey::find_program_address(&[COLLECTION_SEED], &PROGRAM_ID);
        let collection_mint_account = Account::default();

        let collection_token_key =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &authority_key,
                &collection_mint_key,
                &spl_token_2022::id(),
            );
        let collection_token_account = Account::default();

        let (collection_metadata_key, _) = Pubkey::find_program_address(
            &[
                b"metadata",
                &mpl_token_metadata::ID.to_bytes(),
                &collection_mint_key.to_bytes(),
            ],
            &mpl_token_metadata::ID,
        );
        let collection_metadata_account = Account::default();

        let (collection_master_edition_key, _) = Pubkey::find_program_address(
            &[
                b"metadata",
                &mpl_token_metadata::ID.to_bytes(),
                &collection_mint_key.to_bytes(),
                b"edition",
            ],
            &mpl_token_metadata::ID,
        );
        let collection_master_edition_account = Account::default();

        // Create collection instruction
        let create_collection_data = borsh::to_vec(&Instruction::CreateNameCollection).unwrap();
        let create_collection_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &create_collection_data,
            vec![
                AccountMeta::new(authority_key, true),
                AccountMeta::new(collection_mint_key, false),
                AccountMeta::new(collection_metadata_key, false),
                AccountMeta::new(collection_master_edition_key, false),
                AccountMeta::new(collection_token_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(spl_token_2022::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(mpl_token_metadata::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::id(), false),
            ],
        );

        // Name NFT setup
        let test_name = "alice";
        let (name_mint_key, _) =
            Pubkey::find_program_address(&[MINT_SEED, test_name.as_bytes()], &PROGRAM_ID);
        let name_mint_account = Account::default();

        let name_token_key =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &authority_key,
                &name_mint_key,
                &spl_token_2022::id(),
            );
        let name_token_account = Account::default();

        let (name_metadata_key, _) = Pubkey::find_program_address(
            &[
                b"metadata",
                &mpl_token_metadata::ID.to_bytes(),
                &name_mint_key.to_bytes(),
            ],
            &mpl_token_metadata::ID,
        );
        let name_metadata_account = Account::default();

        let (name_master_edition_key, _) = Pubkey::find_program_address(
            &[
                b"metadata",
                &mpl_token_metadata::ID.to_bytes(),
                &name_mint_key.to_bytes(),
                b"edition",
            ],
            &mpl_token_metadata::ID,
        );
        let name_master_edition_account = Account::default();

        let mint_name_data = borsh::to_vec(&Instruction::MintNameNft {
            name: test_name.to_string(),
        })
        .unwrap();
        let mint_name_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &mint_name_data,
            vec![
                AccountMeta::new(authority_key, true),
                AccountMeta::new(name_mint_key, false),
                AccountMeta::new(name_token_key, false),
                AccountMeta::new(name_metadata_key, false),
                AccountMeta::new(name_master_edition_key, false),
                AccountMeta::new(collection_mint_key, false),
                AccountMeta::new(collection_metadata_key, false),
                AccountMeta::new_readonly(collection_master_edition_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(spl_token_2022::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(mpl_token_metadata::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::instructions::id(), false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::id(), false),
            ],
        );

        // Process instruction chain
        mollusk.process_and_validate_instruction_chain(
            &[
                (&create_collection_instruction, &[Check::success()]),
                (&mint_name_instruction, &[Check::success()]),
            ],
            &[
                (authority_key, authority_account.clone()),
                // Collection accounts
                (collection_mint_key, collection_mint_account.clone()),
                (collection_token_key, collection_token_account.clone()),
                (collection_metadata_key, collection_metadata_account.clone()),
                (
                    collection_master_edition_key,
                    collection_master_edition_account.clone(),
                ),
                // Name NFT accounts
                (name_mint_key, name_mint_account.clone()),
                (name_token_key, name_token_account.clone()),
                (name_metadata_key, name_metadata_account.clone()),
                (name_master_edition_key, name_master_edition_account.clone()),
                // Programs
                mollusk_svm_programs_token::token2022::keyed_account(),
                mollusk_svm_programs_token::associated_token::keyed_account(),
                (
                    mpl_token_metadata::ID,
                    mollusk_svm::program::create_program_account_loader_v2(MPL_TOKEN_METADATA_ELF),
                ),
                keyed_account_for_system_program(),
                // Sysvar
                (
                    solana_sdk_ids::sysvar::instructions::id(),
                    Account::default(),
                ),
                mollusk_svm::sysvar::Sysvars::default().keyed_account_for_rent_sysvar(),
            ],
        );
    }
}
