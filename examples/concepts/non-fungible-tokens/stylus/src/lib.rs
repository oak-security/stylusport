#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
extern crate alloc;

use alloc::{string::String, vec::Vec};
use openzeppelin_stylus::{
    token::erc721::{
        self,
        extensions::{Erc721Metadata, IErc721Metadata},
        Erc721, IErc721,
    },
    utils::introspection::erc165::IErc165,
};
use stylus_sdk::{
    abi::Bytes,
    alloy_primitives::{aliases::B32, Address, U256},
    alloy_sol_types::sol,
    prelude::*,
    storage::*,
};

pub const MAX_NAME_LENGTH: usize = 10;

sol! {
    #[derive(Debug)]
    error InvalidNameLength();

    #[derive(Debug)]
    error InvalidNameCharacters();

    #[derive(Debug)]
    error NameAlreadyMinted();
}

#[derive(SolidityError, Debug)]
pub enum ContractError {
    InvalidNameLength(InvalidNameLength),
    InvalidNameCharacters(InvalidNameCharacters),
    NameAlreadyMinted(NameAlreadyMinted),
    Erc721(erc721::Error),
}

#[storage]
#[entrypoint]
pub struct NameCollectionContract {
    erc721: Erc721,
    metadata: Erc721Metadata,
    // Map names to token ID
    minted_names: StorageMap<String, StorageU256>,
    // Map token ID to name
    token_names: StorageMap<U256, StorageString>,
    // track supply
    next_token_id: StorageU256,
}

#[public]
#[implements(IErc721<Error = erc721::Error>, IErc721Metadata<Error = erc721::Error>, IErc165)]
impl NameCollectionContract {
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), ContractError> {
        // Initialize the collection metadata
        self.metadata
            .constructor("Mock Name Service".into(), "MNS".into());
        self.next_token_id.set(U256::ONE);
        Ok(())
    }

    pub fn mint_name_nft(&mut self, to: Address, name: String) -> Result<U256, ContractError> {
        // Validate name length
        if name.is_empty() || name.len() > MAX_NAME_LENGTH {
            return Err(ContractError::InvalidNameLength(InvalidNameLength {}));
        }

        // Validate name characters (alphanumeric and underscore only)
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ContractError::InvalidNameCharacters(
                InvalidNameCharacters {},
            ));
        }

        // Check if name is already minted
        if self.is_name_minted(name.clone()) {
            return Err(NameAlreadyMinted {}.into());
        }

        // Get next token ID
        let token_id = self.next_token_id.get();

        // Mint the NFT
        self.erc721._mint(to, token_id)?;

        // Set the bi-directional name mapping
        self.minted_names.setter(name.clone()).set(token_id);
        self.token_names.setter(token_id).set_str(&name);

        // Increment token ID for next mint
        self.next_token_id.set(token_id + U256::from(1));

        Ok(token_id)
    }

    pub fn get_token_id_by_name(&self, name: String) -> U256 {
        self.minted_names.get(name)
    }

    pub fn get_name_by_token_id(&self, token_id: U256) -> String {
        self.token_names.getter(token_id).get_string()
    }

    pub fn is_name_minted(&self, name: String) -> bool {
        self.minted_names.get(name) > U256::ZERO
    }

    pub fn total_minted(&self) -> U256 {
        self.next_token_id.get() - U256::ONE
    }
}

#[public]
impl IErc721 for NameCollectionContract {
    type Error = erc721::Error;

    fn balance_of(&self, owner: Address) -> Result<U256, Self::Error> {
        self.erc721.balance_of(owner)
    }

    fn owner_of(&self, token_id: U256) -> Result<Address, Self::Error> {
        self.erc721.owner_of(token_id)
    }

    fn safe_transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), Self::Error> {
        self.erc721.safe_transfer_from(from, to, token_id)
    }

    fn safe_transfer_from_with_data(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
        data: Bytes,
    ) -> Result<(), Self::Error> {
        self.erc721
            .safe_transfer_from_with_data(from, to, token_id, data)
    }

    fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), Self::Error> {
        self.erc721.transfer_from(from, to, token_id)
    }

    fn approve(&mut self, to: Address, token_id: U256) -> Result<(), Self::Error> {
        self.erc721.approve(to, token_id)
    }

    fn set_approval_for_all(&mut self, to: Address, approved: bool) -> Result<(), Self::Error> {
        self.erc721.set_approval_for_all(to, approved)
    }

    fn get_approved(&self, token_id: U256) -> Result<Address, Self::Error> {
        self.erc721.get_approved(token_id)
    }

    fn is_approved_for_all(&self, owner: Address, operator: Address) -> bool {
        self.erc721.is_approved_for_all(owner, operator)
    }
}

