#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked,
    },
};

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

pub static MINT_PDA_SEED: &[u8] = b"mint";
pub static STAKE_PDA_SEED: &[u8] = b"stake";
pub const DECIMALS: u8 = 6;
pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000; // 1B tokens

#[program]
pub mod fungible_tokens {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.mint_supply_to.to_account_info(),
                    authority: ctx.accounts.mint.to_account_info(),
                },
                &[&[MINT_PDA_SEED, &[ctx.bumps.mint]]],
            ),
            TOTAL_SUPPLY,
        )?;

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.from_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.stake_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
            ),
            amount,
            DECIMALS,
        )?;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.stake_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.unstake_to_account.to_account_info(),
                    authority: ctx.accounts.stake_account.to_account_info(),
                },
                &[&[
                    STAKE_PDA_SEED,
                    ctx.accounts.signer.key.as_ref(),
                    &[ctx.bumps.stake_account],
                ]],
            ),
            amount,
            DECIMALS,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = signer,
        mint::decimals = DECIMALS,
        mint::authority = mint.key(),
        mint::freeze_authority = mint.key(),
        seeds = [MINT_PDA_SEED],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub mint_supply_to: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = mint,
        token::authority = stake_account,
        token::token_program = token_program,
        seeds = [b"stake", signer.key.as_ref()],
        bump
    )]
    pub stake_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub from_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(
        mut,
        seeds = [b"stake", signer.key.as_ref()],
        bump
    )]
    pub stake_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub unstake_to_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[cfg(test)]
mod tests {
    use super::{
        instruction::{Initialize, Stake},
        DECIMALS, ID as PROGRAM_ID, MINT_PDA_SEED, STAKE_PDA_SEED, TOTAL_SUPPLY,
    };

