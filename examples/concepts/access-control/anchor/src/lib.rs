#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

static CONFIG_PDA_SEED: &[u8] = b"config";
static LAST_PRICE_PDA_SEED: &[u8] = b"last_price";

#[program]
pub mod access_control {
    use super::*;

    pub fn initialize_config(ctx: Context<InitializeConfig>, publisher: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.publisher = publisher;
        Ok(())
    }

    pub fn update_config(ctx: Context<UpdateConfig>, publisher: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.publisher = publisher;
        Ok(())
    }

    pub fn publish_price(ctx: Context<PublishPrice>, base: u64, quote: u64) -> Result<()> {
        let last_price = &mut ctx.accounts.last_price;
        last_price.base = base;
        last_price.quote = quote;
        last_price.timestamp = Clock::get()?.unix_timestamp;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(publisher: Pubkey)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_PDA_SEED],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(publisher: Pubkey)]
pub struct UpdateConfig<'info> {
    #[account(mut, has_one = authority, seeds = [CONFIG_PDA_SEED], bump)]
    pub config: Account<'info, Config>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(base: u64, quote: u64)]
pub struct PublishPrice<'info> {
    #[account(has_one = publisher, seeds = [CONFIG_PDA_SEED], bump)]
    pub config: Account<'info, Config>,
    #[account(
        init_if_needed,
        payer = publisher,
        space = 8 + Price::INIT_SPACE,
        seeds = [LAST_PRICE_PDA_SEED, config.key().as_ref()],
        bump
    )]
    pub last_price: Account<'info, Price>,
    #[account(mut)]
    pub publisher: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(InitSpace)]
#[account]
pub struct Config {
    pub authority: Pubkey,
    pub publisher: Pubkey,
}

#[derive(InitSpace)]
#[account]
pub struct Price {
    pub base: u64,
    pub quote: u64,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use crate::{CONFIG_PDA_SEED, LAST_PRICE_PDA_SEED};

    use super::{
        instruction::{InitializeConfig, PublishPrice, UpdateConfig},
        Config, Price, ID as PROGRAM_ID,
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

        let first_publisher_key = Pubkey::new_unique();
        let first_publisher_lamports = 100_000_000;
        let first_publisher_account =
            Account::new(first_publisher_lamports, 0, &system_program::id());

        let second_publisher_key = Pubkey::new_unique();
        let second_publisher_lamports = 100_000_000;
        let second_publisher_account =
            Account::new(second_publisher_lamports, 0, &system_program::id());

        let (config_pda_key, _) = Pubkey::find_program_address(&[CONFIG_PDA_SEED], &PROGRAM_ID);
        let config_account = Account::default();

        let (last_price_pda_key, _) = Pubkey::find_program_address(
            &[LAST_PRICE_PDA_SEED, config_pda_key.as_ref()],
            &PROGRAM_ID,
        );
        let last_price_account = Account::default();

        let initialize_instruction_data = InitializeConfig {
            publisher: first_publisher_key,
        }
        .data();

        let initialize_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &initialize_instruction_data,
            vec![
                AccountMeta::new(config_pda_key, false),
                AccountMeta::new(authority_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_config_data_post_init = Config {
            authority: authority_key,
            publisher: first_publisher_key,
        }
        .try_to_vec()
        .unwrap();

        let publish_price_instruction_data = PublishPrice {
            base: 1_000_000,
            quote: 1_000_000,
        }
        .data();

        let publish_price_by_first_publisher_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &publish_price_instruction_data,
            vec![
                AccountMeta::new(config_pda_key, false),
                AccountMeta::new(last_price_pda_key, false),
                AccountMeta::new(first_publisher_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_last_price_data_post_publish = Price {
            base: 1_000_000,
            quote: 1_000_000,
            timestamp: mollusk.sysvars.clock.unix_timestamp,
        }
        .try_to_vec()
        .unwrap();

        let update_config_instruction_data = UpdateConfig {
            publisher: second_publisher_key,
        }
        .data();

        let update_config_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &update_config_instruction_data,
            vec![
                AccountMeta::new(config_pda_key, false),
                AccountMeta::new(authority_key, true),
            ],
        );

        let expected_config_data_post_update = Config {
            authority: authority_key,
            publisher: second_publisher_key,
        }
        .try_to_vec()
        .unwrap();

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &initialize_instruction,
                    &[
                        Check::success(),
                        Check::account(&config_pda_key)
                            .data_slice(8, &expected_config_data_post_init)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &publish_price_by_first_publisher_instruction,
                    &[
                        Check::success(),
                        Check::account(&last_price_pda_key)
                            .data_slice(8, &expected_last_price_data_post_publish)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &update_config_instruction,
                    &[
                        Check::success(),
                        Check::account(&config_pda_key)
                            .data_slice(8, &expected_config_data_post_update)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &publish_price_by_first_publisher_instruction,
                    &[Check::err(ProgramError::Custom(
                        ErrorCode::ConstraintHasOne as u32,
                    ))],
                ),
            ],
            &[
                (authority_key, authority_account.clone()),
                (first_publisher_key, first_publisher_account.clone()),
                (second_publisher_key, second_publisher_account.clone()),
                (config_pda_key, config_account.clone()),
                (last_price_pda_key, last_price_account.clone()),
                keyed_account_for_system_program(),
            ],
        );
    }
}
