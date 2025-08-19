# Program structure and instructions

This chapter explains how to translate Solana's instruction-dispatch model to Stylus contracts. The transformation involves converting instruction handlers into direct methods, mapping parameter and return types to ABI-encodable forms.

## Solana program model

### Native

When not using a framework, Solana programs require manual instruction deserialization, account validation and instruction handler routing.

```rust
#[derive(BorshSerialize, BorshDeserialize)]
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

    // Validate accounts, create PDA account & write initial state...

    Ok(())
}

fn process_increment(accounts: &[AccountInfo]) -> ProgramResult {
    let [counter_state_pda] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    // Validate accounts & write incremented value...

    Ok(())
}

fn process_set_value(accounts: &[AccountInfo], new_value: u64) -> ProgramResult {
    let [counter_state_pda, authority] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    // Validate accounts & write new value...

    Ok(())
}
```

### Anchor

The Anchor framework abstracts the boilerplate for deserializing instruction data and function routing behind a combination of derive and procedural macros.

Some business logic, such as access control, developers can encode declaratively using attributes within the `#[derive(Accounts)]` macro.

```rust
use anchor_lang::prelude::*;

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
    #[account(init_if_needed, payer = authority, space = 8 + 8 + 32)]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut)]
    pub counter: Account<'info, Counter>,
}

#[derive(Accounts)]
pub struct SetValue<'info> {
    // Adds the constraint that the `authority` field in the `Counter` account
    // must match the `authority` key within this struct.
    #[account(mut, has_one = authority)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
pub struct Counter {
    pub value: u64,
    pub authority: Pubkey,
}
```

## Stylus contract model

Stylus uses macros to abstract the boilerplate of decoding calldata and handler function selection.

Unlike Solana, state storage couples to business logic and the contract manages it solely. Functions that change the contract state must take `&mut self`. Developers conventionally add read-only `view` functions to access contract state that clients may require. These cost no gas for external callers. Any function that takes `&self` in a development block tagged with `#[public]` appears as an externally viewable `view` function.

The developer can create contracts with some initial state by using the `#[constructor]` attribute macro to mark the initialization function. This function runs automatically as part of the contract creation flow.

```rust
sol! {
    #[derive(Debug, PartialEq, Eq)]
    error Unauthorized(address caller);
}

#[derive(SolidityError, Debug, PartialEq, Eq)]
pub enum CounterError {
    Unauthorized(Unauthorized),
}

#[storage]
#[entrypoint]
pub struct Counter {
    value: StorageU256,
    authority: StorageAddress,
}

#[public]
impl Counter {
    #[constructor]
    pub fn constructor(&mut self, initial_value: U256) {
        let authority = self.vm().msg_sender();

        self.value.set(initial_value);
        self.authority.set(authority);
    }

    pub fn increment(&mut self) -> U256 {
        let new_value = self.value.get() + U256::ONE;

        self.value.set(new_value);

        new_value
    }

    pub fn set_value(&mut self, new_value: U256) -> Result<(), CounterError> {
        let caller = self.vm().msg_sender();

        // Only authority can set value
        if caller != self.authority.get() {
            return Err(CounterError::Unauthorized(Unauthorized { caller }));
        }

        self.value.set(new_value);

        Ok(())
    }

    // View functions
    pub fn get_value(&self) -> U256 {
        self.value.get()
    }

    pub fn get_authority(&self) -> Address {
        self.authority.get()
    }
}
```

## Key transformation: Entry points

### Coming from native Solana

A 1-to-1 mapping exists between instruction `enum` variants and Stylus' `#[public]` functions that can change state (that take `&mut self`).

Any fields associated with the instruction `enum` variants convert to ABI-encodable function parameters.

### Coming from Anchor

Each `#[program]` function that takes a different `Context<T>` maps to a `#[public] &mut self` function in Stylus. Any parameters coming after `ctx` would also be required.

### Stylus idioms

#### Function return types

In both Native and Anchor-based Solana programs, most instruction handlers return a `Result<(), ProgramError>`, meaning no return data exists when no errors occur.

Stylus operates within the EVM ecosystem where successful function results commonly continue into further computation. This provides much more flexibility when it comes to return types.

Functions may return nothing at all, an infallible result (that just `T`) or a `Result<T, E>` where `E` supports `SolidityError` and `T` supports [`AbiType`](https://docs.rs/stylus-sdk/latest/stylus_sdk/abi/trait.AbiType.html). The programmer decides the best approach.

View or pure computation functions typically return infallible results and functions that change state according to some business logic typically return a `Result` type.

The [Errors and Events](./errors-events.md) section covers error type definition in more detail.

#### Contract state initialization

In Solana programs, initialization appears as just another instruction or series of instructions, but where care must protect unauthorized use.

Stylus, like Solidity contracts, provides a specialized `constructor` function that runs during contract creation with parameters provided by the contract deployer. Developers commonly use this pattern to provide initial values such as initial authorized addresses and other contract state as parameters to this function.

#### Parameter types

The constraint on parameter types in Stylus contracts requires that they support [`AbiType`](https://docs.rs/stylus-sdk/latest/stylus_sdk/abi/trait.AbiType.html). This trait parallels the `BorshDeserialize` and `BorshSerialize` traits in Solana programs. All primitive types and tuples of primitive types already support this trait.

The Stylus contract programmer can define more complex types such as `enums` and `structs` using the [`sol!` macro](https://docs.rs/alloy-sol-macro/0.8.20/alloy_sol_macro/macro.sol.html). Those patterns fall outside the scope of this guide.

## Next steps

Now that you understand program structure transformation, explore:
- [State Storage](./state-storage.md) - Converting account-based storage to contract storage
- [Access Control](./access-control.md) - Building ownership and permissions
- [External Calls](./external-calls.md) - Making cross-contract calls
