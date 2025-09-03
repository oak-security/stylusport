#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use openzeppelin_stylus::token::erc20::interface::Erc20Interface;
use stylus_sdk::{alloy_primitives::*, alloy_sol_types::sol, prelude::*, storage::*};

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

#[cfg(test)]
mod tests {
    use super::*;

    use alloy_primitives::U256;
    use motsu::prelude::*;
    use openzeppelin_stylus::token::erc20::{
        ERC20InsufficientAllowance, Erc20, Error as Erc20Error, IErc20,
    };
    use stylus_sdk::call::MethodError;

    pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;

    #[motsu::test]
    fn test_contract(
        stake_token: Contract<Erc20>,
        stake_contract: Contract<StakeErc20Contract>,
        alice: Address,
    ) {
        stake_token
            .sender(alice)
            ._mint(alice, U256::from(TOTAL_SUPPLY))
            .motsu_unwrap();

        stake_contract
            .sender(alice)
            .constructor(stake_token.address());

        // Verify initial state
        assert_eq!(
            stake_token.sender(alice).total_supply(),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            stake_token.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            stake_contract.sender(alice).staked_balance_of(alice),
            U256::ZERO
        );
        assert_eq!(
            stake_token
                .sender(alice)
                .balance_of(stake_contract.address()),
            U256::ZERO
        );

        // Calculate stake amount (1/2 of total supply)
        let stake_amount = U256::from(TOTAL_SUPPLY / 2);
        let remaining_balance = U256::from(TOTAL_SUPPLY / 2);

        // Give stake contract allowance to transfer 1/2 of the total supply
        stake_token
            .sender(alice)
            .approve(stake_contract.address(), stake_amount)
            .motsu_unwrap();

        // Stake 1/2 of the total supply
        stake_contract
            .sender(alice)
            .stake(stake_amount)
            .motsu_unwrap();

        // Verify balances after staking
        assert_eq!(
            stake_token.sender(alice).balance_of(alice),
            remaining_balance
        );
        assert_eq!(
            stake_contract.sender(alice).staked_balance_of(alice),
            stake_amount
        );
        assert_eq!(
            stake_token
                .sender(alice)
                .balance_of(stake_contract.address()),
            stake_amount
        );

        // Attempt to stake more than available balance - should fail
        let err = stake_contract
            .sender(alice)
            .stake(stake_amount)
            .motsu_unwrap_err();
        assert_eq!(
            err,
            Erc20Error::InsufficientAllowance(ERC20InsufficientAllowance {
                spender: stake_contract.address(),
                allowance: U256::ZERO,
                needed: stake_amount
            })
            .encode()
        );

        // Unstake the full staked amount
        stake_contract
            .sender(alice)
            .unstake(stake_amount)
            .motsu_unwrap();

        // Verify balances after unstaking
        assert_eq!(
            stake_token.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            stake_contract.sender(alice).staked_balance_of(alice),
            U256::ZERO
        );
        assert_eq!(
            stake_token
                .sender(alice)
                .balance_of(stake_contract.address()),
            U256::ZERO
        );

        // Attempt to unstake when no tokens are staked - should fail
        let err = stake_contract
            .sender(alice)
            .unstake(stake_amount)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InsufficientStakedBalance(_)));
    }
}
