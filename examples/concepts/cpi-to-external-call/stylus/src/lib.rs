#![cfg_attr(not(any(test)), no_main)]
extern crate alloc;

use stylus_sdk::{abi, alloy_primitives::*, prelude::*, storage::*};

fn add_calldata(a: u64, b: u64) -> Vec<u8> {
    [
        [110u8, 44u8, 115u8, 45u8].as_slice(), // keccak(b"add(uint64,uint64)")[..4],
        abi::encode_params(&(a, b)).as_slice(),
    ]
    .concat()
}

// function add(uint64 a, uint64 b) external view returns (uint128);
// returns a big-endian u128 (16 bytes) padded to 32 bytes
fn parse_add_returndata(returndata: &[u8]) -> Option<u128> {
    if returndata.len() != 32 {
        return None;
    }

    returndata[16..].try_into().map(u128::from_be_bytes).ok()
}

#[storage]
#[entrypoint]
pub struct ExternalCaller {
    /// A negative value indicates no result has been obtained yet
    last_result: StorageI256,
    adder_address: StorageAddress,
}

#[public]
impl ExternalCaller {
    #[constructor]
    pub fn constructor(&mut self, adder_address: Address) {
        self.last_result.set(I256::MINUS_ONE);
        self.adder_address.set(adder_address);
    }

    pub fn add(&mut self, a: u64, b: u64) -> u128 {
        // low-level static call used to allow unit testing
        // sol_interface! generated interfaces can only be tested in a WASM runtime
        // see: https://github.com/OffchainLabs/stylus-sdk-rs/issues/301
        let returndata = self
            .vm()
            .static_call(
                &calls::context::Call::new(),
                self.get_adder_address(),
                &add_calldata(a, b),
            )
            .expect("valid contract call");

        let result = parse_add_returndata(&returndata).expect("valid return data");

        self.last_result.set(I256::unchecked_from(result));

        result
    }

    pub fn get_adder_address(&self) -> Address {
        self.adder_address.get()
    }

    pub fn get_last_result(&self) -> I256 {
        self.last_result.get()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use stylus_sdk::testing::*;

    #[test]
    fn test_contract() {
        let vm = TestVM::default();

        let adder_address = Address::from([0x05; 20]);

        vm.mock_static_call(adder_address, add_calldata(5, 10), Ok(abi::encode(&15u128)));

        let mut c = ExternalCaller::from(&vm);

        c.constructor(adder_address);

        assert_eq!(c.get_last_result(), I256::MINUS_ONE);
        assert_eq!(c.add(5, 10), 15);
        assert_eq!(c.get_last_result(), I256::unchecked_from(15));
    }
}
