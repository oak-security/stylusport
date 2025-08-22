#![cfg_attr(not(any(test)), no_main)]
extern crate alloc;

use stylus_sdk::{alloy_primitives::*, prelude::*, storage::*};

#[storage]
pub struct Transaction {
    amount: StorageU256,
    timestamp: StorageU256,
    completed: StorageBool,
}

#[storage]
#[entrypoint]
pub struct TokenContract {
    allowances: StorageMap<Address, StorageMap<Address, StorageU256>>,
    user_transactions: StorageMap<Address, StorageMap<U256, Transaction>>,
}

#[public]
impl TokenContract {
    pub fn approve(&mut self, spender: Address, amount: U256) {
        self.allowances
            .setter(self.vm().msg_sender())
            .insert(spender, amount);
    }

    pub fn record_transaction(&mut self, tx_id: U256, amount: U256) {
        let block_time = self.vm().block_timestamp();

        // a nested `setter` cannot be called in a single expression
        let mut txs = self.user_transactions.setter(self.vm().msg_sender());
        let mut tx = txs.setter(tx_id);

        tx.amount.set(amount);
        tx.timestamp.set(U256::from(block_time));
        tx.completed.set(true);
    }

    pub fn allowance(&self, owner: Address, spender: Address) -> U256 {
        self.allowances.getter(owner).get(spender)
    }

    pub fn transaction(&self, address: Address, tx_id: U256) -> (U256, U256, bool) {
        let txs = self.user_transactions.getter(address);
        let tx = txs.get(tx_id);

        (tx.amount.get(), tx.timestamp.get(), tx.completed.get())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use stylus_sdk::testing::*;

    static DEADBEEF_ADDRESS: Address = address!("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");

    #[test]
    fn test_contract() {
        let vm = TestVM::default();

        let mut c = TokenContract::from(&vm);

        c.approve(DEADBEEF_ADDRESS, U256::from(1_000_000u32));

        assert_eq!(
            c.allowance(vm.msg_sender(), DEADBEEF_ADDRESS),
            U256::from(1_000_000u128)
        );

        c.record_transaction(U256::ONE, U256::from(1_000_000u32));

        assert_eq!(
            c.transaction(vm.msg_sender(), U256::ONE),
            (
                U256::from(1_000_000u128),
                U256::from(vm.block_timestamp()),
                true
            )
        );
    }
}
