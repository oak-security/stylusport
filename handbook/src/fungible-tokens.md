# Fungible Token Handling

SPL Tokens provide fundamental standardized fungible token functionality for Solana applications. This chapter covers migrating SPL Token operations to ERC-20 patterns in Stylus, including instantiating, minting, transfers, and allowance mechanisms.

To illustrate a range of token operations in a concise example, we will implement a contract that creates a stakeable token with a capped supply.

## Solana

Solana separates token logic from user programs: the SPL Token program owns all mint and token accounts, requiring programs to use CPIs for any token operations. Each token needs a mint account (storing decimals, supply, authorities) and separate token accounts per holder. Programs manage PDAs for both their own state and any token accounts they control, never directly manipulating token balances. The mint authority controls token creation, while freeze authorities handle compliance. Token-2022 adds extensions like transfer fees and metadata while maintaining the same architectural model.

#### Native 

```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    Initialize,
    Stake { amount: u64 },
    Unstake { amount: u64 },
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
        Instruction::Initialize => process_initialize(program_id, accounts),
        Instruction::Stake { amount } => process_stake(program_id, accounts, amount),
        Instruction::Unstake { amount } => process_unstake(program_id, accounts, amount),
    }
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [mint_account, mint_supply_to_account, signer_account, token_program, associated_token_program, system_program, rent_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token_2022::id()
        || *associated_token_program.key != spl_associated_token_account::id()
        || *system_program.key != system_program::id()
        || *rent_sysvar.key != rent::sysvar::id()
    {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (mint_pda_key, mint_bump) = Pubkey::find_program_address(&[MINT_PDA_SEED], program_id);

    if mint_pda_key != *mint_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if !mint_account.data_is_empty()
        || mint_account.lamports() > 0
        || *mint_account.owner == spl_token_2022::id()
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if !mint_supply_to_account.data_is_empty()
        || mint_supply_to_account.lamports() > 0
        || *mint_supply_to_account.owner == spl_token_2022::id()
    {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Derive the expected associated token account address
    let expected_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        signer_account.key,
        &mint_pda_key,
        &spl_token_2022::id(),
    );

    if expected_ata != *mint_supply_to_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create mint account
    let space_required = Mint::get_packed_len();
    let lamports_required = Rent::get()?.minimum_balance(space_required);

    invoke_signed(
        &system_instruction::create_account(
            signer_account.key,
            mint_account.key,
            lamports_required,
            space_required as u64,
            &spl_token_2022::id(),
        ),
        &[
            signer_account.clone(),
            mint_account.clone(),
            system_program.clone(),
        ],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    // Initialize mint
    invoke_signed(
        &token_instruction::initialize_mint(
            &spl_token_2022::id(),
            mint_account.key,
            mint_account.key,
            Some(mint_account.key),
            DECIMALS,
        )?,
        &[mint_account.clone(), rent_sysvar.clone()],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    // Create associated token account
    invoke_signed(
        &associated_token_instruction::create_associated_token_account(
            signer_account.key,
            signer_account.key,
            mint_account.key,
            &spl_token_2022::id(),
        ),
        &[
            signer_account.clone(),
            mint_supply_to_account.clone(),
            signer_account.clone(),
            mint_account.clone(),
            system_program.clone(),
            token_program.clone(),
            associated_token_program.clone(),
        ],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    // Mint total supply to the associated token account
    invoke_signed(
        &token_instruction::mint_to(
            &spl_token_2022::id(),
            mint_account.key,
            mint_supply_to_account.key,
            mint_account.key,
            &[],
            TOTAL_SUPPLY,
        )?,
        &[
            mint_account.clone(),
            mint_supply_to_account.clone(),
            mint_account.clone(),
        ],
        &[&[MINT_PDA_SEED, &[mint_bump]]],
    )?;

    Ok(())
}

fn process_stake(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [stake_account, from_account, signer_account, mint_account, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token_2022::id() || *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify stake PDA
    let (stake_pda_key, stake_bump) =
        Pubkey::find_program_address(&[STAKE_PDA_SEED, signer_account.key.as_ref()], program_id);

    if stake_pda_key != *stake_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Create stake account if it doesn't exist
    if stake_account.data_is_empty() || *stake_account.owner != spl_token_2022::id() {
        let space_required = TokenAccount::get_packed_len();
        let lamports_required = Rent::get()?.minimum_balance(space_required);

        invoke_signed(
            &system_instruction::create_account(
                signer_account.key,
                stake_account.key,
                lamports_required,
                space_required as u64,
                &spl_token_2022::id(),
            ),
            &[
                signer_account.clone(),
                stake_account.clone(),
                system_program.clone(),
            ],
            &[&[STAKE_PDA_SEED, signer_account.key.as_ref(), &[stake_bump]]],
        )?;

        // Initialize the stake token account
        invoke_signed(
            &token_instruction::initialize_account3(
                &spl_token_2022::id(),
                stake_account.key,
                mint_account.key,
                stake_account.key,
            )?,
            &[stake_account.clone(), mint_account.clone()],
            &[&[STAKE_PDA_SEED, signer_account.key.as_ref(), &[stake_bump]]],
        )?;
    }

    // Transfer tokens from user's account to stake account
    invoke(
        &token_instruction::transfer_checked(
            &spl_token_2022::id(),
            from_account.key,
            mint_account.key,
            stake_account.key,
            signer_account.key,
            &[],
            amount,
            DECIMALS,
        )?,
        &[
            from_account.clone(),
            mint_account.clone(),
            stake_account.clone(),
            signer_account.clone(),
        ],
    )?;

    Ok(())
}

fn process_unstake(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [stake_account, unstake_to_account, signer_account, mint_account, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token_2022::id() || *system_program.key != system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Verify stake PDA
    let (stake_pda_key, stake_bump) =
        Pubkey::find_program_address(&[STAKE_PDA_SEED, signer_account.key.as_ref()], program_id);

    if stake_pda_key != *stake_account.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // Transfer tokens from stake account to user's account
    invoke_signed(
        &token_instruction::transfer_checked(
            &spl_token_2022::id(),
            stake_account.key,
            mint_account.key,
            unstake_to_account.key,
            stake_account.key,
            &[],
            amount,
            DECIMALS,
        )?,
        &[
            stake_account.clone(),
            mint_account.clone(),
            unstake_to_account.clone(),
            stake_account.clone(),
        ],
        &[&[STAKE_PDA_SEED, signer_account.key.as_ref(), &[stake_bump]]],
    )?;

    Ok(())
}
```