    use anchor_lang::{
        prelude::AccountMeta, solana_program::instruction::Instruction, InstructionData,
    };
    use anchor_spl::{associated_token, token_2022::spl_token_2022};
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program_pack::Pack;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        mollusk_svm_programs_token::token2022::add_program(&mut mollusk);
        mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);

        let signer_key = Pubkey::new_unique();
        let signer_lamports = 100_000_000;
        let signer_account = Account::new(signer_lamports, 0, &system_program::id());

        let (mint_pda_key, _) = Pubkey::find_program_address(&[MINT_PDA_SEED], &PROGRAM_ID);
        let mint_account = Account::default();

        let mint_supply_to_key = associated_token::get_associated_token_address_with_program_id(
            &signer_key,
            &mint_pda_key,
            &mollusk_svm_programs_token::token2022::ID,
        );
        let mint_supply_to_account = Account::default();

        let create_mint_instruction_data = Initialize {}.data();

        let create_mint_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &create_mint_instruction_data,
            vec![
                AccountMeta::new(mint_pda_key, false),
                AccountMeta::new(mint_supply_to_key, false),
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(mollusk_svm_programs_token::token2022::ID, false),
                AccountMeta::new_readonly(mollusk_svm_programs_token::associated_token::ID, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let mut expected_mint_account_data =
            vec![0u8; spl_token_2022::state::Mint::get_packed_len()];

        Pack::pack(
            spl_token_2022::state::Mint {
                mint_authority: Some(mint_pda_key).into(),
                supply: TOTAL_SUPPLY,
                decimals: DECIMALS,
                is_initialized: true,
                freeze_authority: Some(mint_pda_key).into(),
            },
            &mut expected_mint_account_data,
        )
        .unwrap();

        let mut expected_mint_supply_to_account_data =
            vec![0u8; spl_token_2022::state::Account::get_packed_len()];

        Pack::pack(
            spl_token_2022::state::Account {
                mint: mint_pda_key,
                owner: signer_key,
                amount: TOTAL_SUPPLY,
                ..Default::default()
            },
            &mut expected_mint_supply_to_account_data,
        )
        .unwrap();

        let (stake_pda_key, _) =
            Pubkey::find_program_address(&[STAKE_PDA_SEED, signer_key.as_ref()], &PROGRAM_ID);
        let stake_account = Account::default();

        let stake_instruction_data = Stake {
            amount: (TOTAL_SUPPLY * 3) / 4,
        }
        .data();

        let stake_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &stake_instruction_data,
            vec![
                AccountMeta::new(stake_pda_key, false),
                AccountMeta::new(mint_supply_to_key, false),
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(mint_pda_key, false),
                AccountMeta::new_readonly(mollusk_svm_programs_token::token2022::ID, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let mut expected_stake_account_data_post_stake =
            vec![0u8; spl_token_2022::state::Account::get_packed_len()];

        Pack::pack(
            spl_token_2022::state::Account {
                mint: mint_pda_key,
                owner: stake_pda_key,
                amount: (TOTAL_SUPPLY * 3) / 4,
                ..Default::default()
            },
            &mut expected_stake_account_data_post_stake,
        )
        .unwrap();

        let mut expected_mint_supply_to_account_data_post_stake =
            vec![0u8; spl_token_2022::state::Account::get_packed_len()];

        Pack::pack(
            spl_token_2022::state::Account {
                mint: mint_pda_key,
                owner: signer_key,
                amount: TOTAL_SUPPLY / 4,
                ..Default::default()
            },
            &mut expected_mint_supply_to_account_data_post_stake,
        )
        .unwrap();

        let unstake_instruction_data = Stake {
            amount: (TOTAL_SUPPLY * 3) / 4,
        }
        .data();

        let unstake_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &unstake_instruction_data,
            vec![
                AccountMeta::new(stake_pda_key, false),
                AccountMeta::new(mint_supply_to_key, false),
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(mint_pda_key, false),
                AccountMeta::new_readonly(mollusk_svm_programs_token::token2022::ID, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let mut expected_stake_account_data_post_unstake =
            vec![0u8; spl_token_2022::state::Account::get_packed_len()];

        Pack::pack(
            spl_token_2022::state::Account {
                mint: mint_pda_key,
                owner: stake_pda_key,
                amount: 0,
                ..Default::default()
            },
            &mut expected_stake_account_data_post_unstake,
        )
        .unwrap();

        let mut expected_mint_supply_to_account_data_post_unstake =
            vec![0u8; spl_token_2022::state::Account::get_packed_len()];

        Pack::pack(
            spl_token_2022::state::Account {
                mint: mint_pda_key,
                owner: signer_key,
                amount: TOTAL_SUPPLY,
                ..Default::default()
            },
            &mut expected_mint_supply_to_account_data_post_unstake,
        )
        .unwrap();

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &create_mint_instruction,
                    &[
                        Check::success(),
                        Check::account(&mint_pda_key)
                            .data(&expected_mint_account_data)
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                        Check::account(&mint_supply_to_key)
                            .data_slice(0, &expected_mint_supply_to_account_data[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                    ],
                ),
                (
                    &stake_instruction,
                    &[
                        Check::success(),
                        Check::account(&mint_supply_to_key)
                            .data_slice(
                                0,
                                &expected_mint_supply_to_account_data_post_stake[..32 + 32 + 8],
                            )
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                        Check::account(&stake_pda_key)
                            .data_slice(0, &expected_stake_account_data_post_stake[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                    ],
                ),
                (
                    &stake_instruction,
                    &[Check::err(
                        spl_token_2022::error::TokenError::InsufficientFunds.into(),
                    )],
                ),
                (
                    &unstake_instruction,
                    &[
                        Check::success(),
                        Check::account(&mint_supply_to_key)
                            .data_slice(
                                0,
                                &expected_mint_supply_to_account_data_post_unstake[..32 + 32 + 8],
                            )
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                        Check::account(&stake_pda_key)
                            .data_slice(0, &expected_stake_account_data_post_unstake[..32 + 32 + 8])
                            .owner(&mollusk_svm_programs_token::token2022::ID)
                            .build(),
                    ],
                ),
                (
                    &unstake_instruction,
                    &[Check::err(
                        spl_token_2022::error::TokenError::InsufficientFunds.into(),
                    )],
                ),
            ],
            &[
                (mint_pda_key, mint_account.clone()),
                (mint_supply_to_key, mint_supply_to_account.clone()),
                (stake_pda_key, stake_account.clone()),
                (signer_key, signer_account.clone()),
                mollusk_svm_programs_token::token2022::keyed_account(),
                mollusk_svm_programs_token::associated_token::keyed_account(),
                keyed_account_for_system_program(),
            ],
        );
    }
}
