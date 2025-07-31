#![cfg_attr(not(any(test)), no_main)]
extern crate alloc;

use alloy_sol_types::sol;
use stylus_sdk::{alloy_primitives::*, prelude::*, storage::*};

pub const STARTING_LIVES: u8 = 10;

#[storage]
#[entrypoint]
pub struct Mappings {
    lives: StorageMap<Address, StorageU8>,
}

sol! {
    error PlayerNotFound(address from);
}

#[derive(SolidityError)]
pub enum ContractError {
    PlayerNotFound(PlayerNotFound),
}

#[public]
impl Mappings {
    pub fn create_player_account(&mut self) {
        self.lives
            .insert(self.vm().msg_sender(), U8::from(STARTING_LIVES));
    }

    pub fn get_lives(&self, player: Address) -> U8 {
        self.lives.get(player)
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

        vm.set_sender(DEADBEEF_ADDRESS);

        let mut c = Mappings::from(&vm);

        c.create_player_account();

        assert_eq!(c.get_lives(DEADBEEF_ADDRESS), U8::from(STARTING_LIVES));
    }
}
