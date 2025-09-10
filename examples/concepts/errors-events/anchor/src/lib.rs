#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

#[event]
pub struct OwnerChanged {
    previous_owner: Pubkey,
    current_owner: Pubkey,
}

#[program]
pub mod errors_events {
    use super::*;

    pub fn invalid_amount(_ctx: Context<InvalidAmount>) -> Result<()> {
        Err(ErrorCode::InvalidAmount.into())
    }

    pub fn unauthorized(_ctx: Context<Unauthorized>) -> Result<()> {
        Ok(())
    }

    pub fn emit_event(ctx: Context<EmitEvent>) -> Result<()> {
        emit!(OwnerChanged {
            previous_owner: *ctx.accounts.signer.key,
            current_owner: ID
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InvalidAmount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unauthorized<'info> {
    #[account(mut, constraint = false @ ErrorCode::Unauthorized)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct EmitEvent<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid amount: amount must be greater than 0")]
    InvalidAmount,
    #[msg("Unauthorized")]
    Unauthorized,
}

#[cfg(test)]
mod test {
    use super::{instruction::EmitEvent, ID as PROGRAM_ID};

    use anchor_lang::InstructionData;
    use anchor_lang::{prelude::AccountMeta, solana_program::instruction::Instruction};
    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;

    #[test]
    fn test_program() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let signer_key = Pubkey::new_unique();
        let signer_lamports = 100_000_000;
        let signer_account = Account::new(signer_lamports, 0, &system_program::id());

        let emit_event_instruction_data = EmitEvent {}.data();

        let emit_event_instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &emit_event_instruction_data,
            vec![
                AccountMeta::new(signer_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        mollusk.process_and_validate_instruction_chain(
            &[(&emit_event_instruction, &[Check::success()])],
            &[
                (signer_key, signer_account.clone()),
                keyed_account_for_system_program(),
            ],
        );
    }
}
