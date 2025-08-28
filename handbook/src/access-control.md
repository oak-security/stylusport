# Access control

This chapter demonstrates how to translate Solana's signer checks and PDA patterns to Stylus. Learn how to verify callers with [`MessageAccess::msg_sender`](https://docs.rs/stylus-sdk/latest/stylus_sdk/prelude/trait.MessageAccess.html#tymethod.msg_sender), replace PDAs with contract-controlled state and authorization logic.

## Solana authentication model

Solana's stateless program model requires verifying account relationships and signatures to enforce access control. The first step verifies which accounts have signed the instruction. Next, the program checks PDAs: both those derived from access-control configuration and those tied to the state requiring authenticated, mutable access. It then validates that the provided accounts match their expected seeds and the correct program owns them. Only after these checks can the program compare the signer keys with the authorized keys stored in the verified access-control configuration. This model often leads Solana programs to construct hierarchies of PDAs to guarantee that access-control logic applies consistently across all dependent state.

### Native

```rust
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
```

### Anchor

```rust
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
```

## Stylus authentication model

Stylus contracts handle access control by checking the caller address relative to those stored in the contract's state. The contract obtains the caller address using the [`MessageAccess::msg_sender`](https://docs.rs/stylus-sdk/latest/stylus_sdk/prelude/trait.MessageAccess.html#tymethod.msg_sender) trait method.

```rust
#[storage]
pub struct Config {
    authority: StorageAddress,
    publisher: StorageAddress,
}

#[storage]
pub struct Price {
    base: StorageU256,
    quote: StorageU256,
    timestamp: StorageU64,
}

#[storage]
#[entrypoint]
pub struct AccessControl {
    config: Config,
    last_price: Price,
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    error Unauthorized();
}

#[derive(SolidityError, Debug, PartialEq, Eq)]
pub enum AccessControlError {
    Unauthorized(Unauthorized),
}

#[public]
impl AccessControl {
    #[constructor]
    pub fn constructor(&mut self, authority: Address, publisher: Address) {
        self.config.authority.set(authority);
        self.config.publisher.set(publisher);
    }

    pub fn update_config(&mut self, publisher: Address) -> Result<(), AccessControlError> {
        let sender = self.vm().msg_sender();

        if sender != self.config.authority.get() {
            return Err(AccessControlError::Unauthorized(Unauthorized {}));
        }

        self.config.publisher.set(publisher);

        Ok(())
    }

    pub fn publish_price(&mut self, base: U256, quote: U256) -> Result<(), AccessControlError> {
        let sender = self.vm().msg_sender();

        if sender != self.config.publisher.get() {
            return Err(AccessControlError::Unauthorized(Unauthorized {}));
        }

        let timestamp = self.vm().block_timestamp();

        self.last_price.base.set(base);
        self.last_price.quote.set(quote);
        self.last_price.timestamp.set(U64::from(timestamp));

        Ok(())
    }

    pub fn get_authority(&self) -> Address {
        self.config.authority.get()
    }

    pub fn get_publisher(&self) -> Address {
        self.config.publisher.get()
    }

    pub fn get_last_price(&self) -> (U256, U256, U64) {
        (
            self.last_price.base.get(),
            self.last_price.quote.get(),
            self.last_price.timestamp.get(),
        )
    }
}
```

## Standardized access control patterns

The most common access control pattern involves a contract having an admin or an [owner](https://docs.openzeppelin.com/contracts/5.x/access-control#ownership-and-ownable). The account with the owner role can perform actions such as pausing and unpausing the contract or update the configuration.

OpenZeppelin develops many well-used and audited re-usable components for EVM-based contracts. They have ported many of those components from Solidity to Rust using Stylus' inheritance and state composition features.

For example, the Two-Step Ownership component implements ownership tracking and enables safe ownership transitions.

```rust
sol! {
    #[derive(Debug)]
    error ContractAlreadyPaused();
    #[derive(Debug)]
    error ContractAlreadyUnpaused();
}

#[derive(SolidityError, Debug)]
// In order to generate an ABI for the contract you need to manually wire
// up OpenZeppelin's error types defined with `sol!` rather than the their
// `ownable::Error` type which implements `SolidityError` but not `SolError`
pub enum ContractError {
    InvalidOwner(ownable::OwnableInvalidOwner),
    Unauthorized(ownable::OwnableUnauthorizedAccount),
    AlreadyPaused(ContractAlreadyPaused),
    AlreadyUnpaused(ContractAlreadyUnpaused),
}

impl From<ownable::Error> for ContractError {
    fn from(value: ownable::Error) -> Self {
        match value {
            ownable::Error::UnauthorizedAccount(e) => Self::Unauthorized(e),
            ownable::Error::InvalidOwner(e) => Self::InvalidOwner(e),
        }
    }
}

#[storage]
#[entrypoint]
pub struct OwnableContract {
    // Nest the OpenZeppelin implementation within the contract
    ownable: Ownable2Step,
    is_paused: StorageBool,
}

#[public]
#[implements(IOwnable2Step<Error = ownable::Error>)]
impl OwnableContract {
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), ContractError> {
        // You must ensure the nested implementation constructor is called correctly
        self.ownable.constructor(self.vm().msg_sender())?;

        self.is_paused.set(true);

        Ok(())
    }

    pub fn pause_contract(&mut self) -> Result<(), ContractError> {
        // You can then use convenience methods on the nested implementation
        self.ownable.only_owner()?;

        if self.is_paused() {
            return Err(ContractAlreadyPaused {}.into());
        }

        self.is_paused.set(true);

        Ok(())
    }

    pub fn unpause_contract(&mut self) -> Result<(), ContractError> {
        self.ownable.only_owner()?;

        if !self.is_paused() {
            return Err(ContractAlreadyUnpaused {}.into());
        }

        self.is_paused.set(false);

        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.get()
    }
}

#[public]
// Wire everything up by delegating interface trait methods to the nested implementation.
// You could modify the standard behavior here if you wished.
impl IOwnable2Step for OwnableContract {
    type Error = ownable::Error;

    fn owner(&self) -> Address {
        self.ownable.owner()
    }

    fn pending_owner(&self) -> Address {
        self.ownable.pending_owner()
    }

    fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), Self::Error> {
        self.ownable.transfer_ownership(new_owner)
    }

    fn accept_ownership(&mut self) -> Result<(), Self::Error> {
        self.ownable.accept_ownership()
    }

    fn renounce_ownership(&mut self) -> Result<(), Self::Error> {
        self.ownable.renounce_ownership()
    }
}
```

## Next steps

With access control patterns established, the next chapter covers [External Calls](./external-calls.md) - converting Solana's Cross-Program Invocations (CPIs) to Stylus contract calls.
