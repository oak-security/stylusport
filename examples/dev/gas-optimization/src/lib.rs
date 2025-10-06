extern crate alloc;

use stylus_sdk::{prelude::*, storage::*};

#[storage]
#[allow(dead_code)]
pub struct EfficientStorage {
    flag1: StorageBool,
    flag2: StorageBool,
    x: StorageU256,
}

#[storage]
#[allow(dead_code)]
pub struct InefficientStorage {
    flag1: StorageBool,
    x: StorageU256,
    flag2: StorageBool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_usage() {
        assert_eq!(EfficientStorage::required_slots(), 2);
        assert_eq!(InefficientStorage::required_slots(), 3);
    }
}
