#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use openzeppelin_stylus::access::{
    ownable,
    ownable_two_step::{IOwnable2Step, Ownable2Step},
};
use stylus_sdk::{alloy_primitives::*, alloy_sol_types::sol, prelude::*, storage::*};

sol! {
    #[derive(Debug)]
    error ContractAlreadyPaused();
    #[derive(Debug)]
    error ContractAlreadyUnpaused();
}

#[derive(SolidityError, Debug)]
// In order to generate an ABI for the contract you need to manually wire
// up OpenZeppelin's error types defined with `sol!` rather than the their
// `ownable::Error` type which implements `SolidityError` but not `SolError`
pub enum ContractError {
    InvalidOwner(ownable::OwnableInvalidOwner),
    Unauthorized(ownable::OwnableUnauthorizedAccount),
    AlreadyPaused(ContractAlreadyPaused),
    AlreadyUnpaused(ContractAlreadyUnpaused),
}

impl From<ownable::Error> for ContractError {
    fn from(value: ownable::Error) -> Self {
        match value {
            ownable::Error::UnauthorizedAccount(e) => Self::Unauthorized(e),
            ownable::Error::InvalidOwner(e) => Self::InvalidOwner(e),
        }
    }
}

#[storage]
#[entrypoint]
pub struct OwnableContract {
    ownable: Ownable2Step,
    is_paused: StorageBool,
}

#[public]
#[implements(IOwnable2Step<Error = ownable::Error>)]
impl OwnableContract {
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), ContractError> {
        self.ownable.constructor(self.vm().msg_sender())?;

        self.is_paused.set(true);

        Ok(())
    }

    pub fn pause_contract(&mut self) -> Result<(), ContractError> {
        self.ownable.only_owner()?;

        if self.is_paused() {
            return Err(ContractAlreadyPaused {}.into());
        }

        self.is_paused.set(true);

        Ok(())
    }

    pub fn unpause_contract(&mut self) -> Result<(), ContractError> {
        self.ownable.only_owner()?;

        if !self.is_paused() {
            return Err(ContractAlreadyUnpaused {}.into());
        }

        self.is_paused.set(false);

        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.get()
    }
}

#[public]
impl IOwnable2Step for OwnableContract {
    type Error = ownable::Error;

    fn owner(&self) -> Address {
        self.ownable.owner()
    }

    fn pending_owner(&self) -> Address {
        self.ownable.pending_owner()
    }

    fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), Self::Error> {
        self.ownable.transfer_ownership(new_owner)
    }

    fn accept_ownership(&mut self) -> Result<(), Self::Error> {
        self.ownable.accept_ownership()
    }

    fn renounce_ownership(&mut self) -> Result<(), Self::Error> {
        self.ownable.renounce_ownership()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motsu::prelude::*;

    #[motsu::test]
    fn test_contract(
        contract: Contract<OwnableContract>,
        alice: Address,
        bob: Address,
        charlie: Address,
    ) {
        // Initialize the contract - alice becomes the owner
        contract.sender(alice).constructor().motsu_unwrap();

        // Verify initial state
        assert_eq!(contract.sender(alice).owner(), alice);
        assert_eq!(contract.sender(alice).pending_owner(), Address::ZERO);
        assert_eq!(contract.sender(alice).is_paused(), true);

        // Owner can unpause the contract
        contract.sender(alice).unpause_contract().motsu_unwrap();
        assert_eq!(contract.sender(alice).is_paused(), false);

        // Attempting to unpause when already unpaused should fail
        let err = contract.sender(alice).unpause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::AlreadyUnpaused(_)));

        // Owner can pause the contract
        contract.sender(alice).pause_contract().motsu_unwrap();
        assert_eq!(contract.sender(alice).is_paused(), true);

        // Attempting to pause when already paused should fail
        let err = contract.sender(alice).pause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::AlreadyPaused(_)));

        // Non-owner (bob) cannot pause the contract
        let err = contract.sender(bob).pause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        // Non-owner (bob) cannot unpause the contract
        let err = contract.sender(bob).unpause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        // Owner initiates ownership transfer to bob
        contract
            .sender(alice)
            .transfer_ownership(bob)
            .motsu_unwrap();
        assert_eq!(contract.sender(alice).owner(), alice); // Still alice until accepted
        assert_eq!(contract.sender(alice).pending_owner(), bob);

        // Charlie (non-pending owner) cannot accept ownership
        let err = contract
            .sender(charlie)
            .accept_ownership()
            .motsu_unwrap_err();
        assert!(matches!(err, ownable::Error::UnauthorizedAccount(_)));

        // Alice is still the owner and can perform owner actions
        contract.sender(alice).unpause_contract().motsu_unwrap();
        assert_eq!(contract.sender(alice).is_paused(), false);

        // Bob (pending owner) accepts ownership
        contract.sender(bob).accept_ownership().motsu_unwrap();
        assert_eq!(contract.sender(bob).owner(), bob);
        assert_eq!(contract.sender(bob).pending_owner(), Address::ZERO);

        // Alice is no longer the owner and cannot perform owner actions
        let err = contract.sender(alice).pause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        // Bob (new owner) can perform owner actions
        contract.sender(bob).pause_contract().motsu_unwrap();
        assert_eq!(contract.sender(bob).is_paused(), true);

        // Bob initiates transfer to charlie
        contract
            .sender(bob)
            .transfer_ownership(charlie)
            .motsu_unwrap();
        assert_eq!(contract.sender(bob).pending_owner(), charlie);

        // Bob can cancel the transfer by transferring to Address::ZERO
        contract
            .sender(bob)
            .transfer_ownership(Address::ZERO)
            .motsu_unwrap();
        assert_eq!(contract.sender(bob).pending_owner(), Address::ZERO);

        // Charlie cannot accept ownership anymore
        let err = contract
            .sender(charlie)
            .accept_ownership()
            .motsu_unwrap_err();
        assert!(matches!(err, ownable::Error::UnauthorizedAccount(_)));

        // Bob remains the owner
        assert_eq!(contract.sender(bob).owner(), bob);

        // Bob can renounce ownership
        contract.sender(bob).renounce_ownership().motsu_unwrap();
        assert_eq!(contract.sender(bob).owner(), Address::ZERO);

        // No one can perform owner actions after renouncement
        let err = contract.sender(bob).pause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        let err = contract.sender(alice).unpause_contract().motsu_unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized(_)));

        // Contract remains in its last state (paused)
        assert_eq!(contract.sender(alice).is_paused(), true);
    }
}
