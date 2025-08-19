#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

static STATE_PDA_SEED: &[u8] = b"state";

#[program]
pub mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, value: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value = value;
        counter.authority = ctx.accounts.authority.key();
        Ok(())
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value += 1;
        Ok(())
    }

    pub fn set_value(ctx: Context<SetValue>, new_value: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.value = new_value;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Counter::INIT_SPACE,
        seeds = [STATE_PDA_SEED],
        bump
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut, seeds = [STATE_PDA_SEED], bump)]
    pub counter: Account<'info, Counter>,
}

#[derive(Accounts)]
pub struct SetValue<'info> {
    #[account(mut, has_one = authority, seeds = [STATE_PDA_SEED], bump)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[derive(InitSpace)]
#[account]
pub struct Counter {
    pub value: u64,
    pub authority: Pubkey,
}

#[cfg(test)]
mod tests {
    use crate::STATE_PDA_SEED;

    use super::{
        instruction::{Increment, Initialize, SetValue},
        Counter, ID as PROGRAM_ID,
    };

    use anchor_lang::{
        error::ErrorCode,
        prelude::{AccountMeta, ProgramError},
        solana_program::instruction::Instruction,
        AnchorSerialize, InstructionData,
    };
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let authority_key = Pubkey::new_unique();
        let authority_lamports = 100_000_000;
        let authority_account = Account::new(authority_lamports, 0, &system_program::id());

        let non_authority_key = Pubkey::new_unique();
        let non_authority_lamports = 100_000_000;
        let non_authority_account = Account::new(non_authority_lamports, 0, &system_program::id());

        let (counter_state_key, _) = Pubkey::find_program_address(&[STATE_PDA_SEED], &PROGRAM_ID);

        let counter_account = Account::default();

        let initial_value = 42u64;
        let initialize_instruction_data = Initialize {
            value: initial_value,
        }
        .data();

        let initialize_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &initialize_instruction_data,
            vec![
                AccountMeta::new(counter_state_key, false),
                AccountMeta::new(authority_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_counter_data_post_init = Counter {
            value: initial_value,
            authority: authority_key,
        }
        .try_to_vec()
        .unwrap();

        let increment_instruction_data = Increment {}.data();

        let increment_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &increment_instruction_data,
            vec![AccountMeta::new(counter_state_key, false)],
        );

        let expected_counter_data_post_increment = Counter {
            value: initial_value + 1,
            authority: authority_key,
        }
        .try_to_vec()
        .unwrap();

        let new_value = 100u64;
        let set_value_instruction_data = SetValue { new_value }.data();
        let set_value_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &set_value_instruction_data,
            vec![
                AccountMeta::new(counter_state_key, false),
                AccountMeta::new(authority_key, true),
            ],
        );

        let expected_counter_data_post_set_value = Counter {
            value: new_value,
            authority: authority_key,
        }
        .try_to_vec()
        .unwrap();

        let set_value_wrong_authority_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &set_value_instruction_data,
            vec![
                AccountMeta::new(counter_state_key, false),
                AccountMeta::new(non_authority_key, true),
            ],
        );

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &initialize_instruction,
                    &[
                        Check::success(),
                        Check::account(&counter_state_key)
                            .data_slice(8, &expected_counter_data_post_init)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &increment_instruction,
                    &[
                        Check::success(),
                        Check::account(&counter_state_key)
                            .data_slice(8, &expected_counter_data_post_increment)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &set_value_instruction,
                    &[
                        Check::success(),
                        Check::account(&counter_state_key)
                            .data_slice(8, &expected_counter_data_post_set_value)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &set_value_wrong_authority_instruction,
                    &[Check::err(ProgramError::Custom(
                        ErrorCode::ConstraintHasOne as u32,
                    ))],
                ),
            ],
            &[
                (counter_state_key, counter_account.clone()),
                (authority_key, authority_account.clone()),
                (non_authority_key, non_authority_account.clone()),
                keyed_account_for_system_program(),
            ],
        );
    }
}
