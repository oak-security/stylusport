#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, clock::Clock, declare_id, entrypoint, entrypoint::ProgramResult,
    program::invoke_signed, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
    sysvar::Sysvar,
};
use solana_system_interface::instruction as system_instruction;

declare_id!("JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG");

static CONFIG_PDA_SEED: &[u8] = b"config";
static LAST_PRICE_PDA_SEED: &[u8] = b"last_price";

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Config {
    pub authority: Pubkey,
    pub publisher: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Price {
    pub base: u64,
    pub quote: u64,
    pub timestamp: i64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    InitializeConfig { publisher: Pubkey },
    UpdateConfig { publisher: Pubkey },
    PublishPrice { base: u64, quote: u64 },
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let instruction = Instruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        Instruction::InitializeConfig { publisher } => {
            process_initialize_config(program_id, accounts, publisher)
        }
        Instruction::UpdateConfig { publisher } => {
            process_update_config(program_id, accounts, publisher)
        }
        Instruction::PublishPrice { base, quote } => {
            process_publish_price(program_id, accounts, base, quote)
        }
    }
}

fn process_initialize_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    publisher: Pubkey,
) -> ProgramResult {
    let [config_account, authority_account, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (config_pda_key, config_bump) =
        Pubkey::find_program_address(&[CONFIG_PDA_SEED], program_id);

    if config_pda_key != *config_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if !config_account.data_is_empty()
        || config_account.lamports() > 0
        || *config_account.owner == *program_id
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let space_required = borsh::max_serialized_size::<Config>().expect("infallible");
    let lamports_required = Rent::get()?.minimum_balance(space_required);

    invoke_signed(
        &system_instruction::create_account(
            authority_account.key,
            config_account.key,
            lamports_required,
            space_required as u64,
            program_id,
        ),
        &[
            authority_account.clone(),
            config_account.clone(),
            system_program.clone(),
        ],
        &[&[CONFIG_PDA_SEED, &[config_bump]]],
    )?;

    let mut account_data = config_account.try_borrow_mut_data()?;

    Config {
        authority: *authority_account.key,
        publisher,
    }
    .serialize(&mut account_data.as_mut())?;

    Ok(())
}

fn process_update_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    publisher: Pubkey,
) -> ProgramResult {
    let [config_account, authority_account] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !authority_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (config_pda_key, _) = Pubkey::find_program_address(&[CONFIG_PDA_SEED], program_id);
    if config_pda_key != *config_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if *config_account.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut config_data = Config::try_from_slice(&config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if config_data.authority != *authority_account.key {
        return Err(ProgramError::MissingRequiredSignature);
    }

    config_data.publisher = publisher;

    let mut account_data = config_account.try_borrow_mut_data()?;
    config_data.serialize(&mut account_data.as_mut())?;

    Ok(())
}

fn process_publish_price(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    base: u64,
    quote: u64,
) -> ProgramResult {
    let [config_account, last_price_account, publisher_account, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !publisher_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (config_pda_key, _) = Pubkey::find_program_address(&[CONFIG_PDA_SEED], program_id);

    if config_pda_key != *config_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if *config_account.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let config_data = Config::try_from_slice(&config_account.data.borrow())
        .map_err(|_| ProgramError::InvalidAccountData)?;

    if config_data.publisher != *publisher_account.key {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (last_price_pda_key, last_price_bump) = Pubkey::find_program_address(
        &[LAST_PRICE_PDA_SEED, config_account.key.as_ref()],
        program_id,
    );

    if last_price_pda_key != *last_price_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let needs_init = last_price_account.data_is_empty()
        || last_price_account.lamports() == 0
        || *last_price_account.owner != *program_id;

    if needs_init {
        let space_required = borsh::max_serialized_size::<Price>().expect("infallible");
        let lamports_required = Rent::get()?.minimum_balance(space_required);

        invoke_signed(
            &system_instruction::create_account(
                publisher_account.key,
                last_price_account.key,
                lamports_required,
                space_required as u64,
                program_id,
            ),
            &[
                publisher_account.clone(),
                last_price_account.clone(),
                system_program.clone(),
            ],
            &[&[
                LAST_PRICE_PDA_SEED,
                config_account.key.as_ref(),
                &[last_price_bump],
            ]],
        )?;
    }

    // Update price data
    let price_data = Price {
        base,
        quote,
        timestamp: Clock::get()?.unix_timestamp,
    };

    let mut account_data = last_price_account.try_borrow_mut_data()?;
    price_data.serialize(&mut account_data.as_mut())?;

    Ok(())
}

entrypoint!(process_instruction);

#[cfg(test)]
mod test {
    use super::{
        Config, Instruction, Price, CONFIG_PDA_SEED, ID as PROGRAM_ID, LAST_PRICE_PDA_SEED,
    };

    use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
    use solana_account::Account;
    use solana_program::instruction::Instruction as SolanaInstruction;
    use solana_program::{instruction::AccountMeta, program_error::ProgramError};
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

        // Initialize config instruction
        let initialize_instruction_data = borsh::to_vec(&Instruction::InitializeConfig {
            publisher: first_publisher_key,
        })
        .unwrap();

        let initialize_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &initialize_instruction_data,
            vec![
                AccountMeta::new(config_pda_key, false),
                AccountMeta::new(authority_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_config_data_post_init = borsh::to_vec(&Config {
            authority: authority_key,
            publisher: first_publisher_key,
        })
        .unwrap();

        // Publish price instruction
        let publish_price_instruction_data = borsh::to_vec(&Instruction::PublishPrice {
            base: 1_000_000,
            quote: 1_000_000,
        })
        .unwrap();

        let publish_price_by_first_publisher_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &publish_price_instruction_data,
            vec![
                AccountMeta::new_readonly(config_pda_key, false),
                AccountMeta::new(last_price_pda_key, false),
                AccountMeta::new(first_publisher_key, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        let expected_last_price_data_post_publish = borsh::to_vec(&Price {
            base: 1_000_000,
            quote: 1_000_000,
            timestamp: mollusk.sysvars.clock.unix_timestamp,
        })
        .unwrap();

        // Update config instruction
        let update_config_instruction_data = borsh::to_vec(&Instruction::UpdateConfig {
            publisher: second_publisher_key,
        })
        .unwrap();

        let update_config_instruction = SolanaInstruction::new_with_bytes(
            PROGRAM_ID,
            &update_config_instruction_data,
            vec![
                AccountMeta::new(config_pda_key, false),
                AccountMeta::new(authority_key, true),
            ],
        );

        let expected_config_data_post_update = borsh::to_vec(&Config {
            authority: authority_key,
            publisher: second_publisher_key,
        })
        .unwrap();

        mollusk.process_and_validate_instruction_chain(
            &[
                (
                    &initialize_instruction,
                    &[
                        Check::success(),
                        Check::account(&config_pda_key)
                            .data(&expected_config_data_post_init)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &publish_price_by_first_publisher_instruction,
                    &[
                        Check::success(),
                        Check::account(&last_price_pda_key)
                            .data(&expected_last_price_data_post_publish)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &update_config_instruction,
                    &[
                        Check::success(),
                        Check::account(&config_pda_key)
                            .data(&expected_config_data_post_update)
                            .owner(&PROGRAM_ID)
                            .build(),
                    ],
                ),
                (
                    &publish_price_by_first_publisher_instruction,
                    &[Check::err(ProgramError::MissingRequiredSignature)], // ConstraintHasOne equivalent
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
