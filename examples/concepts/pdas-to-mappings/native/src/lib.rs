#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult,
    program::invoke_signed, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
    sysvar::Sysvar,
};
use solana_system_interface::instruction as system_instruction;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

pub static SEED_SEPARATOR: &[u8] = b"-";
pub static PLAYER_PDA_ACCOUNT_SEED: &[u8] = b"player";

pub const STARTING_LIVES: u8 = 10;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct PlayerAccountState {
    pub lives: u8,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let Ok(args) = PlayerAccountState::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };

    // ensure correct initial player state is provided
    if args.lives != STARTING_LIVES {
        return Err(ProgramError::InvalidInstructionData);
    }

    let [payer, player_pda_account, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    // ensure PDA has not already been initialized
    if !player_pda_account.data_is_empty()
        || player_pda_account.lamports() > 0
        || *player_pda_account.owner == ID
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let (player_pda_account_key, bump) = Pubkey::find_program_address(
        &[PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key.as_ref()],
        &ID,
    );

    if player_pda_account_key != *player_pda_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let lamports_required = Rent::get()?.minimum_balance(instruction_data.len());

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            player_pda_account.key,
            lamports_required,
            instruction_data.len() as u64,
            program_id,
        ),
        &[
            payer.clone(),
            player_pda_account.clone(),
            system_program.clone(),
        ],
        &[&[
            PLAYER_PDA_ACCOUNT_SEED,
            SEED_SEPARATOR,
            payer.key.as_ref(),
            &[bump],
        ]],
    )?;

    let mut data_account_buffer = player_pda_account.try_borrow_mut_data()?;

    data_account_buffer.copy_from_slice(instruction_data);

    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use crate::SEED_SEPARATOR;

    use super::{PlayerAccountState, ID as PROGRAM_ID, PLAYER_PDA_ACCOUNT_SEED};

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let payer_key = Pubkey::new_unique();
        let payer_lamports = 100_000_000;
        let payer_account = Account::new(payer_lamports, 0, &system_program::id());

        let (player_pda_account_key, bump) = Pubkey::find_program_address(
            &[PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer_key.as_ref()],
            &PROGRAM_ID,
        );

        let instruction_data = borsh::to_vec(&PlayerAccountState { lives: 10 }).unwrap();

        let initialize_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &instruction_data,
            vec![
                AccountMeta::new(payer_key, true),
                AccountMeta::new(player_pda_account_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        mollusk.process_and_validate_instruction(
            &initialize_instruction,
            &[
                (payer_key, payer_account),
                (player_pda_account_key, Account::default()),
                keyed_account_for_system_program(),
            ],
            &[
                Check::success(),
                Check::account(&player_pda_account_key)
                    .data(&instruction_data)
                    .owner(&PROGRAM_ID)
                    .build(),
            ],
        );
    }
}
