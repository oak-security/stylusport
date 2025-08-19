#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult,
    program::invoke_signed, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
    sysvar::Sysvar,
};
use solana_system_interface::instruction as system_instruction;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

static STATE_PDA_SEED: &[u8] = b"state";

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct CounterState {
    pub value: u64,
    pub authority: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    Initialize { value: u64 },
    Increment,
    SetValue { new_value: u64 },
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let Ok(ix) = Instruction::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };

    match ix {
        Instruction::Initialize { value } => process_initialize(accounts, value),
        Instruction::Increment => process_increment(accounts),
        Instruction::SetValue { new_value } => process_set_value(accounts, new_value),
    }
}

fn process_initialize(accounts: &[AccountInfo], initial_value: u64) -> ProgramResult {
    let [counter_state_pda, authority, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (counter_state_key, bump) = Pubkey::find_program_address(&[STATE_PDA_SEED], &ID);

    if &counter_state_key != counter_state_pda.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let initial_state = CounterState {
        value: initial_value,
        authority: *authority.key,
    };

    let space_required = borsh::max_serialized_size::<CounterState>().expect("infallible");

    let lamports_required = Rent::get()?.minimum_balance(space_required);

    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            counter_state_pda.key,
            lamports_required,
            space_required as u64,
            &ID,
        ),
        &[
            authority.clone(),
            counter_state_pda.clone(),
            system_program.clone(),
        ],
        &[&[STATE_PDA_SEED, &[bump]]],
    )?;

    let mut counter_state_buffer = counter_state_pda.try_borrow_mut_data()?;

    initial_state.serialize(&mut *counter_state_buffer)?;

    Ok(())
}

fn process_increment(accounts: &[AccountInfo]) -> ProgramResult {
    let [counter_state_pda] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    let (counter_state_key, _) = Pubkey::find_program_address(&[STATE_PDA_SEED], &ID);

    if &counter_state_key != counter_state_pda.key || counter_state_pda.owner != &ID {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut counter_state_buffer = counter_state_pda.try_borrow_mut_data()?;

    let mut counter_state = CounterState::deserialize(&mut counter_state_buffer.as_ref())?;

    counter_state.value += 1;

    counter_state.serialize(&mut *counter_state_buffer)?;

    Ok(())
}

fn process_set_value(accounts: &[AccountInfo], new_value: u64) -> ProgramResult {
    let [counter_state_pda, authority] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (counter_state_key, _) = Pubkey::find_program_address(&[STATE_PDA_SEED], &ID);

    if &counter_state_key != counter_state_pda.key || counter_state_pda.owner != &ID {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut counter_state_buffer = counter_state_pda.try_borrow_mut_data()?;

    let mut counter_state = CounterState::deserialize(&mut counter_state_buffer.as_ref())?;

    if &counter_state.authority != authority.key {
        return Err(ProgramError::MissingRequiredSignature);
    }

    counter_state.value = new_value;

    counter_state.serialize(&mut *counter_state_buffer)?;

    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use crate::{CounterState, Instruction, ID as PROGRAM_ID, STATE_PDA_SEED};

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::{
        instruction::{AccountMeta, Instruction as SolanaInstruction},
        program_error::ProgramError,
    };
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
        let initialize_instruction_data = borsh::to_vec(&Instruction::Initialize {
            value: initial_value,
        })
        .unwrap();

        let initialize_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &initialize_instruction_data,
            vec![
                AccountMeta::new(counter_state_key, false),
                AccountMeta::new(authority_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_counter_data_post_init = borsh::to_vec(&CounterState {
            value: initial_value,
            authority: authority_key,
        })
        .unwrap();

        let increment_instruction_data = borsh::to_vec(&Instruction::Increment).unwrap();

        let increment_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &increment_instruction_data,
            vec![AccountMeta::new(counter_state_key, false)],
        );

        let expected_counter_data_post_increment = borsh::to_vec(&CounterState {
            value: initial_value + 1,
            authority: authority_key,
        })
        .unwrap();

        let new_value = 100u64;
        let set_value_instruction_data =
            borsh::to_vec(&Instruction::SetValue { new_value }).unwrap();
        let set_value_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &set_value_instruction_data,
            vec![
                AccountMeta::new(counter_state_key, false),
                AccountMeta::new(authority_key, true),
            ],
        );

        let expected_counter_data_post_set_value = borsh::to_vec(&CounterState {
            value: new_value,
            authority: authority_key,
        })
        .unwrap();

        let set_value_wrong_authority_instruction = SolanaInstruction::new_with_bytes(
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
                            .data(&expected_counter_data_post_init)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &increment_instruction,
                    &[
                        Check::success(),
                        Check::account(&counter_state_key)
                            .data(&expected_counter_data_post_increment)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &set_value_instruction,
                    &[
                        Check::success(),
                        Check::account(&counter_state_key)
                            .data(&expected_counter_data_post_set_value)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &set_value_wrong_authority_instruction,
                    &[Check::err(ProgramError::MissingRequiredSignature)],
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
