#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[derive(InitSpace)]
#[account]
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
    #[max_len(200)]
    pub string: String,
    #[max_len(200)]
    pub bytes: Vec<u8>,
    pub address: Pubkey,
}

#[derive(Accounts)]
#[instruction(data: Data)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Data::INIT_SPACE
    )]
    pub data_account: Account<'info, Data>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod data_storage {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: Data) -> Result<()> {
        *ctx.accounts.data_account = data;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{instruction::Initialize, Data, ID as PROGRAM_ID};

    use anchor_lang::{
        prelude::AccountMeta, solana_program::instruction::Instruction, InstructionData,
    };
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
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

        let init_instruction_data = Initialize {
            data: initial_values,
        }
        .data();

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
                    .data_slice(8, &init_instruction_data[8..])
                    .owner(&PROGRAM_ID)
                    .build(),
            ],
        );
    }
}
