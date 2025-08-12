#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult,
    program::invoke_signed, program_error::ProgramError, pubkey::Pubkey,
};
use solana_system_interface::instruction as system_instruction;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

pub static SEED_SEPARATOR: &[u8] = b"-";
pub static DEPOSIT_PDA_ACCOUNT_SEED: &[u8] = b"deposit";

#[derive(BorshSerialize, BorshDeserialize)]
pub struct WithdrawAllLamports {}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if WithdrawAllLamports::try_from_slice(instruction_data).is_err() {
        return Err(ProgramError::InvalidInstructionData);
    };

    let [payer, deposit_account, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.key != &solana_program::system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    if deposit_account.owner != &solana_program::system_program::ID {
        return Err(ProgramError::IllegalOwner);
    }

    // Verify the PDA matches seeds
    let (expected_deposit_pda, bump) = Pubkey::find_program_address(
        &[DEPOSIT_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key.as_ref()],
        program_id,
    );

    if deposit_account.key != &expected_deposit_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    let ix =
        system_instruction::transfer(deposit_account.key, payer.key, deposit_account.lamports());

    let signer_seeds: &[&[&[u8]]] = &[&[
        DEPOSIT_PDA_ACCOUNT_SEED,
        SEED_SEPARATOR,
        payer.key.as_ref(),
        &[bump],
    ]];

    invoke_signed(
        &ix,
        &[
            deposit_account.clone(),
            payer.clone(),
            system_program.clone(),
        ],
        signer_seeds,
    )?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::{WithdrawAllLamports, DEPOSIT_PDA_ACCOUNT_SEED, ID as PROGRAM_ID, SEED_SEPARATOR};

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_pubkey::Pubkey;
    use solana_sdk_ids::system_program;
    use solana_system_interface::instruction as system_instruction;

    #[test]
    fn withdraw_all_lamports_happy_path() {
        let mollusk = Mollusk::new(&PROGRAM_ID, env!("CARGO_CRATE_NAME"));

        let deposit_amount: u64 = 50_000_000;
        let payer_key = Pubkey::new_unique();
        let starting_payer_lamports: u64 = 100_000_000;
        let payer_account = Account::new(starting_payer_lamports, 0, &system_program::id());

        // Derive the deposit PDA for this payer
        let (deposit_pda_key, _bump) = Pubkey::find_program_address(
            &[DEPOSIT_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer_key.as_ref()],
            &PROGRAM_ID,
        );

        let deposit_funding_ix =
            system_instruction::transfer(&payer_key, &deposit_pda_key, deposit_amount);

        let withdraw_all_ix_data = borsh::to_vec(&WithdrawAllLamports {}).unwrap();

        let withdraw_all_ix = Instruction::new_with_bytes(
            PROGRAM_ID,
            &withdraw_all_ix_data,
            vec![
                AccountMeta::new(payer_key, true),
                AccountMeta::new(deposit_pda_key, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &deposit_funding_ix,
                    &[
                        Check::success(),
                        Check::account(&deposit_pda_key)
                            .lamports(deposit_amount)
                            .build(),
                        Check::account(&payer_key)
                            .lamports(starting_payer_lamports - deposit_amount)
                            .build(),
                    ],
                ),
                (
                    &withdraw_all_ix,
                    &[
                        Check::success(),
                        Check::account(&deposit_pda_key).lamports(0).build(),
                        Check::account(&payer_key)
                            .lamports(starting_payer_lamports)
                            .build(),
                    ],
                ),
            ],
            &[
                (payer_key, payer_account),
                (deposit_pda_key, Account::default()),
                keyed_account_for_system_program(),
            ],
        );
    }
}
