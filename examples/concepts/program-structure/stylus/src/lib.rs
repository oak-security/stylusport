extern crate alloc;

use stylus_sdk::{
    alloy_primitives::{Address, U256},
    alloy_sol_types::sol,
    prelude::*,
    storage::{StorageAddress, StorageU256},
};

sol! {
    event ContractInitialized(uint256 initial_value, address authority);
    event ValueIncremented(uint256 new_value);
    event ValueUpdated(uint256 new_value);

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
        let authority = self.vm().tx_origin();

        self.value.set(initial_value);
        self.authority.set(authority);

        log(
            self.vm(),
            ContractInitialized {
                initial_value,
                authority,
            },
        );
    }

    pub fn increment(&mut self) -> U256 {
        let new_value = self.value.get() + U256::ONE;

        self.value.set(new_value);

        log(self.vm(), ValueIncremented { new_value });

        new_value
    }

    pub fn set_value(&mut self, new_value: U256) -> Result<(), CounterError> {
        let caller = self.vm().msg_sender();

        // Only authority can set value
        if caller != self.authority.get() {
            return Err(CounterError::Unauthorized(Unauthorized { caller }));
        }

        self.value.set(new_value);

        log(self.vm(), ValueUpdated { new_value });

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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_sol_types::SolEvent;
    use stylus_sdk::testing::*;

    #[test]
    fn test_contract() {
        let vm = TestVM::new();
        let mut c = Counter::from(&vm);

        let initial_value = U256::from(42);
        c.constructor(initial_value);
        assert_eq!(c.get_value(), initial_value);
        assert_eq!(c.get_authority(), vm.msg_sender());
        assert_eq!(
            vm.get_emitted_logs().last(),
            Some(&(
                vec![ContractInitialized::SIGNATURE_HASH],
                ContractInitialized {
                    initial_value,
                    authority: vm.msg_sender()
                }
                .encode_data()
            ))
        );

        assert_eq!(c.increment(), U256::from(43));
        assert_eq!(c.get_value(), U256::from(43));
        assert_eq!(
            vm.get_emitted_logs().last(),
            Some(&(
                vec![ValueIncremented::SIGNATURE_HASH],
                ValueIncremented {
                    new_value: U256::from(43)
                }
                .encode_data()
            ))
        );

        let new_value = U256::from(100);
        assert!(c.set_value(new_value).is_ok());
        assert_eq!(c.get_value(), new_value);
        assert_eq!(
            vm.get_emitted_logs().last(),
            Some(&(
                vec![ValueUpdated::SIGNATURE_HASH],
                ValueUpdated {
                    new_value: U256::from(100)
                }
                .encode_data()
            ))
        );

        vm.set_sender(Address::ZERO);
        assert_eq!(
            c.set_value(U256::ZERO),
            Err(CounterError::Unauthorized(Unauthorized {
                caller: Address::ZERO
            }))
        );
    }
}
