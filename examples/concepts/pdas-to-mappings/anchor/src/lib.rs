#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub static SEED_SEPARATOR: &[u8] = b"-";
pub static PLAYER_PDA_ACCOUNT_SEED: &[u8] = b"player";

pub const STARTING_LIVES: u8 = 10;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[derive(InitSpace)]
#[account]
pub struct PlayerAccountState {
    pub lives: u8,
    pub bump: u8,
}

#[derive(Accounts)]
#[instruction()]
pub struct CreatePlayerAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + PlayerAccountState::INIT_SPACE,
        seeds = [PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key().as_ref()],
        bump,
    )]
    pub player_account: Account<'info, PlayerAccountState>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod data_storage {
    use super::*;

    pub fn create_player_account(ctx: Context<CreatePlayerAccount>) -> Result<()> {
        ctx.accounts.player_account.lives = STARTING_LIVES;
        ctx.accounts.player_account.bump = ctx.bumps.player_account;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{
        instruction::CreatePlayerAccount, PlayerAccountState, ID as PROGRAM_ID,
        PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, STARTING_LIVES,
    };

    use anchor_lang::{
        prelude::AccountMeta, solana_program::instruction::Instruction, AnchorSerialize,
        InstructionData,
    };
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let init_instruction_data = CreatePlayerAccount {}.data();

        let payer_key = Pubkey::new_unique();
        let payer_lamports = 100_000_000;
        let payer_account = Account::new(payer_lamports, 0, &system_program::id());

        let (player_pda_account_key, bump) = Pubkey::find_program_address(
            &[PLAYER_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer_key.as_ref()],
            &PROGRAM_ID,
        );

        let initialize_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &init_instruction_data,
            vec![
                AccountMeta::new(payer_key, true),
                AccountMeta::new(player_pda_account_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_account_data = PlayerAccountState {
            lives: STARTING_LIVES,
            bump,
        }
        .try_to_vec()
        .unwrap();

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
                    .data_slice(8, &expected_account_data)
                    .owner(&PROGRAM_ID)
                    .build(),
            ],
        );
    }
}
