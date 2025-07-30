#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult, program::invoke,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};
use solana_system_interface::instruction as system_instruction;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct Data {
    pub bool: bool,
    pub uint8: u8,
    pub uint16: u16,
    pub uint32: u32,
    pub uint64: u64,
    pub uint128: u128,
    pub int8: i8,
    pub int16: i16,
    pub int32: i32,
    pub int64: i64,
    pub int128: i128,
    pub string: String,
    pub bytes: Vec<u8>,
    pub address: Pubkey,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if Data::try_from_slice(instruction_data).is_err() {
        return Err(ProgramError::InvalidInstructionData);
    };

    let [payer, data_account, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    let lamports_required = Rent::get()?.minimum_balance(instruction_data.len());

    invoke(
        &system_instruction::create_account(
            payer.key,
            data_account.key,
            lamports_required,
            instruction_data.len() as u64,
            program_id,
        ),
        &[payer.clone(), data_account.clone(), system_program.clone()],
    )?;

    let mut data_account_buffer = data_account.try_borrow_mut_data()?;

    data_account_buffer.copy_from_slice(instruction_data);

    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use super::{Data, ID as PROGRAM_ID};

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let data_key = Pubkey::new_unique();

        let initial_values: Data = Data {
            bool: true,
            uint8: u8::MAX,
            uint16: u16::MAX,
            uint32: u32::MAX,
            uint64: u64::MAX,
            uint128: u128::MAX,
            int8: i8::MIN,
            int16: i16::MIN,
            int32: i32::MIN,
            int64: i64::MIN,
            int128: i128::MIN,
            string: "StylusPort::Solana".to_owned(),
            bytes: b"StylusPort::Solana".to_vec(),
            address: data_key,
        };

        let init_instruction_data = borsh::to_vec(&initial_values).unwrap();

        let payer_key = Pubkey::new_unique();
        let payer_lamports = 100_000_000;
        let payer_account = Account::new(payer_lamports, 0, &system_program::id());

        let initialize_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &init_instruction_data,
            vec![
                AccountMeta::new(payer_key, true),
                AccountMeta::new(data_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        mollusk.process_and_validate_instruction(
            &initialize_instruction,
            &[
                (payer_key, payer_account),
                (data_key, Account::default()),
                keyed_account_for_system_program(),
            ],
            &[
                Check::success(),
                Check::account(&data_key)
                    .data(&init_instruction_data)
                    .owner(&PROGRAM_ID)
                    .build(),
            ],
        );
    }
}
