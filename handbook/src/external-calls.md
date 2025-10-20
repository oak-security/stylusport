# External Calls

This chapter demonstrates how to translate Solana CPIs into Stylus external calls.

## Solana

Solana’s Cross-Program Invocation (CPI) model relies on instruction-based communication. Programs build instructions with the target program ID, required accounts, and instruction data. Unlike systems that allow direct state queries, Solana programs must receive all state through accounts passed in the transaction. CPIs are therefore used when a program needs to modify state owned by another program, with the caller explicitly providing all accounts the callee requires.

When a program controls a PDA that must sign for another program’s operation, it uses `invoke_signed` with the PDA’s seeds. The runtime verifies the seeds and grants signing authority.

### Native 

```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LastResultAccount {
    pub last_result: u128,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if !check_id(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    if AdderArgs::try_from_slice(instruction_data).is_err() {
        return Err(ProgramError::InvalidInstructionData);
    };

    let [payer, last_result_account, system_program, adder_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if *adder_program.key != ADDER_PROGRAM_ID {
        return Err(ProgramError::InvalidAccountData);
    }

    // Find the expected PDA and bump
    let (expected_pda, bump) =
        Pubkey::find_program_address(&[LAST_RESULT_ACCOUNT_SEED], program_id);

    // Verify the provided account matches the expected PDA
    if last_result_account.key != &expected_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    invoke(
        &solana_program::instruction::Instruction {
            program_id: cpi_to_external_call_solana_adder::ID,
            accounts: vec![],
            data: instruction_data.to_owned(),
        },
        &[adder_program.clone()],
    )?;

    let (invoked_program, data) = get_return_data().expect("return data is some after invoke");

    assert_eq!(
        invoked_program, ADDER_PROGRAM_ID,
        "expected return data from {ADDER_PROGRAM_ID}, received from {invoked_program}"
    );

    let Response { result } = Response::try_from_slice(&data)?;

    let last_result_account_data = borsh::to_vec(&LastResultAccount {
        last_result: result,
    })?;

    // Check if LastResult PDA Account needs to be created
    if last_result_account.owner != program_id {
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(last_result_account_data.len());

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                last_result_account.key,
                required_lamports,
                last_result_account_data.len() as u64,
                program_id,
            ),
            &[
                payer.clone(),
                last_result_account.clone(),
                system_program.clone(),
            ],
            &[&[LAST_RESULT_ACCOUNT_SEED, &[bump]]],
        )?;
    }

    last_result_account
        .try_borrow_mut_data()?
        .copy_from_slice(&last_result_account_data);

    Ok(())
}
```

### Anchor 

```rust
#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct Args {
    pub a: u64,
    pub b: u64,
}

#[derive(InitSpace)]
#[account]
pub struct LastResultAccount {
    pub last_result: u128,
}

#[derive(Accounts)]
#[instruction(data: Args)]
pub struct Add<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + LastResultAccount::INIT_SPACE,
        seeds = [LAST_RESULT_ACCOUNT_SEED],
        bump,
    )]
    pub last_result: Account<'info, LastResultAccount>,
    pub system_program: Program<'info, System>,
    pub adder_program: UncheckedAccount<'info>,
}

#[program]
pub mod cpi {
    use super::*;

    pub fn add(ctx: Context<Add>, args: Args) -> Result<()> {
        if *ctx.accounts.adder_program.key != ADDER_PROGRAM_ID {
            return Err(ProgramError::InvalidAccountData.into());
        }

        let adder_instruction_data = ::borsh::to_vec(&AdderArgs {
            a: args.a,
            b: args.b,
        })
        .expect("infallible serialization");

        invoke(
            &Instruction {
                program_id: ADDER_PROGRAM_ID,
                accounts: vec![],
                data: adder_instruction_data,
            },
            &[ctx.accounts.adder_program.to_account_info()],
        )?;

        let (invoked_program, data) = get_return_data().expect("return data is some after invoke");

        assert_eq!(
            invoked_program, ADDER_PROGRAM_ID,
            "expected return data from {ADDER_PROGRAM_ID}, received from {invoked_program}"
        );

        let Response { result } = Response::try_from_slice(&data)?;

        ctx.accounts.last_result.last_result = result;

        Ok(())
    }
}
```

## Stylus

Stylus contracts use an EVM ABI encoding model that supports calling both state queries and modification functions. Unlike Solana, where all state must be passed explicitly, Stylus contracts can directly call any other contracts, using static calls to read state or regular calls to modify it.

Call contexts are configured via the Call type, giving fine-grained control over gas limits and value transfers. Stylus provides two abstraction layers: high-level typed interfaces generated by `sol_interface!`, and low-level `call`, `static_call`, and `RawCall` methods for direct byte manipulation when needed.

<div class="warning">
Stylus contracts revert on reentrant calls by default, blocking an entire class of exploits. You can enable reentrancy with the reentrant feature flag, but this is highly dangerous and should only be done after expert review.
</div>

```rust
fn add_calldata(a: u64, b: u64) -> Vec<u8> {
    [
        [110u8, 44u8, 115u8, 45u8].as_slice(), // keccak(b"add(uint64,uint64)")[..4],
        abi::encode_params(&(a, b)).as_slice(),
    ]
    .concat()
}

// function add(uint64 a, uint64 b) external view returns (uint128);
// returns a big-endian u128 (16 bytes) padded to 32 bytes
fn parse_add_returndata(returndata: &[u8]) -> Option<u128> {
    if returndata.len() != 32 {
        return None;
    }

    returndata[16..].try_into().map(u128::from_be_bytes).ok()
}

#[storage]
#[entrypoint]
pub struct ExternalCaller {
    /// A negative value indicates no result has been obtained yet
    last_result: StorageI256,
    adder_address: StorageAddress,
}

#[public]
impl ExternalCaller {
    #[constructor]
    pub fn constructor(&mut self, adder_address: Address) {
        assert_ne!(
            adder_address,
            Address::ZERO,
            "adder_address cannot be a zero-address"
        );
        assert!(
            self.vm().code_size(adder_address) > 0,
            "adder_address must be a contract"
        );

        self.last_result.set(I256::MINUS_ONE);
        self.adder_address.set(adder_address);
    }

    pub fn add(&mut self, a: u64, b: u64) -> u128 {
        // low-level static call used to allow unit testing
        // sol_interface! generated interfaces can only be tested in a WASM runtime
        // see: https://github.com/OffchainLabs/stylus-sdk-rs/issues/301
        let returndata = self
            .vm()
            .static_call(
                &calls::context::Call::new(),
                self.get_adder_address(),
                &add_calldata(a, b),
            )
            .expect("valid contract call");

        let result = parse_add_returndata(&returndata).expect("valid return data");

        self.last_result.set(I256::unchecked_from(result));

        result
    }

    pub fn get_adder_address(&self) -> Address {
        self.adder_address.get()
    }

    pub fn get_last_result(&self) -> I256 {
        self.last_result.get()
    }
}
```

## Next Steps

With external calls mastered, you're ready to explore:
- [Native Token Handling](./native-tokens.md) - Differences in receiving, escrowing and transferring native SOL and ETH
- [Fungible Tokens](./fungible-tokens.md) - Migrating SPL tokens to ERC-20
- [Non-Fungible Tokens](./non-fungible-tokens.md) - Migrating Metaplex NFTs to ERC-721

