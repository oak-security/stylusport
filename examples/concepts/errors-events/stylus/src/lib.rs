extern crate alloc;

use stylus_sdk::{
    alloy_primitives::U256, alloy_sol_types::sol, prelude::*, storage::StorageAddress,
};

sol! {
    error InvalidAmount(uint256 expected, uint256 received);
    error Unauthorized(address account);

    event ItChanged(address previous_it, address current_it);
}

#[storage]
#[entrypoint]
pub struct ErrorsEvents {
    it: StorageAddress,
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

    /// Tags the caller as "it", emitting an event for the state change
    pub fn tag(&mut self) {
        let msg_sender = self.vm().msg_sender();

        let previous_it = self.it.get();

        self.it.set(msg_sender);

        log(
            self.vm(),
            ItChanged {
                previous_it,
                current_it: msg_sender,
            },
        );
    }
}
