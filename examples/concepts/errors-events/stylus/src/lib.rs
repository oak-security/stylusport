extern crate alloc;

use stylus_sdk::{
    alloy_primitives::U256, alloy_sol_types::sol, prelude::*, storage::StorageAddress,
};

sol! {
    error InvalidAmount(uint256 expected, uint256 received);
    error Unauthorized(address account);

    event OwnerChanged(address previous_owner, address current_owner);
}

#[storage]
#[entrypoint]
pub struct ErrorsEvents {
    owner: StorageAddress,
}

#[derive(SolidityError)]
pub enum ContractError {
    InvalidAmount(InvalidAmount),
    Unauthorized(Unauthorized),
}

#[public]
impl ErrorsEvents {
    pub fn invalid_amount(&mut self, expected: U256, received: U256) -> Result<(), ContractError> {
        Err(InvalidAmount { expected, received }.into())
    }

    pub fn unauthorized(&mut self) -> Result<(), ContractError> {
        Err(Unauthorized {
            account: self.vm().msg_sender(),
        }
        .into())
    }

    pub fn take_ownership(&mut self) {
        let msg_sender = self.vm().msg_sender();

        let previous_owner = self.owner.get();

        self.owner.set(msg_sender);

        log(
            self.vm(),
            OwnerChanged {
                previous_owner,
                current_owner: msg_sender,
            },
        );
    }
}
