#![cfg_attr(not(any(test)), no_main)]
extern crate alloc;

use stylus_sdk::{alloy_primitives::*, prelude::*, storage::*};

#[storage]
#[entrypoint]
pub struct DataStorage {
    bool: StorageBool,
    uint8: StorageU8,
    uint16: StorageU16,
    uint32: StorageU32,
    uint64: StorageU64,
    uint128: StorageU128,
    uint256: StorageU256,
    int8: StorageI8,
    int16: StorageI16,
    int32: StorageI32,
    int64: StorageI64,
    int128: StorageI128,
    int256: StorageI256,
    string: StorageString,
    bytes: StorageBytes,
    fixed_bytes: StorageFixedBytes<4>,
    vec: StorageVec<StorageU64>,
    address: StorageAddress,
}

#[public]
impl DataStorage {
    #[constructor]
    // for example purposes only
    #[allow(clippy::too_many_arguments)]
    pub fn constructor(
        &mut self,
        bool: bool,
        uint8: U8,
        uint16: U16,
        uint32: U32,
        uint64: U64,
        uint128: U128,
        uint256: U256,
        int8: I8,
        int16: I16,
        int32: I32,
        int64: I64,
        int128: I128,
        int256: I256,
        string: String,
        bytes: Vec<u8>,
        fixed_bytes: FixedBytes<4>,
        vec: Vec<U64>,
        address: Address,
    ) {
        self.bool.set(bool);
        self.uint8.set(uint8);
        self.uint16.set(uint16);
        self.uint32.set(uint32);
        self.uint64.set(uint64);
        self.uint128.set(uint128);
        self.uint256.set(uint256);
        self.int8.set(int8);
        self.int16.set(int16);
        self.int32.set(int32);
        self.int64.set(int64);
        self.int128.set(int128);
        self.int256.set(int256);
        self.string.set_str(string);
        self.bytes.set_bytes(bytes);
        self.fixed_bytes.set(fixed_bytes);

        for x in vec {
            self.vec.push(x);
        }

        self.address.set(address);
    }

    fn get_bool(&self) -> bool {
        self.bool.get()
    }
    fn get_uint8(&self) -> U8 {
        self.uint8.get()
    }
    fn get_uint16(&self) -> U16 {
        self.uint16.get()
    }
    fn get_uint32(&self) -> U32 {
        self.uint32.get()
    }
    fn get_uint64(&self) -> U64 {
        self.uint64.get()
    }
    fn get_uint128(&self) -> U128 {
        self.uint128.get()
    }
    fn get_uint256(&self) -> U256 {
        self.uint256.get()
    }
    fn get_int8(&self) -> I8 {
        self.int8.get()
    }
    fn get_int16(&self) -> I16 {
        self.int16.get()
    }
    fn get_int32(&self) -> I32 {
        self.int32.get()
    }
    fn get_int64(&self) -> I64 {
        self.int64.get()
    }
    fn get_int128(&self) -> I128 {
        self.int128.get()
    }
    fn get_int256(&self) -> I256 {
        self.int256.get()
    }
    fn get_string(&self) -> String {
        self.string.get_string()
    }
    fn get_bytes(&self) -> Vec<u8> {
        self.bytes.get_bytes()
    }
    fn get_fixed_bytes(&self) -> FixedBytes<4> {
        self.fixed_bytes.get()
    }
    fn get_address(&self) -> Address {
        self.address.get()
    }

    fn get_vec_item(&self, idx: u32) -> (bool, U64) {
        self.vec.get(idx).map_or((false, U64::ZERO), |x| (true, x))
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
        let mut c = DataStorage::from(&vm);

        c.constructor(
            true,
            U8::MAX,
            U16::MAX,
            U32::MAX,
            U64::MAX,
            U128::MAX,
            U256::MAX,
            I8::MIN,
            I16::MIN,
            I32::MIN,
            I64::MIN,
            I128::MIN,
            I256::MIN,
            "StylusPort::Solana".to_owned(),
            b"StylusPort::Solana".to_vec(),
            fixed_bytes!("0xdeadbeef"),
            vec![U64::ONE, U64::ONE, U64::ONE, U64::ONE],
            DEADBEEF_ADDRESS,
        );

        assert!(c.get_bool());
        assert_eq!(c.get_uint8(), U8::MAX);
        assert_eq!(c.get_uint16(), U16::MAX);
        assert_eq!(c.get_uint32(), U32::MAX);
        assert_eq!(c.get_uint64(), U64::MAX);
        assert_eq!(c.get_uint128(), U128::MAX);
        assert_eq!(c.get_uint256(), U256::MAX);
        assert_eq!(c.get_int8(), I8::MIN);
        assert_eq!(c.get_int16(), I16::MIN);
        assert_eq!(c.get_int32(), I32::MIN);
        assert_eq!(c.get_int64(), I64::MIN);
        assert_eq!(c.get_int128(), I128::MIN);
        assert_eq!(c.get_int256(), I256::MIN);
        assert_eq!(c.get_string(), "StylusPort::Solana".to_owned());
        assert_eq!(c.get_bytes(), b"StylusPort::Solana".to_vec());
        assert_eq!(c.get_fixed_bytes(), fixed_bytes!("0xdeadbeef"));
        assert_eq!(c.get_vec_item(0), (true, U64::ONE));
        assert_eq!(c.get_vec_item(1), (true, U64::ONE));
        assert_eq!(c.get_vec_item(2), (true, U64::ONE));
        assert_eq!(c.get_vec_item(3), (true, U64::ONE));
        assert_eq!(c.get_vec_item(4), (false, U64::ZERO));
        assert_eq!(c.get_address(), DEADBEEF_ADDRESS,);
    }
}
