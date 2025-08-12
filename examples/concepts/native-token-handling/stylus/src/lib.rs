#![cfg_attr(not(any(test)), no_main)]
extern crate alloc;

use alloy_sol_types::sol;
use stylus_sdk::{alloy_primitives::*, prelude::*, storage::*};

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

#[cfg(test)]
mod test {
    use super::*;
    use stylus_sdk::testing::*;

    #[test]
    fn test_contract() {
        let vm = TestVM::default();

        let mut c = NativeTokenHandling::from(&vm);

        let one_eth = U256::from(10).pow(U256::from(18));
        let half_eth = one_eth / U256::from(2);

        let user_addr = Address::new([1; 20]);

        vm.set_sender(user_addr);
        vm.set_value(half_eth);

        assert_eq!(c.deposit(), Ok(()));
        assert_eq!(c.balance(user_addr), half_eth);
        assert_eq!(c.deposit(), Ok(()));
        assert_eq!(c.balance(user_addr), one_eth);
        vm.set_balance(vm.contract_address(), one_eth);
        assert_eq!(c.withdraw_all(), Ok(()));
        assert_eq!(vm.balance(user_addr), one_eth);
    }
}