#### Anchor 

```rust
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
                    authority: ctx.accounts.signer.to_account_info(),
                },
                &[&[STAKE_PDA_SEED, &[ctx.bumps.stake_account]]],
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
```

## Stylus

Stylus tokens follow the ERC-20 standard: each token is a self-contained contract storing balances in mappings and implementing transfer logic directly. Token operations are direct method calls to the contract. The standard interface - transfer, approve, transferFrom - enables application composability, while contracts extend functionality through inheritance. OpenZeppelin's Stylus implementations provide components for minting caps, pausability, and access control. Before implementing custom token functionality, it is best practice to check if an existing [standard](https://docs.rs/openzeppelin-stylus/latest/openzeppelin_stylus/token/index.html) or their [extensions](https://docs.rs/openzeppelin-stylus/latest/openzeppelin_stylus/token/erc20/extensions/index.html) fits the use case.

```rust
use openzeppelin_stylus::token::erc20::{Erc20, Error as Erc20Error, IErc20};

sol! {
    #[derive(Debug)]
    error InsufficientStakedBalance(address account, uint256 staked_balance);
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    InsufficientStakedBalance(InsufficientStakedBalance),
}

#[storage]
#[entrypoint]
pub struct FungibleTokenContract {
    erc20: Erc20,
    staked_balance: StorageMap<Address, StorageU256>,
}

#[public]
#[implements(IErc20<Error = Erc20Error>)]
impl FungibleTokenContract {
    #[constructor]
    pub fn constructor(&mut self, mint_to: Address) -> Result<(), Erc20Error> {
        assert_ne!(mint_to, Address::ZERO, "mint_to cannot be a zero-address");

        self.erc20._mint(mint_to, U256::from(TOTAL_SUPPLY))?;

        Ok(())
    }

    pub fn stake(&mut self, amount: U256) -> Result<(), Erc20Error> {
        let msg_sender = self.vm().msg_sender();

        let staked_balance = self.staked_balance_of(msg_sender);

        // Overflow not possible:
        // `amount` + `staked_balance` <= `total_supply` < `U256::MAX`
        self.staked_balance
            .setter(msg_sender)
            .set(staked_balance + amount);

        // Returns `ERC20InsufficientBalance` if `from_balance` < `amount`
        self.erc20
            ._update(msg_sender, self.vm().contract_address(), amount)
    }

    pub fn unstake(&mut self, amount: U256) -> Result<(), ContractError> {
        let msg_sender = self.vm().msg_sender();

        let staked_balance = self.staked_balance_of(msg_sender);

        if staked_balance < amount {
            return Err(InsufficientStakedBalance {
                account: msg_sender,
                staked_balance,
            }
            .into());
        }

        // Overflow not possible:
        // `amount` <= `staked_balance`
        self.staked_balance
            .setter(msg_sender)
            .set(staked_balance - amount);

        self.erc20
            ._update(self.vm().contract_address(), msg_sender, amount)
            .expect("amount <= staked_balance");

        Ok(())
    }

    pub fn staked_balance_of(&self, account: Address) -> U256 {
        self.staked_balance.get(account)
    }

    pub fn decimals(&self) -> U8 {
        U8::from(DECIMALS)
    }
}

#[public]
impl IErc20 for FungibleTokenContract {
    type Error = Erc20Error;

    fn total_supply(&self) -> U256 {
        self.erc20.total_supply()
    }

    fn balance_of(&self, account: Address) -> U256 {
        self.erc20.balance_of(account)
    }

    fn transfer(&mut self, to: Address, value: U256) -> Result<bool, Self::Error> {
        self.erc20.transfer(to, value)
    }

    fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.erc20.allowance(owner, spender)
    }

    fn approve(&mut self, spender: Address, value: U256) -> Result<bool, Self::Error> {
        self.erc20.approve(spender, value)
    }

    fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        value: U256,
    ) -> Result<bool, Self::Error> {
        self.erc20.transfer_from(from, to, value)
    }
}
```

