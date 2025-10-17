#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

pub static SEED_SEPARATOR: &[u8] = b"-";
pub static DEPOSIT_PDA_ACCOUNT_SEED: &[u8] = b"deposit";

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[program]
pub mod native_token_handling {
    use super::*;

    pub fn withdraw_all_lamports(ctx: Context<WithdrawAllLamports>) -> Result<()> {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.deposit_account.to_account_info(),
                    to: ctx.accounts.payer.to_account_info(),
                },
            )
            .with_signer(&[&[
                DEPOSIT_PDA_ACCOUNT_SEED,
                SEED_SEPARATOR,
                ctx.accounts.payer.key.as_ref(),
                &[ctx.bumps.deposit_account],
            ]]),
            ctx.accounts.deposit_account.lamports(),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction()]
pub struct WithdrawAllLamports<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [DEPOSIT_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key().as_ref()],
        bump,
    )]
    pub deposit_account: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[cfg(test)]
mod test {
    use super::{
        instruction::WithdrawAllLamports, DEPOSIT_PDA_ACCOUNT_SEED, ID as PROGRAM_ID,
        SEED_SEPARATOR,
    };

    use anchor_lang::{prelude::*, solana_program::instruction::Instruction, InstructionData};
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;
    use solana_system_interface::instruction as system_instruction;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let deposit_amount = 50_000_000;

        let withdraw_all_instruction_data = WithdrawAllLamports {}.data();

        let payer_key = Pubkey::new_unique();
        let payer_lamports = 100_000_000;
        let payer_account = Account::new(payer_lamports, 0, &system_program::id());

        let (deposit_pda_account_key, _) = Pubkey::find_program_address(
            &[DEPOSIT_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer_key.as_ref()],
            &PROGRAM_ID,
        );

        let deposit_instruction =
            system_instruction::transfer(&payer_key, &deposit_pda_account_key, deposit_amount);

        let withdraw_all_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &withdraw_all_instruction_data,
            vec![
                AccountMeta::new(payer_key, true),
                AccountMeta::new(deposit_pda_account_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &deposit_instruction,
                    &[
                        Check::success(),
                        Check::account(&deposit_pda_account_key)
                            .lamports(deposit_amount)
                            .build(),
                        Check::account(&payer_key)
                            .lamports(payer_lamports - deposit_amount)
                            .build(),
                    ],
                ),
                (
                    &withdraw_all_instruction,
                    &[
                        Check::success(),
                        Check::account(&deposit_pda_account_key).lamports(0).build(),
                        Check::account(&payer_key).lamports(payer_lamports).build(),
                    ],
                ),
            ],
            &[
                (payer_key, payer_account),
                (deposit_pda_account_key, Account::default()),
                keyed_account_for_system_program(),
            ],
        );
    }
}
