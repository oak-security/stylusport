#![allow(unexpected_cfgs)]

use ::borsh::BorshDeserialize;
use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        program::{get_return_data, invoke},
    },
};
use cpi_to_external_call_solana_adder::{Args as AdderArgs, Response, ID as ADDER_PROGRAM_ID};

pub static LAST_RESULT_ACCOUNT_SEED: &[u8] = b"last_result";

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct Args {
    pub a: u64,
    pub b: u64,
}

#[derive(InitSpace)]
#[account]
pub struct LastResultAccount {
    pub last_result: u128,
}

#[derive(Accounts)]
#[instruction(data: Args)]
pub struct Add<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + LastResultAccount::INIT_SPACE,
        seeds = [LAST_RESULT_ACCOUNT_SEED],
        bump,
    )]
    pub last_result: Account<'info, LastResultAccount>,
    pub system_program: Program<'info, System>,
    pub adder_program: UncheckedAccount<'info>,
}

#[program]
pub mod cpi {
    use super::*;

    pub fn add(ctx: Context<Add>, args: Args) -> Result<()> {
        if *ctx.accounts.adder_program.key != ADDER_PROGRAM_ID {
            return Err(ProgramError::InvalidAccountData.into());
        }

        let adder_instruction_data = ::borsh::to_vec(&AdderArgs {
            a: args.a,
            b: args.b,
        })
        .expect("infallible serialization");

        invoke(
            &Instruction {
                program_id: ADDER_PROGRAM_ID,
                accounts: vec![],
                data: adder_instruction_data,
            },
            &[ctx.accounts.adder_program.to_account_info()],
        )?;

        let (invoked_program, data) = get_return_data().expect("return data is some after invoke");

        assert_eq!(
            invoked_program, ADDER_PROGRAM_ID,
            "expected return data from {ADDER_PROGRAM_ID}, received from {invoked_program}"
        );

        let Response { result } = Response::try_from_slice(&data)?;

        ctx.accounts.last_result.last_result = result;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{
        instruction::Add, Args, LastResultAccount, ID as PROGRAM_ID, LAST_RESULT_ACCOUNT_SEED,
    };

    use anchor_lang::{
        prelude::AccountMeta, solana_program::instruction::Instruction, AnchorSerialize,
        InstructionData,
    };
    use mollusk_svm::{
        program::{create_program_account_loader_v3, keyed_account_for_system_program},
        result::Check,
        Mollusk,
    };
    use solana_account::Account;
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

        let instruction_data = Add {
            args: Args { a: 5, b: 10 },
        }
        .data();

        let payer_key = Pubkey::new_unique();
        let payer_lamports = 100_000_000;
        let payer_account = Account::new(payer_lamports, 0, &system_program::id());

        let (last_result_pda_account_key, _) =
            Pubkey::find_program_address(&[LAST_RESULT_ACCOUNT_SEED], &PROGRAM_ID);

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

        let expected_account_data = LastResultAccount { last_result: 15 }.try_to_vec().unwrap();

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
                    .data_slice(8, &expected_account_data)
                    .owner(&PROGRAM_ID)
                    .build(),
            ],
        );
    }
}
