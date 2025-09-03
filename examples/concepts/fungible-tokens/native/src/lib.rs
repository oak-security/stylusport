#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    declare_id, entrypoint,
    entrypoint::ProgramResult,
    program::invoke,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::{self, Rent},
    sysvar::Sysvar,
};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;
use spl_associated_token_account::instruction as associated_token_instruction;
use spl_token_2022::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint},
};

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

pub static MINT_PDA_SEED: &[u8] = b"mint";
pub static STAKE_PDA_SEED: &[u8] = b"stake";
pub const DECIMALS: u8 = 6;
pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000; // 1B tokens

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    Initialize,
    Stake { amount: u64 },
    Unstake { amount: u64 },
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
        Instruction::Initialize => process_initialize(program_id, accounts),
        Instruction::Stake { amount } => process_stake(program_id, accounts, amount),
        Instruction::Unstake { amount } => process_unstake(program_id, accounts, amount),
    }
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [mint_account, mint_supply_to_account, signer_account, token_program, associated_token_program, system_program, rent_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token_2022::id()
        || *associated_token_program.key != spl_associated_token_account::id()
        || *system_program.key != system_program::id()
        || *rent_sysvar.key != rent::sysvar::id()
    {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (mint_pda_key, mint_bump) = Pubkey::find_program_address(&[MINT_PDA_SEED], program_id);

    if mint_pda_key != *mint_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if !mint_account.data_is_empty()
        || mint_account.lamports() > 0
        || *mint_account.owner == spl_token_2022::id()
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if !mint_supply_to_account.data_is_empty()
        || mint_supply_to_account.lamports() > 0
        || *mint_supply_to_account.owner == spl_token_2022::id()
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Derive the expected associated token account address
    let expected_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        signer_account.key,
        &mint_pda_key,
        &spl_token_2022::id(),
    );

    if expected_ata != *mint_supply_to_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create mint account
    let space_required = Mint::get_packed_len();
    let lamports_required = Rent::get()?.minimum_balance(space_required);

    invoke_signed(
        &system_instruction::create_account(
            signer_account.key,
            mint_account.key,
            lamports_required,
            space_required as u64,
            &spl_token_2022::id(),
        ),
        &[
            signer_account.clone(),
            mint_account.clone(),
            system_program.clone(),
        ],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    // Initialize mint
    invoke_signed(
        &token_instruction::initialize_mint(
            &spl_token_2022::id(),
            mint_account.key,
            mint_account.key,
            Some(mint_account.key),
            DECIMALS,
        )?,
        &[mint_account.clone(), rent_sysvar.clone()],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    // Create associated token account
    invoke_signed(
        &associated_token_instruction::create_associated_token_account(
            signer_account.key,
            signer_account.key,
            mint_account.key,
            &spl_token_2022::id(),
        ),
        &[
            signer_account.clone(),
            mint_supply_to_account.clone(),
            signer_account.clone(),
            mint_account.clone(),
            system_program.clone(),
            token_program.clone(),
            associated_token_program.clone(),
        ],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    // Mint total supply to the associated token account
    invoke_signed(
        &token_instruction::mint_to(
            &spl_token_2022::id(),
            mint_account.key,
            mint_supply_to_account.key,
            mint_account.key,
            &[],
            TOTAL_SUPPLY,
        )?,
        &[
            mint_account.clone(),
            mint_supply_to_account.clone(),
            mint_account.clone(),
        ],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    Ok(())
}

fn process_stake(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [stake_account, from_account, signer_account, mint_account, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token_2022::id() || *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify stake PDA
    let (stake_pda_key, stake_bump) =
        Pubkey::find_program_address(&[STAKE_PDA_SEED, signer_account.key.as_ref()], program_id);

    if stake_pda_key != *stake_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Create stake account if it doesn't exist
    if stake_account.data_is_empty() || *stake_account.owner != spl_token_2022::id() {
        let space_required = TokenAccount::get_packed_len();
        let lamports_required = Rent::get()?.minimum_balance(space_required);

        invoke_signed(
            &system_instruction::create_account(
                signer_account.key,
                stake_account.key,
                lamports_required,
                space_required as u64,
                &spl_token_2022::id(),
            ),
            &[
                signer_account.clone(),
                stake_account.clone(),
                system_program.clone(),
            ],
            &[&[STAKE_PDA_SEED, signer_account.key.as_ref(), &[stake_bump]]],
        )?;

        // Initialize the stake token account
        invoke_signed(
            &token_instruction::initialize_account3(
                &spl_token_2022::id(),
                stake_account.key,
                mint_account.key,
                stake_account.key,
            )?,
            &[stake_account.clone(), mint_account.clone()],
            &[&[STAKE_PDA_SEED, signer_account.key.as_ref(), &[stake_bump]]],
        )?;
    }

    // Transfer tokens from user's account to stake account
    invoke(
        &token_instruction::transfer_checked(
            &spl_token_2022::id(),
            from_account.key,
            mint_account.key,
            stake_account.key,
            signer_account.key,
            &[],
            amount,
            DECIMALS,
        )?,
        &[
            from_account.clone(),
            mint_account.clone(),
            stake_account.clone(),
            signer_account.clone(),
        ],
    )?;

    Ok(())
}

fn process_unstake(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [stake_account, unstake_to_account, signer_account, mint_account, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token_2022::id() || *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify stake PDA
    let (stake_pda_key, stake_bump) =
        Pubkey::find_program_address(&[STAKE_PDA_SEED, signer_account.key.as_ref()], program_id);

    if stake_pda_key != *stake_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Transfer tokens from stake account to user's account
    invoke_signed(
        &token_instruction::transfer_checked(
            &spl_token_2022::id(),
            stake_account.key,
            mint_account.key,
            unstake_to_account.key,
            stake_account.key,
            &[],
            amount,
            DECIMALS,
        )?,
        &[
            stake_account.clone(),
            mint_account.clone(),
            unstake_to_account.clone(),
            stake_account.clone(),
        ],
        &[&[STAKE_PDA_SEED, signer_account.key.as_ref(), &[stake_bump]]],
    )?;

    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use super::{
        Instruction, DECIMALS, ID as PROGRAM_ID, MINT_PDA_SEED, STAKE_PDA_SEED, TOTAL_SUPPLY,
    };

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::AccountMeta;
    use solana_program::instruction::Instruction as SolanaInstruction;
    use solana_program_pack::Pack;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;
    use spl_token_2022::state::{Account as TokenAccount, AccountState, Mint};

    #[test]
    fn test_program() {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        // Add required programs to mollusk
        mollusk_svm_programs_token::token2022::add_program(&mut mollusk);
        mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);

        let signer_key = Pubkey::new_unique();
        let signer_lamports = 100_000_000;
        let signer_account = Account::new(signer_lamports, 0, &system_program::id());

        let (mint_pda_key, _) = Pubkey::find_program_address(&[MINT_PDA_SEED], &PROGRAM_ID);
        let mint_account = Account::default();

        let mint_supply_to_key =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &signer_key,
                &mint_pda_key,
                &mollusk_svm_programs_token::token2022::ID,
            );
        let mint_supply_to_account = Account::default();

        let (stake_pda_key, _) =
            Pubkey::find_program_address(&[STAKE_PDA_SEED, signer_key.as_ref()], &PROGRAM_ID);
        let stake_account = Account::default();

        // Create initialize instruction
        let initialize_instruction_data = borsh::to_vec(&Instruction::Initialize).unwrap();

        let initialize_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &initialize_instruction_data,
            vec![
                AccountMeta::new(mint_pda_key, false),
                AccountMeta::new(mint_supply_to_key, false),
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(mollusk_svm_programs_token::token2022::ID, false),
                AccountMeta::new_readonly(mollusk_svm_programs_token::associated_token::ID, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::id(), false),
            ],
        );

        // Create stake instruction
        let stake_amount = (TOTAL_SUPPLY * 3) / 4;
        let stake_instruction_data = borsh::to_vec(&Instruction::Stake {
            amount: stake_amount,
        })
        .unwrap();

        let stake_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &stake_instruction_data,
            vec![
                AccountMeta::new(stake_pda_key, false),
                AccountMeta::new(mint_supply_to_key, false),
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(mint_pda_key, false),
                AccountMeta::new_readonly(mollusk_svm_programs_token::token2022::ID, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Create unstake instruction
        let unstake_instruction_data = borsh::to_vec(&Instruction::Unstake {
            amount: stake_amount,
        })
        .unwrap();

        let unstake_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &unstake_instruction_data,
            vec![
                AccountMeta::new(stake_pda_key, false),
                AccountMeta::new(mint_supply_to_key, false),
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(mint_pda_key, false),
                AccountMeta::new_readonly(mollusk_svm_programs_token::token2022::ID, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Expected mint account data after initialization
        let mut expected_mint_data = vec![0u8; Mint::LEN];
        Pack::pack(
            Mint {
                mint_authority: Some(mint_pda_key).into(),
                supply: TOTAL_SUPPLY,
                decimals: DECIMALS,
                is_initialized: true,
                freeze_authority: Some(mint_pda_key).into(),
            },
            &mut expected_mint_data,
        )
        .unwrap();

        // Expected token account data after initialization
        let mut expected_mint_supply_to_data = vec![0u8; TokenAccount::LEN];
        Pack::pack(
            TokenAccount {
                mint: mint_pda_key,
                owner: signer_key,
                amount: TOTAL_SUPPLY,
                delegate: None.into(),
                state: AccountState::Initialized,
                is_native: None.into(),
                delegated_amount: 0,
                close_authority: None.into(),
            },
            &mut expected_mint_supply_to_data,
        )
        .unwrap();

        // Expected stake account data after staking
        let mut expected_stake_account_data_post_stake = vec![0u8; TokenAccount::LEN];
        Pack::pack(
            TokenAccount {
                mint: mint_pda_key,
                owner: stake_pda_key,
                amount: stake_amount,
                delegate: None.into(),
                state: AccountState::Initialized,
                is_native: None.into(),
                delegated_amount: 0,
                close_authority: None.into(),
            },
            &mut expected_stake_account_data_post_stake,
        )
        .unwrap();

        // Expected mint_supply_to account data after staking
        let mut expected_mint_supply_to_data_post_stake = vec![0u8; TokenAccount::LEN];
        Pack::pack(
            TokenAccount {
                mint: mint_pda_key,
                owner: signer_key,
                amount: TOTAL_SUPPLY / 4,
                delegate: None.into(),
                state: AccountState::Initialized,
                is_native: None.into(),
                delegated_amount: 0,
                close_authority: None.into(),
            },
            &mut expected_mint_supply_to_data_post_stake,
        )
        .unwrap();

        // Expected stake account data after unstaking
        let mut expected_stake_account_data_post_unstake = vec![0u8; TokenAccount::LEN];
        Pack::pack(
            TokenAccount {
                mint: mint_pda_key,
                owner: stake_pda_key,
                amount: 0,
                delegate: None.into(),
                state: AccountState::Initialized,
                is_native: None.into(),
                delegated_amount: 0,
                close_authority: None.into(),
            },
            &mut expected_stake_account_data_post_unstake,
        )
        .unwrap();

        // Expected mint_supply_to account data after unstaking
        let mut expected_mint_supply_to_data_post_unstake = vec![0u8; TokenAccount::LEN];
        Pack::pack(
            TokenAccount {
                mint: mint_pda_key,
                owner: signer_key,
                amount: TOTAL_SUPPLY,
                delegate: None.into(),
                state: AccountState::Initialized,
                is_native: None.into(),
                delegated_amount: 0,
                close_authority: None.into(),
            },
            &mut expected_mint_supply_to_data_post_unstake,
        )
        .unwrap();

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &initialize_instruction,
                    &[
                        Check::success(),
                        Check::account(&mint_pda_key)
                            .data(&expected_mint_data)
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                        Check::account(&mint_supply_to_key)
                            .data_slice(0, &expected_mint_supply_to_data[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                    ],
                ),
                (
                    &stake_instruction,
                    &[
                        Check::success(),
                        Check::account(&mint_supply_to_key)
                            .data_slice(0, &expected_mint_supply_to_data_post_stake[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                        Check::account(&stake_pda_key)
                            .data_slice(0, &expected_stake_account_data_post_stake[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                    ],
                ),
                (
                    &stake_instruction,
                    &[Check::err(
                        spl_token_2022::error::TokenError::InsufficientFunds.into(),
                    )],
                ),
                (
                    &unstake_instruction,
                    &[
                        Check::success(),
                        Check::account(&mint_supply_to_key)
                            .data_slice(
                                0,
                                &expected_mint_supply_to_data_post_unstake[..32 + 32 + 8],
                            )
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                        Check::account(&stake_pda_key)
                            .data_slice(0, &expected_stake_account_data_post_unstake[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                    ],
                ),
                (
                    &unstake_instruction,
                    &[Check::err(
                        spl_token_2022::error::TokenError::InsufficientFunds.into(),
                    )],
                ),
            ],
            &[
                (mint_pda_key, mint_account),
                (mint_supply_to_key, mint_supply_to_account),
                (stake_pda_key, stake_account),
                (signer_key, signer_account),
                mollusk_svm_programs_token::token2022::keyed_account(),
                mollusk_svm_programs_token::associated_token::keyed_account(),
                keyed_account_for_system_program(),
                mollusk_svm::sysvar::Sysvars::default().keyed_account_for_rent_sysvar(),
            ],
        );
    }
}