### Allowance system

The allowance mechanism is central to ERC-20: users approve contracts to spend tokens on their behalf, then contracts pull tokens using [`IERC20::transfer_from`](https://docs.rs/openzeppelin-stylus/latest/openzeppelin_stylus/token/erc20/trait.IErc20.html#tymethod.transfer_from). This pull-based model is fundamental to DeFi composability on EVM chains.

In order for Stylus contracts to receive ERC20 tokens from a user, the user must first grant them an allowance to transfer a pre-determined maximum amount of tokens.

```rust
sol! {
    #[derive(Debug)]
    error InsufficientStakedBalance(address account, uint256 staked_balance);
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    InsufficientStakedBalance(InsufficientStakedBalance),
}

#[storage]
#[entrypoint]
pub struct StakeErc20Contract {
    stake_token: StorageAddress,
    staked_balance: StorageMap<Address, StorageU256>,
}

impl StakeErc20Contract {
    fn stake_token(&self) -> Erc20Interface {
        Erc20Interface::new(self.stake_token.get())
    }
}

#[public]
impl StakeErc20Contract {
    #[constructor]
    pub fn constructor(&mut self, stake_token: Address) {
        self.stake_token.set(stake_token);
    }

    pub fn stake(&mut self, amount: U256) -> Result<(), Vec<u8>> {
        let msg_sender = self.vm().msg_sender();

        let staked_balance = self.staked_balance_of(msg_sender);

        // Overflow not possible:
        // `amount` + `staked_balance` <= `total_supply` < `U256::MAX`
        self.staked_balance
            .setter(msg_sender)
            .set(staked_balance + amount);

        // Reverts with `ERC20InsufficientBalance` if `from_balance` < `amount` or
        // `ERC20InsufficientAllowance` if `contract_allowance` < `amount`
        let contract_addr = self.vm().contract_address();
        self.stake_token()
            .transfer_from(self, msg_sender, contract_addr, amount)?;

        Ok(())
    }

    pub fn unstake(&mut self, amount: U256) -> Result<(), ContractError> {
        let msg_sender = self.vm().msg_sender();

        let staked_balance = self.staked_balance_of(msg_sender);

        if staked_balance < amount {
            return Err(InsufficientStakedBalance {
                account: msg_sender,
                staked_balance,
            }
            .into());
        }

        // Overflow not possible:
        // `amount` <= `staked_balance`
        self.staked_balance
            .setter(msg_sender)
            .set(staked_balance - amount);

        self.stake_token()
            .transfer(self, msg_sender, amount)
            .expect("amount <= staked_balance");

        Ok(())
    }

    pub fn staked_balance_of(&self, account: Address) -> U256 {
        self.staked_balance.get(account)
    }
}
```

## Next Steps

With fungible tokens covered, the next chapter explores [Non-Fungible Token Handling](./non-fungible-tokens.md) - migrating from Metaplex NFTs to ERC-721 patterns in Stylus.

