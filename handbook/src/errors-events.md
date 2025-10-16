# Chapter 8: Errors and Events

Proper error reporting and event emission are crucial for robust smart contracts and user experience. This chapter covers migrating from Solana's `msg!()` logging and `ProgramError` type to Stylus's structured events and custom error types.

## Errors

### Solana

The error type used for all Solana programs is [`solana_program::program_error::ProgramError`](https://docs.rs/solana-program/latest/solana_program/program_error/enum.ProgramError.html) which is defined as:

```rust
pub enum ProgramError {
    /// Allows on-chain programs to implement program-specific error types and see them returned
    /// by the Solana runtime. A program-specific error may be any type that is represented as
    /// or serialized to a u32 integer.
    Custom(u32),
    InvalidArgument,
    InvalidInstructionData,
    InvalidAccountData,
    AccountDataTooSmall,
    InsufficientFunds,
    IncorrectProgramId,
    MissingRequiredSignature,
    AccountAlreadyInitialized,
    UninitializedAccount,
    NotEnoughAccountKeys,
    AccountBorrowFailed,
    MaxSeedLengthExceeded,
    InvalidSeeds,
    BorshIoError(String),
    AccountNotRentExempt,
    UnsupportedSysvar,
    IllegalOwner,
    MaxAccountsDataAllocationsExceeded,
    InvalidRealloc,
    MaxInstructionTraceLengthExceeded,
    BuiltinProgramsMustConsumeComputeUnits,
    InvalidAccountOwner,
    ArithmeticOverflow,
    Immutable,
    IncorrectAuthority,
}
```

Many of these generic variants can be returned during account and instruction validation. The `Custom` variant can be used to return program-specific errors such as those arising from business logic. The user simply needs to be able to convert their custom error to a `u32` integer.

In native Solana programs, this is done like so:

```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    InvalidAmount {},
    Unauthorized {},
}

#[derive(Debug, Clone, Copy)]
// allows casting to u32 for value enums (no associated data)
#[repr(u32)]
pub enum ErrorCode {
    InvalidAmount,
    Unauthorized,
}

impl From<ErrorCode> for ProgramError {
    fn from(value: ErrorCode) -> Self {
        Self::Custom(value as _)
    }
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
        Instruction::InvalidAmount {} => process_invalid_value(accounts),
        Instruction::Unauthorized {} => process_unauthorized(accounts),
    }
}

fn process_invalid_value(_accounts: &[AccountInfo]) -> ProgramResult {
    Err(ErrorCode::InvalidAmount.into())
}

fn process_unauthorized(_accounts: &[AccountInfo]) -> ProgramResult {
    Err(ErrorCode::Unauthorized.into())
}
```

If we expand the `entrypoint!` macro, we can see that ultimately the program returns a `u64` integer after processing an instruction:

```rust
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) = unsafe {
        ::solana_program_entrypoint::deserialize(input)
    };
    match process_instruction(program_id, &accounts, instruction_data) {
        // returns 0 for success
        Ok(()) => ::solana_program_entrypoint::SUCCESS,
        // returns solana_program::program_error::ProgramError converted to u64
        // Every variant apart from Custom(_) is mapped to a value > u32::MAX + 1
        // Custom(0) is converted to 1 << 32, ensuring that every custom error: 0 < error_code <= u32::MAX + 1
        Err(error) => error.into(),
    }
}
```

Anchor provides the `#[error_code]` macro to reduce the boilerplate required to setup custom errors. Custom errors can also be specified within constraint rules:

```rust
#[program]
pub mod errors_events {
    use super::*;

    pub fn invalid_amount(_ctx: Context<InvalidAmount>) -> Result<()> {
        Err(ErrorCode::InvalidAmount.into())
    }

    pub fn unauthorized(_ctx: Context<Unauthorized>) -> Result<()> {
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

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid amount: amount must be greater than 0")]
    InvalidAmount,
    #[msg("Unauthorized")]
    Unauthorized,
}
```

The `#[error_code]` macro expands to:

```rust
#[repr(u32)]
pub enum ErrorCode {
    InvalidAmount,
    Unauthorized,
}

impl ErrorCode {
    /// Gets the name of this [#enum_name].
    pub fn name(&self) -> String {
        match self {
            ErrorCode::InvalidAmount => "InvalidAmount".to_string(),
            ErrorCode::Unauthorized => "Unauthorized".to_string(),
        }
    }
}

impl From<ErrorCode> for u32 {
    fn from(e: ErrorCode) -> u32 {
        e as u32 + anchor_lang::error::ERROR_CODE_OFFSET
    }
}

impl From<ErrorCode> for anchor_lang::error::Error {
    fn from(error_code: ErrorCode) -> anchor_lang::error::Error {
        anchor_lang::error::Error::from(anchor_lang::error::AnchorError {
            error_name: error_code.name(),
            error_code_number: error_code.into(),
            error_msg: error_code.to_string(),
            error_origin: None,
            compared_values: None,
        })
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(
        &self,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        match self {
            ErrorCode::InvalidAmount => {
                fmt.write_fmt(
                    format_args!("Invalid amount: amount must be greater than 0"),
                )
            }
            ErrorCode::Unauthorized => fmt.write_fmt(format_args!("Unauthorized")),
        }
    }
}
```

Note that `anchor_lang::error::ERROR_CODE_OFFSET` is used to reserve space for Anchor's own custom errors.

Each instruction handler returns `Result<T, anchor_lang::error::Error>`. If a handler returns `Err(anchor_lang::error::Error)`, it is converted first to a `solana_program::error::ProgramError` before ultimately being returned as an integer, as show in the macro expansion below:

```rust
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let (program_id, accounts, instruction_data) = unsafe {
        ::solana_program_entrypoint::deserialize(input)
    };
    match entry(program_id, &accounts, instruction_data) {
        Ok(()) => ::solana_program_entrypoint::SUCCESS,
        Err(error) => error.into(),
    }
}

pub fn entry<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult {
    try_entry(program_id, accounts, data)
        .map_err(|e| {
            e.log();
            e.into()
        })
}

fn try_entry<'info>(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    data: &[u8],
) -> Result<(), > {
    if *program_id != ID {
        return Err(anchor_lang::error::ErrorCode::DeclaredProgramIdMismatch.into());
    }
    dispatch(program_id, accounts, data)
}
```

### Stylus

In contrast to Solana programs, a Stylus contract entrypoint always returns either zero or one, where zero denotes a successful call and one signifies an error occured. For a contract function with returns, `Result<T, E>`, the error type `E` is converted to a byte array and written to the return data buffer:

```rust
#[no_mangle]
pub extern "C" fn user_entrypoint(len: usize) -> usize {
    let host = stylus_sdk::host::VM(stylus_sdk::host::WasmVM {});
    if host.msg_reentrant() {
        return 1;
    }
    host.pay_for_memory_grow(0);
    let input = host.read_args(len);
    // Calls the stylus_sdk::abi::router_entrypoint function returning ArbResult aka Result<Vec<u8>, Vec<u8>>
    let (data, status) = match __stylus_struct_entrypoint(input, host.clone()) {
        Ok(data) => (data, 0),
        Err(data) => (data, 1),
    };
    host.flush_cache(false);
    host.write_result(&data);
    status
}
```

The `SolidityError` derive macro can be used to implement `From<E>` for `Vec<u8>` for the contract defined error type `E`:

```rust
sol! {
    error InvalidAmount(uint256 expected, uint256 received);
    error Unauthorized(address account);
}

#[derive(SolidityError)]
pub enum ContractError {
    InvalidAmount(InvalidAmount),
    Unauthorized(Unauthorized),
}
```

Note that there is not also a trait with the name `SolidityError` like most Rust derive macros, instead it expands to the following:

```rust 
impl From<InvalidAmount> for ContractError {
    fn from(value: InvalidAmount) -> Self {
        ContractError::InvalidAmount(value)
    }
}

impl From<Unauthorized> for ContractError {
    fn from(value: Unauthorized) -> Self {
        ContractError::Unauthorized(value)
    }
}

impl From<ContractError> for alloc::vec::Vec<u8> {
    fn from(err: ContractError) -> Self {
        match err {
            ContractError::InvalidAmount(e) => stylus_sdk::call::MethodError::encode(e),
            ContractError::Unauthorized(e) => stylus_sdk::call::MethodError::encode(e),
        }
    }
}
```

The derive macro expects an enum consisting on one or more unit variants containing a single type implementing the [`stylus_sdk::call::MethodError`](https://docs.rs/stylus-sdk/latest/stylus_sdk/call/trait.MethodError.html) trait. There is a [blanket implementation](https://docs.rs/stylus-sdk/latest/stylus_sdk/call/trait.MethodError.html#impl-MethodError-for-T) of `stylus_sdk::call::MethodError` for any type which also implements [`alloy_sol_types::SolError`](https://docs.rs/alloy-sol-types/0.8.20/alloy_sol_types/trait.SolError.html). The [`sol!`](https://docs.rs/alloy-sol-macro/0.8.20/alloy_sol_macro/macro.sol.html) macro is the easiest way to define types that implement `SolError`.

The above mechanisms can be combined to allow Stylus contracts to return structured custom errors:

```rust
#[storage]
#[entrypoint]
pub struct ErrorsEvents {}

sol! {
    error InvalidAmount(uint256 expected, uint256 received);
    error Unauthorized(address account);
}

#[derive(SolidityError)]
pub enum ContractError {
    InvalidAmount(InvalidAmount),
    Unauthorized(Unauthorized),
}

#[public]
impl ErrorsEvents {
    pub fn invalid_amount(&mut self, expected: U256, received: U256) -> Result<(), ContractError> {
        Err(InvalidAmount { expected, received }.into())
    }

    pub fn unauthorized(&mut self) -> Result<(), ContractError> {
        Err(Unauthorized {
            account: self.vm().msg_sender(),
        }
        .into())
    }
}
```

## Logging and events

### Solana

[Logging](https://docs.rs/solana-program/latest/solana_program/log/index.html) in Solana is in the form of lines of free text. Due to the lack of standardized ABI for function selection and all errors being reduced to integers, Solana program logs are an important part of instruction execution auditing and tracking. Additionally, they are frequently used for debugging programs during the development process.

The following excerpt from the [spl-token-2022](https://github.com/solana-program/token-2022/blob/57b3bcbd3c15de22db47ae2024fc73b43dafdd8a/program/src/processor.rs#L1637-L1945) illustrates the convention of logging the name of the instruction being executed:

```rust
 pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        if let Ok(instruction_type) = decode_instruction_type(input) {
            match instruction_type {
                PodTokenInstruction::InitializeMint => {
                    msg!("Instruction: InitializeMint");
                    let (data, freeze_authority) =
                        decode_instruction_data_with_coption_pubkey::<InitializeMintData>(input)?;
                    Self::process_initialize_mint(
                        accounts,
                        data.decimals,
                        &data.mint_authority,
                        freeze_authority,
                    )
                }
                PodTokenInstruction::InitializeMint2 => {
                    msg!("Instruction: InitializeMint2");
                    let (data, freeze_authority) =
                        decode_instruction_data_with_coption_pubkey::<InitializeMintData>(input)?;
                    Self::process_initialize_mint2(
                        accounts,
                        data.decimals,
                        &data.mint_authority,
                        freeze_authority,
                    )
                }
                PodTokenInstruction::InitializeAccount => {
                    msg!("Instruction: InitializeAccount");
                    Self::process_initialize_account(accounts)
                }
                // ...
                PodTokenInstruction::PausableExtension => {
                    msg!("Instruction: PausableExtension");
                    pausable::processor::process_instruction(program_id, accounts, &input[1..])
                }
            }
        } else if let Ok(instruction) = TokenMetadataInstruction::unpack(input) {
            token_metadata::processor::process_instruction(program_id, accounts, instruction)
        } else if let Ok(instruction) = TokenGroupInstruction::unpack(input) {
            token_group::processor::process_instruction(program_id, accounts, instruction)
        } else {
            Err(TokenError::InvalidInstruction.into())
        }
    }
}
```

Another common use is to provide additional context before returning errors, as can be seen in the [metaplex-token-metadata program](https://github.com/metaplex-foundation/mpl-token-metadata/blob/a7ee5e17ed60feaafeaa5582a4f46d9317c1b412/programs/token-metadata/program/src/utils/token.rs#L145-L202):

```rust
pub(crate) fn validate_mint(
    mint: &AccountInfo,
    metadata: &AccountInfo,
    token_standard: TokenStandard,
) -> Result<Mint, ProgramError> {
let mint_data = &mint.data.borrow();
    let mint = StateWithExtensions::<Mint>::unpack(mint_data)?;

    if !mint.base.is_initialized() {
        return Err(MetadataError::Uninitialized.into());
    }

    if matches!(
        token_standard,
        TokenStandard::NonFungible | TokenStandard::ProgrammableNonFungible
    ) {
        // validates the mint extensions
        mint.get_extension_types()?
            .iter()
            .try_for_each(|extension_type| {
                if !NON_FUNGIBLE_MINT_EXTENSIONS.contains(extension_type) {
                    msg!("Invalid mint extension: {:?}", extension_type);
                    return Err(MetadataError::InvalidMintExtensionType);
                }
                Ok(())
            })?;
    }

    // For all token standards:
    //
    // 1) if the mint close authority extension is enabled, it must
    //    be set to be the metadata account; and
    if let Ok(extension) = mint.get_extension::<MintCloseAuthority>() {
        let close_authority: Option<Pubkey> = extension.close_authority.into();
        if close_authority.is_none() || close_authority != Some(*metadata.key) {
            return Err(MetadataError::InvalidMintCloseAuthority.into());
        }
    }

    // 2) if the metadata pointer extension is enabled, it must be set
    //    to the metadata account address
    if let Ok(extension) = mint.get_extension::<MetadataPointer>() {
        let authority: Option<Pubkey> = extension.authority.into();
        let metadata_address: Option<Pubkey> = extension.metadata_address.into();

        if authority.is_some() {
            msg!("Metadata pointer extension: authority must be None");
            return Err(MetadataError::InvalidMetadataPointer.into());
        }

        if metadata_address != Some(*metadata.key) {
            msg!("Metadata pointer extension: metadata address mismatch");
            return Err(MetadataError::InvalidMetadataPointer.into());
        }
    }

    Ok(mint.base)
}
```

In addition to the `msg!` macro providing string logging with formatting, the `solana::log` module provides a number of other options:

```rust
fn process_log(accounts: &[AccountInfo]) -> ProgramResult {
    log::sol_log("just a regular string");
    log::sol_log_64(1, 2, 3, 4, 5);
    log::sol_log_compute_units();
    log::sol_log_data(&[b"some", b"serialized", b"structures", b"as base64"]);
    log::sol_log_params(accounts, &[]);
    log::sol_log_slice(b"some bytes as hex");
    Ok(())
}
```

The program log from executing the above instruction handler is:

```
# sol_log:
Program log: just a regular string

# sol_log_u64:
Program log: 0x1, 0x2, 0x3, 0x4, 0x5

# sol_log_compute_units:
Program consumption: 1399140 units remaining

# sol_log_data:
Program data: c29tZQ== c2VyaWFsaXplZA== c3RydWN0dXJlcw== YXMgYmFzZTY0

# sol_log_params:
Program log: AccountInfo
Program log: 0x0, 0x0, 0x0, 0x0, 0x0
Program log: - Is signer
Program log: 0x0, 0x0, 0x0, 0x0, 0x1
Program log: - Key
Program log: 11157t3sqMV725NVRLrVQbAu98Jjfk1uCKehJnXXQs
Program log: - Lamports
Program log: 0x0, 0x0, 0x0, 0x0, 0x5f5e100
Program log: - Account data length
Program log: 0x0, 0x0, 0x0, 0x0, 0x0
Program log: - Owner
Program log: 11111111111111111111111111111111
Program log: AccountInfo
Program log: 0x0, 0x0, 0x0, 0x0, 0x1
Program log: - Is signer
Program log: 0x0, 0x0, 0x0, 0x0, 0x0
Program log: - Key
Program log: 11111111111111111111111111111111
Program log: - Lamports
Program log: 0x0, 0x0, 0x0, 0x0, 0xf14a0
Program log: - Account data length
Program log: 0x0, 0x0, 0x0, 0x0, 0xe
Program log: - Owner
Program log: NativeLoader1111111111111111111111111111111
Program log: Instruction data
Program log: 0x0, 0x0, 0x0, 0x0, 0x69
Program log: 0x0, 0x0, 0x0, 0x1, 0x6e
Program log: 0x0, 0x0, 0x0, 0x2, 0x73
Program log: 0x0, 0x0, 0x0, 0x3, 0x74
Program log: 0x0, 0x0, 0x0, 0x4, 0x72
Program log: 0x0, 0x0, 0x0, 0x5, 0x75
Program log: 0x0, 0x0, 0x0, 0x6, 0x63
Program log: 0x0, 0x0, 0x0, 0x7, 0x74
Program log: 0x0, 0x0, 0x0, 0x8, 0x69
Program log: 0x0, 0x0, 0x0, 0x9, 0x6f
Program log: 0x0, 0x0, 0x0, 0xa, 0x6e
Program log: 0x0, 0x0, 0x0, 0xb, 0x20
Program log: 0x0, 0x0, 0x0, 0xc, 0x64
Program log: 0x0, 0x0, 0x0, 0xd, 0x61
Program log: 0x0, 0x0, 0x0, 0xe, 0x74
Program log: 0x0, 0x0, 0x0, 0xf, 0x61

# sol_log_slice:
Program log: 0x0, 0x0, 0x0, 0x0, 0x73
Program log: 0x0, 0x0, 0x0, 0x1, 0x6f
Program log: 0x0, 0x0, 0x0, 0x2, 0x6d
Program log: 0x0, 0x0, 0x0, 0x3, 0x65
Program log: 0x0, 0x0, 0x0, 0x4, 0x20
Program log: 0x0, 0x0, 0x0, 0x5, 0x62
Program log: 0x0, 0x0, 0x0, 0x6, 0x79
Program log: 0x0, 0x0, 0x0, 0x7, 0x74
Program log: 0x0, 0x0, 0x0, 0x8, 0x65
Program log: 0x0, 0x0, 0x0, 0x9, 0x73
Program log: 0x0, 0x0, 0x0, 0xa, 0x20
Program log: 0x0, 0x0, 0x0, 0xb, 0x61
Program log: 0x0, 0x0, 0x0, 0xc, 0x73
Program log: 0x0, 0x0, 0x0, 0xd, 0x20
Program log: 0x0, 0x0, 0x0, 0xe, 0x68
Program log: 0x0, 0x0, 0x0, 0xf, 0x65
Program log: 0x0, 0x0, 0x0, 0x10, 0x78
```

In addition to the logging facilities provided by `solana_program::log`, Anchor provides macros to reduce the boilerplate in emiting structured events via the underlying `sol_log_data` function:

```rust
#[event]
pub struct OwnerChanged {
    previous_owner: Pubkey,
    current_owner: Pubkey,
}

#[program]
pub mod errors_events {
    use super::*;

    // ...

    pub fn emit_event(ctx: Context<EmitEvent>) -> Result<()> {
        emit!(OwnerChanged {
            previous_owner: *ctx.accounts.signer.key,
            current_owner: ID
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct EmitEvent<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

Executing the `EmitEvent` instruction results in the following program log:

```
Program JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG invoke [1]
Program log: Instruction: EmitEvent
Program data: It9n4e/nMzUAAAABkHB7w+8lvcmO11y3DWHIsQbcJI2O9h4dHbHKQP//////////////////////////////////////////
Program JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG consumed 1074 of 1400000 compute units
Program JEKNVnkbo3jma5nREBBJCDoXFVeKkD56V3xKrvRmWxFG success
```

Note how Anchor automatically inserts the `Instruction: EmitEvent` log message.

### Stylus

For Stylus contracts, emitting structured events is considered best practice whenever contract state changes. Similar to errors, events are defined using the `sol!` macro and then emitted using the [`log`](https://docs.rs/stylus-sdk/latest/stylus_sdk/prelude/fn.log.html) function:

```rust
sol! {
    event ItChanged(address previous_it, address current_it);
}

#[storage]
#[entrypoint]
pub struct ErrorsEvents {
    it: StorageAddress,
}

#[public]
impl ErrorsEvents {
    /// Tags the caller as "it", emitting an event for the state change
    pub fn tag(&mut self) {
        let msg_sender = self.vm().msg_sender();

        let previous_it = self.it.get();

        self.it.set(msg_sender);

        log(
            self.vm(),
            ItChanged {
                previous_it,
                current_it: msg_sender,
            },
        );
    }
}
```

## Next Steps

With error handling and events covered, you've completed the core migration concepts.

Continue to [Case Study - Migrating Bonafida's Token Vesting to Stylus](./case-study-bonafida-token-vesting.md) to see these concepts applied in a complete program migration.
