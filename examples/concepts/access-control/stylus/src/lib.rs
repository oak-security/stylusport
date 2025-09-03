extern crate alloc;

use stylus_sdk::{
    alloy_primitives::{Address, U256, U64},
    alloy_sol_types::sol,
    prelude::*,
    storage::{StorageAddress, StorageU256, StorageU64},
};

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

#[cfg(test)]
mod test {
    use super::*;

    use stylus_sdk::{alloy_primitives::address, testing::*};

    static AUTHORITY_ADDRESS: Address = address!("0x1111111111111111111111111111111111111111");
    static FIRST_PUBLISHER_ADDRESS: Address =
        address!("0x2222222222222222222222222222222222222222");
    static SECOND_PUBLISHER_ADDRESS: Address =
        address!("0x3333333333333333333333333333333333333333");

    #[test]
    fn test_access_control_flow() {
        let vm = TestVM::default();
        let mut contract = AccessControl::from(&vm);

        contract.constructor(AUTHORITY_ADDRESS, FIRST_PUBLISHER_ADDRESS);
        assert_eq!(contract.get_authority(), AUTHORITY_ADDRESS);
        assert_eq!(contract.get_publisher(), FIRST_PUBLISHER_ADDRESS);

        vm.set_sender(FIRST_PUBLISHER_ADDRESS);
        vm.set_block_timestamp(1600000000);
        assert!(contract
            .publish_price(U256::from(1_000_000), U256::from(1_000_000))
            .is_ok());

        assert_eq!(
            contract.get_last_price(),
            (
                U256::from(1_000_000),
                U256::from(1_000_000),
                U64::from(1600000000)
            )
        );

        vm.set_sender(AUTHORITY_ADDRESS);
        assert!(contract.update_config(SECOND_PUBLISHER_ADDRESS).is_ok());
        assert_eq!(contract.get_publisher(), SECOND_PUBLISHER_ADDRESS);

        vm.set_sender(FIRST_PUBLISHER_ADDRESS);
        assert_eq!(
            contract.publish_price(U256::from(2_000_000), U256::from(2_000_000)),
            Err(AccessControlError::Unauthorized(Unauthorized {}))
        );

        // Test second publisher can now publish price successfully
        vm.set_sender(SECOND_PUBLISHER_ADDRESS);
        vm.set_block_timestamp(1700000000);
        assert!(contract
            .publish_price(U256::from(1_000_000), U256::from(2_000_000))
            .is_ok());
        assert_eq!(
            contract.get_last_price(),
            (
                U256::from(1_000_000),
                U256::from(2_000_000),
                U64::from(1700000000)
            )
        );
    }
}
