<PART 1 - RAW DOCS EXAMPLE>
>>> solana-com/content/docs/en/programs/rust/program-structure.mdx
---
title: Program Structure
description:
  Learn how to structure Solana programs in Rust, including entrypoints, state
  management, instruction handling, and testing.
h1: Rust Program Structure
---

Solana programs written in Rust have minimal structural requirements, allowing
for flexibility in how code is organized. The only requirement is that a program
must have an `entrypoint`, which defines where the execution of a program
begins.

## Program Structure

While there are no strict rules for file structure, Solana programs typically
follow a common pattern:

- `entrypoint.rs`: Defines the entrypoint that routes incoming instructions.
- `state.rs`: Define program-specific state (account data).
- `instructions.rs`: Defines the instructions that the program can execute.
- `processor.rs`: Defines the instruction handlers (functions) that implement
  the business logic for each instruction.
- `error.rs`: Defines custom errors that the program can return.

You can find examples in the
[Solana Program Library](https://github.com/solana-program/token/tree/main/program/src).

## Example Program

To demonstrate how to build a native Rust program with multiple instructions,
we'll walk through a simple counter program that implements two instructions:

1. `InitializeCounter`: Creates and initializes a new account with an initial
   value.
2. `IncrementCounter`: Increments the value stored in an existing account.

For simplicity, the program will be implemented in a single `lib.rs` file,
though in practice you may want to split larger programs into multiple files.

<Accordions>
<Accordion title="Full Program Code">

```rs title="lib.rs"
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

// Program entrypoint
entrypoint!(process_instruction);

// Function to route instructions to the correct handler
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Unpack instruction data
    let instruction = CounterInstruction::unpack(instruction_data)?;

    // Match instruction type
    match instruction {
        CounterInstruction::InitializeCounter { initial_value } => {
            process_initialize_counter(program_id, accounts, initial_value)?
        }
        CounterInstruction::IncrementCounter => process_increment_counter(program_id, accounts)?,
    };
    Ok(())
}

// Instructions that our program can execute
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    InitializeCounter { initial_value: u64 }, // variant 0
    IncrementCounter,                         // variant 1
}

impl CounterInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Get the instruction variant from the first byte
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        // Match instruction type and parse the remaining bytes based on the variant
        match variant {
            0 => {
                // For InitializeCounter, parse a u64 from the remaining bytes
                let initial_value = u64::from_le_bytes(
                    rest.try_into()
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                );
                Ok(Self::InitializeCounter { initial_value })
            }
            1 => Ok(Self::IncrementCounter), // No additional data needed
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

// Initialize a new counter account
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Size of our counter account
    let account_space = 8; // Size in bytes to store a u64

    // Calculate minimum balance for rent exemption
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    // Create the counter account
    invoke(
        &system_instruction::create_account(
            payer_account.key,    // Account paying for the new account
            counter_account.key,  // Account to be created
            required_lamports,    // Amount of lamports to transfer to the new account
            account_space as u64, // Size in bytes to allocate for the data field
            program_id,           // Set program owner to our program
        ),
        &[
            payer_account.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Create a new CounterAccount struct with the initial value
    let counter_data = CounterAccount {
        count: initial_value,
    };

    // Get a mutable reference to the counter account's data
    let mut account_data = &mut counter_account.data.borrow_mut()[..];

    // Serialize the CounterAccount struct into the account's data
    counter_data.serialize(&mut account_data)?;

    msg!("Counter initialized with value: {}", initial_value);

    Ok(())
}

// Update an existing counter's value
fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Mutable borrow the account data
    let mut data = counter_account.data.borrow_mut();

    // Deserialize the account data into our CounterAccount struct
    let mut counter_data: CounterAccount = CounterAccount::try_from_slice(&data)?;

    // Increment the counter value
    counter_data.count = counter_data
        .count
        .checked_add(1)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Serialize the updated counter data back into the account
    counter_data.serialize(&mut &mut data[..])?;

    msg!("Counter incremented to: {}", counter_data.count);
    Ok(())
}

// Struct representing our counter account's data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CounterAccount {
    count: u64,
}

#[cfg(test)]
mod test {
    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_counter_program() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "counter_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Create a new keypair to use as the address for our counter account
        let counter_keypair = Keypair::new();
        let initial_value: u64 = 42;

        // Step 1: Initialize the counter
        println!("Testing counter initialization...");

        // Create initialization instruction
        let mut init_instruction_data = vec![0]; // 0 = initialize instruction
        init_instruction_data.extend_from_slice(&initial_value.to_le_bytes());

        let initialize_instruction = Instruction::new_with_bytes(
            program_id,
            &init_instruction_data,
            vec![
                AccountMeta::new(counter_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Send transaction with initialize instruction
        let mut transaction =
            Transaction::new_with_payer(&[initialize_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 42);
            println!(
                "✅ Counter initialized successfully with value: {}",
                counter.count
            );
        }

        // Step 2: Increment the counter
        println!("Testing counter increment...");

        // Create increment instruction
        let increment_instruction = Instruction::new_with_bytes(
            program_id,
            &[1], // 1 = increment instruction
            vec![AccountMeta::new(counter_keypair.pubkey(), true)],
        );

        // Send transaction with increment instruction
        let mut transaction =
            Transaction::new_with_payer(&[increment_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 43);
            println!("✅ Counter incremented successfully to: {}", counter.count);
        }
    }
}
```

```toml title="Cargo.toml"
[package]
name = "counter_program"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
borsh = "1.5.1"
solana-program = "1.18.26"

[dev-dependencies]
solana-program-test = "1.18.26"
solana-sdk = "1.18.26"
tokio = "1.41.0"
```

</Accordion>
</Accordions>

<Steps>
<Step>


### Create a new Program

First, create a new Rust project using the standard `cargo init` command with
the `--lib` flag.

```shell title="Terminal"
cargo init counter_program --lib
```

Navigate to the project directory. You should see the default `src/lib.rs` and
`Cargo.toml` files

```shell title="Terminal"
cd counter_program
```

Next, add the `solana-program` dependency. This is the minimum dependency
required to build a Solana program.

```shell title="Terminal"
cargo add solana-program@1.18.26
```

Next, add the following snippet to `Cargo.toml`. If you don't include this
config, the `target/deploy` directory will not be generated when you build the
program.

```toml title="Cargo.toml"
[lib]
crate-type = ["cdylib", "lib"]
```

Your `Cargo.toml` file should look like the following:

```toml title="Cargo.toml"
[package]
name = "counter_program"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
solana-program = "1.18.26"
```

</Step>
<Step>

### Program Entrypoint

A Solana program entrypoint is the function that gets called when a program is
invoked. The entrypoint has the following raw definition and developers are free
to create their own implementation of the entrypoint function.

For simplicity, use the
[`entrypoint!`](https://github.com/solana-labs/solana/blob/v2.0/sdk/program/src/entrypoint.rs#L124-L140)
macro from the `solana_program` crate to define the entrypoint in your program.

```rs
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64;
```

Replace the default code in `lib.rs` with the following code. This snippet:

1. Imports the required dependencies from `solana_program`
2. Defines the program entrypoint using the `entrypoint!` macro
3. Implements the `process_instruction` function that will route instructions to
   the appropriate handler functions

```rs title="lib.rs" {13} /process_instruction/
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Your program logic
    Ok(())
}
```

The `entrypoint!` macro requires a function with the the following
[type signature](https://github.com/solana-labs/solana/blob/v2.0/sdk/program/src/entrypoint.rs#L28-L29)
as an argument:

```rs
pub type ProcessInstruction =
    fn(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult;
```

When a Solana program is invoked, the entrypoint
[deserializes](https://github.com/solana-labs/solana/blob/v2.0/sdk/program/src/entrypoint.rs#L277)
the
[input data](https://github.com/solana-labs/solana/blob/v2.0/sdk/program/src/entrypoint.rs#L129-L131)
(provided as bytes) into three values and passes them to the
[`process_instruction`](https://github.com/solana-labs/solana/blob/v2.0/sdk/program/src/entrypoint.rs#L132)
function:

- `program_id`: The public key of the program being invoked (current program)
- `accounts`: The `AccountInfo` for accounts required by the instruction being
  invoked
- `instruction_data`: Additional data passed to the program which specifies the
  instruction to execute and its required arguments

These three parameters directly correspond to the data that clients must provide
when building an instruction to invoke a program.

</Step>
<Step>

### Define Program State

When building a Solana program, you'll typically start by defining your
program's state - the data that will be stored in accounts created and owned by
your program.

Program state is defined using Rust structs that represent the data layout of
your program's accounts. You can define multiple structs to represent different
types of accounts for your program.

When working with accounts, you need a way to convert your program's data types
to and from the raw bytes stored in an account's data field:

- Serialization: Converting your data types into bytes to store in an account's
  data field
- Deserialization: Converting the bytes stored in an account back into your data
  types

While you can use any serialization format for Solana program development,
[Borsh](https://borsh.io/) is commonly used. To use Borsh in your Solana
program:

1. Add the `borsh` crate as a dependency to your `Cargo.toml`:

```shell title="Terminal"
cargo add borsh
```

2. Import the Borsh traits and use the derive macro to implement the traits for
   your structs:

```rust
use borsh::{BorshSerialize, BorshDeserialize};

// Define struct representing our counter account's data
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CounterAccount {
    count: u64,
}
```

Add the `CounterAccount` struct to `lib.rs` to define the program state. This
struct will be used in both the initialization and increment instructions.

```rs title="lib.rs" {12} {25-29}
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use borsh::{BorshSerialize, BorshDeserialize};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Your program logic
    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CounterAccount {
    count: u64,
}
```

</Step>
<Step>

### Define Instructions

Instructions refer to the different operations that your Solana program can
perform. Think of them as public APIs for your program - they define what
actions users can take when interacting with your program.

Instructions are typically defined using a Rust enum where:

- Each enum variant represents a different instruction
- The variant's payload represents the instruction's parameters

Note that Rust enum variants are implicitly numbered starting from 0.

Below is an example of an enum defining two instructions:

```rust
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    InitializeCounter { initial_value: u64 }, // variant 0
    IncrementCounter,                         // variant 1
}
```

When a client invokes your program, they must provide instruction data (as a
buffer of bytes) where:

- The first byte identifies which instruction variant to execute (0, 1, etc.)
- The remaining bytes contain the serialized instruction parameters (if
  required)

To convert the instruction data (bytes) into a variant of the enum, it is common
to implement a helper method. This method:

1. Splits the first byte to get the instruction variant
2. Matches on the variant and parses any additional parameters from the
   remaining bytes
3. Returns the corresponding enum variant

For example, the `unpack` method for the `CounterInstruction` enum:

```rust
impl CounterInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Get the instruction variant from the first byte
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        // Match instruction type and parse the remaining bytes based on the variant
        match variant {
            0 => {
                // For InitializeCounter, parse a u64 from the remaining bytes
                let initial_value = u64::from_le_bytes(
                    rest.try_into()
                        .map_err(|_| ProgramError::InvalidInstructionData)?
                );
                Ok(Self::InitializeCounter { initial_value })
            }
            1 => Ok(Self::IncrementCounter), // No additional data needed
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
```

Add the following code to `lib.rs` to define the instructions for the counter
program.

```rs title="lib.rs" {18-46}
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Your program logic
    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterInstruction {
    InitializeCounter { initial_value: u64 }, // variant 0
    IncrementCounter,                         // variant 1
}

impl CounterInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        // Get the instruction variant from the first byte
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        // Match instruction type and parse the remaining bytes based on the variant
        match variant {
            0 => {
                // For InitializeCounter, parse a u64 from the remaining bytes
                let initial_value = u64::from_le_bytes(
                    rest.try_into()
                        .map_err(|_| ProgramError::InvalidInstructionData)?,
                );
                Ok(Self::InitializeCounter { initial_value })
            }
            1 => Ok(Self::IncrementCounter), // No additional data needed
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
```

</Step>
<Step>

### Instruction Handlers

Instruction handlers refer to the functions that contain the business logic for
each instruction. It's common to name handler functions as
`process_<instruction_name>`, but you're free to choose any naming convention.

Add the following code to `lib.rs`. This code uses the `CounterInstruction` enum
and `unpack` method defined in the previous step to route incoming instructions
to the appropriate handler functions:

```rs title="lib.rs" {8-17} {20-32} /process_initialize_counter/1 /process_increment_counter/1
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Unpack instruction data
    let instruction = CounterInstruction::unpack(instruction_data)?;

    // Match instruction type
    match instruction {
        CounterInstruction::InitializeCounter { initial_value } => {
            process_initialize_counter(program_id, accounts, initial_value)?
        }
        CounterInstruction::IncrementCounter => process_increment_counter(program_id, accounts)?,
    };
}

fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    // Implementation details...
    Ok(())
}

fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    // Implementation details...
    Ok(())
}
```

Next, add the implementation of the `process_initialize_counter` function. This
instruction handler:

1. Creates and allocates space for a new account to store the counter data
2. Initializing the account data with `initial_value` passed to the instruction

<Accordions>
<Accordion title="Explanation">

The `process_initialize_counter` function requires three accounts:

1. The counter account that will be created and initialized
2. The payer account that will fund the new account creation
3. The System Program that we invoke to create the new account

To define the accounts required by the instruction, we create an iterator over
the `accounts` slice and use the `next_account_info` function to get each
account. The number of accounts you define are the accounts required by the
instruction.

The order of accounts is important - when building the instruction on the client
side, accounts must be provided in the same order as it is defined in the
program for the instruction to execute successfully.

While the variable names for the accounts have no effect on the program's
functionality, using descriptive names is recommended.

```rs title="lib.rs" {6-10}
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    Ok(())
}
```

Before creating an account, we need to:

1. Specify the space (in bytes) to allocate to the account's data field. Since
   we're storing a u64 value (`count`), we need 8 bytes.

2. Calculate the minimum "rent" balance required. On Solana, accounts must
   maintain a minimum balance of lamports (rent) based on amount of data stored
   on the account.

```rs title="lib.rs" {12-17}
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Size of our counter account
    let account_space = 8; // Size in bytes to store a u64

    // Calculate minimum balance for rent exemption
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    Ok(())
}
```

Once the space is defined and rent is calculated, create the account by invoking
the System Program's `create_account` instruction.

On Solana, new accounts can only be created by the System Program. When creating
an account, we specify the amount of bytes to allocate and the program owner of
the new account. The System Program:

1. Creates the new account
2. Allocates the specified space for the account's data field
3. Transfers ownership to the specified program

This ownership transfer is important because only the program owner of an
account can modify an account's data. In this case, we set our program as the
owner, which will allow us to modify the account's data to store the counter
value.

To invoke the System Program from our program's instruction, we make a Cross
Program Invocation (CPI) via the `invoke` function. A CPI allows one program to
call instructions on other programs - in this case, the System Program's
`create_account` instruction.

```rs title="lib.rs" {19-33}
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Size of our counter account
    let account_space = 8; // Size in bytes to store a u64

    // Calculate minimum balance for rent exemption
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    // Create the counter account
    invoke(
        &system_instruction::create_account(
            payer_account.key,    // Account paying for the new account
            counter_account.key,  // Account to be created
            required_lamports,    // Amount of lamports to transfer to the new account
            account_space as u64, // Size in bytes to allocate for the data field
            program_id,           // Set program owner to our program
        ),
        &[
            payer_account.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
    )?;

    Ok(())
}
```

Once the account is created, we initialize the account data by:

1. Creating a new `CounterAccount` struct with the `initial_value` provided to
   the instruction.
2. Getting a mutable reference to the new account's data field.
3. Serializing the `CounterAccount` struct into the account's data field,
   effectively storing the `initial_value` on the account.

```rs title="lib.rs" {35-44} /initial_value/
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Size of our counter account
    let account_space = 8; // Size in bytes to store a u64

    // Calculate minimum balance for rent exemption
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    // Create the counter account
    invoke(
        &system_instruction::create_account(
            payer_account.key,    // Account paying for the new account
            counter_account.key,  // Account to be created
            required_lamports,    // Amount of lamports to transfer to the new account
            account_space as u64, // Size in bytes to allocate for the data field
            program_id,           // Set program owner to our program
        ),
        &[
            payer_account.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Create a new CounterAccount struct with the initial value
    let counter_data = CounterAccount {
        count: initial_value,
    };

    // Get a mutable reference to the counter account's data
    let mut account_data = &mut counter_account.data.borrow_mut()[..];

    // Serialize the CounterAccount struct into the account's data
    counter_data.serialize(&mut account_data)?;

    msg!("Counter initialized with value: {}", initial_value);

    Ok(())
}
```

</Accordion>
</Accordions>

```rs title="lib.rs"
// Initialize a new counter account
fn process_initialize_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    initial_value: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let counter_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Size of our counter account
    let account_space = 8; // Size in bytes to store a u64

    // Calculate minimum balance for rent exemption
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_space);

    // Create the counter account
    invoke(
        &system_instruction::create_account(
            payer_account.key,    // Account paying for the new account
            counter_account.key,  // Account to be created
            required_lamports,    // Amount of lamports to transfer to the new account
            account_space as u64, // Size in bytes to allocate for the data field
            program_id,           // Set program owner to our program
        ),
        &[
            payer_account.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Create a new CounterAccount struct with the initial value
    let counter_data = CounterAccount {
        count: initial_value,
    };

    // Get a mutable reference to the counter account's data
    let mut account_data = &mut counter_account.data.borrow_mut()[..];

    // Serialize the CounterAccount struct into the account's data
    counter_data.serialize(&mut account_data)?;

    msg!("Counter initialized with value: {}", initial_value);

    Ok(())
}
```

Next, add the implementation of the `process_increment_counter` function. This
instruction increments the value of an existing counter account.

<Accordions>
<Accordion title="Explanation">

Just like the `process_initialize_counter` function, we start by creating an
iterator over the accounts. In this case, we are only expecting one account,
which is the account to be updated.

Note that in practice, a developer must implement various security checks to
validate the accounts passed to the program. Since all accounts are provided by
the caller of the instruction, there is no guarantee that the accounts provided
are the ones the program expects. Missing account validation checks are a common
source of program vulnerabilities.

The example below includes a check to ensure the account we're referring to as
the `counter_account` is owned by the executing program.

```rs title="lib.rs" {6-9}
// Update an existing counter's value
fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}
```

To update the account data, we:

- Mutably borrow the existing account's data field
- Deserialize the raw bytes into our `CounterAccount` struct
- Update the `count` value
- Serialize the modified struct back into the account's data field

```rs title="lib.rs" {11-24}
// Update an existing counter's value
fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Mutable borrow the account data
    let mut data = counter_account.data.borrow_mut();

    // Deserialize the account data into our CounterAccount struct
    let mut counter_data: CounterAccount = CounterAccount::try_from_slice(&data)?;

    // Increment the counter value
    counter_data.count = counter_data
        .count
        .checked_add(1)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Serialize the updated counter data back into the account
    counter_data.serialize(&mut &mut data[..])?;

    msg!("Counter incremented to: {}", counter_data.count);
    Ok(())
}
```

</Accordion>
</Accordions>

```rs title="lib.rs"
// Update an existing counter's value
fn process_increment_counter(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    // Verify account ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Mutable borrow the account data
    let mut data = counter_account.data.borrow_mut();

    // Deserialize the account data into our CounterAccount struct
    let mut counter_data: CounterAccount = CounterAccount::try_from_slice(&data)?;

    // Increment the counter value
    counter_data.count = counter_data
        .count
        .checked_add(1)
        .ok_or(ProgramError::InvalidAccountData)?;

    // Serialize the updated counter data back into the account
    counter_data.serialize(&mut &mut data[..])?;

    msg!("Counter incremented to: {}", counter_data.count);
    Ok(())
}
```

</Step>
<Step>

### Instruction Testing

To test the program instructions, add the following dependencies to
`Cargo.toml`.

```shell title="Terminal"
cargo add solana-program-test@1.18.26 --dev
cargo add solana-sdk@1.18.26 --dev
cargo add tokio --dev
```

Then add the following test module to `lib.rs` and run `cargo test-sbf` to
execute the tests. Optionally, use the `--nocapture` flag to see the print
statements in the output.

```shell title="Terminal"
cargo test-sbf -- --nocapture
```

<Accordions>
<Accordion title="Explanation">

First, set up the test module and import required dependencies:

```rs title="lib.rs"
#[cfg(test)]
mod test {
    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_counter_program() {
        // Test code will go here
    }
}
```

Next, set up the test using `ProgramTest`. Then create a new keypair to use as
the address for the counter account we'll initialize and define an initial value
to set for the counter.

```rs title="lib.rs"
#[cfg(test)]
mod test {
    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_counter_program() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "counter_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Create a new keypair to use as the address for our counter account
        let counter_keypair = Keypair::new();
        let initial_value: u64 = 42;
    }
}
```

When building an instruction, each account must be provided as an
[`AccountMeta`](https://github.com/solana-labs/solana/blob/v2.0/sdk/program/src/instruction.rs#L539-L545),
which specifies:

- The account's public key (`Pubkey`)
- `is_writable`: Whether the account data will be modified
- `is_signer`: Whether the account must sign the transaction

```rs
AccountMeta::new(account1_pubkey, true),           // writable, signer
AccountMeta::new(account2_pubkey, false),          // writable, not signer
AccountMeta::new_readonly(account3_pubkey, false), // not writable, not signer
AccountMeta::new_readonly(account4_pubkey, true),  // writable, signer
```

To test the initialize instruction:

- Create instruction data with variant 0 (`InitializeCounter`) and initial value
- Build the instruction with the program ID, instruction data, and required
  accounts
- Send a transaction with the initialize instruction
- Check the account was created with the correct initial value

```rs title="lib.rs" {16-53}
    #[tokio::test]
    async fn test_counter_program() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "counter_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Create a new keypair to use as the address for our counter account
        let counter_keypair = Keypair::new();
        let initial_value: u64 = 42;

        // Step 1: Initialize the counter
        println!("Testing counter initialization...");

        // Create initialization instruction
        let mut init_instruction_data = vec![0]; // 0 = initialize instruction
        init_instruction_data.extend_from_slice(&initial_value.to_le_bytes());

        let initialize_instruction = Instruction::new_with_bytes(
            program_id,
            &init_instruction_data,
            vec![
                AccountMeta::new(counter_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Send transaction with initialize instruction
        let mut transaction =
            Transaction::new_with_payer(&[initialize_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 42);
            println!(
                "✅ Counter initialized successfully with value: {}",
                counter.count
            );
        }
    }
```

To test the increment instruction:

- Build the instruction with the program ID, instruction data, and required
  accounts
- Send a transaction with the increment instruction
- Check the account was incremented to the correct value

Note that the instruction data for the increment instruction is `[1]`, which
corresponds to variant 1 (`IncrementCounter`). Since there are no additional
parameters to the increment instruction, the data is simply the instruction
variant.

```rs title="lib.rs" {55-82}
    #[tokio::test]
    async fn test_counter_program() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "counter_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Create a new keypair to use as the address for our counter account
        let counter_keypair = Keypair::new();
        let initial_value: u64 = 42;

        // Step 1: Initialize the counter
        println!("Testing counter initialization...");

        // Create initialization instruction
        let mut init_instruction_data = vec![0]; // 0 = initialize instruction
        init_instruction_data.extend_from_slice(&initial_value.to_le_bytes());

        let initialize_instruction = Instruction::new_with_bytes(
            program_id,
            &init_instruction_data,
            vec![
                AccountMeta::new(counter_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Send transaction with initialize instruction
        let mut transaction =
            Transaction::new_with_payer(&[initialize_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 42);
            println!(
                "✅ Counter initialized successfully with value: {}",
                counter.count
            );
        }

        // Step 2: Increment the counter
        println!("Testing counter increment...");

        // Create increment instruction
        let increment_instruction = Instruction::new_with_bytes(
            program_id,
            &[1], // 1 = increment instruction
            vec![AccountMeta::new(counter_keypair.pubkey(), true)],
        );

        // Send transaction with increment instruction
        let mut transaction =
            Transaction::new_with_payer(&[increment_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 43);
            println!("✅ Counter incremented successfully to: {}", counter.count);
        }
    }
```

</Accordion>
</Accordions>

```rs title="lib.rs"
#[cfg(test)]
mod test {
    use super::*;
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };

    #[tokio::test]
    async fn test_counter_program() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "counter_program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // Create a new keypair to use as the address for our counter account
        let counter_keypair = Keypair::new();
        let initial_value: u64 = 42;

        // Step 1: Initialize the counter
        println!("Testing counter initialization...");

        // Create initialization instruction
        let mut init_instruction_data = vec![0]; // 0 = initialize instruction
        init_instruction_data.extend_from_slice(&initial_value.to_le_bytes());

        let initialize_instruction = Instruction::new_with_bytes(
            program_id,
            &init_instruction_data,
            vec![
                AccountMeta::new(counter_keypair.pubkey(), true),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        );

        // Send transaction with initialize instruction
        let mut transaction =
            Transaction::new_with_payer(&[initialize_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 42);
            println!(
                "✅ Counter initialized successfully with value: {}",
                counter.count
            );
        }

        // Step 2: Increment the counter
        println!("Testing counter increment...");

        // Create increment instruction
        let increment_instruction = Instruction::new_with_bytes(
            program_id,
            &[1], // 1 = increment instruction
            vec![AccountMeta::new(counter_keypair.pubkey(), true)],
        );

        // Send transaction with increment instruction
        let mut transaction =
            Transaction::new_with_payer(&[increment_instruction], Some(&payer.pubkey()));
        transaction.sign(&[&payer, &counter_keypair], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();

        // Check account data
        let account = banks_client
            .get_account(counter_keypair.pubkey())
            .await
            .expect("Failed to get counter account");

        if let Some(account_data) = account {
            let counter: CounterAccount = CounterAccount::try_from_slice(&account_data.data)
                .expect("Failed to deserialize counter data");
            assert_eq!(counter.count, 43);
            println!("✅ Counter incremented successfully to: {}", counter.count);
        }
    }
}
```

Example output:

```shell title="Terminal" {6} {10}
running 1 test
[2024-10-29T20:51:13.783708000Z INFO  solana_program_test] "counter_program" SBF program from /counter_program/target/deploy/counter_program.so, modified 2 seconds, 169 ms, 153 µs and 461 ns ago
[2024-10-29T20:51:13.855204000Z DEBUG solana_runtime::message_processor::stable_log] Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM invoke [1]
[2024-10-29T20:51:13.856052000Z DEBUG solana_runtime::message_processor::stable_log] Program 11111111111111111111111111111111 invoke [2]
[2024-10-29T20:51:13.856135000Z DEBUG solana_runtime::message_processor::stable_log] Program 11111111111111111111111111111111 success
[2024-10-29T20:51:13.856242000Z DEBUG solana_runtime::message_processor::stable_log] Program log: Counter initialized with value: 42
[2024-10-29T20:51:13.856285000Z DEBUG solana_runtime::message_processor::stable_log] Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM consumed 3791 of 200000 compute units
[2024-10-29T20:51:13.856307000Z DEBUG solana_runtime::message_processor::stable_log] Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM success
[2024-10-29T20:51:13.860038000Z DEBUG solana_runtime::message_processor::stable_log] Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM invoke [1]
[2024-10-29T20:51:13.860333000Z DEBUG solana_runtime::message_processor::stable_log] Program log: Counter incremented to: 43
[2024-10-29T20:51:13.860355000Z DEBUG solana_runtime::message_processor::stable_log] Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM consumed 756 of 200000 compute units
[2024-10-29T20:51:13.860375000Z DEBUG solana_runtime::message_processor::stable_log] Program 1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM success
test test::test_counter_program ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```


</Step>
</Steps>

>>> solana-com/content/docs/en/core/accounts.mdx
---
title: Solana Account Model
description:
  Learn about Solana's account model, including how accounts store data and
  programs, rent mechanics, account ownership, and the relationship between
  programs and data accounts. Understand the core concepts of Solana's key-value
  storage system.
---

On Solana, all data is stored in what are called "accounts." You can think of
data on Solana as a public database with a single "Accounts" table, where each
entry in this table is an "account." Every Solana account shares the same base
[Account type](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/account/src/lib.rs#L48-L60).

![Accounts](/assets/docs/core/accounts/accounts.png)

## Key Points

- Accounts can store up to
  [10MiB](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/program/src/system_instruction.rs#L85)
  of data, which contains either executable program code or program state.
- Accounts require a
  [rent deposit](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/rent/src/lib.rs#L93-L97)
  in lamports (SOL) that's proportional to the amount of data stored, and you
  can fully recover it when you close the account.
- Every account has a program
  [owner](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/account/src/lib.rs#L55).
  Only the program that owns an account can change its data or deduct its
  lamport balance. But anyone can increase the balance.
- **Sysvar accounts** are special accounts that store network cluster state.
- **Program accounts** store the executable code of smart contracts.
- **Data accounts** are created by programs to store and manage program state.

## Account

Every account on Solana has a unique 32-byte address, often shown as a base58
encoded string (e.g. `14grJpemFaf88c8tiVb77W7TYg2W3ir6pfkKz3YjhhZ5`).

The relationship between the account and its address works like a key-value
pair, where the address is the key to locate the corresponding on-chain data of
the account. The account address acts as the "unique ID" for each entry in the
"Accounts" table.

![Account Address](/assets/docs/core/accounts/account-address.svg)

Most Solana accounts use an [Ed25519](https://ed25519.cr.yp.to/) public key as
their address.

<CodeTabs storage="accounts" flags="r">

```ts !! title="Kit"
import { generateKeyPairSigner } from "@solana/kit";

// Kit does not enable extractable private keys
const keypairSigner = await generateKeyPairSigner();
console.log(keypairSigner);
```

```ts !! title="Legacy"
import { Keypair } from "@solana/web3.js";

const keypair = Keypair.generate();
console.log(`Public Key: ${keypair.publicKey}`);
console.log(`Secret Key: ${keypair.secretKey}`);
```

```rs !! title="Rust"
use solana_sdk::signer::{keypair::Keypair, Signer};

#[tokio::main]
async fn main() {
    let keypair = Keypair::new();
    println!("Public Key: {}", keypair.pubkey());
    println!("Secret Key: {:?}", keypair.to_bytes());
}
```

</CodeTabs>

While public keys are commonly used as account addresses, Solana also supports a
feature called [Program Derived Addresses](/docs/core/pda) (PDAs). PDAs are
special addresses that you can deterministically derive from a program ID and
optional inputs (seeds).

<CodeTabs storage="accounts" flags="r">

```ts !! title="Kit"
import { Address, getProgramDerivedAddress } from "@solana/kit";

const programAddress = "11111111111111111111111111111111" as Address;

const seeds = ["helloWorld"];
const [pda, bump] = await getProgramDerivedAddress({
  programAddress,
  seeds
});

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```ts !! title="Legacy"
import { PublicKey } from "@solana/web3.js";

const programAddress = new PublicKey("11111111111111111111111111111111");

const seeds = [Buffer.from("helloWorld")];
const [pda, bump] = await PublicKey.findProgramAddressSync(
  seeds,
  programAddress
);

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```rs !! title="Rust"
use solana_sdk::pubkey; // macro
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() {
    let program_address = pubkey!("11111111111111111111111111111111");
    let seeds = [b"helloWorld".as_ref()];
    let (pda, bump) = Pubkey::find_program_address(&seeds, &program_address);
    println!("PDA: {}", pda);
    println!("Bump: {}", bump);
}
```

</CodeTabs>

### Account Type

Accounts have a max size of
[10MiB](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/program/src/system_instruction.rs#L85)
and every account on Solana shares the same base
[Account](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/account/src/lib.rs#L48-L60)
type.

![Account Type](/assets/docs/core/accounts/account-type.svg)

Every Account on Solana has the following fields:

- `data`: A byte array that stores arbitrary data for an account. For
  non-executable accounts, this often stores state that's meant be read from.
  For program accounts (smart contracts), this contains the executable program
  code. The data field is commonly called "account data."
- `executable`: This flag shows if an account is a program.
- `lamports`: The account's balance in lamports, the smallest unit of SOL (1 SOL
  = 1 billion lamports).
- `owner`: The program ID (public key) of the program that owns this account.
  Only the owner program can change the account's data or deduct its lamports
  balance.
- `rent_epoch`: A legacy field from when Solana had a mechanism that
  periodically deducted lamports from accounts. While this field still exists in
  the Account type, it is no longer used since rent collection was deprecated.

```rust title="Base Account Type"
pub struct Account {
    /// lamports in the account
    pub lamports: u64,
    /// data held in this account
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub data: Vec<u8>,
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,
    /// the epoch at which this account will next owe rent
    pub rent_epoch: Epoch,
}
```

<CodeTabs storage="accounts">

```ts !! title="Kit"
import {
  airdropFactory,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  generateKeyPairSigner,
  lamports
} from "@solana/kit";

// Create a connection to Solana cluster
const rpc = createSolanaRpc("http://localhost:8899");
const rpcSubscriptions = createSolanaRpcSubscriptions("ws://localhost:8900");

// Generate a new keypair
const keypair = await generateKeyPairSigner();
console.log(`Public Key: ${keypair.address}`);

// Funding an address with SOL automatically creates an account
const signature = await airdropFactory({ rpc, rpcSubscriptions })({
  recipientAddress: keypair.address,
  lamports: lamports(1_000_000_000n),
  commitment: "confirmed"
});

const accountInfo = await rpc.getAccountInfo(keypair.address).send();
console.log(accountInfo);
```

```ts !! title="Legacy"
import { Keypair, Connection, LAMPORTS_PER_SOL } from "@solana/web3.js";

// Generate a new keypair
const keypair = Keypair.generate();
console.log(`Public Key: ${keypair.publicKey}`);

// Create a connection to the Solana cluster
const connection = new Connection("http://localhost:8899", "confirmed");

// Funding an address with SOL automatically creates an account
const signature = await connection.requestAirdrop(
  keypair.publicKey,
  LAMPORTS_PER_SOL
);
await connection.confirmTransaction(signature, "confirmed");

const accountInfo = await connection.getAccountInfo(keypair.publicKey);
console.log(JSON.stringify(accountInfo, null, 2));
```

```rs !! title="Rust"
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::LAMPORTS_PER_SOL,
    signer::{keypair::Keypair, Signer},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Generate a new keypair
    let keypair = Keypair::new();
    println!("Public Key: {}", keypair.pubkey());

    // Create a connection to Solana cluster
    let connection = RpcClient::new_with_commitment(
        "http://localhost:8899".to_string(),
        CommitmentConfig::confirmed(),
    );

    // Funding an address with SOL automatically creates an account
    let signature = connection
        .request_airdrop(&keypair.pubkey(), LAMPORTS_PER_SOL)
        .await?;
    connection.confirm_transaction(&signature).await?;

    let account_info = connection.get_account(&keypair.pubkey()).await?;
    println!("{:#?}", account_info);

    Ok(())
}
```

</CodeTabs>

### Rent

To store data on-chain, accounts must also keep a lamport (SOL) balance that's
proportional to the amount of data stored on the account (in bytes). This
balance is called "rent," but it works more like a deposit because you can
recover the full amount when you close an account. You can find the calculation
[here](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/rent/src/lib.rs#L93-L97)
using these
[constants](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/rent/src/lib.rs#L47-L70).

The term "rent" comes from a deprecated mechanism that regularly deducted
lamports from accounts that fell below the rent threshold. This mechanism isn't
active anymore.

### Program Owner

On Solana, "smart contracts" are called [programs](/docs/core/programs). Program
ownership is a key part of the Solana Account Model. Every account has a
designated program as its owner. Only the owner program can:

- Change the account's `data` field
- Deduct lamports from the account's balance

## System Program

By default, all new accounts are owned to the
[System Program](https://github.com/anza-xyz/agave/tree/v2.1.13/programs/system/src).
The System Program does a few key things:

- [New Account Creation](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/system/src/system_processor.rs#L146):
  Only the System Program can create new accounts.
- [Space Allocation](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/system/src/system_processor.rs#L71):
  Sets the byte capacity for the data field of each account.
- [Transfer / Assign Program Ownership](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/system/src/system_processor.rs#L113):
  Once the System Program creates an account, it can reassign the designated
  program owner to a different program account. That's how custom programs take
  ownership of new accounts created by the System Program.

All "wallet" accounts on Solana are just accounts owned by the System Program.
The lamport balance in these accounts shows the amount of SOL owned by the
wallet. Only accounts owned by the System Program can pay transaction fees.

![System Account](/assets/docs/core/accounts/system-account.svg)

## Sysvar Accounts

Sysvar accounts are special accounts at predefined addresses that provide access
to cluster state data. These accounts update dynamically with data about the
network cluster. You can find the full list of Sysvar Accounts
[here](https://docs.anza.xyz/runtime/sysvars).

<CodeTabs storage="accounts" flags="r">

```ts !! title="Kit"
import { Address, createSolanaRpc } from "@solana/kit";

const rpc = createSolanaRpc("https://api.mainnet-beta.solana.com");

const SYSVAR_CLOCK_ADDRESS =
  "SysvarC1ock11111111111111111111111111111111" as Address;

const accountInfo = await rpc
  .getAccountInfo(SYSVAR_CLOCK_ADDRESS, { encoding: "base64" })
  .send();
console.log(accountInfo);
```

```ts !! title="Legacy"
import { Connection, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

const connection = new Connection(
  "https://api.mainnet-beta.solana.com",
  "confirmed"
);

const accountInfo = await connection.getAccountInfo(SYSVAR_CLOCK_PUBKEY);
// !collapse(1:17) collapsed
console.log(
  JSON.stringify(
    accountInfo,
    (key, value) => {
      if (key === "data" && value && value.length > 1) {
        return [
          value[0],
          "...truncated, total bytes: " + value.length + "...",
          value[value.length - 1]
        ];
      }
      return value;
    },
    2
  )
);
```

```rs !! title="Rust"
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, sysvar};

#[tokio::main]
async fn main() -> Result<()> {
    let connection = RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com".to_string(),
        CommitmentConfig::confirmed(),
    );

    let account_info = connection.get_account(&sysvar::clock::ID).await?;
    println!("{:#?}", account_info);

    Ok(())
}
```

</CodeTabs>

## Program Account

Deploying a Solana program creates an executable program account. The program
account stores the executable code of the program.

Program accounts are owned by a
[Loader Program](/docs/core/programs#loader-programs).

![Program Account](/assets/docs/core/accounts/program-account-simple.svg)

For simplicity, you can treat the program account as the program itself. When
you invoke a program's instructions, you specify the program account's address
(commonly called the "Program ID").

<CodeTabs storage="accounts" flags="r">

```ts !! title="Kit"
import { Address, createSolanaRpc } from "@solana/kit";

const rpc = createSolanaRpc("https://api.mainnet-beta.solana.com");

const programId = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" as Address;

const accountInfo = await rpc
  .getAccountInfo(programId, { encoding: "base64" })
  .send();
console.log(accountInfo);
```

```ts !! title="Legacy"
import { Connection, PublicKey } from "@solana/web3.js";

const connection = new Connection(
  "https://api.mainnet-beta.solana.com",
  "confirmed"
);

const programId = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const accountInfo = await connection.getAccountInfo(programId);
// !collapse(1:17) collapsed
console.log(
  JSON.stringify(
    accountInfo,
    (key, value) => {
      if (key === "data" && value && value.length > 1) {
        return [
          value[0],
          "...truncated, total bytes: " + value.length + "...",
          value[value.length - 1]
        ];
      }
      return value;
    },
    2
  )
);
```

```rs !! title="Rust"
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey};

#[tokio::main]
async fn main() -> Result<()> {
    let connection = RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com".to_string(),
        CommitmentConfig::confirmed(),
    );

    let program_id = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    let account_info = connection.get_account(&program_id).await?;
    println!("{:#?}", account_info);

    Ok(())
}
```

</CodeTabs>

<Callout type="info">

When you deploy a Solana program, it's stored in a program account. Program
accounts are owned by a [Loader Program](/docs/core/programs#loader-programs).
There are several versions of the loader, but all except loader-v3 store the
executable code directly in the program account. Loader-v3 stores the executable
code in a separate "program data account" and the program account just points to
it. When you deploy a new program, the Solana CLI uses the latest loader version
by default.

</Callout>

### Buffer Account

Loader-v3 has a special account type for temporarily staging the upload of a
program during deployment or redeployment/upgrades. In loader-v4, there are
still buffers, but they're just normal program accounts.

### Program Data Account

Loader-v3 works differently from all other BPF Loader programs. The program
account only contains the address of a program data account, which stores the
actual executable code:
![Program Data Account](/assets/docs/core/accounts/program-account-expanded.svg)

Don't confuse these program data accounts with the data accounts of programs
(see below).

## Data Account

On Solana, the executable code of a program is stored in a different account
than the program's state. This is like how operating systems typically have
separate files for programs and their data.

To maintain state, programs define instructions to create separate accounts that
they own. Each of these accounts has its own unique address and can store any
arbitrary data defined by the program.

![Data Account](/assets/docs/core/accounts/data-account.svg)

Note that only the [System Program](/docs/core/accounts#system-program) can
create new accounts. Once the System Program creates an account, it can then
transfer or assign ownership of the new account to another program.

In other words, creating a data account for a custom program takes two steps:

1. Invoke the System Program to create an account, then transfer ownership to
   the custom program
2. Invoke the custom program, which now owns the account, to initialize the
   account data as defined by the program's instruction

This account creation process is often abstracted as a single step, but it's
helpful to understand the underlying process.

<CodeTabs storage="accounts">

```ts !! title="Kit"
import {
  airdropFactory,
  appendTransactionMessageInstructions,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  generateKeyPairSigner,
  getSignatureFromTransaction,
  lamports,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners
} from "@solana/kit";
import { getCreateAccountInstruction } from "@solana-program/system";
import {
  getInitializeMintInstruction,
  getMintSize,
  TOKEN_2022_PROGRAM_ADDRESS
} from "@solana-program/token-2022";

// Create Connection, local validator in this example
const rpc = createSolanaRpc("http://127.0.0.1:8899");
const rpcSubscriptions = createSolanaRpcSubscriptions("ws://localhost:8900");

// Generate keypairs for fee payer
const feePayer = await generateKeyPairSigner();

// Fund fee payer
await airdropFactory({ rpc, rpcSubscriptions })({
  recipientAddress: feePayer.address,
  lamports: lamports(1_000_000_000n),
  commitment: "confirmed"
});

// Generate keypair to use as address of mint
const mint = await generateKeyPairSigner();

// Get default mint account size (in bytes), no extensions enabled
const space = BigInt(getMintSize());

// Get minimum balance for rent exemption
const rent = await rpc.getMinimumBalanceForRentExemption(space).send();

// Instruction to create new account for mint (token 2022 program)
// Invokes the system program
const createAccountInstruction = getCreateAccountInstruction({
  payer: feePayer,
  newAccount: mint,
  lamports: rent,
  space,
  programAddress: TOKEN_2022_PROGRAM_ADDRESS
});

// Instruction to initialize mint account data
// Invokes the token 2022 program
const initializeMintInstruction = getInitializeMintInstruction({
  mint: mint.address,
  decimals: 9,
  mintAuthority: feePayer.address
});

const instructions = [createAccountInstruction, initializeMintInstruction];

// Get latest blockhash to include in transaction
const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

// Create transaction message
const transactionMessage = pipe(
  createTransactionMessage({ version: 0 }), // Create transaction message
  (tx) => setTransactionMessageFeePayerSigner(feePayer, tx), // Set fee payer
  (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx), // Set transaction blockhash
  (tx) => appendTransactionMessageInstructions(instructions, tx) // Append instructions
);

// Sign transaction message with required signers (fee payer and mint keypair)
const signedTransaction =
  await signTransactionMessageWithSigners(transactionMessage);

// Send and confirm transaction
await sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions })(
  signedTransaction,
  { commitment: "confirmed" }
);

// Get transaction signature
const transactionSignature = getSignatureFromTransaction(signedTransaction);

console.log("Mint Address:", mint.address);
console.log("Transaction Signature:", transactionSignature);

const accountInfo = await rpc.getAccountInfo(mint.address).send();
console.log(accountInfo);
```

```ts !! title="Legacy"
import {
  Connection,
  Keypair,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
  LAMPORTS_PER_SOL
} from "@solana/web3.js";
import {
  createInitializeMintInstruction,
  TOKEN_2022_PROGRAM_ID,
  MINT_SIZE,
  getMinimumBalanceForRentExemptMint
} from "@solana/spl-token";

// Create connection to local validator
const connection = new Connection("http://localhost:8899", "confirmed");
const recentBlockhash = await connection.getLatestBlockhash();

// Generate a new keypair for the fee payer
const feePayer = Keypair.generate();

// Airdrop 1 SOL to fee payer
const airdropSignature = await connection.requestAirdrop(
  feePayer.publicKey,
  LAMPORTS_PER_SOL
);
await connection.confirmTransaction({
  blockhash: recentBlockhash.blockhash,
  lastValidBlockHeight: recentBlockhash.lastValidBlockHeight,
  signature: airdropSignature
});

// Generate keypair to use as address of mint
const mint = Keypair.generate();

const createAccountInstruction = SystemProgram.createAccount({
  fromPubkey: feePayer.publicKey,
  newAccountPubkey: mint.publicKey,
  space: MINT_SIZE,
  lamports: await getMinimumBalanceForRentExemptMint(connection),
  programId: TOKEN_2022_PROGRAM_ID
});

const initializeMintInstruction = createInitializeMintInstruction(
  mint.publicKey, // mint pubkey
  9, // decimals
  feePayer.publicKey, // mint authority
  feePayer.publicKey, // freeze authority
  TOKEN_2022_PROGRAM_ID
);

const transaction = new Transaction().add(
  createAccountInstruction,
  initializeMintInstruction
);

const transactionSignature = await sendAndConfirmTransaction(
  connection,
  transaction,
  [feePayer, mint] // Signers
);

console.log("Mint Address: ", mint.publicKey.toBase58());
console.log("Transaction Signature: ", transactionSignature);

const accountInfo = await connection.getAccountInfo(mint.publicKey);
// !collapse(1:17) collapsed
console.log(
  JSON.stringify(
    accountInfo,
    (key, value) => {
      if (key === "data" && value && value.length > 1) {
        return [
          value[0],
          "...truncated, total bytes: " + value.length + "...",
          value[value.length - 1]
        ];
      }
      return value;
    },
    2
  )
);
```

```rs !! title="Rust"
use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    program_pack::Pack,
    signature::{Keypair, Signer},
    system_instruction::create_account,
    transaction::Transaction,
};
use spl_token_2022::{id as token_2022_program_id, instruction::initialize_mint, state::Mint};

#[tokio::main]
async fn main() -> Result<()> {
    // Create connection to local validator
    let client = RpcClient::new_with_commitment(
        String::from("http://127.0.0.1:8899"),
        CommitmentConfig::confirmed(),
    );
    let recent_blockhash = client.get_latest_blockhash().await?;

    // Generate a new keypair for the fee payer
    let fee_payer = Keypair::new();

    // Airdrop 1 SOL to fee payer
    let airdrop_signature = client
        .request_airdrop(&fee_payer.pubkey(), 1_000_000_000)
        .await?;
    client.confirm_transaction(&airdrop_signature).await?;

    loop {
        let confirmed = client.confirm_transaction(&airdrop_signature).await?;
        if confirmed {
            break;
        }
    }

    // Generate keypair to use as address of mint
    let mint = Keypair::new();

    let space = Mint::LEN;
    let rent = client.get_minimum_balance_for_rent_exemption(space).await?;

    // Create account instruction
    let create_account_instruction = create_account(
        &fee_payer.pubkey(),      // fee payer
        &mint.pubkey(),           // mint address
        rent,                     // rent
        space as u64,             // space
        &token_2022_program_id(), // program id
    );

    // Initialize mint instruction
    let initialize_mint_instruction = initialize_mint(
        &token_2022_program_id(),
        &mint.pubkey(),            // mint address
        &fee_payer.pubkey(),       // mint authority
        Some(&fee_payer.pubkey()), // freeze authority
        9,                         // decimals
    )?;

    // Create transaction and add instructions
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_instruction, initialize_mint_instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer, &mint],
        recent_blockhash,
    );

    // Send and confirm transaction
    let transaction_signature = client.send_and_confirm_transaction(&transaction).await?;

    println!("Mint Address: {}", mint.pubkey());
    println!("Transaction Signature: {}", transaction_signature);

    let account_info = client.get_account(&mint.pubkey()).await?;
    println!("{:#?}", account_info);

    Ok(())
}
```

</CodeTabs>

>>> solana-com/content/docs/en/core/cpi.mdx
---
title: Cross Program Invocation
description:
  Learn about Cross Program Invocation (CPI) on Solana - how programs can call
  instructions on other programs, handle PDA signers, and compose functionality
  across the Solana network.
h1: Cross Program Invocation (CPI)
---

A Cross Program Invocation (CPI) refers to when one program invokes the
instructions of another program. This allows for the composability of Solana
programs.

You can think of instructions as API endpoints that a program exposes to the
network and a CPI as one API internally invoking another API.

![Cross Program Invocation](/assets/docs/core/cpi/cpi.svg)

## Key Points

- **Cross Program Invocations** enable Solana program instructions to directly
  invoke instructions on another program.
- **Signer privileges** from a caller program extend to the callee program.
- When making a Cross Program Invocation, **programs can sign on behalf of
  PDAs** derived from their own program ID.
- The callee program can make further CPIs to other programs, up to a depth
  of 4.

## What is a CPI?

A Cross Program Invocation (CPI) is when one program invokes the instructions of
another program.

Writing a program instruction with a CPI follows the same pattern as building an
[instruction](/docs/core/transactions#instruction) to add to a transaction.
Under the hood, each CPI instruction must specify:

- **Program address**: Specifies the program to invoke
- **Accounts**: Lists every account the instruction reads from or writes to,
  including other programs
- **Instruction Data**: Specifies which instruction to invoke on the program,
  plus any data the instruction needs (function arguments)

When a program makes a Cross Program Invocation (CPI) to another program:

- The signer privileges from the initial transaction extend to the callee
  program (ex. A->B)
- The callee program can make further CPIs to other programs, up to a depth of 4
  (ex. B->C, C->D)
- The programs can "sign" on behalf of the [PDAs](/docs/core/pda) derived from
  its program ID

<Callout>
  The Solana program runtime sets a
  [`max_instruction_stack_depth`](https://github.com/anza-xyz/agave/blob/v2.1.13/compute-budget/src/compute_budget.rs#L38)
  constant
  [MAX_INSTRUCTION_STACK_DEPTH](https://github.com/anza-xyz/agave/blob/v2.1.13/compute-budget/src/compute_budget.rs#L13)
  of 5. This represents the max height of the program instruction invocation
  stack. The stack height begins at 1 for the initial transaction and increases
  by 1 each time a program invokes another instruction. This setting limits
  invocation depth for CPIs to 4.
</Callout>

When a transaction is processed, account privileges extend from one program to
another. Here's what that means:

Let's say Program A receives an instruction with:

- An account that signed the transaction
- An account that can be written to (mutable)

When Program A makes a CPI to Program B:

- Program B gets to use these same accounts with their original permissions
- Program B can sign with the signer account
- Program B can write to the writable account
- Program B can even pass these same permissions forward if it makes its own
  CPIs

## Cross Program Invocations

The
[`invoke`](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/program/src/program.rs#L26-L28)
function handles CPIs that don't require PDA signers. The function calls the
`invoke_signed` function with an empty `signers_seeds` array, indicating no PDAs
required for signing.

```rust title="Invoke Function"
pub fn invoke(instruction: &Instruction, account_infos: &[AccountInfo]) -> ProgramResult {
    invoke_signed(instruction, account_infos, &[])
}
```

The following examples show how to make a CPI using the
[Anchor Framework](https://www.anchor-lang.com/docs) and Native Rust. The
example programs include a single instruction that transfers SOL from one
account to another using a CPI.

### Anchor Framework

The following examples present three ways to create Cross Program Invocations
(CPIs) in an Anchor program, each at a different level of abstraction. All
examples work the same way. The main purpose is to show the implementation
details of a CPI.

- Example 1: Uses Anchor's _rs`CpiContext`_ and helper function to construct the
  CPI instruction.
- Example 2: Uses the _rs`system_instruction::transfer`_ function from the
  `solana_program` crate to construct the CPI instruction. Example 1 abstracts
  this implementation.
- Example 3: Constructs the CPI instruction manually. This approach is useful
  when no crate exists to help build the instruction.

<CodeTabs>

```rs !! title="Example 1"
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("9AvUNHjxscdkiKQ8tUn12QCMXtcnbR9BVGq3ULNzFMRi");

#[program]
pub mod cpi {
    use super::*;

    pub fn sol_transfer(ctx: Context<SolTransfer>, amount: u64) -> Result<()> {
        let from_pubkey = ctx.accounts.sender.to_account_info();
        let to_pubkey = ctx.accounts.recipient.to_account_info();
        let program_id = ctx.accounts.system_program.to_account_info();

        let cpi_context = CpiContext::new(
            program_id,
            Transfer {
                from: from_pubkey,
                to: to_pubkey,
            },
        );

        transfer(cpi_context, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(mut)]
    sender: Signer<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

```rs !! title="Example 2"
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction};

declare_id!("BrcdB9sV7z9DvF9rDHG263HUxXgJM3iCQdF36TcxbFEn");

#[program]
pub mod cpi {
    use super::*;

    pub fn sol_transfer(ctx: Context<SolTransfer>, amount: u64) -> Result<()> {
        let from_pubkey = ctx.accounts.sender.to_account_info();
        let to_pubkey = ctx.accounts.recipient.to_account_info();
        let program_id = ctx.accounts.system_program.to_account_info();

        let instruction =
            &system_instruction::transfer(&from_pubkey.key(), &to_pubkey.key(), amount);

        invoke(instruction, &[from_pubkey, to_pubkey, program_id])?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(mut)]
    sender: Signer<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

```rs !! title="Example 3"
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, instruction::Instruction};

declare_id!("BrcdB9sV7z9DvF9rDHG263HUxXgJM3iCQdF36TcxbFEn");

#[program]
pub mod cpi {
    use super::*;

    pub fn sol_transfer(ctx: Context<SolTransfer>, amount: u64) -> Result<()> {
        let from_pubkey = ctx.accounts.sender.to_account_info();
        let to_pubkey = ctx.accounts.recipient.to_account_info();
        let program_id = ctx.accounts.system_program.to_account_info();

        // Prepare instruction AccountMetas
        let account_metas = vec![
            AccountMeta::new(from_pubkey.key(), true),
            AccountMeta::new(to_pubkey.key(), false),
        ];

        // SOL transfer instruction discriminator
        let instruction_discriminator: u32 = 2;

        // Prepare instruction data
        let mut instruction_data = Vec::with_capacity(4 + 8);
        instruction_data.extend_from_slice(&instruction_discriminator.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        // Create instruction
        let instruction = Instruction {
            program_id: program_id.key(),
            accounts: account_metas,
            data: instruction_data,
        };

        // Invoke instruction
        invoke(&instruction, &[from_pubkey, to_pubkey, program_id])?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(mut)]
    sender: Signer<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

```ts !! title="Test"
import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { Cpi } from "../target/types/cpi";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";

describe("cpi", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Cpi as Program<Cpi>;
  const sender = provider.wallet as anchor.Wallet;
  const recipient = new Keypair();

  const transferAmount = 0.01 * LAMPORTS_PER_SOL;

  it("SOL Transfer Anchor", async () => {
    const transactionSignature = await program.methods
      .solTransfer(new BN(transferAmount))
      .accounts({
        sender: sender.publicKey,
        recipient: recipient.publicKey
      })
      .rpc();

    console.log(`\nTransaction Signature: ${transactionSignature}`);
  });
});
```

</CodeTabs>

### Native Rust

The following example shows how to make a CPI from a program written in Native
Rust. The program includes a single instruction that transfers SOL from one
account to another using a CPI. The test file uses the
[LiteSVM](https://github.com/LiteSVM/litesvm) to test the program.

<CodeTabs>

```rs !! title="Example"
use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

// Declare program entrypoint
entrypoint!(process_instruction);

// Define program instructions
#[derive(BorshDeserialize)]
enum ProgramInstruction {
    SolTransfer { amount: u64 },
}

impl ProgramInstruction {
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(input).map_err(|_| ProgramError::InvalidInstructionData)
    }
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Deserialize instruction data
    let instruction = ProgramInstruction::unpack(instruction_data)?;

    // Process instruction
    match instruction {
        ProgramInstruction::SolTransfer { amount } => {
            // Parse accounts
            let [sender_info, recipient_info, system_program_info] = accounts else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };

            // Verify the sender is a signer
            if !sender_info.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }

            // Create and invoke the transfer instruction
            let transfer_ix = system_instruction::transfer(
                sender_info.key,
                recipient_info.key,
                amount,
            );

            invoke(
                &transfer_ix,
                &[
                    sender_info.clone(),
                    recipient_info.clone(),
                    system_program_info.clone(),
                ],
            )?;

            Ok(())
        }
    }
}
```

```ts !! title="Test"
import * as path from "path";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction
} from "@solana/web3.js";
import { LiteSVM } from "litesvm";

test("sol transfer cpi", () => {
  const svm = new LiteSVM();

  const programId = PublicKey.unique();
  const programPath = path.join(__dirname, "program.so");
  svm.addProgramFromFile(programId, programPath);

  // Create sender and recipient
  const sender = new Keypair();
  const recipient = new Keypair();

  // Fund sender
  const amount = BigInt(LAMPORTS_PER_SOL);
  svm.airdrop(sender.publicKey, amount); // 1 SOL

  // Create instruction data buffer
  const transferAmount = amount / BigInt(2); // 0.5 SOL
  const instructionIndex = 0; // instruction index 0 for SolTransfer enum

  const data = Buffer.alloc(9); // 1 byte for instruction enum + 8 bytes for u64
  data.writeUInt8(instructionIndex, 0); // first byte identifies the instruction
  data.writeBigUInt64LE(transferAmount, 1); // remaining bytes are instruction arguments

  // Create instruction
  const instruction = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: sender.publicKey, isSigner: true, isWritable: true },
      { pubkey: recipient.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
    ],
    data
  });

  // Create and send transaction
  const transaction = new Transaction().add(instruction);
  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(sender);

  svm.sendTransaction(transaction);

  // Check balances
  const recipientBalance = svm.getBalance(recipient.publicKey);
  const senderBalance = svm.getBalance(sender.publicKey);

  const transactionFee = BigInt(5000);
  expect(recipientBalance).toBe(transferAmount);
  expect(senderBalance).toBe(amount - transferAmount - transactionFee);
});
```

</CodeTabs>

## Cross Program Invocations with PDA Signers

The
[`invoke_signed`](https://github.com/anza-xyz/agave/blob/v2.1.13/sdk/program/src/program.rs#L51-L73)
function handles CPIs that require PDA signers. The function takes the seeds for
deriving signer PDAs as `signer_seeds`.

You can reference the [Program Derived Address](/docs/core/pda) page for details
on how to derive PDAs.

```rust title="Invoke Signed"
pub fn invoke_signed(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    // --snip--
    invoke_signed_unchecked(instruction, account_infos, signers_seeds)
}
```

When processing an instruction that includes a CPI, the Solana runtime
internally calls
[`create_program_address`](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/bpf_loader/src/syscalls/cpi.rs#L552)
using the `signers_seeds` and the `program_id` of the calling program. When a
valid PDA verified, the address is
[added as a valid signer](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/bpf_loader/src/syscalls/cpi.rs#L554).

The following examples demonstrate how to make a CPI with PDA signers using the
[Anchor Framework](https://www.anchor-lang.com/docs) and Native Rust. The
example programs include a single instruction that transfers SOL from a PDA to
the recipient account using a CPI signed by the PDA.

### Anchor Framework

The following examples include three approaches to implementing Cross Program
Invocations (CPIs) in an Anchor program, each at a different level of
abstraction. All examples are functionally equivalent. The main purpose is to
illustrate the implementation details of a CPI.

- Example 1: Uses Anchor's _rs`CpiContext`_ and helper function to construct the
  CPI instruction.
- Example 2: Uses the _rs`system_instruction::transfer`_ function from
  `solana_program` crate to construct the CPI instruction. Example 1 is an
  abstraction of this implementation.
- Example 3: Constructs the CPI instruction manually. This approach is useful
  when there is no crate available to help build the instruction you want to
  invoke.

<CodeTabs>

```rs !! title="Example 1"
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("BrcdB9sV7z9DvF9rDHG263HUxXgJM3iCQdF36TcxbFEn");

#[program]
pub mod cpi {
    use super::*;

    pub fn sol_transfer(ctx: Context<SolTransfer>, amount: u64) -> Result<()> {
        let from_pubkey = ctx.accounts.pda_account.to_account_info();
        let to_pubkey = ctx.accounts.recipient.to_account_info();
        let program_id = ctx.accounts.system_program.to_account_info();

        let seed = to_pubkey.key();
        let bump_seed = ctx.bumps.pda_account;
        let signer_seeds: &[&[&[u8]]] = &[&[b"pda", seed.as_ref(), &[bump_seed]]];

        let cpi_context = CpiContext::new(
            program_id,
            Transfer {
                from: from_pubkey,
                to: to_pubkey,
            },
        )
        .with_signer(signer_seeds);

        transfer(cpi_context, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pda", recipient.key().as_ref()],
        bump,
    )]
    pda_account: SystemAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

```rs !! title="Example 2"
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, system_instruction};

declare_id!("BrcdB9sV7z9DvF9rDHG263HUxXgJM3iCQdF36TcxbFEn");

#[program]
pub mod cpi {
    use super::*;

    pub fn sol_transfer(ctx: Context<SolTransfer>, amount: u64) -> Result<()> {
        let from_pubkey = ctx.accounts.pda_account.to_account_info();
        let to_pubkey = ctx.accounts.recipient.to_account_info();
        let program_id = ctx.accounts.system_program.to_account_info();

        let seed = to_pubkey.key();
        let bump_seed = ctx.bumps.pda_account;

        let signer_seeds: &[&[&[u8]]] = &[&[b"pda", seed.as_ref(), &[bump_seed]]];

        let instruction =
            &system_instruction::transfer(&from_pubkey.key(), &to_pubkey.key(), amount);

        invoke_signed(instruction, &[from_pubkey, to_pubkey, program_id], signer_seeds)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pda", recipient.key().as_ref()],
        bump,
    )]
    pda_account: SystemAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

```rs !! title="Example 3"
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke_signed, instruction::{Instruction, AccountMeta}};

declare_id!("BrcdB9sV7z9DvF9rDHG263HUxXgJM3iCQdF36TcxbFEn");

#[program]
pub mod cpi {
    use super::*;

    pub fn sol_transfer(ctx: Context<SolTransfer>, amount: u64) -> Result<()> {
        let from_pubkey = ctx.accounts.pda_account.to_account_info();
        let to_pubkey = ctx.accounts.recipient.to_account_info();
        let program_id = ctx.accounts.system_program.to_account_info();

        // Get PDA signer seeds
        let seed = to_pubkey.key();
        let bump_seed = ctx.bumps.pda_account;
        let signer_seeds: &[&[&[u8]]] = &[&[b"pda", seed.as_ref(), &[bump_seed]]];

        // Prepare instruction AccountMetas
        let account_metas = vec![
            AccountMeta::new(from_pubkey.key(), true),
            AccountMeta::new(to_pubkey.key(), false),
        ];

        // SOL transfer instruction discriminator
        let instruction_discriminator: u32 = 2;

        // Prepare instruction data
        let mut instruction_data = Vec::with_capacity(4 + 8);
        instruction_data.extend_from_slice(&instruction_discriminator.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        // Create instruction
        let instruction = Instruction {
            program_id: program_id.key(),
            accounts: account_metas,
            data: instruction_data,
        };

        // Invoke instruction with PDA signer
        invoke_signed(&instruction, &[from_pubkey, to_pubkey, program_id], signer_seeds)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SolTransfer<'info> {
    #[account(
        mut,
        seeds = [b"pda", recipient.key().as_ref()],
        bump,
    )]
    pda_account: SystemAccount<'info>,
    #[account(mut)]
    recipient: SystemAccount<'info>,
    system_program: Program<'info, System>,
}
```

```ts !! title="Test"
import * as anchor from "@coral-xyz/anchor";

import { BN, Program } from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";

import { Cpi } from "../target/types/cpi";

describe("cpi", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Cpi as Program<Cpi>;
  const connection = program.provider.connection;
  const wallet = provider.wallet as anchor.Wallet;

  const [PDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("pda"), wallet.publicKey.toBuffer()],
    program.programId
  );

  const transferAmount = 0.1 * LAMPORTS_PER_SOL;

  before(async () => {
    // Request airdrop to fund PDA
    const signature = await connection.requestAirdrop(PDA, transferAmount);

    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash();

    await connection.confirmTransaction({
      signature,
      blockhash,
      lastValidBlockHeight
    });
  });

  it("SOL Transfer with PDA signer", async () => {
    const transactionSignature = await program.methods
      .solTransfer(new BN(transferAmount))
      .accounts({
        recipient: wallet.publicKey
      })
      .rpc();

    console.log(`\nTransaction Signature: ${transactionSignature}`);
  });
});
```

</CodeTabs>

### Native Rust

The following example shows how to make a CPI with PDA signers from a program
written in Native Rust. The program includes a single instruction that transfers
SOL from a PDA to the recipient account using a CPI signed by the PDA. The test
file uses the [LiteSVM](https://github.com/LiteSVM/litesvm) to test the program.

<CodeTabs>

```rs !! title="Example"
use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
};

// Declare program entrypoint
entrypoint!(process_instruction);

// Define program instructions
#[derive(BorshDeserialize)]
enum ProgramInstruction {
    SolTransfer { amount: u64 },
}

impl ProgramInstruction {
    fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(input).map_err(|_| ProgramError::InvalidInstructionData)
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Deserialize instruction data
    let instruction = ProgramInstruction::unpack(instruction_data)?;

    // Process instruction
    match instruction {
        ProgramInstruction::SolTransfer { amount } => {
            // Parse accounts
            let [pda_account_info, recipient_info, system_program_info] = accounts else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };

            // Derive PDA and verify it matches the account provided by client
            let recipient_pubkey = recipient_info.key;
            let seeds = &[b"pda", recipient_pubkey.as_ref()];
            let (expected_pda, bump_seed) = Pubkey::find_program_address(seeds, program_id);

            if expected_pda != *pda_account_info.key {
                return Err(ProgramError::InvalidArgument);
            }

            // Create the transfer instruction
            let transfer_ix = system_instruction::transfer(
                pda_account_info.key,
                recipient_info.key,
                amount,
            );

            // Create signer seeds for PDA
            let signer_seeds: &[&[&[u8]]] = &[&[b"pda", recipient_pubkey.as_ref(), &[bump_seed]]];

            // Invoke the transfer instruction with PDA as signer
            invoke_signed(
                &transfer_ix,
                &[
                    pda_account_info.clone(),
                    recipient_info.clone(),
                    system_program_info.clone(),
                ],
                signer_seeds,
            )?;

            Ok(())
        }
    }
}
```

```ts !! title="Test"
import * as path from "path";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction
} from "@solana/web3.js";
import { LiteSVM } from "litesvm";

test("sol transfer cpi with pda signer", () => {
  const svm = new LiteSVM();

  const programId = PublicKey.unique();
  const programPath = path.join(__dirname, "program.so");
  svm.addProgramFromFile(programId, programPath);

  // Create recipient
  const recipient = new Keypair();

  // Derive PDA that will hold and send funds
  const [pdaAddress] = PublicKey.findProgramAddressSync(
    [Buffer.from("pda"), recipient.publicKey.toBuffer()],
    programId
  );

  // Fund accounts
  const amount = BigInt(LAMPORTS_PER_SOL);
  svm.airdrop(recipient.publicKey, amount); // 1 SOL
  svm.airdrop(pdaAddress, amount); // 1 SOL

  // Create instruction data buffer
  const transferAmount = amount / BigInt(2); // 0.5 SOL
  const instructionIndex = 0; // instruction index 0 for SolTransfer enum

  const data = Buffer.alloc(9); // 1 byte for instruction enum + 8 bytes for u64
  data.writeUInt8(instructionIndex, 0); // first byte identifies the instruction
  data.writeBigUInt64LE(transferAmount, 1); // remaining bytes are instruction arguments

  // Create instruction
  const instruction = new TransactionInstruction({
    programId,
    keys: [
      { pubkey: pdaAddress, isSigner: false, isWritable: true },
      { pubkey: recipient.publicKey, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
    ],
    data
  });

  // Create and send transaction
  const transaction = new Transaction().add(instruction);
  transaction.recentBlockhash = svm.latestBlockhash();
  transaction.sign(recipient);

  svm.sendTransaction(transaction);

  // Check balances
  const recipientBalance = svm.getBalance(recipient.publicKey);
  const pdaBalance = svm.getBalance(pdaAddress);

  const transactionFee = BigInt(5000);
  // Recipient starts with 1 SOL, receives 0.5 SOL, pays tx fee
  expect(recipientBalance).toBe(amount + transferAmount - transactionFee);
  // PDA starts with 1 SOL, sends 0.5 SOL
  expect(pdaBalance).toBe(amount - transferAmount);
});
```

</CodeTabs>

>>> solana-com/content/docs/en/core/pda.mdx
---
title: Program Derived Address
description:
  Learn about Program Derived Addresses (PDAs) on Solana - deterministic account
  addresses that enable secure program signing. Understand PDA derivation,
  canonical bumps, and how to create PDA accounts.
h1: Program Derived Address (PDA)
---

Program Derived Addresses (PDAs) provide developers on Solana with two main use
cases:

- **Deterministic Account Addresses**: PDAs provide a mechanism to
  deterministically create an address using a combination of optional "seeds"
  (predefined inputs) and a specific program ID.
- **Enable Program Signing**: The Solana runtime enables programs to "sign" for
  PDAs which are derived from the program's address.

You can think of PDAs as a way to create hashmap-like structures on-chain from a
predefined set of inputs (e.g. strings, numbers, and other account addresses).

The benefit of this approach is that it eliminates the need to keep track of an
exact address. Instead, you simply need to recall the specific inputs used for
its derivation.

![Program Derived Address](/assets/docs/core/pda/pda.svg)

It's important to understand that simply deriving a Program Derived Address
(PDA) doesn't automatically create an on-chain account at that address. Accounts
with a PDA as the on-chain address must be explicitly created through the
program used to derive the address. You can think of deriving a PDA as finding
an address on a map. Just having an address doesn't mean there is anything built
at that location.

<Callout type="info">
  This section covers the details of deriving PDAs. The section on [Cross
  Program Invocations (CPIs)](/docs/core/cpi) explains how programs use PDAs for
  signing.
</Callout>

## Key Points

- PDAs are addresses **derived deterministically** using a combination of
  predefined seeds, a bump seed, and a program's ID.
- PDAs are addresses that fall off the Ed25519 curve and **have no corresponding
  private key**.
- Solana programs can **sign on behalf of PDAs derived from its program ID**.
- Deriving a PDA doesn't automatically create an on-chain account.
- An account using a PDA as its address must be created through an instruction
  within a Solana program.

## What's a PDA

PDAs are addresses that derive deterministically that look like public keys, but
have no private keys. This means it is not possible to generate a valid
signature for the address. However, the Solana runtime enables programs to
"sign" for PDAs without needing a private key.

For context, Solana
[Keypairs](https://github.com/anza-xyz/solana-sdk/blob/sdk%40v2.2.2/keypair/src/lib.rs#L26)
are points on the Ed25519 curve (elliptic-curve cryptography) with a public key
and corresponding private key. Public keys are used as addresses (unique
identifier) for on-chain accounts.

![On Curve Address](/assets/docs/core/pda/address-on-curve.svg)

A PDA is a point that's intentionally derived to fall off the Ed25519 curve
using a predefined set of inputs. A point that's not on the Ed25519 curve does
not have a valid corresponding private key and can't perform cryptographic
operations (signing).

A PDA can serve as the address (unique identifier) for an on-chain account,
providing a method to easily store, map, and fetch program state.

![Off Curve Address](/assets/docs/core/pda/address-off-curve.svg)

## How to derive a PDA

The derivation of a PDA requires three inputs:

- **Optional seeds**: Predefined inputs (e.g. strings, numbers, other account
  addresses) for PDA derivation.
- **Bump seed**: An extra byte appended to the optional seeds to ensure a valid
  PDA (off curve) is generated. The bump seed starts at 255 and decrements by 1
  until a valid PDA is found.
- **Program ID**: The address of the program from which the PDA is derived. This
  program can sign on behalf of the PDA.

![PDA Derivation](/assets/docs/core/pda/pda-derivation.svg)

Use the following functions from the respective SDKs to derive a PDA.

| SDK                            | Function                                                                                                                         |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `@solana/kit` (Typescript)     | [`getProgramDerivedAddress`](https://github.com/anza-xyz/kit/blob/v2.1.0/packages/addresses/src/program-derived-address.ts#L157) |
| `@solana/web3.js` (Typescript) | [`findProgramAddressSync`](https://github.com/solana-foundation/solana-web3.js/blob/v1.98.0/src/publickey.ts#L212)               |
| `solana_sdk` (Rust)            | [`find_program_address`](https://github.com/anza-xyz/solana-sdk/blob/sdk%40v2.2.2/pubkey/src/lib.rs#L617)                        |

To derive a PDA, provide the following inputs to the SDK function:

- The predefined optional seeds converted to bytes
- The program ID (address) used for derivation

Once a valid PDA is found, the function returns both the address (PDA) and the
bump seed used for derivation.

### Examples

The following examples show how to derive a PDA using the respective SDKs.

Click the "Run" button to execute the code.

#### Derive a PDA with optional string seed

<CodeTabs storage="pda-examples" flags="r">

```ts !! title="Kit"
import { Address, getProgramDerivedAddress } from "@solana/kit";

const programAddress = "11111111111111111111111111111111" as Address;

const seeds = ["helloWorld"];
const [pda, bump] = await getProgramDerivedAddress({
  programAddress,
  seeds
});

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```ts !! title="Legacy"
import { PublicKey } from "@solana/web3.js";

const programAddress = new PublicKey("11111111111111111111111111111111");

const seeds = [Buffer.from("helloWorld")];
const [pda, bump] = await PublicKey.findProgramAddressSync(
  seeds,
  programAddress
);

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```rs !! title="Rust"
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let program_address = Pubkey::from_str("11111111111111111111111111111111")?;

    let seeds: &[&[u8]] = &[b"helloWorld"];
    let (pda, bump) = Pubkey::find_program_address(seeds, &program_address);

    println!("PDA: {}", pda);
    println!("Bump: {}", bump);
    Ok(())
}
```

</CodeTabs>

#### Derive a PDA with optional address seed

<CodeTabs storage="pda-examples"flags="r">

```ts !! title="Kit"
import {
  Address,
  getAddressEncoder,
  getProgramDerivedAddress
} from "@solana/kit";

const programAddress = "11111111111111111111111111111111" as Address;

const addressEncoder = getAddressEncoder();
const optionalSeedAddress = addressEncoder.encode(
  "B9Lf9z5BfNPT4d5KMeaBFx8x1G4CULZYR1jA2kmxRDka" as Address
);
const seeds = [optionalSeedAddress];
const [pda, bump] = await getProgramDerivedAddress({
  programAddress,
  seeds
});

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```ts !! title="Legacy"
import { PublicKey } from "@solana/web3.js";

const programAddress = new PublicKey("11111111111111111111111111111111");

const optionalSeedAddress = new PublicKey(
  "B9Lf9z5BfNPT4d5KMeaBFx8x1G4CULZYR1jA2kmxRDka"
);
const seeds = [optionalSeedAddress.toBuffer()];
const [pda, bump] = await PublicKey.findProgramAddressSync(
  seeds,
  programAddress
);

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```rs !! title="Rust"
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let program_address = Pubkey::from_str("11111111111111111111111111111111")?;

    let optional_seed_address = Pubkey::from_str("B9Lf9z5BfNPT4d5KMeaBFx8x1G4CULZYR1jA2kmxRDka")?;
    let seeds: &[&[u8]] = &[optional_seed_address.as_ref()];
    let (pda, bump) = Pubkey::find_program_address(seeds, &program_address);

    println!("PDA: {}", pda);
    println!("Bump: {}", bump);
    Ok(())
}
```

</CodeTabs>

#### Derive a PDA with multiple optional seeds

<CodeTabs storage="pda-examples" flags="r">

```ts !! title="Kit"
import {
  Address,
  getAddressEncoder,
  getProgramDerivedAddress
} from "@solana/kit";

const programAddress = "11111111111111111111111111111111" as Address;

const optionalSeedString = "helloWorld";
const addressEncoder = getAddressEncoder();
const optionalSeedAddress = addressEncoder.encode(
  "B9Lf9z5BfNPT4d5KMeaBFx8x1G4CULZYR1jA2kmxRDka" as Address
);
const seeds = [optionalSeedString, optionalSeedAddress];
const [pda, bump] = await getProgramDerivedAddress({
  programAddress,
  seeds
});

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```ts !! title="Legacy"
import { PublicKey } from "@solana/web3.js";

const programAddress = new PublicKey("11111111111111111111111111111111");

const optionalSeedString = "helloWorld";
const optionalSeedAddress = new PublicKey(
  "B9Lf9z5BfNPT4d5KMeaBFx8x1G4CULZYR1jA2kmxRDka"
);
const seeds = [Buffer.from(optionalSeedString), optionalSeedAddress.toBuffer()];
const [pda, bump] = await PublicKey.findProgramAddressSync(
  seeds,
  programAddress
);

console.log(`PDA: ${pda}`);
console.log(`Bump: ${bump}`);
```

```rs !! title="Rust"
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let program_address = Pubkey::from_str("11111111111111111111111111111111")?;

    let optional_seed_bytes = b"helloWorld";
    let optional_seed_address = Pubkey::from_str("B9Lf9z5BfNPT4d5KMeaBFx8x1G4CULZYR1jA2kmxRDka")?;
    let seeds: &[&[u8]] = &[optional_seed_bytes, optional_seed_address.as_ref()];
    let (pda, bump) = Pubkey::find_program_address(seeds, &program_address);

    println!("PDA: {}", pda);
    println!("Bump: {}", bump);
    Ok(())
}
```

</CodeTabs>

### Canonical Bump

PDA derivation requires a "bump seed", an extra byte appended to the optional
seeds. The derivation function iterates through bump values, starting at 255 and
decrementing by 1, until a value produces a valid off-curve address. The first
bump value that produces a valid off-curve address is the "canonical bump."

The following examples show PDA derivation using all possible bump seeds (255 to
0):

<Callout type="info">
  Kit example not included because the
  [createProgramDerivedAddress](https://github.com/anza-xyz/kit/blob/v2.1.0/packages/addresses/src/program-derived-address.ts#L101)
  function isn't exported.
</Callout>

<CodeTabs flags="r">

```ts !! title="Legacy"
import { PublicKey } from "@solana/web3.js";

const programId = new PublicKey("11111111111111111111111111111111");
const optionalSeed = "helloWorld";

// Loop through all bump seeds (255 down to 0)
for (let bump = 255; bump >= 0; bump--) {
  try {
    const PDA = PublicKey.createProgramAddressSync(
      [Buffer.from(optionalSeed), Buffer.from([bump])],
      programId
    );
    console.log("bump " + bump + ": " + PDA);
  } catch (error) {
    console.log("bump " + bump + ": " + error);
  }
}
```

```rs !! title="Rust"
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let program_id = Pubkey::from_str("11111111111111111111111111111111")?;
    let optional_seed = b"helloWorld";

    // Loop through all bump seeds (255 down to 0)
    for bump in (0..=255).rev() {
        match Pubkey::create_program_address(&[optional_seed.as_ref(), &[bump]], &program_id) {
            Ok(pda) => println!("bump {}: {}", bump, pda),
            Err(err) => println!("bump {}: {}", bump, err),
        }
    }

    Ok(())
}
```

</CodeTabs>

<CodeTabs>

```sh !! title="Expected TS Output"
bump 255: Error: Invalid seeds, address must fall off the curve
bump 254: 46GZzzetjCURsdFPb7rcnspbEMnCBXe9kpjrsZAkKb6X
bump 253: GBNWBGxKmdcd7JrMnBdZke9Fumj9sir4rpbruwEGmR4y
bump 252: THfBMgduMonjaNsCisKa7Qz2cBoG1VCUYHyso7UXYHH
bump 251: EuRrNqJAofo7y3Jy6MGvF7eZAYegqYTwH2dnLCwDDGdP
bump 250: Error: Invalid seeds, address must fall off the curve
...
// remaining bump outputs
```

```sh !! title="Expected Rust Output"
bump 255: Provided seeds do not result in a valid address
bump 254: 46GZzzetjCURsdFPb7rcnspbEMnCBXe9kpjrsZAkKb6X
bump 253: GBNWBGxKmdcd7JrMnBdZke9Fumj9sir4rpbruwEGmR4y
bump 252: THfBMgduMonjaNsCisKa7Qz2cBoG1VCUYHyso7UXYHH
bump 251: EuRrNqJAofo7y3Jy6MGvF7eZAYegqYTwH2dnLCwDDGdP
bump 250: Provided seeds do not result in a valid address
...
// remaining bump outputs
```

</CodeTabs>

The bump seed 255 throws an error and the first bump seed to derive a valid PDA
is 254.

Note that bump seeds 253-251 all derive valid PDAs with different addresses.
This means that given the same optional seeds and `programId`, a bump seed with
a different value can still derive a valid PDA.

<Callout type="warn">
  When building Solana programs, always include security checks to ensure a PDA
  passed to the program is derived from the canonical bump. Failing to include
  these checks may introduce vulnerabilities that allow unexpected accounts to
  be used in the program instructions. It is best practice to only use the
  canonical bump when deriving PDAs.
</Callout>

## Create PDA Accounts

The example program below shows how to create an account using a PDA as the
address of the new account. The example program uses the
[Anchor framework](https://www.anchor-lang.com/docs).

The program includes a single `initialize` instruction to create a new account
using a PDA as the address of the account. The new account stores the address of
the `user` and the `bump` seed used to derive the PDA.

<CodeTabs>

```rs !! title="Program"
use anchor_lang::prelude::*;

declare_id!("75GJVCJNhaukaa2vCCqhreY31gaphv7XTScBChmr1ueR");

#[program]
pub mod pda_account {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let account_data = &mut ctx.accounts.pda_account;
        // store the address of the `user`
        // !mark
        account_data.user = *ctx.accounts.user.key;
        // store the canonical bump
        // !mark
        account_data.bump = ctx.bumps.pda_account;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        // define the seeds to derive the PDA
        // !mark
        seeds = [b"data", user.key().as_ref()],
        // use the canonical bump
        // !mark
        bump,
        payer = user,
        space = 8 + DataAccount::INIT_SPACE
    )]
    pub pda_account: Account<'info, DataAccount>,
    pub system_program: Program<'info, System>,
}

#[account]

#[derive(InitSpace)]
pub struct DataAccount {
    pub user: Pubkey,
    pub bump: u8,
}
```

```ts !! title="Test"
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PdaAccount } from "../target/types/pda_account";
import { PublicKey } from "@solana/web3.js";

describe("pda-account", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PdaAccount as Program<PdaAccount>;
  const user = provider.wallet as anchor.Wallet;

  // Derive the PDA address using the seeds specified on the program
  const [PDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("data"), user.publicKey.toBuffer()],
    program.programId
  );

  it("Is initialized!", async () => {
    const transactionSignature = await program.methods
      .initialize()
      .accounts({
        user: user.publicKey
      })
      .rpc();

    console.log("Transaction Signature:", transactionSignature);
  });

  it("Fetch Account", async () => {
    const pdaAccount = await program.account.dataAccount.fetch(PDA);
    console.log(JSON.stringify(pdaAccount, null, 2));
  });
});
```

</CodeTabs>

In this example, the seeds for PDA derivation include the fixed string `data`
and the address of the `user` account provided in the instruction. The Anchor
framework automatically finds the canonical `bump` seed.

```rust title="pda_account"
#[account(
    init,
    // !mark[/seeds/]
    seeds = [b"data", user.key().as_ref()],
    // !mark[/bump/]
    bump,
    payer = user,
    space = 8 + DataAccount::INIT_SPACE
)]
pub pda_account: Account<'info, DataAccount>,
```

The `init` constraint instructs Anchor to invoke the System Program to create a
new account using the PDA as the address. Anchor does this through a
[CPI](/docs/core/cpi).

```rust title="pda_account"
#[account(
    // !mark[/init/]
    init,
    seeds = [b"data", user.key().as_ref()],
    bump,
    payer = user,
    space = 8 + DataAccount::INIT_SPACE
)]
pub pda_account: Account<'info, DataAccount>,
```

The test file contains the Typescript code to derive the PDA.

```ts title="Derive PDA"
const [PDA] = PublicKey.findProgramAddressSync(
  // !mark
  [Buffer.from("data"), user.publicKey.toBuffer()],
  program.programId
);
```

The transaction in the test file invokes the `initialize` instruction to create
a new on-chain account using the PDA as the address. In this example, Anchor can
infer the PDA address in the instruction accounts, so it doesn't need to be
explicitly provided.

```ts title="Invoke Initialize Instruction"
it("Is initialized!", async () => {
  const transactionSignature = await program.methods
    // !mark
    .initialize()
    .accounts({
      user: user.publicKey
    })
    .rpc();

  console.log("Transaction Signature:", transactionSignature);
});
```

The test file also shows how fetch the on-chain account created at that address
once the transaction is sent.

```ts title="Fetch Account"
it("Fetch Account", async () => {
  // !mark
  const pdaAccount = await program.account.dataAccount.fetch(PDA);
  console.log(JSON.stringify(pdaAccount, null, 2));
});
```

Note that in this example, if you invoke the `initialize` instruction more than
once using the same `user` address as a seed, then the transaction fails. This
happens because an account already exists at the derived address.

>>> solana-com/content/docs/en/core/programs.mdx
---
title: Programs on Solana
description:
  Learn about Solana programs (smart contracts) and how to develop them using
  Rust or the Anchor framework. Understand program deployment, upgrades, and
  verification on the Solana network.
h1: Programs
---

On Solana, "smart contracts" are called programs.
[Programs](/docs/core/accounts#program-account) are deployed on-chain to
accounts that contain the program's compiled executable binary. Users interact
with programs by sending transactions containing
[instructions](/docs/core/transactions#instruction) that tell the program what
to do.

## Key Points

- Programs are accounts containing **executable code**, organized into functions
  called **instructions**.
- While programs are **stateless**, they can include instructions that create
  and update other accounts to store data.
- An **upgrade authority** can update programs. Once this authority is removed,
  the program becomes immutable.
- Users can verify an on-chain program account's data matches its public source
  code through verifiable builds.

## Writing Solana Programs

Solana programs are predominantly written in the
[Rust](https://rust-book.cs.brown.edu/title-page.html) programming language,
with two common approaches for development:

- [Anchor](https://www.anchor-lang.com/docs): A framework designed for Solana
  program development. It provides a faster and simpler way to write programs,
  using Rust macros to reduce boilerplate code. For beginners, it is recommended
  to start with the Anchor framework.

- [Native Rust](/docs/programs/rust): This approach involves writing Solana
  programs in Rust without leveraging any frameworks. It offers more flexibility
  but comes with increased complexity.

## Updating Solana Programs

To learn more about deploying and upgrading programs, see the
[deploying programs](/docs/programs/deploying) page.

Programs can be
[directly modified](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/bpf_loader/src/lib.rs#L704)
by an account designated as the "upgrade authority", which is typically the
account that originally deployed the program. If the
[upgrade authority](https://github.com/anza-xyz/agave/blob/v2.1.13/programs/bpf_loader/src/lib.rs#L894)
is revoked and set to `None`, the program becomes immutable and can no longer be
updated.

## Verifiable Programs

Verifiable builds allow anyone to check if a program's on-chain code matches its
public source code, making it possible to detect discrepancies between source
and deployed versions.

The Solana developer community has introduced tools to support verifiable
builds, enabling both developers and users to verify that on-chain programs
accurately reflect their publicly shared source code.

- **Searching for Verified Programs**: To quickly check for verified programs,
  users can search for a program address on
  [Solana Explorer](https://explorer.solana.com/). View an example of a verified
  program
  [here](https://explorer.solana.com/address/PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY).

- **Verification Tools**: The
  [Solana Verifiable Build CLI](https://github.com/Ellipsis-Labs/solana-verifiable-build)
  by Ellipsis Labs enables users to independently verify on-chain programs
  against published source code.

- **Support for Verifiable Builds in Anchor**: Anchor provides built-in support
  for verifiable builds. Details can be found in the
  [Anchor documentation](https://www.anchor-lang.com/docs/verifiable-builds).

## Berkeley Packet Filter (BPF)

Solana uses [LLVM](https://llvm.org/) (Low Level Virtual Machine) to compile
programs into
[ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format) (Executable
and Linkable Format) files. These files contain Solana's custom version of
[eBPF](https://en.wikipedia.org/wiki/EBPF) bytecode, called "Solana Bytecode
Format" (sBPF). The ELF file contains the program's binary and is stored
on-chain in an executable account when the program is deployed.

## Built-in Programs

### Loader Programs

Every program itself is owned by another program, which is its loader.
Currently, five loaders programs exist:

| Loader | Program ID                                    | Notes                                                            | Instructions Link                                                                                                                               |
| ------ | --------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| native | `NativeLoader1111111111111111111111111111111` | Owns the other four loaders                                      | —                                                                                                                                               |
| v1     | `BPFLoader1111111111111111111111111111111111` | Management instructions are disabled, but programs still execute | —                                                                                                                                               |
| v2     | `BPFLoader2111111111111111111111111111111111` | Management instructions are disabled, but programs still execute | [Instructions](https://docs.rs/solana-loader-v2-interface/latest/solana_loader_v2_interface/enum.LoaderInstruction.html)                        |
| v3     | `BPFLoaderUpgradeab1e11111111111111111111111` | Is being phased out                                              | [Instructions](https://docs.rs/solana-loader-v3-interface/latest/solana_loader_v3_interface/instruction/enum.UpgradeableLoaderInstruction.html) |
| v4     | `LoaderV411111111111111111111111111111111111` | v4 is expected to become the standard loader                     | [Instructions](https://docs.rs/solana-loader-v4-interface/latest/solana_loader_v4_interface/instruction/enum.LoaderV4Instruction.html)          |

These loaders are necessary to create and manage custom programs:

- Deploy a new program or buffer
- Close a program or buffer
- Redeploy / upgrade an existing program
- Transfer the authority over a program
- Finalize a program

Loader-v3 and loader-v4 support modifications to programs after their initial
deployment. Permission to do so is regulated by the authority of a program
because the account ownership of each program resides with the loader.

### Precompiled Programs

#### Ed25519 Program

| Program         | Program ID                                    | Description                                                                | Instructions                                                                                    |
| --------------- | --------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Ed25519 Program | `Ed25519SigVerify111111111111111111111111111` | Verifies ed25519 signatures. If any signature fails, an error is returned. | [Instructions](https://docs.rs/solana-ed25519-program/latest/solana_ed25519_program/index.html) |

The ed25519 program processes an instruction. The first `u8` is a count of the
number of signatures to check, which is followed by a single byte padding. After
that, the following struct is serialized, one for each signature to check.

```rs title="Ed25519SignatureOffsets"
struct Ed25519SignatureOffsets {
    signature_offset: u16,             // offset to ed25519 signature of 64 bytes
    signature_instruction_index: u16,  // instruction index to find signature
    public_key_offset: u16,            // offset to public key of 32 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get message data
}
```

The pseudo code of the signature verification:

```
process_instruction() {
    for i in 0..count {
        // i'th index values referenced:
        instructions = &transaction.message().instructions
        instruction_index = ed25519_signature_instruction_index != u16::MAX ? ed25519_signature_instruction_index : current_instruction;
        signature = instructions[instruction_index].data[ed25519_signature_offset..ed25519_signature_offset + 64]
        instruction_index = ed25519_pubkey_instruction_index != u16::MAX ? ed25519_pubkey_instruction_index : current_instruction;
        pubkey = instructions[instruction_index].data[ed25519_pubkey_offset..ed25519_pubkey_offset + 32]
        instruction_index = ed25519_message_instruction_index != u16::MAX ? ed25519_message_instruction_index : current_instruction;
        message = instructions[instruction_index].data[ed25519_message_data_offset..ed25519_message_data_offset + ed25519_message_data_size]
        if pubkey.verify(signature, message) != Success {
            return Error
        }
    }
    return Success
}
```

#### Secp256k1 Program

| Program           | Program ID                                    | Description                                                    | Instructions                                                                                        |
| ----------------- | --------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Secp256k1 Program | `KeccakSecp256k11111111111111111111111111111` | Verifies secp256k1 public key recovery operations (ecrecover). | [Instructions](https://docs.rs/solana-secp256k1-program/latest/solana_secp256k1_program/index.html) |

The secp256k1 program processes an instruction which takes in as the first byte
a count of the following struct serialized in the instruction data:

```rs title="Secp256k1SignatureOffsets"
struct Secp256k1SignatureOffsets {
    secp_signature_offset: u16,            // offset to [signature,recovery_id] of 64+1 bytes
    secp_signature_instruction_index: u8,  // instruction index to find signature
    secp_pubkey_offset: u16,               // offset to ethereum_address pubkey of 20 bytes
    secp_pubkey_instruction_index: u8,     // instruction index to find pubkey
    secp_message_data_offset: u16,         // offset to start of message data
    secp_message_data_size: u16,           // size of message data
    secp_message_instruction_index: u8,    // instruction index to find message data
}
```

The pseudo code of the recovery verification:

```
process_instruction() {
  for i in 0..count {
      // i'th index values referenced:
      instructions = &transaction.message().instructions
      signature = instructions[secp_signature_instruction_index].data[secp_signature_offset..secp_signature_offset + 64]
      recovery_id = instructions[secp_signature_instruction_index].data[secp_signature_offset + 64]
      ref_eth_pubkey = instructions[secp_pubkey_instruction_index].data[secp_pubkey_offset..secp_pubkey_offset + 20]
      message_hash = keccak256(instructions[secp_message_instruction_index].data[secp_message_data_offset..secp_message_data_offset + secp_message_data_size])
      pubkey = ecrecover(signature, recovery_id, message_hash)
      eth_pubkey = keccak256(pubkey[1..])[12..]
      if eth_pubkey != ref_eth_pubkey {
          return Error
      }
  }
  return Success
}
```

This allows the user to specify any instruction data in the transaction for
signature and message data. By specifying a special instructions sysvar, one can
also receive data from the transaction itself.

Cost of the transaction will count the number of signatures to verify multiplied
by the signature cost verify multiplier.

#### Secp256r1 Program

| Program           | Program ID                                    | Description                                                                                                   | Instructions                                                                                        |
| ----------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Secp256r1 Program | `Secp256r1SigVerify1111111111111111111111111` | Verifies up to 8 secp256r1 signatures. Takes a signature, public key, and message. Returns error if any fail. | [Instructions](https://docs.rs/solana-secp256r1-program/latest/solana_secp256r1_program/all.html) |

The secp256r1 program processes an instruction. The first `u8` is a count of the
number of signatures to check, followed by a single byte padding. After that,
the following struct is serialized, one for each signature to check:

```rs title="Secp256r1SignatureOffsets"
struct Secp256r1SignatureOffsets {
    signature_offset: u16,             // offset to compact secp256r1 signature of 64 bytes
    signature_instruction_index: u16,  // instruction index to find signature
    public_key_offset: u16,            // offset to compressed public key of 33 bytes
    public_key_instruction_index: u16, // instruction index to find public key
    message_data_offset: u16,          // offset to start of message data
    message_data_size: u16,            // size of message data
    message_instruction_index: u16,    // index of instruction data to get message data
}

```

The pseudo code of the signature verification:

```
process_instruction() {
    if data.len() < SIGNATURE_OFFSETS_START {
        return Error
    }

    num_signatures = data[0] as usize
    if num_signatures == 0 || num_signatures > 8 {
        return Error
    }

    expected_data_size = num_signatures * SIGNATURE_OFFSETS_SERIALIZED_SIZE + SIGNATURE_OFFSETS_START
    if data.len() < expected_data_size {
        return Error
    }

    for i in 0..num_signatures {
        offsets = parse_signature_offsets(data, i)

        signature = get_data_slice(data, instruction_datas, offsets.signature_instruction_index, offsets.signature_offset, SIGNATURE_SERIALIZED_SIZE)

        if s > half_curve_order {
            return Error
        }

        pubkey = get_data_slice(data, instruction_datas, offsets.public_key_instruction_index, offsets.public_key_offset, COMPRESSED_PUBKEY_SERIALIZED_SIZE)

        message = get_data_slice(data, instruction_datas, offsets.message_instruction_index, offsets.message_data_offset, offsets.message_data_size)

        if !verify_signature(signature, pubkey, message) {
            return Error
        }
    }

    return Success
}
```

Note: Low S values are enforced for all signatures to avoid accidental signature
malleability.

### Core Programs

The Solana cluster genesis includes a list of special programs that provide core
functionalities for the network. Historically these were referred to as "native"
programs and they used to be distributed together with the validator code.

| Program                          | Program ID                                    | Description                                                                                                                                                                                                                                                                                                                                                 | Instructions                                                                                                                                          |
| -------------------------------- | --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **System Program**               | `11111111111111111111111111111111`            | Create new accounts, allocate account data, assign accounts to owning programs, transfer lamports from System Program owned accounts, and pay transaction fees.                                                                                                                                                                                             | [SystemInstruction](https://docs.rs/solana-program/latest/solana_program/system_instruction/enum.SystemInstruction.html)                              |
| **Vote Program**                 | `Vote111111111111111111111111111111111111111` | Create and manage accounts that track validator voting state and rewards.                                                                                                                                                                                                                                                                                   | [VoteInstruction](https://docs.rs/solana-vote-program/latest/solana_vote_program/vote_instruction/enum.VoteInstruction.html)                          |
| **Stake Program**                | `Stake11111111111111111111111111111111111111` | Create and manage accounts representing stake and rewards for delegations to validators.                                                                                                                                                                                                                                                                    | [StakeInstruction](https://docs.rs/solana-sdk/latest/solana_sdk/stake/instruction/enum.StakeInstruction.html)                                         |
| **Config Program**               | `Config1111111111111111111111111111111111111` | Add configuration data to the chain, followed by the list of public keys that are allowed to modify it. Unlike the other programs, the Config program does not define any individual instructions. It has just one implicit instruction: "store". Its instruction data is a set of keys that gate access to the account and the data to store inside of it. | [ConfigInstruction](https://docs.rs/solana-config-program/latest/solana_config_program/config_instruction/index.html)                                 |
| **Compute Budget Program**       | `ComputeBudget111111111111111111111111111111` | Set compute unit limits and prices for transactions, allowing users to control compute resources and prioritization fees.                                                                                                                                                                                                                                   | [ComputeBudgetInstruction](https://docs.rs/solana-compute-budget-interface/latest/solana_compute_budget_interface/enum.ComputeBudgetInstruction.html) |
| **Address Lookup Table Program** | `AddressLookupTab1e1111111111111111111111111` | Manage address lookup tables, which allow transactions to reference more accounts than would otherwise fit in the transaction's account list.                                                                                                                                                                                                               | [ProgramInstruction](https://docs.rs/solana-sdk/latest/solana_sdk/address_lookup_table/instruction/enum.ProgramInstruction.html)                      |
| **ZK ElGamal Proof Program**     | `ZkE1Gama1Proof11111111111111111111111111111` | Provides zero-knowledge proof verification for ElGamal-encrypted data.                                                                                                                                                                                                                                                                                      | —                                                                                                                                                     |

</PART1>
<PART 2 - RAW CODE EXAMPLE>
>>> program-examples/tools/shank-and-solita/native/program/src/state/mod.rs
use {
    borsh::{
        BorshDeserialize,
        BorshSerialize
    },
    shank::ShankAccount,
    solana_program::pubkey::Pubkey,
};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankAccount)]
#[seeds(
    "car",
    program_id,
    make("The car's make", String),
    model("The car's model", String),
)]
pub struct Car {
    pub year: u16,
    pub make: String,
    pub model: String,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub enum RentalOrderStatus {
    Created,
    PickedUp,
    Returned,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankAccount)]
#[seeds(
    "rental_order",
    program_id,
    car_public_key("The car's public key", Pubkey),
    payer_public_key("The payer's public key", Pubkey),
)]
pub struct RentalOrder {
    pub car: Pubkey,
    pub name: String,
    pub pick_up_date: String,
    pub return_date: String,
    pub price: u64,
    pub status: RentalOrderStatus,
}

impl RentalOrder {
    pub const SEED_PREFIX: &'static str = "rental_order";
}

>>> program-examples/tools/shank-and-solita/native/program/src/lib.rs
mod instructions;
mod state;

use {
    borsh::BorshDeserialize,
    solana_program::{
        account_info::AccountInfo, 
        declare_id,
        entrypoint, 
        entrypoint::ProgramResult, 
        pubkey::Pubkey,
    },
};
use crate::instructions::*;

declare_id!("8avNGHVXDwsELJaWMSoUZ44CirQd4zyU9Ez4ZmP4jNjZ");
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {

    let instruction = CarRentalServiceInstruction::try_from_slice(instruction_data)?;
    match instruction {
        CarRentalServiceInstruction::AddCar(car) => add_car(program_id, accounts, car),
        CarRentalServiceInstruction::BookRental(order) => book_rental(program_id, accounts, order),
        CarRentalServiceInstruction::PickUpCar => pick_up_car(program_id, accounts),
        CarRentalServiceInstruction::ReturnCar => return_car(program_id, accounts),
    }
}
>>> program-examples/tools/shank-and-solita/native/program/src/instructions/return_car.rs
use {
    borsh::{
        BorshDeserialize,
        BorshSerialize,
    },
    solana_program::{
        account_info::{AccountInfo, next_account_info}, 
        entrypoint::ProgramResult, 
        pubkey::Pubkey,
    },
};
use crate::state::{
    RentalOrder,
    RentalOrderStatus,
};

pub fn return_car(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {

    let accounts_iter = &mut accounts.iter();
    let rental_order_account = next_account_info(accounts_iter)?;
    let car_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;

    let (rental_order_account_pda, _) = Pubkey::find_program_address(
        &[
            RentalOrder::SEED_PREFIX.as_bytes().as_ref(),
            car_account.key.as_ref(),
            payer.key.as_ref(),
        ],
        program_id,
    );
    assert!(&rental_order_account_pda == rental_order_account.key);

    let rental_order = &mut RentalOrder::try_from_slice(&rental_order_account.data.borrow())?;
    rental_order.status = RentalOrderStatus::Returned;
    rental_order.serialize(&mut &mut rental_order_account.data.borrow_mut()[..])?;

    Ok(())
}
>>> program-examples/tools/shank-and-solita/native/program/src/instructions/mod.rs
pub mod add_car;
pub mod book_rental;
pub mod pick_up_car;
pub mod return_car;

pub use add_car::*;
pub use book_rental::*;
pub use pick_up_car::*;
pub use return_car::*;

use {
    borsh::{
        BorshDeserialize, 
        BorshSerialize,
    },
    shank::ShankInstruction,
};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankInstruction)]
pub enum CarRentalServiceInstruction {
    
    #[account(0, writable, name="car_account",
              desc="The account that will represent the Car being created")]
    #[account(1, writable, name="payer",
            desc = "Fee payer")]
    #[account(2, name="system_program",
            desc = "The System Program")]
    AddCar(AddCarArgs),

    #[account(0, writable, name="rental_account",
              desc="The account that will represent the actual order for the rental")]
    #[account(1, name="car_account",
              desc="The account representing the Car being rented in this order")]
    #[account(2, writable, name="payer",
            desc = "Fee payer")]
    #[account(3, name="system_program",
            desc = "The System Program")]
    BookRental(BookRentalArgs),

    #[account(0, writable, name="rental_account",
              desc="The account representing the active rental")]
    #[account(1, name="car_account",
              desc="The account representing the Car being rented in this order")]
    #[account(2, writable, name="payer",
            desc = "Fee payer")]
    PickUpCar,

    #[account(0, writable, name="rental_account",
              desc="The account representing the active rental")]
    #[account(1, name="car_account",
              desc="The account representing the Car being rented in this order")]
    #[account(2, writable, name="payer",
            desc = "Fee payer")]
    ReturnCar,
}
>>> program-examples/tools/shank-and-solita/native/program/src/instructions/add_car.rs
use {
    borsh::{
        BorshDeserialize, 
        BorshSerialize 
    },
    shank::ShankAccount,
    solana_program::{
        account_info::{AccountInfo, next_account_info}, 
        entrypoint::ProgramResult, 
        program::invoke_signed,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
};
use crate::state::Car;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct AddCarArgs {
    pub year: u16,
    pub make: String,
    pub model: String,
}

pub fn add_car(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: AddCarArgs,
) -> ProgramResult {

    let accounts_iter = &mut accounts.iter();
    let car_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (car_account_pda, car_account_bump) = Car::shank_pda(program_id, args.make, args.model);
    assert!(&car_account_pda == car_account.key);

    let car_data = Car {
        year: args.year,
        make: args.make,
        model: args.model,
    };

    let account_span = (car_data.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke_signed(
        &system_instruction::create_account(
            &payer.key,
            &car_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            payer.clone(), car_account.clone(), system_program.clone()
        ],
        Car::shank_seeds_with_bump(args.make, args.model, &[car_account_bump]),
    )?;
    
    car_data.serialize(&mut &mut car_account.data.borrow_mut()[..])?;

    Ok(())
}
>>> program-examples/tools/shank-and-solita/native/program/src/instructions/book_rental.rs
use {
    borsh::{
        BorshDeserialize, 
        BorshSerialize 
    },
    shank::ShankAccount,
    solana_program::{
        account_info::{AccountInfo, next_account_info}, 
        entrypoint::ProgramResult, 
        program::invoke_signed,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
};
use crate::state::{
    RentalOrder,
    RentalOrderStatus,
};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct BookRentalArgs {
    pub name: String,
    pub pick_up_date: String,
    pub return_date: String,
    pub price: u64,
}

pub fn book_rental(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: BookRentalArgs,
) -> ProgramResult {

    let accounts_iter = &mut accounts.iter();
    let rental_order_account = next_account_info(accounts_iter)?;
    let car_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (rental_order_account_pda, rental_order_account_bump) = RentalOrder::shank_pda(program_id, car_account.key, payer.key);
    assert!(&rental_order_account_pda == rental_order_account.key);

    let rental_order_data = RentalOrder {
        car: *car_account.key,
        name: args.name,
        pick_up_date: args.pick_up_date,
        return_date: args.return_date,
        price: args.price,
        status: RentalOrderStatus::Created,
    };

    let account_span = (rental_order_data.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke_signed(
        &system_instruction::create_account(
            &payer.key,
            &rental_order_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            payer.clone(), rental_order_account.clone(), system_program.clone()
        ],
        RentalOrder::shank_seeds_with_bump(car_account.key, payer.key, &[rental_order_account_bump]),
    )?;
    
    rental_order_data.serialize(&mut &mut rental_order_account.data.borrow_mut()[..])?;

    Ok(())
}
>>> program-examples/tools/shank-and-solita/native/program/src/instructions/pick_up_car.rs
use {
    borsh::{
        BorshDeserialize,
        BorshSerialize,
    },
    solana_program::{
        account_info::{AccountInfo, next_account_info}, 
        entrypoint::ProgramResult, 
        pubkey::Pubkey,
    },
};
use crate::state::{
    RentalOrder,
    RentalOrderStatus,
};

pub fn pick_up_car(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {

    let accounts_iter = &mut accounts.iter();
    let rental_order_account = next_account_info(accounts_iter)?;
    let car_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;

    let (rental_order_account_pda, _) = Pubkey::find_program_address(
        &[
            RentalOrder::SEED_PREFIX.as_bytes().as_ref(),
            car_account.key.as_ref(),
            payer.key.as_ref(),
        ],
        program_id,
    );
    assert!(&rental_order_account_pda == rental_order_account.key);

    let rental_order = &mut RentalOrder::try_from_slice(&rental_order_account.data.borrow())?;
    rental_order.status = RentalOrderStatus::PickedUp;
    rental_order.serialize(&mut &mut rental_order_account.data.borrow_mut()[..])?;

    Ok(())
}
>>> program-examples/tokens/create-token/native/program/src/lib.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token::{instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_title: String,
    pub token_symbol: String,
    pub token_uri: String,
    pub token_decimals: u8,
}

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let args = CreateTokenArgs::try_from_slice(instruction_data)?;

    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    // First create the account for the Mint
    //
    msg!("Creating mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            (Rent::get()?).minimum_balance(Mint::LEN),
            Mint::LEN as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // Now initialize that account as a Mint (standard Mint)
    //
    msg!("Initializing mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.token_decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Now create the account for that Mint's metadata
    //
    msg!("Creating metadata account...");
    msg!("Metadata account address: {}", metadata_account.key);
    invoke(
        &mpl_instruction::create_metadata_accounts_v3(
            *token_metadata_program.key,
            *metadata_account.key,
            *mint_account.key,
            *mint_authority.key,
            *payer.key,
            *mint_authority.key,
            args.token_title,
            args.token_symbol,
            args.token_uri,
            None,
            0,
            true,
            false,
            None,
            None,
            None,
        ),
        &[
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Token mint created successfully.");

    Ok(())
}

>>> program-examples/tokens/spl-token-minter/native/program/src/lib.rs
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod instructions;
pub mod processor;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}

>>> program-examples/tokens/escrow/native/program/src/utils.rs
use crate::error::EscrowError;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

pub fn assert_is_associated_token_account(
    token_address: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<(), ProgramError> {
    let associated_token_account_address =
        &spl_associated_token_account::get_associated_token_address(owner, mint);

    if token_address != associated_token_account_address {
        return Err(EscrowError::TokenAccountMismatch.into());
    }

    Ok(())
}

>>> program-examples/tokens/escrow/native/program/src/state.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Offer {
    pub id: u64,
    pub maker: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_b_wanted_amount: u64,
    pub bump: u8,
}

impl Offer {
    pub const SEED_PREFIX: &'static [u8] = b"offer";
}

>>> program-examples/tokens/escrow/native/program/src/lib.rs
mod error;
mod instructions;
mod state;
mod utils;

use {
    borsh::{BorshDeserialize, BorshSerialize},
    instructions::*,
    solana_program::{
        account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
    },
};

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = EscrowInstruction::try_from_slice(instruction_data)?;

    match instruction {
        EscrowInstruction::MakeOffer(data) => MakeOffer::process(program_id, accounts, data),
        EscrowInstruction::TakeOffer => TakeOffer::process(program_id, accounts),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum EscrowInstruction {
    MakeOffer(MakeOffer),
    TakeOffer,
}

>>> program-examples/tokens/escrow/native/program/src/error.rs
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EscrowError {
    #[error("Offer key provided does not match expected")]
    OfferKeyMismatch,

    #[error("Token account provided does not match expected")]
    TokenAccountMismatch,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

>>> program-examples/tokens/spl-token-minter/native/program/src/instructions/mint.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MintToArgs {
    pub quantity: u64,
}

pub fn mint_to(accounts: &[AccountInfo], args: MintToArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let associated_token_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;

    if associated_token_account.lamports() == 0 {
        msg!("Creating associated token account...");
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                payer.key,
                mint_account.key,
                token_program.key,
            ),
            &[
                mint_account.clone(),
                associated_token_account.clone(),
                payer.clone(),
                system_program.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    } else {
        msg!("Associated token account exists.");
    }
    msg!("Associated Token Address: {}", associated_token_account.key);

    msg!(
        "Minting {} tokens to associated token account...",
        args.quantity
    );
    invoke(
        &token_instruction::mint_to(
            token_program.key,
            mint_account.key,
            associated_token_account.key,
            mint_authority.key,
            &[mint_authority.key],
            args.quantity,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            associated_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Tokens minted to wallet successfully.");

    Ok(())
}

>>> program-examples/tokens/spl-token-minter/native/program/src/instructions/mod.rs
pub mod create;
pub mod mint;

pub use create::*;
pub use mint::*;

>>> program-examples/tokens/spl-token-minter/native/program/src/instructions/create.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_pack::Pack,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token::{instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_title: String,
    pub token_symbol: String,
    pub token_uri: String,
}

pub fn create_token(accounts: &[AccountInfo], args: CreateTokenArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    // First create the account for the Mint
    //
    msg!("Creating mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            (Rent::get()?).minimum_balance(Mint::LEN),
            Mint::LEN as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // Now initialize that account as a Mint (standard Mint)
    //
    msg!("Initializing mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            9, // 9 Decimals for the default SPL Token standard
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Now create the account for that Mint's metadata
    //
    msg!("Creating metadata account...");
    msg!("Metadata account address: {}", metadata_account.key);
    invoke(
        &mpl_instruction::create_metadata_accounts_v3(
            *token_metadata_program.key,
            *metadata_account.key,
            *mint_account.key,
            *mint_authority.key,
            *payer.key,
            *mint_authority.key,
            args.token_title,
            args.token_symbol,
            args.token_uri,
            None,
            0,
            true,
            false,
            None,
            None,
            None,
        ),
        &[
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Token mint created successfully.");

    Ok(())
}

>>> program-examples/tokens/spl-token-minter/native/program/src/processor.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey},
};

use crate::instructions::{
    create::{create_token, CreateTokenArgs},
    mint::{mint_to, MintToArgs},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum SplMinterIntstruction {
    Create(CreateTokenArgs),
    Mint(MintToArgs),
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = SplMinterIntstruction::try_from_slice(instruction_data)?;

    match instruction {
        SplMinterIntstruction::Create(args) => create_token(accounts, args),
        SplMinterIntstruction::Mint(args) => mint_to(accounts, args),
    }
}

>>> program-examples/tokens/pda-mint-authority/native/program/src/state/mod.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct MintAuthorityPda {
    pub bump: u8,
}

impl MintAuthorityPda {
    pub const SEED_PREFIX: &'static str = "mint_authority";
    pub const SIZE: usize = 8 + 8;
}

>>> program-examples/tokens/pda-mint-authority/native/program/src/lib.rs
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod instructions;
pub mod processor;
pub mod state;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}

>>> program-examples/tokens/escrow/native/program/src/instructions/make_offer.rs
use {
    crate::{error::*, state::*, utils::assert_is_associated_token_account},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::{instruction as token_instruction, state::Account as TokenAccount},
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MakeOffer {
    pub id: u64,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
}

impl MakeOffer {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        args: MakeOffer,
    ) -> ProgramResult {
        // accounts in order.
        //
        let [
            offer_info, // offer account info
            token_mint_a, // token_mint a
            token_mint_b, // token mint b
            maker_token_account_a, // maker token account a
            vault, // vault
            maker, // maker
            payer, // payer
            token_program, // token program
            associated_token_program, // associated token program
            system_program// system program
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // ensure the maker signs the instruction
        //
        if !maker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let offer_seeds = &[
            Offer::SEED_PREFIX,
            maker.key.as_ref(),
            &args.id.to_le_bytes(),
        ];

        let (offer_key, bump) = Pubkey::find_program_address(offer_seeds, program_id);

        // make sure the offer key is the same
        //
        if *offer_info.key != offer_key {
            return Err(EscrowError::OfferKeyMismatch.into());
        };

        // check vault is owned by the offer account
        //
        assert_is_associated_token_account(vault.key, offer_info.key, token_mint_a.key)?;

        let offer = Offer {
            bump,
            maker: *maker.key,
            id: args.id,
            token_b_wanted_amount: args.token_b_wanted_amount,
            token_mint_a: *token_mint_a.key,
            token_mint_b: *token_mint_b.key,
        };

        let size = borsh::to_vec::<Offer>(&offer)?.len();
        let lamports_required = (Rent::get()?).minimum_balance(size);

        // create account
        //
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                offer_info.key,
                lamports_required,
                size as u64,
                program_id,
            ),
            &[payer.clone(), offer_info.clone(), system_program.clone()],
            &[&[
                Offer::SEED_PREFIX,
                maker.key.as_ref(),
                args.id.to_le_bytes().as_ref(),
                &[bump],
            ]],
        )?;

        // create the vault token account
        //
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                offer_info.key,
                token_mint_a.key,
                token_program.key,
            ),
            &[
                token_mint_a.clone(),
                vault.clone(),
                offer_info.clone(),
                payer.clone(),
                system_program.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;

        // transfer Mint A tokens to vault
        //
        invoke(
            &token_instruction::transfer(
                token_program.key,
                maker_token_account_a.key,
                vault.key,
                maker.key,
                &[maker.key],
                args.token_a_offered_amount,
            )?,
            &[
                token_program.clone(),
                maker_token_account_a.clone(),
                vault.clone(),
                maker.clone(),
            ],
        )?;

        let vault_token_amount = TokenAccount::unpack(&vault.data.borrow())?.amount;

        solana_program::msg!("Amount in vault: {}", vault_token_amount);

        assert_eq!(vault_token_amount, args.token_a_offered_amount);

        // write data into offer account
        //
        offer.serialize(&mut *offer_info.data.borrow_mut())?;

        Ok(())
    }
}

>>> program-examples/tokens/escrow/native/program/src/instructions/take_offer.rs
use {
    crate::{error::*, state::*, utils::*},
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::{instruction as token_instruction, state::Account as TokenAccount},
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct TakeOffer {}

impl TakeOffer {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo<'_>]) -> ProgramResult {
        // accounts in order
        //
        let [
            offer_info, // offer account info
            token_mint_a, // token mint A
            token_mint_b, // token mint b
            maker_token_account_b, // maker token a account
            taker_token_account_a, // mkaer token b account
            taker_token_account_b, // taker token a account
            vault, // vault
            maker, // maker
            taker, // taker
            payer, // payer
            token_program, // token program
            associated_token_program, // associated token program
            system_program// system program
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // ensure the taker signs the instruction
        //
        if !taker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // get the offer data
        //
        let offer = Offer::try_from_slice(&offer_info.data.borrow()[..])?;

        // validate the offer
        //
        assert_eq!(&offer.maker, maker.key);
        assert_eq!(&offer.token_mint_a, token_mint_a.key);
        assert_eq!(&offer.token_mint_b, token_mint_b.key);

        // validate the offer accout with signer seeds
        let offer_signer_seeds = &[
            Offer::SEED_PREFIX,
            maker.key.as_ref(),
            &offer.id.to_le_bytes(),
            &[offer.bump],
        ];

        let offer_key = Pubkey::create_program_address(offer_signer_seeds, program_id)?;

        // make sure the offer key is the same
        //
        if *offer_info.key != offer_key {
            return Err(EscrowError::OfferKeyMismatch.into());
        };

        // validate receiving addresses
        //
        assert_is_associated_token_account(maker_token_account_b.key, maker.key, token_mint_b.key)?;
        assert_is_associated_token_account(taker_token_account_a.key, taker.key, token_mint_a.key)?;

        // create taker token A account if needed, before receiveing tokens
        //
        if taker_token_account_a.lamports() == 0 {
            // create the vault token account
            invoke(
                &associated_token_account_instruction::create_associated_token_account(
                    payer.key,
                    taker.key,
                    token_mint_a.key,
                    token_program.key,
                ),
                &[
                    token_mint_a.clone(),
                    taker_token_account_a.clone(),
                    taker.clone(),
                    payer.clone(),
                    system_program.clone(),
                    token_program.clone(),
                    associated_token_program.clone(),
                ],
            )?;
        }

        // create maker token B account if needed, before receiveing tokens
        //
        if maker_token_account_b.lamports() == 0 {
            // create the vault token account
            invoke(
                &associated_token_account_instruction::create_associated_token_account(
                    payer.key,
                    maker.key,
                    token_mint_b.key,
                    token_program.key,
                ),
                &[
                    token_mint_b.clone(),
                    maker_token_account_b.clone(),
                    maker.clone(),
                    payer.clone(),
                    system_program.clone(),
                    token_program.clone(),
                    associated_token_program.clone(),
                ],
            )?;
        }

        // read token accounts
        //
        let vault_amount_a = TokenAccount::unpack(&vault.data.borrow())?.amount;
        let taker_amount_a_before_transfer =
            TokenAccount::unpack(&taker_token_account_a.data.borrow())?.amount;
        let maker_amount_b_before_transfer =
            TokenAccount::unpack(&maker_token_account_b.data.borrow())?.amount;
        let taker_amount_b = TokenAccount::unpack(&taker_token_account_b.data.borrow())?.amount;

        solana_program::msg!("Vault A Balance Before Transfer: {}", vault_amount_a);
        solana_program::msg!(
            "Taker A Balance Before Transfer: {}",
            taker_amount_a_before_transfer
        );
        solana_program::msg!(
            "Maker B Balance Before Transfer: {}",
            maker_amount_b_before_transfer
        );
        solana_program::msg!("Taker B Balance Before Transfer: {}", taker_amount_b);

        // taker transfer mint a tokens to vault
        //
        invoke(
            &token_instruction::transfer(
                token_program.key,
                taker_token_account_b.key,
                maker_token_account_b.key,
                taker.key,
                &[taker.key],
                offer.token_b_wanted_amount,
            )?,
            &[
                token_program.clone(),
                taker_token_account_b.clone(),
                maker_token_account_b.clone(),
                taker.clone(),
            ],
        )?;

        // transfer from vault to taker
        //
        invoke_signed(
            &token_instruction::transfer(
                token_program.key,
                vault.key,
                taker_token_account_a.key,
                offer_info.key,
                &[offer_info.key, taker.key],
                vault_amount_a,
            )?,
            &[
                token_mint_a.clone(),
                vault.clone(),
                taker_token_account_a.clone(),
                offer_info.clone(),
                taker.clone(),
                token_program.clone(),
            ],
            &[offer_signer_seeds],
        )?;

        let taker_amount_a = TokenAccount::unpack(&taker_token_account_a.data.borrow())?.amount;
        let maker_amount_b = TokenAccount::unpack(&maker_token_account_b.data.borrow())?.amount;

        assert_eq!(
            taker_amount_a,
            taker_amount_a_before_transfer + vault_amount_a
        );
        assert_eq!(
            maker_amount_b,
            taker_amount_a_before_transfer + offer.token_b_wanted_amount
        );

        let taker_amount_b = TokenAccount::unpack(&taker_token_account_b.data.borrow())?.amount;
        let vault_amount_a = TokenAccount::unpack(&vault.data.borrow())?.amount;

        solana_program::msg!("Vault A Balance After Transfer: {}", vault_amount_a);
        solana_program::msg!("Taker A Balance After Transfer: {}", taker_amount_a);
        solana_program::msg!("Maker B Balance After Transfer: {}", maker_amount_b);
        solana_program::msg!("Taker B Balance After Transfer: {}", taker_amount_b);

        // close the vault account
        //
        invoke_signed(
            &spl_token::instruction::close_account(
                token_program.key,
                vault.key,
                taker.key,
                offer_info.key,
                &[],
            )?,
            &[vault.clone(), taker.clone(), offer_info.clone()],
            &[offer_signer_seeds],
        )?;

        // Send the rent back to the payer
        //
        let lamports = offer_info.lamports();
        **offer_info.lamports.borrow_mut() -= lamports;
        **payer.lamports.borrow_mut() += lamports;

        // Realloc the account to zero
        //
        offer_info.realloc(0, true)?;

        // Assign the account to the System Program
        //
        offer_info.assign(system_program.key);

        Ok(())
    }
}

>>> program-examples/tokens/escrow/native/program/src/instructions/mod.rs
pub mod make_offer;
pub use make_offer::*;

pub mod take_offer;
pub use take_offer::*;

>>> program-examples/tokens/pda-mint-authority/native/program/src/processor.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey},
};

use crate::instructions::{
    create::{create_token, CreateTokenArgs},
    init::init,
    mint::mint_to,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum MyInstruction {
    Init,
    Create(CreateTokenArgs),
    Mint,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MyInstruction::try_from_slice(instruction_data)?;

    match instruction {
        MyInstruction::Init => init(program_id, accounts),
        MyInstruction::Create(args) => create_token(program_id, accounts, args),
        MyInstruction::Mint => mint_to(program_id, accounts),
    }
}

>>> program-examples/tokens/nft-minter/native/program/src/lib.rs
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod instructions;
pub mod processor;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}

>>> program-examples/tokens/nft-minter/native/program/src/processor.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey},
};

use crate::instructions::{
    create::{create_token, CreateTokenArgs},
    mint::mint_to,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum SplMinterIntstruction {
    Create(CreateTokenArgs),
    Mint,
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = SplMinterIntstruction::try_from_slice(instruction_data)?;

    match instruction {
        SplMinterIntstruction::Create(args) => create_token(accounts, args),
        SplMinterIntstruction::Mint => mint_to(accounts),
    }
}

>>> program-examples/tokens/pda-mint-authority/native/program/src/instructions/mint.rs
use {
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        pubkey::Pubkey,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

use crate::state::MintAuthorityPda;

pub fn mint_to(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let edition_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let associated_token_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    let (mint_authority_pda, bump) =
        Pubkey::find_program_address(&[MintAuthorityPda::SEED_PREFIX.as_bytes()], program_id);
    assert!(&mint_authority_pda.eq(mint_authority.key));

    if associated_token_account.lamports() == 0 {
        msg!("Creating associated token account...");
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                payer.key,
                mint_account.key,
                token_program.key,
            ),
            &[
                mint_account.clone(),
                associated_token_account.clone(),
                payer.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    } else {
        msg!("Associated token account exists.");
    }
    msg!("Associated Token Address: {}", associated_token_account.key);

    // Mint the NFT to the user's wallet
    //
    msg!("Minting NFT to associated token account...");
    invoke_signed(
        &token_instruction::mint_to(
            token_program.key,
            mint_account.key,
            associated_token_account.key,
            mint_authority.key,
            &[mint_authority.key],
            1,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            associated_token_account.clone(),
            token_program.clone(),
        ],
        &[&[MintAuthorityPda::SEED_PREFIX.as_bytes(), &[bump]]],
    )?;

    // We can make this a Limited Edition NFT through Metaplex,
    //      which will disable minting by setting the Mint & Freeze Authorities to the
    //      Edition Account.
    //
    msg!("Creating edition account...");
    msg!("Edition account address: {}", edition_account.key);
    invoke_signed(
        &mpl_instruction::create_master_edition_v3(
            *token_metadata_program.key, // Program ID
            *edition_account.key,        // Edition
            *mint_account.key,           // Mint
            *mint_authority.key,         // Update Authority
            *mint_authority.key,         // Mint Authority
            *metadata_account.key,       // Metadata
            *payer.key,                  // Payer
            Some(1),                     // Max Supply
        ),
        &[
            edition_account.clone(),
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
        &[&[MintAuthorityPda::SEED_PREFIX.as_bytes(), &[bump]]],
    )?;

    // If we don't use Metaplex Editions, we must disable minting manually
    //
    // -------------------------------------------------------------------
    // msg!("Disabling future minting of this NFT...");
    // invoke_signed(
    //     &token_instruction::set_authority(
    //         &token_program.key,
    //         &mint_account.key,
    //         None,
    //         token_instruction::AuthorityType::MintTokens,
    //         &mint_authority.key,
    //         &[&mint_authority.key],
    //     )?,
    //     &[
    //         mint_account.clone(),
    //         mint_authority.clone(),
    //         token_program.clone(),
    //     ],
    // )?;
    // invoke_signed(
    //     &token_instruction::set_authority(
    //         &token_program.key,
    //         &mint_account.key,
    //         None,
    //         token_instruction::AuthorityType::FreezeAccount,
    //         &mint_authority.key,
    //         &[&mint_authority.key],
    //     )?,
    //     &[
    //         mint_account.clone(),
    //         mint_authority.clone(),
    //         token_program.clone(),
    //     ],
    //     &[&[MintAuthorityPda::SEED_PREFIX.as_bytes(), &[bump]]],
    // )?;

    msg!("NFT minted successfully.");

    Ok(())
}

>>> program-examples/tokens/pda-mint-authority/native/program/src/instructions/init.rs
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::MintAuthorityPda;

pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_authority = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (mint_authority_pda, bump) =
        Pubkey::find_program_address(&[MintAuthorityPda::SEED_PREFIX.as_bytes()], program_id);
    assert!(&mint_authority_pda.eq(mint_authority.key));

    msg!("Creating mint authority PDA...");
    msg!("Mint Authority: {}", &mint_authority.key);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            mint_authority.key,
            (Rent::get()?).minimum_balance(MintAuthorityPda::SIZE),
            MintAuthorityPda::SIZE as u64,
            program_id,
        ),
        &[
            mint_authority.clone(),
            payer.clone(),
            system_program.clone(),
        ],
        &[&[MintAuthorityPda::SEED_PREFIX.as_bytes(), &[bump]]],
    )?;

    let data = MintAuthorityPda { bump };
    data.serialize(&mut &mut mint_authority.data.borrow_mut()[..])?;

    Ok(())
}

>>> program-examples/tokens/pda-mint-authority/native/program/src/instructions/mod.rs
pub mod create;
pub mod init;
pub mod mint;

pub use create::*;
pub use init::*;
pub use mint::*;

>>> program-examples/tokens/pda-mint-authority/native/program/src/instructions/create.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token::{instruction as token_instruction, state::Mint},
};

use crate::state::MintAuthorityPda;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub nft_title: String,
    pub nft_symbol: String,
    pub nft_uri: String,
}

pub fn create_token(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateTokenArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    let (mint_authority_pda, bump) =
        Pubkey::find_program_address(&[MintAuthorityPda::SEED_PREFIX.as_bytes()], program_id);
    assert!(&mint_authority_pda.eq(mint_authority.key));

    // First create the account for the Mint
    //
    msg!("Creating mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            (Rent::get()?).minimum_balance(Mint::LEN),
            Mint::LEN as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // Now initialize that account as a Mint (standard Mint)
    //
    msg!("Initializing mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            0, // 0 Decimals for the NFT standard
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Now create the account for that Mint's metadata
    //
    msg!("Creating metadata account...");
    msg!("Metadata account address: {}", metadata_account.key);
    invoke_signed(
        &mpl_instruction::create_metadata_accounts_v3(
            *token_metadata_program.key,
            *metadata_account.key,
            *mint_account.key,
            *mint_authority.key,
            *payer.key,
            *mint_authority.key,
            args.nft_title,
            args.nft_symbol,
            args.nft_uri,
            None,
            0,
            true,
            false,
            None,
            None,
            None,
        ),
        &[
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
        &[&[MintAuthorityPda::SEED_PREFIX.as_bytes(), &[bump]]],
    )?;

    msg!("Token mint created successfully.");

    Ok(())
}

>>> program-examples/tokens/nft-minter/native/program/src/instructions/mint.rs
use {
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

pub fn mint_to(accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let edition_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let associated_token_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    if associated_token_account.lamports() == 0 {
        msg!("Creating associated token account...");
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                payer.key,
                mint_account.key,
                token_program.key,
            ),
            &[
                mint_account.clone(),
                associated_token_account.clone(),
                payer.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    } else {
        msg!("Associated token account exists.");
    }
    msg!("Associated Token Address: {}", associated_token_account.key);

    // Mint the NFT to the user's wallet
    //
    msg!("Minting NFT to associated token account...");
    invoke(
        &token_instruction::mint_to(
            token_program.key,
            mint_account.key,
            associated_token_account.key,
            mint_authority.key,
            &[mint_authority.key],
            1,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            associated_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    // We can make this a Limited Edition NFT through Metaplex,
    //      which will disable minting by setting the Mint & Freeze Authorities to the
    //      Edition Account.
    //
    msg!("Creating edition account...");
    msg!("Edition account address: {}", edition_account.key);
    invoke(
        &mpl_instruction::create_master_edition_v3(
            *token_metadata_program.key, // Program ID
            *edition_account.key,        // Edition
            *mint_account.key,           // Mint
            *mint_authority.key,         // Update Authority
            *mint_authority.key,         // Mint Authority
            *metadata_account.key,       // Metadata
            *payer.key,                  // Payer
            Some(1),                     // Max Supply
        ),
        &[
            edition_account.clone(),
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
    )?;

    // If we don't use Metaplex Editions, we must disable minting manually
    //
    // -------------------------------------------------------------------
    // msg!("Disabling future minting of this NFT...");
    // invoke(
    //     &token_instruction::set_authority(
    //         &token_program.key,
    //         &mint_account.key,
    //         None,
    //         token_instruction::AuthorityType::MintTokens,
    //         &mint_authority.key,
    //         &[&mint_authority.key],
    //     )?,
    //     &[
    //         mint_account.clone(),
    //         mint_authority.clone(),
    //         token_program.clone(),
    //     ],
    // )?;
    // invoke(
    //     &token_instruction::set_authority(
    //         &token_program.key,
    //         &mint_account.key,
    //         None,
    //         token_instruction::AuthorityType::FreezeAccount,
    //         &mint_authority.key,
    //         &[&mint_authority.key],
    //     )?,
    //     &[
    //         mint_account.clone(),
    //         mint_authority.clone(),
    //         token_program.clone(),
    //     ],
    // )?;

    msg!("NFT minted successfully.");

    Ok(())
}

>>> program-examples/tokens/nft-minter/native/program/src/instructions/mod.rs
pub mod create;
pub mod mint;

pub use create::*;
pub use mint::*;

>>> program-examples/tokens/nft-minter/native/program/src/instructions/create.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_pack::Pack,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token::{instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub nft_title: String,
    pub nft_symbol: String,
    pub nft_uri: String,
}

pub fn create_token(accounts: &[AccountInfo], args: CreateTokenArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    // First create the account for the Mint
    //
    msg!("Creating mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            (Rent::get()?).minimum_balance(Mint::LEN),
            Mint::LEN as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // Now initialize that account as a Mint (standard Mint)
    //
    msg!("Initializing mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            0, // 0 Decimals for the NFT standard
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Now create the account for that Mint's metadata
    //
    msg!("Creating metadata account...");
    msg!("Metadata account address: {}", metadata_account.key);
    invoke(
        &mpl_instruction::create_metadata_accounts_v3(
            *token_metadata_program.key,
            *metadata_account.key,
            *mint_account.key,
            *mint_authority.key,
            *payer.key,
            *mint_authority.key,
            args.nft_title,
            args.nft_symbol,
            args.nft_uri,
            None,
            0,
            true,
            false,
            None,
            None,
            None,
        ),
        &[
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Token mint created successfully.");

    Ok(())
}

>>> program-examples/tokens/token-2022/multiple-extensions/native/program/src/lib.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token_2022::{extension::ExtensionType, instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_decimals: u8,
}

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let args = CreateTokenArgs::try_from_slice(instruction_data)?;

    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let close_authority = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    // Find the size for the Mint account with the the number of extensions we want to use.
    let space = ExtensionType::get_account_len::<Mint>(&[
        ExtensionType::MintCloseAuthority,
        ExtensionType::NonTransferable,
    ]);

    // Get the required rent exemption amount for the account
    let rent_required = Rent::get()?.minimum_balance(space);

    // Create the account for the Mint and allocate space
    msg!("Mint account address : {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            rent_required,
            space as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // Here, let's enable two extensions for the Mint. This needs to be done before the Mint is initialized

    // Initialize the Mint close authority Extension
    invoke(
        &token_instruction::initialize_mint_close_authority(
            token_program.key,
            mint_account.key,
            Some(close_authority.key),
        )
        .unwrap(),
        &[
            mint_account.clone(),
            close_authority.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the Non Transferable Mint Extension
    invoke(
        &token_instruction::initialize_non_transferable_mint(token_program.key, mint_account.key)
            .unwrap(),
        &[
            mint_account.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the Token Mint
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.token_decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Mint created!");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/native/program/src/lib.rs
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod instructions;
pub mod processor;

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}

>>> program-examples/tokens/transfer-tokens/native/program/src/instructions/mint_spl.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MintSplArgs {
    pub quantity: u64,
}

pub fn mint_spl(accounts: &[AccountInfo], args: MintSplArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let associated_token_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;

    if associated_token_account.lamports() == 0 {
        msg!("Creating associated token account...");
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                payer.key,
                mint_account.key,
                token_program.key,
            ),
            &[
                mint_account.clone(),
                associated_token_account.clone(),
                payer.clone(),
                system_program.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    } else {
        msg!("Associated token account exists.");
    }
    msg!("Associated Token Address: {}", associated_token_account.key);

    msg!(
        "Minting {} tokens to associated token account...",
        args.quantity
    );
    invoke(
        &token_instruction::mint_to(
            token_program.key,
            mint_account.key,
            associated_token_account.key,
            mint_authority.key,
            &[mint_authority.key],
            args.quantity,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            associated_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Tokens minted to wallet successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/native/program/src/instructions/transfer.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TransferTokensArgs {
    pub quantity: u64,
}

pub fn transfer_tokens(accounts: &[AccountInfo], args: TransferTokensArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let from_associated_token_account = next_account_info(accounts_iter)?;
    let to_associated_token_account = next_account_info(accounts_iter)?;
    let owner = next_account_info(accounts_iter)?;
    let recipient = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;

    if to_associated_token_account.lamports() == 0 {
        msg!("Creating associated token account for recipient...");
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                recipient.key,
                mint_account.key,
                token_program.key,
            ),
            &[
                mint_account.clone(),
                to_associated_token_account.clone(),
                recipient.clone(),
                payer.clone(),
                system_program.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    } else {
        msg!("Associated token account exists.");
    }
    msg!(
        "Recipient Associated Token Address: {}",
        to_associated_token_account.key
    );

    msg!("Transferring {} tokens...", args.quantity);
    msg!("Mint: {}", mint_account.key);
    msg!("Owner Token Address: {}", from_associated_token_account.key);
    msg!(
        "Recipient Token Address: {}",
        to_associated_token_account.key
    );
    invoke(
        &token_instruction::transfer(
            token_program.key,
            from_associated_token_account.key,
            to_associated_token_account.key,
            owner.key,
            &[owner.key, recipient.key],
            args.quantity,
        )?,
        &[
            mint_account.clone(),
            from_associated_token_account.clone(),
            to_associated_token_account.clone(),
            owner.clone(),
            recipient.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Tokens transferred successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/native/program/src/instructions/mod.rs
pub mod create;
pub mod mint_nft;
pub mod mint_spl;
pub mod transfer;

pub use create::*;
pub use mint_nft::*;
pub use mint_spl::*;
pub use transfer::*;

>>> program-examples/tokens/transfer-tokens/native/program/src/instructions/mint_nft.rs
use {
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
    },
    spl_associated_token_account::instruction as associated_token_account_instruction,
    spl_token::instruction as token_instruction,
};

pub fn mint_nft(accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let edition_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let associated_token_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let associated_token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    if associated_token_account.lamports() == 0 {
        msg!("Creating associated token account...");
        invoke(
            &associated_token_account_instruction::create_associated_token_account(
                payer.key,
                payer.key,
                mint_account.key,
                token_program.key,
            ),
            &[
                mint_account.clone(),
                associated_token_account.clone(),
                payer.clone(),
                token_program.clone(),
                associated_token_program.clone(),
            ],
        )?;
    } else {
        msg!("Associated token account exists.");
    }
    msg!("Associated Token Address: {}", associated_token_account.key);

    // Mint the NFT to the user's wallet
    //
    msg!("Minting NFT to associated token account...");
    invoke(
        &token_instruction::mint_to(
            token_program.key,
            mint_account.key,
            associated_token_account.key,
            mint_authority.key,
            &[mint_authority.key],
            1,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            associated_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    // We can make this a Limited Edition NFT through Metaplex,
    //      which will disable minting by setting the Mint & Freeze Authorities to the
    //      Edition Account.
    //
    msg!("Creating edition account...");
    msg!("Edition account address: {}", edition_account.key);
    invoke(
        &mpl_instruction::create_master_edition_v3(
            *token_metadata_program.key, // Program ID
            *edition_account.key,        // Edition
            *mint_account.key,           // Mint
            *mint_authority.key,         // Update Authority
            *mint_authority.key,         // Mint Authority
            *metadata_account.key,       // Metadata
            *payer.key,                  // Payer
            Some(1),                     // Max Supply
        ),
        &[
            edition_account.clone(),
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
    )?;

    // If we don't use Metaplex Editions, we must disable minting manually
    //
    // -------------------------------------------------------------------
    // msg!("Disabling future minting of this NFT...");
    // invoke(
    //     &token_instruction::set_authority(
    //         &token_program.key,
    //         &mint_account.key,
    //         None,
    //         token_instruction::AuthorityType::MintTokens,
    //         &mint_authority.key,
    //         &[&mint_authority.key],
    //     )?,
    //     &[
    //         mint_account.clone(),
    //         mint_authority.clone(),
    //         token_program.clone(),
    //     ],
    // )?;
    // invoke(
    //     &token_instruction::set_authority(
    //         &token_program.key,
    //         &mint_account.key,
    //         None,
    //         token_instruction::AuthorityType::FreezeAccount,
    //         &mint_authority.key,
    //         &[&mint_authority.key],
    //     )?,
    //     &[
    //         mint_account.clone(),
    //         mint_authority.clone(),
    //         token_program.clone(),
    //     ],
    // )?;

    msg!("NFT minted successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/native/program/src/instructions/create.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    mpl_token_metadata::instruction as mpl_instruction,
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        program_pack::Pack,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token::{instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_title: String,
    pub token_symbol: String,
    pub token_uri: String,
    pub decimals: u8,
}

pub fn create_token(accounts: &[AccountInfo], args: CreateTokenArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let metadata_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let token_metadata_program = next_account_info(accounts_iter)?;

    // First create the account for the Mint
    //
    msg!("Creating mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            (Rent::get()?).minimum_balance(Mint::LEN),
            Mint::LEN as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // Now initialize that account as a Mint (standard Mint)
    //
    msg!("Initializing mint account...");
    msg!("Mint: {}", mint_account.key);
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Now create the account for that Mint's metadata
    //
    msg!("Creating metadata account...");
    msg!("Metadata account address: {}", metadata_account.key);
    invoke(
        &mpl_instruction::create_metadata_accounts_v3(
            *token_metadata_program.key,
            *metadata_account.key,
            *mint_account.key,
            *mint_authority.key,
            *payer.key,
            *mint_authority.key,
            args.token_title,
            args.token_symbol,
            args.token_uri,
            None,
            0,
            true,
            false,
            None,
            None,
            None,
        ),
        &[
            metadata_account.clone(),
            mint_account.clone(),
            mint_authority.clone(),
            payer.clone(),
            token_metadata_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Token mint created successfully.");

    Ok(())
}

>>> program-examples/tokens/transfer-tokens/native/program/src/processor.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey},
};

use crate::instructions::{
    create::{create_token, CreateTokenArgs},
    mint_nft::mint_nft,
    mint_spl::{mint_spl, MintSplArgs},
    transfer::{transfer_tokens, TransferTokensArgs},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum MyInstruction {
    Create(CreateTokenArgs),
    MintNft,
    MintSpl(MintSplArgs),
    TransferTokens(TransferTokensArgs),
}

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MyInstruction::try_from_slice(instruction_data)?;

    match instruction {
        MyInstruction::Create(args) => create_token(accounts, args),
        MyInstruction::MintNft => mint_nft(accounts),
        MyInstruction::MintSpl(args) => mint_spl(accounts, args),
        MyInstruction::TransferTokens(args) => transfer_tokens(accounts, args),
    }
}

>>> program-examples/basics/hello-solana/native/program/src/lib.rs
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

// Tells Solana that the entrypoint to this program
//  is the "process_instruction" function.
//
entrypoint!(process_instruction);

// Our entrypoint's parameters have to match the
//  anatomy of a transaction instruction (see README).
//
fn process_instruction(
    program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello, Solana!");

    msg!("Our program's Program ID: {}", &program_id);

    Ok(())
}

>>> program-examples/basics/pda-rent-payer/native/program/src/processor.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instructions::{
    create_new_account::create_new_account,
    init_rent_vault::{init_rent_vault, InitRentVaultArgs},
};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum MyInstruction {
    InitRentVault(InitRentVaultArgs),
    CreateNewAccount,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MyInstruction::try_from_slice(input)?;
    match instruction {
        MyInstruction::InitRentVault(args) => init_rent_vault(program_id, accounts, args),
        MyInstruction::CreateNewAccount => create_new_account(program_id, accounts),
    }
}

>>> program-examples/basics/pda-rent-payer/native/program/src/state/mod.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct RentVault {}

impl RentVault {
    pub const SEED_PREFIX: &'static str = "rent_vault";
}

>>> program-examples/basics/pda-rent-payer/native/program/src/lib.rs
use solana_program::entrypoint;

use processor::process_instruction;

pub mod instructions;
pub mod processor;
pub mod state;

entrypoint!(process_instruction);

>>> program-examples/basics/pda-rent-payer/native/program/src/instructions/init_rent_vault.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::RentVault;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct InitRentVaultArgs {
    fund_lamports: u64,
}

pub fn init_rent_vault(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: InitRentVaultArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let rent_vault = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (rent_vault_pda, rent_vault_bump) =
        Pubkey::find_program_address(&[RentVault::SEED_PREFIX.as_bytes()], program_id);
    assert!(rent_vault.key.eq(&rent_vault_pda));

    // Lamports for rent on the vault, plus the desired additional funding
    //
    let lamports_required = (Rent::get()?).minimum_balance(0) + args.fund_lamports;

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            rent_vault.key,
            lamports_required,
            0,
            program_id,
        ),
        &[payer.clone(), rent_vault.clone(), system_program.clone()],
        &[&[RentVault::SEED_PREFIX.as_bytes(), &[rent_vault_bump]]],
    )?;

    Ok(())
}

>>> program-examples/basics/pda-rent-payer/native/program/src/instructions/mod.rs
pub mod create_new_account;
pub mod init_rent_vault;

pub use create_new_account::*;
pub use init_rent_vault::*;

>>> program-examples/basics/pda-rent-payer/native/program/src/instructions/create_new_account.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::state::RentVault;

pub fn create_new_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let new_account = next_account_info(accounts_iter)?;
    let rent_vault = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;

    let (rent_vault_pda, _rent_vault_bump) =
        Pubkey::find_program_address(&[RentVault::SEED_PREFIX.as_bytes()], program_id);
    assert!(rent_vault.key.eq(&rent_vault_pda));

    // Assuming this account has no inner data (size 0)
    //
    let lamports_required_for_rent = (Rent::get()?).minimum_balance(0);

    **rent_vault.lamports.borrow_mut() -= lamports_required_for_rent;
    **new_account.lamports.borrow_mut() += lamports_required_for_rent;

    Ok(())
}

>>> program-examples/basics/repository-layout/native/program/src/state/mod.rs
pub mod food;
pub mod game;
pub mod ride;

>>> program-examples/basics/repository-layout/native/program/src/state/food.rs
// Objects

pub struct FoodStand {
    pub name: String,
    pub food_type: String,
    pub tickets: u32,
}

impl FoodStand {
    pub fn new(name: String, food_type: String, tickets: u32) -> FoodStand {
        FoodStand {
            name,
            food_type,
            tickets,
        }
    }
}

pub fn get_food_stands() -> Vec<FoodStand> {
    vec![
        FoodStand::new("Larry's Pizza".to_string(), "pizza".to_string(), 3),
        FoodStand::new("Taco Shack".to_string(), "taco".to_string(), 2),
        FoodStand::new("Dough Boy's".to_string(), "fried dough".to_string(), 1),
    ]
}

>>> program-examples/basics/repository-layout/native/program/src/state/ride.rs
// Objects

pub struct Ride {
    pub name: String,
    pub upside_down: bool,
    pub tickets: u32,
    pub min_height: u32,
}

impl Ride {
    pub fn new(name: String, upside_down: bool, tickets: u32, min_height: u32) -> Ride {
        Ride {
            name,
            upside_down,
            tickets,
            min_height,
        }
    }
}

pub fn get_rides() -> Vec<Ride> {
    vec![
        Ride::new("Tilt-a-Whirl".to_string(), false, 3, 48),
        Ride::new("Scrambler".to_string(), false, 3, 48),
        Ride::new("Ferris Wheel".to_string(), false, 5, 55),
        Ride::new("Zero Gravity".to_string(), true, 5, 60),
    ]
}

>>> program-examples/basics/repository-layout/native/program/src/state/game.rs
// Objects

pub struct Game {
    pub name: String,
    pub tickets: u32,
    pub tries: u32,
    pub prize: String,
}

const DEFAULT_TICKETS_TO_PLAY: u32 = 3;

impl Game {
    pub fn new(name: String, tries: u32, prize: String) -> Game {
        Game {
            name,
            tickets: DEFAULT_TICKETS_TO_PLAY,
            tries,
            prize,
        }
    }
}

pub fn get_games() -> Vec<Game> {
    vec![
        Game::new("Ring Toss".to_string(), 5, "teddy bear".to_string()),
        Game::new("I Got It!".to_string(), 12, "goldfish".to_string()),
        Game::new("Ladder Climb".to_string(), 1, "popcorn bucket".to_string()),
    ]
}

>>> program-examples/basics/repository-layout/native/program/src/lib.rs
// For setting up modules & configs

pub mod error;
pub mod instructions;
pub mod processor;
pub mod state;

>>> program-examples/basics/repository-layout/native/program/src/error.rs
// For any custom errors

>>> program-examples/basics/repository-layout/native/program/src/instructions/get_on_ride.rs
use solana_program::{entrypoint::ProgramResult, msg, program_error::ProgramError};

use crate::state::ride;

// InstructionData Data

pub struct GetOnRideInstructionData {
    pub rider_name: String,
    pub rider_height: u32,
    pub rider_ticket_count: u32,
    pub ride: String,
}

pub fn get_on_ride(ix: GetOnRideInstructionData) -> ProgramResult {
    let rides_list = ride::get_rides();

    for ride in rides_list.iter() {
        if ix.ride.eq(&ride.name) {
            msg!("You're about to ride the {}!", ride.name);

            if ix.rider_ticket_count < ride.tickets {
                msg!(
                    "  Sorry {}, you need {} tickets to ride the {}!",
                    ix.rider_name,
                    ride.tickets,
                    ride.name
                );
                return Ok(());
            };

            if ix.rider_height < ride.min_height {
                msg!(
                    "  Sorry {}, you need to be {}\" tall to ride the {}!",
                    ix.rider_name,
                    ride.min_height,
                    ride.name
                );
                return Ok(());
            };

            msg!("  Welcome aboard the {}!", ride.name);

            if ride.upside_down {
                msg!("  Btw, this ride goes upside down. Hold on tight!");
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData)
}

>>> program-examples/basics/repository-layout/native/program/src/instructions/play_game.rs
use solana_program::{entrypoint::ProgramResult, msg, program_error::ProgramError};

use crate::state::game;

// InstructionData Data

pub struct PlayGameInstructionData {
    pub gamer_name: String,
    pub gamer_ticket_count: u32,
    pub game: String,
}

pub fn play_game(ix: PlayGameInstructionData) -> ProgramResult {
    let games_list = game::get_games();

    for game in games_list.iter() {
        if ix.game.eq(&game.name) {
            msg!("You're about to play {}!", game.name);

            if ix.gamer_ticket_count < game.tickets {
                msg!(
                    "  Sorry {}, you need {} tickets to play {}!",
                    ix.gamer_name,
                    game.tickets,
                    game.name
                );
            } else {
                msg!("  Let's see what you got!");
                msg!(
                    "  You get {} attempts and the prize is a {}!",
                    game.tries,
                    game.prize
                );
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData)
}

>>> program-examples/basics/repository-layout/native/program/src/instructions/mod.rs
pub mod eat_food;
pub mod get_on_ride;
pub mod play_game;

>>> program-examples/basics/repository-layout/native/program/src/instructions/eat_food.rs
use solana_program::{entrypoint::ProgramResult, msg, program_error::ProgramError};

use crate::state::food;

// InstructionData Data

pub struct EatFoodInstructionData {
    pub eater_name: String,
    pub eater_ticket_count: u32,
    pub food_stand: String,
}

pub fn eat_food(ix: EatFoodInstructionData) -> ProgramResult {
    let food_stands_list = food::get_food_stands();

    for food_stand in food_stands_list.iter() {
        if ix.food_stand.eq(&food_stand.name) {
            msg!("Welcome to {}! What can I get you?", food_stand.name);

            if ix.eater_ticket_count < food_stand.tickets {
                msg!(
                    "  Sorry {}, our {} is {} tickets!",
                    ix.eater_name,
                    food_stand.food_type,
                    food_stand.tickets
                );
            } else {
                msg!("  Enjoy your {}!", food_stand.food_type);
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData)
}

>>> program-examples/basics/repository-layout/native/program/src/processor.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::instructions::{eat_food, get_on_ride, play_game};

// For processing everything at the entrypoint

entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CarnivalInstructionData {
    pub name: String,
    pub height: u32,
    pub ticket_count: u32,
    pub attraction: String,
    pub attraction_name: String,
}

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let ix_data_object = CarnivalInstructionData::try_from_slice(instruction_data)?;

    msg!("Welcome to the carnival, {}!", ix_data_object.name);

    match ix_data_object.attraction.as_str() {
        "ride" => get_on_ride::get_on_ride(get_on_ride::GetOnRideInstructionData {
            rider_name: ix_data_object.name,
            rider_height: ix_data_object.height,
            rider_ticket_count: ix_data_object.ticket_count,
            ride: ix_data_object.attraction_name,
        }),
        "game" => play_game::play_game(play_game::PlayGameInstructionData {
            gamer_name: ix_data_object.name,
            gamer_ticket_count: ix_data_object.ticket_count,
            game: ix_data_object.attraction_name,
        }),
        "food" => eat_food::eat_food(eat_food::EatFoodInstructionData {
            eater_name: ix_data_object.name,
            eater_ticket_count: ix_data_object.ticket_count,
            food_stand: ix_data_object.attraction_name,
        }),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

>>> program-examples/basics/checking-accounts/native/program/src/lib.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // You can verify the program ID from the instruction is in fact
    //      the program ID of your program.
    if system_program::check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    };

    // You can verify the list has the correct number of accounts.
    // This error will get thrown by default if you
    //      try to reach past the end of the iter.
    if accounts.len() < 4 {
        msg!("This instruction requires 4 accounts:");
        msg!("  payer, account_to_create, account_to_change, system_program");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Accounts passed in a vector must be in the expected order.
    let accounts_iter = &mut accounts.iter();
    let _payer = next_account_info(accounts_iter)?;
    let account_to_create = next_account_info(accounts_iter)?;
    let account_to_change = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // You can make sure an account has NOT been initialized.

    msg!("New account: {}", account_to_create.key);
    if account_to_create.lamports() != 0 {
        msg!("The program expected the account to create to not yet be initialized.");
        return Err(ProgramError::AccountAlreadyInitialized);
    };
    // (Create account...)

    // You can also make sure an account has been initialized.
    msg!("Account to change: {}", account_to_change.key);
    if account_to_change.lamports() == 0 {
        msg!("The program expected the account to change to be initialized.");
        return Err(ProgramError::UninitializedAccount);
    };

    // If we want to modify an account's data, it must be owned by our program.
    if account_to_change.owner != program_id {
        msg!("Account to change does not have the correct program id.");
        return Err(ProgramError::IncorrectProgramId);
    };

    // You can also check pubkeys against constants.
    if system_program.key != &system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    };

    Ok(())
}

>>> program-examples/basics/processing-instructions/native/program/src/lib.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Attempt to serialize the BPF format to our struct
    //  using Borsh
    //
    let instruction_data_object = InstructionData::try_from_slice(instruction_data)?;

    msg!("Welcome to the park, {}!", instruction_data_object.name);
    if instruction_data_object.height > 5 {
        msg!("You are tall enough to ride this ride. Congratulations.");
    } else {
        msg!("You are NOT tall enough to ride this ride. Sorry mate.");
    };

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct InstructionData {
    name: String,
    height: u32,
}

>>> program-examples/basics/rent/native/program/src/lib.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let new_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    msg!("Program invoked. Creating a system account...");
    msg!("  New public key will be: {}", &new_account.key.to_string());

    // Determine the necessary minimum rent by calculating the account's size
    //
    let account_span = instruction_data.len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    msg!("Account span: {}", &account_span);
    msg!("Lamports required: {}", &lamports_required);

    invoke(
        &system_instruction::create_account(
            payer.key,
            new_account.key,
            lamports_required,
            account_span as u64,
            &system_program::ID,
        ),
        &[payer.clone(), new_account.clone(), system_program.clone()],
    )?;

    msg!("Account created succesfully.");
    Ok(())
}

>>> program-examples/basics/program-derived-addresses/native/program/src/state/mod.rs
pub mod page_visits;

pub use page_visits::*;

>>> program-examples/basics/program-derived-addresses/native/program/src/state/page_visits.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct IncrementPageVisits {}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct PageVisits {
    pub page_visits: u32,
    pub bump: u8,
}

impl PageVisits {
    pub const ACCOUNT_SPACE: usize = 8 + 32;

    pub const SEED_PREFIX: &'static str = "page_visits";

    pub fn new(page_visits: u32, bump: u8) -> Self {
        PageVisits { page_visits, bump }
    }

    pub fn increment(&mut self) {
        self.page_visits += 1;
    }
}

>>> program-examples/basics/program-derived-addresses/native/program/src/lib.rs
use solana_program::entrypoint;

use processor::process_instruction;

pub mod instructions;
pub mod processor;
pub mod state;

entrypoint!(process_instruction);

>>> program-examples/basics/program-derived-addresses/native/program/src/instructions/mod.rs
pub mod create;
pub mod increment;

pub use create::*;
pub use increment::*;

>>> program-examples/basics/program-derived-addresses/native/program/src/instructions/create.rs
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::PageVisits;

pub fn create_page_visits(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    page_visits: PageVisits,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let page_visits_account = next_account_info(accounts_iter)?;
    let user = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_span = (page_visits.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            page_visits_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            payer.clone(),
            page_visits_account.clone(),
            system_program.clone(),
        ],
        &[&[
            PageVisits::SEED_PREFIX.as_bytes(),
            user.key.as_ref(),
            &[page_visits.bump],
        ]],
    )?;

    Ok(())
}

>>> program-examples/basics/program-derived-addresses/native/program/src/instructions/increment.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
};

use crate::state::PageVisits;

pub fn increment_page_visits(accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let page_visits_account = next_account_info(accounts_iter)?;

    let page_visits = &mut PageVisits::try_from_slice(&page_visits_account.data.borrow())?;
    page_visits.increment();
    page_visits.serialize(&mut &mut page_visits_account.data.borrow_mut()[..])?;
    Ok(())
}

>>> program-examples/basics/program-derived-addresses/native/program/src/processor.rs
use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instructions;
use crate::state::IncrementPageVisits;
use crate::state::PageVisits;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Ok(page_visits) = PageVisits::try_from_slice(instruction_data) {
        return instructions::create::create_page_visits(program_id, accounts, page_visits);
    };

    if IncrementPageVisits::try_from_slice(instruction_data).is_ok() {
        return instructions::increment::increment_page_visits(accounts);
    }

    Err(ProgramError::InvalidInstructionData)
}

>>> program-examples/basics/account-data/native/program/src/state/address_info.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressInfo {
    pub name: String,
    pub house_number: u8,
    pub street: String,
    pub city: String,
}

impl AddressInfo {
    pub fn new(name: String, house_number: u8, street: String, city: String) -> Self {
        AddressInfo {
            name,
            house_number,
            street,
            city,
        }
    }
}

>>> program-examples/basics/account-data/native/program/src/state/mod.rs
pub mod address_info;

pub use address_info::*;

>>> program-examples/basics/account-data/native/program/src/lib.rs
use solana_program::entrypoint;

use processor::process_instruction;

pub mod instructions;
pub mod processor;
pub mod state;

entrypoint!(process_instruction);

>>> program-examples/basics/counter/native/program/src/state.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Counter {
    pub count: u64,
}

>>> program-examples/basics/counter/native/program/src/lib.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    declare_id,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

mod state;
use state::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (instruction_discriminant, instruction_data_inner) = instruction_data.split_at(1);
    match instruction_discriminant[0] {
        0 => {
            msg!("Instruction: Increment");
            process_increment_counter(accounts, instruction_data_inner)?;
        }
        _ => {
            msg!("Error: unknown instruction")
        }
    }
    Ok(())
}

pub fn process_increment_counter(
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let account_info_iter = &mut accounts.iter();

    let counter_account = next_account_info(account_info_iter)?;
    assert!(
        counter_account.is_writable,
        "Counter account must be writable"
    );

    let mut counter = Counter::try_from_slice(&counter_account.try_borrow_mut_data()?)?;
    counter.count += 1;
    counter.serialize(&mut *counter_account.data.borrow_mut())?;

    msg!("Counter state incremented to {:?}", counter.count);
    Ok(())
}

>>> program-examples/basics/account-data/native/program/src/instructions/mod.rs
pub mod create;

pub use create::*;

>>> program-examples/basics/account-data/native/program/src/instructions/create.rs
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::AddressInfo;

pub fn create_address_info(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    address_info: AddressInfo,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let address_info_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_span = (address_info.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke(
        &system_instruction::create_account(
            payer.key,
            address_info_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            payer.clone(),
            address_info_account.clone(),
            system_program.clone(),
        ],
    )?;

    address_info.serialize(&mut &mut address_info_account.data.borrow_mut()[..])?;
    Ok(())
}

>>> program-examples/basics/account-data/native/program/src/processor.rs
use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instructions;
use crate::state::AddressInfo;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Ok(address_info) = AddressInfo::try_from_slice(instruction_data) {
        return instructions::create::create_address_info(program_id, accounts, address_info);
    };

    Err(ProgramError::InvalidInstructionData)
}

>>> program-examples/tokens/token-2022/mint-close-authority/native/program/src/lib.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token_2022::{extension::ExtensionType, instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_decimals: u8,
}

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let args = CreateTokenArgs::try_from_slice(instruction_data)?;

    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let close_authority = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    // Find the size for the account with the Extension
    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::MintCloseAuthority]);

    // Get the required rent exemption amount for the account
    let rent_required = Rent::get()?.minimum_balance(space);

    // Create the account for the Mint and allocate space
    msg!("Mint account address : {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            rent_required,
            space as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // This needs to be done before the Mint is initialized

    // Initialize the Mint close authority Extension
    invoke(
        &token_instruction::initialize_mint_close_authority(
            token_program.key,
            mint_account.key,
            Some(close_authority.key),
        )
        .unwrap(),
        &[
            mint_account.clone(),
            close_authority.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the Token Mint
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.token_decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Mint created!");

    Ok(())
}

>>> program-examples/basics/realloc/native/program/src/state/enhanced_address_info.rs
use borsh::{BorshDeserialize, BorshSerialize};

use crate::state::AddressInfo;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct EnhancedAddressInfoExtender {
    pub state: String,
    pub zip: u32,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct EnhancedAddressInfo {
    pub name: String,
    pub house_number: u8,
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: u32,
}

impl EnhancedAddressInfo {
    pub fn from_address_info(address_info: AddressInfo, state: String, zip: u32) -> Self {
        EnhancedAddressInfo {
            name: address_info.name,
            house_number: address_info.house_number,
            street: address_info.street,
            city: address_info.city,
            state,
            zip,
        }
    }
}

>>> program-examples/basics/realloc/native/program/src/state/address_info.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct AddressInfo {
    pub name: String,
    pub house_number: u8,
    pub street: String,
    pub city: String,
}

impl AddressInfo {
    pub fn new(name: String, house_number: u8, street: String, city: String) -> Self {
        AddressInfo {
            name,
            house_number,
            street,
            city,
        }
    }
}

>>> program-examples/basics/realloc/native/program/src/state/mod.rs
pub mod address_info;
pub mod enhanced_address_info;
pub mod work_info;

pub use address_info::*;
pub use enhanced_address_info::*;
pub use work_info::*;

>>> program-examples/basics/realloc/native/program/src/state/work_info.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct WorkInfo {
    pub name: String,
    pub position: String,
    pub company: String,
    pub years_employed: u8,
}

impl WorkInfo {
    pub fn new(name: String, position: String, company: String, years_employed: u8) -> Self {
        WorkInfo {
            name,
            position,
            company,
            years_employed,
        }
    }
}

>>> program-examples/basics/realloc/native/program/src/lib.rs
pub mod instructions;
pub mod processor;
pub mod state;

use {crate::processor::process_instruction, solana_program::entrypoint};

entrypoint!(process_instruction);

>>> program-examples/basics/realloc/native/program/src/instructions/reallocate.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::{AddressInfo, EnhancedAddressInfo, EnhancedAddressInfoExtender, WorkInfo};

pub fn reallocate_without_zero_init(
    accounts: &[AccountInfo],
    args: EnhancedAddressInfoExtender,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let target_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let address_info_data = AddressInfo::try_from_slice(&target_account.data.borrow())?;
    let enhanced_address_info_data =
        EnhancedAddressInfo::from_address_info(address_info_data, args.state, args.zip);

    let account_span = (enhanced_address_info_data.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    let diff = lamports_required - target_account.lamports();
    invoke(
        &system_instruction::transfer(payer.key, target_account.key, diff),
        &[
            payer.clone(),
            target_account.clone(),
            system_program.clone(),
        ],
    )?;

    target_account.realloc(account_span, false)?;

    enhanced_address_info_data.serialize(&mut &mut target_account.data.borrow_mut()[..])?;

    Ok(())
}

pub fn reallocate_zero_init(accounts: &[AccountInfo], data: WorkInfo) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let target_account = next_account_info(accounts_iter)?;

    let account_span = (data.try_to_vec()?).len();

    target_account.realloc(account_span, true)?;

    data.serialize(&mut &mut target_account.data.borrow_mut()[..])?;

    Ok(())
}

>>> program-examples/basics/realloc/native/program/src/instructions/mod.rs
pub mod create;
pub mod reallocate;

pub use create::*;
pub use reallocate::*;

>>> program-examples/basics/realloc/native/program/src/instructions/create.rs
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::AddressInfo;

pub fn create_address_info(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: AddressInfo,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let target_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_span = (data.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke(
        &system_instruction::create_account(
            payer.key,
            target_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            payer.clone(),
            target_account.clone(),
            system_program.clone(),
        ],
    )?;

    data.serialize(&mut &mut target_account.data.borrow_mut()[..])?;
    Ok(())
}

>>> program-examples/basics/realloc/native/program/src/processor.rs
use crate::instructions::*;
use crate::state::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ReallocInstruction {
    Create(AddressInfo),
    ReallocateWithoutZeroInit(EnhancedAddressInfoExtender),
    ReallocateZeroInit(WorkInfo),
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = ReallocInstruction::try_from_slice(input)?;
    match instruction {
        ReallocInstruction::Create(data) => create_address_info(program_id, accounts, data),
        ReallocInstruction::ReallocateWithoutZeroInit(data) => {
            reallocate_without_zero_init(accounts, data)
        }
        ReallocInstruction::ReallocateZeroInit(data) => reallocate_zero_init(accounts, data),
    }
}

>>> program-examples/basics/favorites/native/program/src/state.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Favorites {
    pub number: u64,
    pub color: String,
    pub hobbies: Vec<String>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct GetFavorites {}

>>> program-examples/basics/favorites/native/program/src/lib.rs
use solana_program::entrypoint;

pub mod instructions;
pub mod processor;
pub mod state;

use processor::process_instruction;

entrypoint!(process_instruction);

>>> program-examples/basics/favorites/native/program/src/instructions/get_pda.rs
use crate::state::Favorites;
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn get_pda(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let user = next_account_info(account_iter)?;
    let favorite_account = next_account_info(account_iter)?;

    // deriving the favorite pda
    let (favorite_pda, _) =
        Pubkey::find_program_address(&[b"favorite", user.key.as_ref()], program_id);

    // Checking if the favorite account is same as the derived favorite pda
    if favorite_account.key != &favorite_pda {
        return Err(ProgramError::IncorrectProgramId);
    };

    let favorites = Favorites::try_from_slice(&favorite_account.data.borrow())?;

    msg!(
        "User {}'s favorite number is {}, favorite color is: {}, and their hobbies are {:#?}",
        user.key,
        favorites.number,
        favorites.color,
        favorites.hobbies
    );
    Ok(())
}

>>> program-examples/basics/favorites/native/program/src/instructions/mod.rs
pub mod create_pda;
pub mod get_pda;

use create_pda::*;
use get_pda::*;

>>> program-examples/basics/favorites/native/program/src/instructions/create_pda.rs
use crate::state::Favorites;
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn create_pda(program_id: &Pubkey, accounts: &[AccountInfo], data: Favorites) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let user = next_account_info(account_iter)?; // the user who's signing the transaction
    let favorite_account = next_account_info(account_iter)?; // The target account that will be created in the process
    let system_program = next_account_info(account_iter)?;

    // deriving the favorite pda
    let (favorite_pda, favorite_bump) =
        Pubkey::find_program_address(&[b"favorite", user.key.as_ref()], program_id);

    // Checking if the favorite account is same as the derived favorite pda
    if favorite_account.key != &favorite_pda {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Checking if the pda is already initialized
    if favorite_account.data.borrow().len() == 0 {
        // Initialize the favorite account if it's not initialized
        let space = data.try_to_vec()?.len();
        let lamports = (Rent::get()?).minimum_balance(space);

        let ix = system_instruction::create_account(
            user.key,
            favorite_account.key,
            lamports,
            space as u64,
            program_id,
        );

        invoke_signed(
            &ix,
            &[
                user.clone(),
                favorite_account.clone(),
                system_program.clone(),
            ],
            &[&[b"favorite", user.key.as_ref(), &[favorite_bump]]],
        )?;

        // Serialize and store the data
        data.serialize(&mut &mut favorite_account.data.borrow_mut()[..])?;
        msg!("{:#?}", data);
    } else {
        return Err(ProgramError::AccountAlreadyInitialized.into());
    }

    Ok(())
}

>>> program-examples/basics/favorites/native/program/src/processor.rs
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instructions::{create_pda::*, get_pda::*};
use crate::state::Favorites;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize)]
pub enum FavoritesInstruction {
    CreatePda(Favorites),
    GetPda,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = FavoritesInstruction::try_from_slice(instruction_data)?;

    match instruction {
        FavoritesInstruction::CreatePda(data) => create_pda(program_id, accounts, data),
        FavoritesInstruction::GetPda => get_pda(program_id, accounts),
    }?;

    Ok(())
}

>>> program-examples/tokens/token-2022/default-account-state/native/program/src/lib.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token_2022::{
        extension::{
            default_account_state::instruction::{
                initialize_default_account_state, update_default_account_state,
            },
            ExtensionType,
        },
        instruction as token_instruction,
        state::AccountState,
        state::Mint,
    },
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_decimals: u8,
}

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let args = CreateTokenArgs::try_from_slice(instruction_data)?;

    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    // Find the size for the account with the Extension
    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::DefaultAccountState]);

    // Get the required rent exemption amount for the account
    let rent_required = Rent::get()?.minimum_balance(space);

    // Create the account for the Mint and allocate space

    msg!("Mint account address : {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            rent_required,
            space as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // This needs to be done before the Mint is initialized

    // Initialize the Default Account State as Frozen
    invoke(
        &initialize_default_account_state(
            token_program.key,
            mint_account.key,
            &AccountState::Frozen,
        )
        .unwrap(),
        &[
            mint_account.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the Token Mint
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.token_decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Update the Default Account State to Initialized
    invoke(
        &update_default_account_state(
            token_program.key,
            mint_account.key,
            payer.key,
            &[payer.key],
            &AccountState::Initialized,
        )
        .unwrap(),
        &[
            mint_account.clone(),
            payer.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
    )?;

    msg!("Mint created!");

    Ok(())
}

>>> program-examples/basics/close-account/native/program/src/state/mod.rs
pub mod user;

>>> program-examples/basics/close-account/native/program/src/state/user.rs
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct User {
    pub name: String,
}

impl User {
    pub const SEED_PREFIX: &'static str = "USER";
}

>>> program-examples/basics/close-account/native/program/src/lib.rs
pub mod instructions;
pub mod processor;
pub mod state;

use {crate::processor::process_instruction, solana_program::entrypoint};

entrypoint!(process_instruction);

>>> program-examples/basics/close-account/native/program/src/instructions/close_user.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    rent::Rent,
    sysvar::Sysvar,
};

pub fn close_user(accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let target_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_span = 0usize;
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    let diff = target_account.lamports() - lamports_required;

    // Send the rent back to the payer
    **target_account.lamports.borrow_mut() -= diff;
    **payer.lamports.borrow_mut() += diff;

    // Realloc the account to zero
    target_account.realloc(account_span, true)?;

    // Assign the account to the System Program
    target_account.assign(system_program.key);

    Ok(())
}

>>> program-examples/basics/close-account/native/program/src/instructions/mod.rs
pub mod close_user;
pub mod create_user;

>>> program-examples/basics/close-account/native/program/src/instructions/create_user.rs
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::user::User;

pub fn create_user(program_id: &Pubkey, accounts: &[AccountInfo], data: User) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let target_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_span = (data.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    let (_, bump) = Pubkey::find_program_address(
        &[User::SEED_PREFIX.as_bytes(), payer.key.as_ref()],
        program_id,
    );

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            target_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[
            payer.clone(),
            target_account.clone(),
            system_program.clone(),
        ],
        &[&[User::SEED_PREFIX.as_bytes(), payer.key.as_ref(), &[bump]]],
    )?;

    data.serialize(&mut &mut target_account.data.borrow_mut()[..])?;
    Ok(())
}

>>> program-examples/basics/close-account/native/program/src/processor.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instructions::{close_user::close_user, create_user::create_user};
use crate::state::user::User;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum MyInstruction {
    CreateUser(User),
    CloseUser,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MyInstruction::try_from_slice(input)?;
    match instruction {
        MyInstruction::CreateUser(data) => create_user(program_id, accounts, data),
        MyInstruction::CloseUser => close_user(accounts),
    }
}

>>> program-examples/tokens/token-2022/transfer-fee/native/program/src/lib.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token_2022::{
        extension::{
            transfer_fee::instruction::{initialize_transfer_fee_config, set_transfer_fee},
            ExtensionType,
        },
        instruction as token_instruction,
        state::Mint,
    },
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_decimals: u8,
}

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let args = CreateTokenArgs::try_from_slice(instruction_data)?;

    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    // Find the size for the account with the Extension
    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::TransferFeeConfig]);

    // Get the required rent exemption amount for the account
    let rent_required = Rent::get()?.minimum_balance(space);

    // Create the account for the Mint and allocate space
    msg!("Mint account address : {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            rent_required,
            space as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // The max fee will be 5 tokens, here we adjust it with the tokens decimals
    let max_fee = 5 * 10u64.pow(args.token_decimals as u32);

    // This needs to be done before the Mint is initialized
    // Initialize the Transfer Fee config
    invoke(
        &initialize_transfer_fee_config(
            token_program.key,
            mint_account.key,
            Some(payer.key),
            Some(payer.key),
            // 1% fee on transfers
            100,
            max_fee,
        )
        .unwrap(),
        &[
            mint_account.clone(),
            token_program.clone(),
            payer.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the Token Mint
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.token_decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    // Initialize the Transfer Fee config
    invoke(
        &set_transfer_fee(
            token_program.key,
            mint_account.key,
            payer.key,
            &[payer.key],
            // 10% fee on transfers
            1000,
            max_fee,
        )
        .unwrap(),
        &[
            mint_account.clone(),
            token_program.clone(),
            payer.clone(),
            system_program.clone(),
        ],
    )?;

    msg!("Mint created!");

    Ok(())
}

>>> program-examples/basics/create-account/native/program/src/lib.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    program::invoke,
    pubkey::Pubkey,
    system_instruction, system_program,
};

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let new_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    msg!("Program invoked. Creating a system account...");
    msg!("  New public key will be: {}", &new_account.key.to_string());

    invoke(
        &system_instruction::create_account(
            payer.key,
            new_account.key,
            LAMPORTS_PER_SOL,
            0,
            &system_program::ID,
        ),
        &[payer.clone(), new_account.clone(), system_program.clone()],
    )?;

    msg!("Account created succesfully.");
    Ok(())
}

>>> program-examples/basics/transfer-sol/native/program/src/instruction.rs
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    pubkey::Pubkey,
    system_instruction,
};

pub fn transfer_sol_with_cpi(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let recipient = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    invoke(
        &system_instruction::transfer(payer.key, recipient.key, amount),
        &[payer.clone(), recipient.clone(), system_program.clone()],
    )?;

    Ok(())
}

pub fn transfer_sol_with_program(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let recipient = next_account_info(accounts_iter)?;

    **payer.try_borrow_mut_lamports()? -= amount;
    **recipient.try_borrow_mut_lamports()? += amount;

    Ok(())
}

>>> program-examples/basics/transfer-sol/native/program/src/lib.rs
pub mod instruction;
pub mod processor;

use {crate::processor::process_instruction, solana_program::entrypoint};

entrypoint!(process_instruction);

>>> program-examples/basics/transfer-sol/native/program/src/processor.rs
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction::transfer_sol_with_cpi;
use crate::instruction::transfer_sol_with_program;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TransferInstruction {
    CpiTransfer(u64),
    ProgramTransfer(u64),
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TransferInstruction::try_from_slice(input)?;
    match instruction {
        TransferInstruction::CpiTransfer(args) => transfer_sol_with_cpi(accounts, args),
        TransferInstruction::ProgramTransfer(args) => {
            transfer_sol_with_program(program_id, accounts, args)
        }
    }
}

>>> program-examples/tokens/token-2022/non-transferable/native/program/src/lib.rs
use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::invoke,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
        sysvar::Sysvar,
    },
    spl_token_2022::{extension::ExtensionType, instruction as token_instruction, state::Mint},
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct CreateTokenArgs {
    pub token_decimals: u8,
}

entrypoint!(process_instruction);

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let args = CreateTokenArgs::try_from_slice(instruction_data)?;

    let accounts_iter = &mut accounts.iter();

    let mint_account = next_account_info(accounts_iter)?;
    let mint_authority = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let rent = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    // Find the size for the account with the Extension
    let space = ExtensionType::get_account_len::<Mint>(&[ExtensionType::NonTransferable]);

    // Get the required rent exemption amount for the account
    let rent_required = Rent::get()?.minimum_balance(space);

    // Create the account for the Mint and allocate space
    msg!("Mint account address : {}", mint_account.key);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint_account.key,
            rent_required,
            space as u64,
            token_program.key,
        ),
        &[
            mint_account.clone(),
            payer.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;

    // This needs to be done before the Mint is initialized

    // Initialize the Non Transferable Mint Extension
    invoke(
        &token_instruction::initialize_non_transferable_mint(token_program.key, mint_account.key)
            .unwrap(),
        &[
            mint_account.clone(),
            token_program.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the Token Mint
    invoke(
        &token_instruction::initialize_mint(
            token_program.key,
            mint_account.key,
            mint_authority.key,
            Some(mint_authority.key),
            args.token_decimals,
        )?,
        &[
            mint_account.clone(),
            mint_authority.clone(),
            token_program.clone(),
            rent.clone(),
        ],
    )?;

    msg!("Mint created!");

    Ok(())
}

>>> program-examples/basics/cross-program-invocation/native/programs/lever/src/lib.rs
use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(not(feature = "no-entrypoint"))]
use solana_program::entrypoint;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Ok(power_status) = PowerStatus::try_from_slice(instruction_data) {
        return initialize(program_id, accounts, power_status);
    }

    if let Ok(set_power_status) = SetPowerStatus::try_from_slice(instruction_data) {
        return switch_power(accounts, set_power_status.name);
    }

    Err(ProgramError::InvalidInstructionData)
}

pub fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    power_status: PowerStatus,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let power = next_account_info(accounts_iter)?;
    let user = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let account_span = (power_status.try_to_vec()?).len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke(
        &system_instruction::create_account(
            user.key,
            power.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[user.clone(), power.clone(), system_program.clone()],
    )?;

    power_status.serialize(&mut &mut power.data.borrow_mut()[..])?;

    Ok(())
}

pub fn switch_power(accounts: &[AccountInfo], name: String) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let power = next_account_info(accounts_iter)?;

    let mut power_status = PowerStatus::try_from_slice(&power.data.borrow())?;
    power_status.is_on = !power_status.is_on;
    power_status.serialize(&mut &mut power.data.borrow_mut()[..])?;

    msg!("{} is pulling the power switch!", &name);

    match power_status.is_on {
        true => msg!("The power is now on."),
        false => msg!("The power is now off!"),
    };

    Ok(())
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct SetPowerStatus {
    pub name: String,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct PowerStatus {
    pub is_on: bool,
}

>>> program-examples/basics/cross-program-invocation/native/programs/hand/src/lib.rs
use borsh::BorshDeserialize;
use cross_program_invocatio_native_lever::SetPowerStatus;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    pubkey::Pubkey,
};

entrypoint!(pull_lever);

fn pull_lever(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let power = next_account_info(accounts_iter)?;
    let lever_program = next_account_info(accounts_iter)?;

    let set_power_status_instruction = SetPowerStatus::try_from_slice(instruction_data)?;

    let ix = Instruction::new_with_borsh(
        *lever_program.key,                        // Our lever program's ID
        &set_power_status_instruction,             // Passing instructions through
        vec![AccountMeta::new(*power.key, false)], // Just the required account for the other program
    );

    invoke(&ix, &[power.clone()])
}

</PART2>
# System Prompt Generator for Solana Rust Code Analysis

## Instructions for Processing Raw Concatenated Files

You are tasked with processing raw concatenated files containing Solana documentation and code examples to create a comprehensive system prompt. Your output will be used to configure an LLM as a specialized Solana program Rust code analyst.

## Critical Requirements

**DO NOT TRUNCATE OR SHORTEN YOUR OUTPUT UNDER ANY CIRCUMSTANCES**

- Use as many tokens as necessary to preserve ALL relevant context
- Include every single Rust code example, function signature, and implementation detail
- Maintain complete documentation explanations for all Rust concepts
- Preserve all code comments and explanatory text
- Include all import statements, dependency information, and setup instructions
- Keep all function implementations in their entirety
- Maintain all struct definitions, enums, and type information
- Preserve all error handling patterns and examples
- Include complete test code and examples

## Processing Instructions

1. **Extract Only Rust-Related Content**: Filter out non-Rust content while preserving all Rust code, documentation, and related explanations.

2. **Organize Content Hierarchically**: Structure the information logically while maintaining all details.

3. **Preserve Context**: Ensure that code examples retain their surrounding explanatory text and context.

4. **Maintain Completeness**: Every line of Rust code, every explanation, and every technical detail must be included.

## Output Format

Create a comprehensive system prompt with the following structure:

```
# SYSTEM PROMPT: Solana Rust Program Code Analyst

You are a highly specialized Solana blockchain program analyst with deep expertise in Rust programming and Solana's account model, program architecture, and development patterns.

## Your Role and Capabilities

[Detailed role description based on the processed content]

## Core Solana Concepts You Must Understand

### Account Model
[Complete documentation about Solana's account model with all technical details]

### Program Structure
[Full program structure documentation with all code examples]

### [Additional core concepts with complete explanations]

## Rust Code Patterns and Examples

### Complete Code Examples
[Every single complete code example from the source material]

### Function Implementations
[All function implementations with full code]

### Struct and Enum Definitions
[All type definitions with complete code]

### Error Handling Patterns
[All error handling examples with complete implementations]

## Dependencies
[Complete dependency information]

## Advanced Topics
[All advanced concepts with complete explanations and code]

## Code Analysis Guidelines
[Instructions for how to analyze Solana Rust code based on the processed content]

## Common Patterns and Best Practices
[All best practices and patterns with complete examples]

## Troubleshooting and Common Issues
[All troubleshooting information with complete solutions]
```

## Key Principles for Your Output

1. **Completeness Over Brevity**: Include everything, even if it makes the prompt very long
2. **Preserve Technical Accuracy**: Maintain all technical details and specifications
3. **Keep Code Intact**: Never truncate or summarize code examples
4. **Maintain Context**: Ensure explanations accompany their relevant code
5. **Hierarchical Organization**: Structure content logically while keeping all details

## What NOT to Do

- Do not summarize or paraphrase code examples
- Do not remove any function implementations
- Do not skip any struct or enum definitions
- Do not omit any documentation explanations
- Do not reduce the level of technical detail
- Do not truncate for length considerations
- Do not remove any import statements or dependencies
- Do not skip any test code or examples

## Final Output Goal

The resulting system prompt should be comprehensive enough that an LLM configured with it can:
- Analyze any Solana Rust program with deep understanding
- Provide detailed explanations of program architecture
- Identify patterns, best practices, and potential issues
- Suggest improvements and optimizations
- Understand the complete context of Solana development

**Remember: Your output should be as long as necessary to preserve all relevant information. Token count is not a constraint - completeness is the only requirement.**
