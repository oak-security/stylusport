#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, program::set_return_data,
    program_error::ProgramError, pubkey::Pubkey,
};

declare_id!("JAQ5MVHbCkSYRzXunsrNuM2m1LS859PGveHfoYPAmcvZ");

#[cfg(feature = "no-entrypoint")]
pub static PROGRAM_NAME: &str = env!("CARGO_CRATE_NAME");

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Args {
    pub a: u64,
    pub b: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Response {
    pub result: u128,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let Ok(args) = Args::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };

    if !accounts.is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    let result = u128::from(args.a) + u128::from(args.b);

    let return_data = borsh::to_vec(&Response { result }).expect("infallible serialization");

    set_return_data(&return_data);

    Ok(())
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use super::{Args, Response, ID as PROGRAM_ID};
    use mollusk_svm::{result::Check, Mollusk};
    use solana_program::instruction::Instruction;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let args = Args { a: 5, b: 10 };
        let instruction_data = borsh::to_vec(&args).unwrap();

        let instruction = Instruction::new_with_bytes(PROGRAM_ID, &instruction_data, vec![]);

        let expected_response = Response { result: 15 };
        let expected_return_data = borsh::to_vec(&expected_response).unwrap();

        mollusk.process_and_validate_instruction(
            &instruction,
            &[],
            &[Check::success(), Check::return_data(&expected_return_data)],
        );
    }
}
