#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult, log, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    InvalidAmount {},
    Unauthorized {},
    Log {},
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum ErrorCode {
    InvalidAmount,
    Unauthorized,
}

impl From<ErrorCode> for ProgramError {
    fn from(value: ErrorCode) -> Self {
        Self::Custom(value as _)
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let Ok(ix) = Instruction::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };

    match ix {
        Instruction::InvalidAmount {} => {
            msg!("Instruction: InvalidAmount");
            process_invalid_value(accounts)
        }
        Instruction::Unauthorized {} => {
            msg!("Instruction: Unauthorized");
            process_unauthorized(accounts)
        }
        Instruction::Log {} => {
            msg!("Instruction: Log");
            process_log(accounts)
        }
    }
}

fn process_invalid_value(_accounts: &[AccountInfo]) -> ProgramResult {
    Err(ErrorCode::InvalidAmount.into())
}

fn process_unauthorized(_accounts: &[AccountInfo]) -> ProgramResult {
    Err(ErrorCode::Unauthorized.into())
}

fn process_log(accounts: &[AccountInfo]) -> ProgramResult {
    log::sol_log("just a regular string");
    log::sol_log_64(1, 2, 3, 4, 5);
    log::sol_log_compute_units();
    log::sol_log_data(&[b"some", b"serialized", b"structures", b"as base64"]);
    log::sol_log_params(accounts, b"instruction data");
    log::sol_log_slice(b"some bytes as hex");
    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use super::{Instruction, ID as PROGRAM_ID};

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::AccountMeta;
    use solana_program::instruction::Instruction as SolanaInstruction;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let signer_key = Pubkey::new_unique();
        let signer_lamports = 100_000_000;
        let signer_account = Account::new(signer_lamports, 0, &system_program::id());

        let log_instruction_data = borsh::to_vec(&Instruction::Log {}).unwrap();

        let log_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &log_instruction_data,
            vec![
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        mollusk.process_and_validate_instruction_chain(
            &[(&log_instruction, &[Check::success()])],
            &[
                (signer_key, signer_account.clone()),
                keyed_account_for_system_program(),
            ],
        );
    }
}
