#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        mpl_token_metadata::{
            instructions::{
                CreateMasterEditionV3Cpi, CreateMasterEditionV3CpiAccounts,
                CreateMasterEditionV3InstructionArgs, CreateMetadataAccountV3Cpi,
                CreateMetadataAccountV3CpiAccounts, CreateMetadataAccountV3InstructionArgs,
                VerifyCollectionV1Cpi, VerifyCollectionV1CpiAccounts,
            },
            types::{Collection, CollectionDetails, Creator, DataV2},
        },
        MasterEditionAccount, Metadata, MetadataAccount,
    },
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

declare_id!("3EMcczaGi9ivdLxvvFwRbGYeEUEHpGwabXegARw4jLxa");

pub static COLLECTION_SEED: &[u8] = b"collection";
pub static MINT_SEED: &[u8] = b"mint";
pub const MAX_NAME_LENGTH: usize = 10;

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

        msg!("MPL Metadata account created!");

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

        msg!("MPL Master Edition account created!");

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

    // // System accounts
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,
    #[account(address = solana_sdk_ids::sysvar::instructions::ID)]
    /// CHECK: Sysvar instruction account that is being checked with an address constraint
    pub sysvar_instruction: UncheckedAccount<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Name must be between 1 and 10 characters")]
    InvalidNameLength,
    #[msg("Name can only contain alphanumeric characters and underscores")]
    InvalidNameCharacters,
}

#[cfg(test)]
mod tests {
    use super::{
        instruction::{CreateNameCollection, MintNameNft},
        COLLECTION_SEED, ID as PROGRAM_ID, MINT_SEED,
    };

    use anchor_lang::{
        prelude::AccountMeta,
        solana_program::{
            instruction::Instruction,
            sysvar::instructions::{
                construct_instructions_data, BorrowedAccountMeta, BorrowedInstruction,
            },
        },
        InstructionData,
    };
    use anchor_spl::{associated_token, metadata::mpl_token_metadata, token::spl_token};
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::{Account, AccountSharedData};
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    static MPL_TOKEN_METADATA_ELF: &[u8] = include_bytes!("../../elf/mpl-token-metadata.so");

    // https://github.com/anza-xyz/mollusk/issues/100
    fn get_account_instructions_sysvar(
        mollusk: &mut Mollusk,
        instructions: &[Instruction],
    ) -> (Pubkey, Account) {
        // Construct the instructions data from all instructions
        let mut data = construct_instructions_data(
            instructions
                .iter()
                .map(|instruction| BorrowedInstruction {
                    program_id: &instruction.program_id,
                    accounts: instruction
                        .accounts
                        .iter()
                        .map(|meta| BorrowedAccountMeta {
                            pubkey: &meta.pubkey,
                            is_signer: meta.is_signer,
                            is_writable: meta.is_writable,
                        })
                        .collect(),
                    data: &instruction.data,
                })
                .collect::<Vec<_>>()
                .as_slice(),
        );

        // Find which instruction contains the sysvar account and at what position
        if let Some((ix_index, _)) = instructions.iter().enumerate().find(|(_, instruction)| {
            instruction
                .accounts
                .iter()
                .any(|meta| meta.pubkey == solana_sdk_ids::sysvar::instructions::ID)
        }) {
            #[allow(deprecated)]
            spl_token::solana_program::sysvar::instructions::store_current_index(
                &mut data,
                ix_index as u16,
            );
        }

        // Set the account data
        let mut instruction_sysvar_account = AccountSharedData::new(
            mollusk.sysvars.rent.minimum_balance(data.len()),
            data.len(),
            &solana_sdk_ids::sysvar::id(),
        );
        instruction_sysvar_account.set_data_from_slice(&data);

        (
            solana_sdk_ids::sysvar::instructions::ID,
            instruction_sysvar_account.into(),
        )
    }

    #[test]
    fn test_program() {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        // Add required programs
        mollusk_svm_programs_token::token::add_program(&mut mollusk);
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
            associated_token::get_associated_token_address(&authority_key, &collection_mint_key);
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
        let create_collection_data = CreateNameCollection {}.data();
        let create_collection_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &create_collection_data,
            vec![
                AccountMeta::new(authority_key, true),
                AccountMeta::new(collection_mint_key, false),
                AccountMeta::new(collection_metadata_key, false),
                AccountMeta::new(collection_master_edition_key, false),
                AccountMeta::new(collection_token_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(associated_token::ID, false),
                AccountMeta::new_readonly(mpl_token_metadata::ID, false),
            ],
        );

        let test_name = "alice";
        let (name_mint_key, _) =
            Pubkey::find_program_address(&[MINT_SEED, test_name.as_bytes()], &PROGRAM_ID);
        let name_mint_account = Account::default();

        let name_token_key =
            associated_token::get_associated_token_address(&authority_key, &name_mint_key);
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

        let mint_name_data = MintNameNft {
            name: test_name.to_string(),
        }
        .data();
        let mint_name_instruction = Instruction::new_with_bytes(
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
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(associated_token::ID, false),
                AccountMeta::new_readonly(mpl_token_metadata::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::instructions::ID, false),
            ],
        );

        let keyed_instructions_sysvar_account = get_account_instructions_sysvar(
            &mut mollusk,
            &[
                create_collection_instruction.clone(),
                mint_name_instruction.clone(),
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
                mollusk_svm_programs_token::token::keyed_account(),
                mollusk_svm_programs_token::associated_token::keyed_account(),
                (
                    mpl_token_metadata::ID,
                    mollusk_svm::program::create_program_account_loader_v2(MPL_TOKEN_METADATA_ELF),
                ),
                keyed_account_for_system_program(),
                // Sysvar
                keyed_instructions_sysvar_account,
            ],
        );
    }
}