#[public]
impl IErc721Metadata for NameCollectionContract {
    type Error = erc721::Error;

    fn name(&self) -> String {
        self.metadata.name()
    }

    fn symbol(&self) -> String {
        self.metadata.symbol()
    }

    /// unused
    fn token_uri(&self, _token_id: U256) -> Result<String, Self::Error> {
        Ok(String::new())
    }
}

#[public]
impl IErc165 for NameCollectionContract {
    fn supports_interface(&self, interface_id: B32) -> bool {
        self.erc721.supports_interface(interface_id)
            || <Self as IErc721Metadata>::interface_id() == interface_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use motsu::prelude::*;

    #[motsu::test]
    fn test_name_collection(
        contract: Contract<NameCollectionContract>,
        alice: Address,
        bob: Address,
    ) {
        // Initialize the contract
        contract.sender(alice).constructor().motsu_unwrap();

        // Verify collection metadata
        assert_eq!(contract.sender(alice).name(), "Mock Name Service");
        assert_eq!(contract.sender(alice).symbol(), "MNS");

        // Test minting a valid name
        let name1 = "alice".to_string();
        let token_id1 = contract
            .sender(alice)
            .mint_name_nft(alice, name1.clone())
            .motsu_unwrap();

        // Verify token was minted correctly
        assert_eq!(token_id1, U256::from(1));
        assert_eq!(
            contract.sender(alice).balance_of(alice).motsu_unwrap(),
            U256::from(1)
        );
        assert_eq!(
            contract.sender(alice).owner_of(token_id1).motsu_unwrap(),
            alice
        );
        assert_eq!(
            contract.sender(alice).get_name_by_token_id(token_id1),
            name1
        );
        assert!(contract.sender(alice).is_name_minted(name1.clone()));

        // Test minting another name to different address
        let name2 = "bob".to_string();
        let token_id2 = contract
            .sender(bob)
            .mint_name_nft(bob, name2.clone())
            .motsu_unwrap();

        assert_eq!(token_id2, U256::from(2));
        assert_eq!(
            contract.sender(bob).balance_of(bob).motsu_unwrap(),
            U256::from(1)
        );
        assert_eq!(contract.sender(bob).owner_of(token_id2).motsu_unwrap(), bob);

        // Test total minted count
        assert_eq!(contract.sender(bob).total_minted(), U256::from(2));

        // Test duplicate name should fail
        let err = contract
            .sender(alice)
            .mint_name_nft(alice, name1.clone())
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::NameAlreadyMinted(_)));

        // Test invalid name (empty) should fail
        let err = contract
            .sender(alice)
            .mint_name_nft(alice, "".to_string())
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidNameLength(_)));

        // Test invalid name (too long) should fail
        let long_name = "a".repeat(MAX_NAME_LENGTH + 1);
        let err = contract
            .sender(alice)
            .mint_name_nft(alice, long_name)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidNameLength(_)));

        // Test invalid characters should fail
        let err = contract
            .sender(alice)
            .mint_name_nft(alice, "alice@123".to_string())
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InvalidNameCharacters(_)));

        // Test transfer functionality
        contract
            .sender(alice)
            .transfer_from(alice, bob, token_id1)
            .motsu_unwrap();

        assert_eq!(
            contract.sender(alice).owner_of(token_id1).motsu_unwrap(),
            bob
        );
        assert_eq!(
            contract.sender(alice).balance_of(alice).motsu_unwrap(),
            U256::from(0)
        );
        assert_eq!(
            contract.sender(alice).balance_of(bob).motsu_unwrap(),
            U256::from(2)
        );

        // Test approve and transfer
        contract
            .sender(bob)
            .approve(alice, token_id2)
            .motsu_unwrap();

        assert_eq!(
            contract
                .sender(alice)
                .get_approved(token_id2)
                .motsu_unwrap(),
            alice
        );

        contract
            .sender(alice)
            .transfer_from(bob, alice, token_id2)
            .motsu_unwrap();

        assert_eq!(
            contract.sender(alice).owner_of(token_id2).motsu_unwrap(),
            alice
        );
    }
}
