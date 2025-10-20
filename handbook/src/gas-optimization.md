# Gas optimization

This chapter covers strategies to reduce gas consumption with specific focus on how gas usage differs from Solana's compute unit model.

## Compute Units vs Gas & Ink

The fundamental difference between Solana and Ethereum/Stylus fee models:

| Aspect | Solana | Stylus/Ethereum |
|--------|---------|-----------------|
| **Unit** | Compute Units (CU) | Gas (and ink in Stylus VM) |
| **Pricing** | Fixed: 5,000 Lamports (0.000005 SOL) per signature | Variable: Gas price fluctuates with network demand |
| **Limits** | Per-transaction: 1.4M CU max | Per-block gas limit: ~30M gas |
| **Measurement** | Instruction-based (each instruction deducts from CU budget) | Operation-based (WASM opcodes measured in ink) |
| **State Access** | Rent-exempt deposits (one-time, refundable) | Per-operation gas cost (with SDK caching optimization) |
| **Optimization Focus** | Reduce CU usage and account size | Reduce storage operations; leverage compute efficiency |

### Stylus-Specific Concepts

Stylus introduces **ink** as a sub-gas unit for measuring WASM execution:
- **1 gas = 10,000 ink** (configurable exchange rate)
- WASM opcodes are orders of magnitude faster than EVM opcodes, thus requiring fractional gas in the form of ink.

**Cost Advantages in Stylus vs EVM:**

