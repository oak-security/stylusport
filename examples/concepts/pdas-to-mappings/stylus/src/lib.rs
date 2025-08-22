#![cfg_attr(not(any(test)), no_main)]
extern crate alloc;

use alloy_sol_types::sol;
use stylus_sdk::{alloy_primitives::*, prelude::*, storage::*};

pub const STARTING_LIVES: u8 = 10;

#[storage]
#[entrypoint]
pub struct Mappings {
    player_lives: StorageMap<Address, StorageU8>,
    player_is_dead: StorageMap<Address, StorageBool>,
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    error PlayerAlreadyExists(address player);

    #[derive(Debug, PartialEq, Eq)]
    error PlayerNotFound(address player);
}

#[derive(SolidityError, Debug, PartialEq, Eq)]
pub enum ContractError {
    PlayerAlreadyExists(PlayerAlreadyExists),
    PlayerNotFound(PlayerNotFound),
}

impl Mappings {
    fn player_exists(&self, player: Address) -> bool {
        self.player_lives.get(player) > U8::ZERO || self.player_is_dead.get(player)
    }
}

#[public]
impl Mappings {
    pub fn create_player_account(&mut self) -> Result<(), ContractError> {
        let msg_sender = self.vm().msg_sender();

        if self.player_exists(msg_sender) {
            return Err(PlayerAlreadyExists { player: msg_sender }.into());
        }

        self.player_lives
            .insert(self.vm().msg_sender(), U8::from(STARTING_LIVES));

        Ok(())
    }

    pub fn get_is_dead(&self, player: Address) -> Result<bool, ContractError> {
        if !self.player_exists(player) {
            return Err(PlayerNotFound { player }.into());
        }

        Ok(self.player_is_dead.get(player))
    }

    pub fn get_lives(&self, player: Address) -> Result<U8, ContractError> {
        if !self.player_exists(player) {
            return Err(PlayerNotFound { player }.into());
        }

        Ok(self.player_lives.get(player))
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

        assert_eq!(
            c.get_is_dead(DEADBEEF_ADDRESS),
            Err(ContractError::PlayerNotFound(PlayerNotFound {
                player: DEADBEEF_ADDRESS
            }))
        );
        assert_eq!(
            c.get_lives(DEADBEEF_ADDRESS),
            Err(ContractError::PlayerNotFound(PlayerNotFound {
                player: DEADBEEF_ADDRESS
            }))
        );
        assert!(c.create_player_account().is_ok());
        assert_eq!(
            c.create_player_account(),
            Err(ContractError::PlayerAlreadyExists(PlayerAlreadyExists {
                player: DEADBEEF_ADDRESS
            }))
        );
        assert_eq!(c.get_is_dead(DEADBEEF_ADDRESS), Ok(false),);
        assert_eq!(c.get_lives(DEADBEEF_ADDRESS), Ok(U8::from(STARTING_LIVES)));
    }
}
