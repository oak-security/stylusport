# Native Token Handling

This chapter maps handling native SOL to ETH in Stylus: payable functions, internal balance accounting, safe withdrawals, and the key behavioral differences between Lamports and Wei.

## Solana

Solana's native token SOL is handled through the System Program. Programs transfer Lamports via CPIs, check balances through account fields, and receive SOL by accepting transfers to program-owned accounts. Each account maintains a `lamports` field that tracks its SOL balance, and rent requirements mean accounts must maintain minimum balances. Programs use PDAs to escrow SOL and manage program-owned funds separately from user accounts.

#### Native

```rust
#[derive(BorshSerialize, BorshDeserialize)]
pub struct WithdrawAllLamports {}

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
```

#### Anchor

```rust
#[program]
pub mod native_token_handling {
    use super::*;

    pub fn withdraw_all_lamports(ctx: Context<WithdrawAllLamports>) -> Result<()> {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.deposit_account.to_account_info(),
                    to: ctx.accounts.payer.to_account_info(),
                },
            )
            .with_signer(&[&[
                DEPOSIT_PDA_ACCOUNT_SEED,
                SEED_SEPARATOR,
                ctx.accounts.payer.key.as_ref(),
                &[ctx.bumps.deposit_account],
            ]]),
            ctx.accounts.deposit_account.lamports(),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction()]
pub struct WithdrawAllLamports<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [DEPOSIT_PDA_ACCOUNT_SEED, SEED_SEPARATOR, payer.key().as_ref()],
        bump,
    )]
    pub deposit_account: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}
```

## Stylus

Stylus contracts receive ETH by marking functions as `#[payable]`, check received amounts via `MessageAccess::msg_value`, and transfer ETH using `ValueTransfer::transfer_eth`. Unlike Solana's account model, contracts maintain internal mappings for user balances and implement withdrawal patterns.

```rust
#[storage]
#[entrypoint]
pub struct NativeTokenHandling {
    deposits: StorageMap<Address, StorageU256>,
}

sol! {
    #[derive(Debug, PartialEq)]
    error ZeroDeposit();
    #[derive(Debug, PartialEq)]
    error BalanceOverflow(address address, uint existing_balance, uint deposit);
    #[derive(Debug, PartialEq)]
    error DepositNotFound(address address);
    #[derive(Debug, PartialEq)]
    error TransferFailed(address to, uint amount, bytes error);
}

#[derive(SolidityError, Debug, PartialEq)]
pub enum ContractError {
    ZeroDeposit(ZeroDeposit),
    BalanceOverflow(BalanceOverflow),
    DepositNotFound(DepositNotFound),
    TransferFailed(TransferFailed),
}

#[public]
impl NativeTokenHandling {
    #[payable]
    pub fn deposit(&mut self) -> Result<(), ContractError> {
        let sender = self.vm().msg_sender();

        let amount = self.vm().msg_value();

        if amount.is_zero() {
            return Err(ZeroDeposit {}.into());
        }

        let existing_balance = self.balance(sender);

        let new_balance = existing_balance
            .checked_add(amount)
            .ok_or(BalanceOverflow {
                address: sender,
                existing_balance,
                deposit: amount,
            })?;

        self.deposits.insert(sender, new_balance);

        Ok(())
    }

    pub fn withdraw_all(&mut self) -> Result<(), ContractError> {
        let sender = self.vm().msg_sender();

        let balance = self.deposits.take(sender);

        if balance.is_zero() {
            return Err(DepositNotFound { address: sender }.into());
        }

        self.vm()
            .transfer_eth(sender, balance)
            .map_err(Bytes::from)
            .map_err(|error| TransferFailed {
                to: sender,
                amount: balance,
                error,
            })?;

        Ok(())
    }

    pub fn balance(&self, address: Address) -> U256 {
        self.deposits.get(address)
    }
}
```

## Next Steps

With native token handling covered, the next chapter explores [Fungible Token Handling](./fungible-tokens.md) - migrating SPL Token operations to ERC-20 patterns in Stylus contracts.