- **Compute**: 10-100x cheaper than EVM due to WASM efficiency and compiled Rust/C/C++ code quality
- **Memory**: 100-500x cheaper with novel exponential pricing (vs. EVM's quadratic per-call model)
- **Storage**: SLOAD/SSTORE cost the same as EVM, but Stylus SDK implements optimal caching to minimize operations

**Stylus Storage Cache:**

The Stylus VM implements an [storage cache](https://github.com/OffchainLabs/nitro/blob/master/arbitrator/arbutil/src/evm/storage.rs#L35-L61) that dramatically reduces the cost of repeated storage access:

- **Storage reads**: First 32 reads are free (0 gas), reads 33-128 cost 2 gas each, subsequent reads cost 10 gas each
- **Storage writes**: First 8 writes are free (0 gas), writes 9-64 cost 7 gas each, subsequent writes cost 10 gas each
- **Cache mechanics**: The [`StorageCache`](https://github.com/OffchainLabs/nitro/blob/92445764cb0df9f22cf6861d7f7260903b72302b/arbitrator/arbutil/src/evm/req.rs#L103-L152) is used to track the value of accessed slots, with dirty writes batched and flushed to the host EVM
- **Per-transaction scope**: Cache persists for the duration of a single transaction/call, resetting between calls

This caching strategy means repeatedly accessing the same storage slots within a transaction is nearly free after the initial access, unlike standard EVM where each warm SLOAD costs 100 gas.

**Cost Comparison:**

**Solana:**
- Base transaction fee: 5,000 Lamports (0.000005 SOL) per signature
- Simple transfer: ~300 CU (when optimized with `SetComputeUnitLimit`)
- System program CPI: ~2,215 CU
- Token transfer (direct): ~3,000 CU
- Token transfer via CPI: ~4,100 CU (adds ~1,000 CU overhead)
- Account creation requires rent-exempt deposit based on data size:
  - Empty account: ~890,880 Lamports (~0.00089 SOL)
  - 32-byte account: ~1,113,600 Lamports (~0.0011 SOL)
  - Deposits are fully refundable when accounts are closed

> Note: Unlike EVM/Stylus the amount of compute units used does not affect the overall transaction fee but it does affect the block inclusion latency. The lower the compute unit usage, the higher the reward ratio for validators to include the transaction in a block based on the fixed base fee plus any proposed priority fee.

**Stylus:**
- Simple I32Add: 70 ink = 0.007 gas
- Simple I64Add: 100 ink = 0.01 gas
- Keccak hash: 121,800 + 21,000w ink (w = EVM words)
- Storage operations with SDK caching:
  - First 32 reads: 0 gas (cached)
  - First 8 writes: 0 gas (cached)
  - Subsequent cached reads: 2-10 gas
  - Subsequent cached writes: 7-10 gas
  - Cold SLOAD (first access, not in cache): ~2,100 gas (EVM standard)
  - Cold SSTORE (first write): ~20,000 gas new slot, ~5,000 gas update (EVM standard)
- Host I/O call overhead: ~0.84 gas per host function invocation
- External contract call: 128-2,048 gas base overhead
- WASM contract entry: 128-2,048 gas per Stylus contract call

> Note: due to the gas overhead of entering the WASM VM when calling a Stylus contract, it may be cheaper gas-wise to use Solidity for trivial contracts.

## Optimization Techniques

With industry-leading compiler technology compiling contracts to WASM, assembly-level gas optimizations common in Solidity/EVM development are not required.

As a general principle, favor code readability and simplicity over premature optimization. However, **field ordering matters** when working with storage.

### Storage Slot Packing

The Stylus SDK's `#[storage]` macro automatically packs storage fields efficiently, but it can only pack **adjacent fields**. The macro processes fields sequentially and cannot reorder them, so the order you declare fields determines the storage layout.

```rust
#[storage]
pub struct EfficientStorage {
    flag1: StorageBool,  // Slot 0, byte 0
    flag2: StorageBool,  // Slot 0, byte 1
    x: StorageU256,      // Slot 1, bytes 0-31
}

#[storage]
pub struct InefficientStorage {
    flag1: StorageBool,  // Slot 0, byte 0
    x: StorageU256,      // Slot 1, bytes 0-31
    flag2: StorageBool,  // Slot 2, byte 0 (wasted slot!)
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
```

**Key insight:** Group fields by size to fill 32-byte slots completely. When `flag1` and `flag2` are adjacent, they share a slot. When separated by `StorageU256`, each requires its own slot, wasting 31 bytes per slot.

### Understanding the Macro Expansion

The `#[storage]` macro uses a greedy packing algorithm that tracks two variables:
- `space`: Remaining bytes in the current slot (starts at 32)
- `slot`: Current slot index (starts at 0)

For each field, the macro:
1. Checks if the field fits in remaining space
2. If not, moves to the next slot
3. Allocates the field and updates tracking variables

**Efficient Layout Expansion:**

```rust
impl stylus_sdk::storage::StorageType for EfficientStorage {
    unsafe fn new(
        mut root: stylus_sdk::alloy_primitives::U256,
        offset: u8,
        host: stylus_sdk::host::VM,
    ) -> Self {
        let mut space: usize = 32;  // Available bytes in current slot
        let mut slot: usize = 0;    // Current slot index
        
        let accessor = Self {
            __stylus_host: host.clone(),
            
            flag1: {
                let bytes = <StorageBool as storage::StorageType>::SLOT_BYTES;      // = 1 byte
                let words = <StorageBool as storage::StorageType>::REQUIRED_SLOTS;  // = 0 (number of full slots required)
                
                if space < bytes {  // 32 < 1? → false, fits in current slot
                    space = 32;
                    slot += 1;
                }
                space -= bytes;  // 32 - 1 = 31 bytes remaining
                
                let root = root + alloy_primitives::U256::from(slot);  // slot = 0
                let field = <StorageBool as storage::StorageType>::new(
                    root,
                    space as u8,  // offset = 31
                    host.clone(),
                );
                
                if words > 0 {  // 0 > 0? → false, no full slots consumed
                    slot += words;
                    space = 32;
                }
                field
            },
            
            flag2: {
                let bytes = <StorageBool as storage::StorageType>::SLOT_BYTES;      // = 1 byte
                let words = <StorageBool as storage::StorageType>::REQUIRED_SLOTS;  // = 0
                
                if space < bytes {  // 31 < 1? → false, still fits
                    space = 32;
                    slot += 1;
                }
                space -= bytes;  // 31 - 1 = 30 bytes remaining
                
                let root = root + alloy_primitives::U256::from(slot);  // slot = 0
                let field = <StorageBool as storage::StorageType>::new(
                    root,
                    space as u8,  // offset = 30
                    host.clone(),
                );
                
                if words > 0 {  // 0 > 0? → false
                    slot += words;
                    space = 32;
                }
                field
            },
            
            x: {
                let bytes = <StorageU256 as storage::StorageType>::SLOT_BYTES;      // = 32 bytes
                let words = <StorageU256 as storage::StorageType>::REQUIRED_SLOTS;  // = 0
                
                if space < bytes {  // 30 < 32? → true, needs new slot
                    space = 32;
                    slot += 1;  // slot = 1
                }
                space -= bytes;  // 32 - 32 = 0 bytes remaining
                
                let root = root + alloy_primitives::U256::from(slot);  // slot = 1
                let field = <StorageU256 as storage::StorageType>::new(
                    root,
                    space as u8,  // offset = 0
                    host.clone(),
                );
                
                if words > 0 {  // 0 > 0? → false
                    slot += words;
                    space = 32;
                }
                field
            },
        };
        accessor
    }
}
```

Fields `flag1` and `flag2` both live in slot 0 at different byte offsets. `x` uses slot 1. Total: **2 slots**.

**Inefficient Layout Expansion:**

```rust
impl stylus_sdk::storage::StorageType for InefficientStorage {
    unsafe fn new(
        mut root: stylus_sdk::alloy_primitives::U256,
        offset: u8,
        host: stylus_sdk::host::VM,
    ) -> Self {
        let mut space: usize = 32;  // Available bytes in current slot
        let mut slot: usize = 0;    // Current slot index

        let accessor = Self {
            __stylus_host: host.clone(),
            
            flag1: {
                let bytes = <StorageBool as storage::StorageType>::SLOT_BYTES;      // = 1 byte
                let words = <StorageBool as storage::StorageType>::REQUIRED_SLOTS;  // = 0
                
                if space < bytes {  // 32 < 1? → false, fits
                    space = 32;
                    slot += 1;
                }
                space -= bytes;  // 32 - 1 = 31 bytes remaining
                
                let root = root + alloy_primitives::U256::from(slot);  // slot = 0
                let field = <StorageBool as storage::StorageType>::new(
                    root,
                    space as u8,  // offset = 31
                    host.clone(),
                );
                
                if words > 0 {  // 0 > 0? → false
                    slot += words;
                    space = 32;
                }
                field
            },
            
            x: {
                let bytes = <StorageU256 as storage::StorageType>::SLOT_BYTES;      // = 32 bytes
                let words = <StorageU256 as storage::StorageType>::REQUIRED_SLOTS;  // = 0
                
                if space < bytes {  // 31 < 32? → true, needs new slot
                    space = 32;
                    slot += 1;  // slot = 1
                }
                space -= bytes;  // 32 - 32 = 0 bytes remaining
                
                let root = root + alloy_primitives::U256::from(slot);  // slot = 1
                let field = <StorageU256 as storage::StorageType>::new(
                    root,
                    space as u8,  // offset = 0
                    host.clone(),
                );
                
                if words > 0 {  // 0 > 0? → false
                    slot += words;
                    space = 32;
                }
                field
            },
            
            flag2: {
                let bytes = <StorageBool as storage::StorageType>::SLOT_BYTES;      // = 1 byte
                let words = <StorageBool as storage::StorageType>::REQUIRED_SLOTS;  // = 0
                
                if space < bytes {  // 0 < 1? → true, needs new slot
                    space = 32;
                    slot += 1;  // slot = 2
                }
                space -= bytes;  // 32 - 1 = 31 bytes remaining
                
                let root = root + alloy_primitives::U256::from(slot);  // slot = 2
                let field = <StorageBool as storage::StorageType>::new(
                    root,
                    space as u8,  // offset = 31
                    host.clone(),
                );
                
                if words > 0 {  // 0 > 0? → false
                    slot += words;
                    space = 32;
                }
                field
            },
        };
        accessor
    }
}
```

Field `flag1` uses slot 0, `x` uses slot 1, `flag2` uses slot 2. Total: **3 slots**, with 31 wasted bytes in slots 0 and 2.

> Best Practice: **Group fields of similar sizes together to maximize slot utilization.** The macro processes fields sequentially, so arrange them to minimize wasted space within each 32-byte slot. Small fields can appear before or after large fields, as long as they're grouped together to efficiently fill slots.
