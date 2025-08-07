#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    declare_id, entrypoint,
    entrypoint::ProgramResult,
    program::{get_return_data, invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use solana_system_interface::instruction as system_instruction;

use cpi_to_external_call_solana_adder::{Args as AdderArgs, Response, ID as ADDER_PROGRAM_ID};

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

pub static LAST_RESULT_ACCOUNT_SEED: &[u8] = b"last_result";

#[derive(BorshSerialize, BorshDeserialize)]
pub struct LastResultAccount {
    pub last_result: u128,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if AdderArgs::try_from_slice(instruction_data).is_err() {
        return Err(ProgramError::InvalidInstructionData);
    };

    let [payer, last_result_account, system_program, adder_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if *adder_program.key != ADDER_PROGRAM_ID {
        return Err(ProgramError::InvalidAccountData);
    }

    // Find the expected PDA and bump
    let (expected_pda, bump) =
        Pubkey::find_program_address(&[LAST_RESULT_ACCOUNT_SEED], program_id);

    // Verify the provided account matches the expected PDA
    if last_result_account.key != &expected_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    invoke(
        &solana_program::instruction::Instruction {
            program_id: cpi_to_external_call_solana_adder::ID,
            accounts: vec![],
            data: instruction_data.to_owned(),
        },
        &[adder_program.clone()],
    )?;

    let (invoked_program, data) = get_return_data().expect("return data is some after invoke");

    assert_eq!(
        invoked_program, ADDER_PROGRAM_ID,
        "expected return data from {ADDER_PROGRAM_ID}, received from {invoked_program}"
    );

    let Response { result } = Response::try_from_slice(&data)?;

    let last_result_account_data = borsh::to_vec(&LastResultAccount {
        last_result: result,
    })?;

    // Check if LastResult PDA Account needs to be created
    if last_result_account.owner != program_id {
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(last_result_account_data.len());

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                last_result_account.key,
                required_lamports,
                last_result_account_data.len() as u64,
                program_id,
            ),
            &[
                payer.clone(),
                last_result_account.clone(),
                system_program.clone(),
            ],
            &[&[LAST_RESULT_ACCOUNT_SEED, &[bump]]],
        )?;
    }

    last_result_account
        .try_borrow_mut_data()?
        .copy_from_slice(&last_result_account_data);

    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use super::{ID as PROGRAM_ID, *};

    use mollusk_svm::{
        program::{create_program_account_loader_v3, keyed_account_for_system_program},
        result::Check,
        Mollusk,
    };
    use solana_account::Account;
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        mollusk.add_program(
            &cpi_to_external_call_solana_adder::ID,
            cpi_to_external_call_solana_adder::PROGRAM_NAME,
            &mollusk_svm::program::loader_keys::LOADER_V3,
        );

        let payer_key = Pubkey::new_unique();
        let payer_lamports = 100_000_000;
        let payer_account = Account::new(payer_lamports, 0, &system_program::id());

        let (last_result_pda_account_key, _) =
            Pubkey::find_program_address(&[LAST_RESULT_ACCOUNT_SEED], &PROGRAM_ID);

        let instruction_data = borsh::to_vec(&AdderArgs { a: 5, b: 10 }).unwrap();

        let initialize_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &instruction_data,
            vec![
                AccountMeta::new(payer_key, true),
                AccountMeta::new(last_result_pda_account_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(cpi_to_external_call_solana_adder::ID, false),
            ],
        );

        let expected_account_data = borsh::to_vec(&LastResultAccount { last_result: 15 }).unwrap();

        mollusk.process_and_validate_instruction(
            &initialize_instruction,
            &[
                (payer_key, payer_account),
                (last_result_pda_account_key, Account::default()),
                keyed_account_for_system_program(),
                (
                    cpi_to_external_call_solana_adder::ID,
                    create_program_account_loader_v3(&cpi_to_external_call_solana_adder::ID),
                ),
            ],
            &[
                Check::success(),
                Check::account(&last_result_pda_account_key)
                    .data(&expected_account_data)
                    .owner(&PROGRAM_ID)
                    .build(),
            ],
        );
    }
}
