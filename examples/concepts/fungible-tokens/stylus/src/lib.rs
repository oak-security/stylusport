#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use openzeppelin_stylus::token::erc20::{Erc20, Error as Erc20Error, IErc20};
use stylus_sdk::{alloy_primitives::*, alloy_sol_types::sol, prelude::*, storage::*};

pub const DECIMALS: u8 = 6;
pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000; // 1B tokens

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
    pub fn constructor(&mut self) -> Result<(), Erc20Error> {
        self.erc20
            ._mint(self.vm().tx_origin(), U256::from(TOTAL_SUPPLY))?;

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

        // Reverts with `ERC20InsufficientBalance` if `from_balance` < `amount`
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

#[cfg(test)]
mod tests {
    use super::*;
    use motsu::prelude::*;
    use openzeppelin_stylus::token::erc20::Error as Erc20Error;

    #[motsu::test]
    fn test_contract(contract: Contract<FungibleTokenContract>, alice: Address) {
        // Initialize the contract - mints total supply to the deployer (alice)
        contract.sender(alice).constructor().motsu_unwrap();

        // Verify initial state
        assert_eq!(
            contract.sender(alice).total_supply(),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            contract.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(contract.sender(alice).staked_balance_of(alice), U256::ZERO);
        assert_eq!(
            contract.sender(alice).balance_of(contract.address()),
            U256::ZERO
        );

        // Calculate stake amount (3/4 of total supply)
        let stake_amount = U256::from((TOTAL_SUPPLY * 3) / 4);
        let remaining_balance = U256::from(TOTAL_SUPPLY / 4);

        // Stake 3/4 of the total supply
        contract.sender(alice).stake(stake_amount).motsu_unwrap();

        // Verify balances after staking
        assert_eq!(contract.sender(alice).balance_of(alice), remaining_balance);
        assert_eq!(
            contract.sender(alice).staked_balance_of(alice),
            stake_amount
        );
        assert_eq!(
            contract.sender(alice).balance_of(contract.address()),
            stake_amount
        );

        // Attempt to stake more than available balance - should fail
        let err = contract
            .sender(alice)
            .stake(stake_amount)
            .motsu_unwrap_err();
        assert!(matches!(err, Erc20Error::InsufficientBalance(_)));

        // Verify balances haven't changed after failed stake
        assert_eq!(contract.sender(alice).balance_of(alice), remaining_balance);
        assert_eq!(
            contract.sender(alice).staked_balance_of(alice),
            stake_amount
        );

        // Unstake the full staked amount
        contract.sender(alice).unstake(stake_amount).motsu_unwrap();

        // Verify balances after unstaking
        assert_eq!(
            contract.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(contract.sender(alice).staked_balance_of(alice), U256::ZERO);
        assert_eq!(
            contract.sender(alice).balance_of(contract.address()),
            U256::ZERO
        );

        // Attempt to unstake when no tokens are staked - should fail
        let err = contract
            .sender(alice)
            .unstake(stake_amount)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InsufficientStakedBalance(_)));

        // Final verification - balances should remain unchanged
        assert_eq!(
            contract.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(contract.sender(alice).staked_balance_of(alice), U256::ZERO);
    }
}
