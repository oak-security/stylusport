# Testing and Debugging

## Unit Testing

Test harnesses for Solana programs, such as [LiteSVM](https://github.com/LiteSVM/litesvm) or [Mollusk](https://github.com/anza-xyz/mollusk), require loading the target program binary, as well any dependency program binaries, into a cutdown implementation of the Solana Virtual Machine (SVM).

In contrast, Stylus contracts can be tested by instantiating the contract with a mock [`Host`](https://docs.rs/stylus-sdk/0.9.2/stylus_sdk/prelude/trait.Host.html) trait implementation without needing to first build the WASM binary. 

### Setup

In order for the contract to be instantiated from a test `Host` implementation, the `stylus-test` feature must be enabled for the `stylus-sdk` dependency. This ensures that the `#[storage]` attribute macro, applied to the top-level contract struct, [generates](https://github.com/OffchainLabs/stylus-sdk-rs/blob/975c8349598d2bdb3a12d91455315bd12305d132/stylus-proc/src/macros/storage.rs#L202-L231) the `From<&HostImpl>` implementation. 

> Note: The coupling of the test setup implementation with the `#[storage]` macro allows a contract to be split into sub-components that can be independently instantiated and tested. This pattern is used extensively by OpenZeppelin, an example of which is the `Ownable` component [tests](https://github.com/OpenZeppelin/rust-contracts-stylus/blob/47ad80064cf37be08ae714257695ba281e5471ad/contracts/src/access/ownable.rs#L284-L432).

While the [`TestVM`](https://docs.rs/stylus-test/0.9.2/stylus_test/vm/struct.TestVM.html) provided in the `stylys_sdk::testing` module is sufficient for simple contracts, versions up to `0.9.0` do not support the use of interfaces to call external contracts.

The [motsu](https://docs.rs/motsu/latest/motsu/index.html) test harness library, developed by OpenZeppelin, allows for the use of interfaces and the testing of the interaction between multiple contracts, as well as improved test setup ergonomics. This is particularly useful if the contract under test uses ERC20 or ERC721 tokens.

```toml
[package]
# ...

[dependencies]
alloy-primitives = "=0.8.20"
alloy-sol-types = "=0.8.20"
stylus-sdk = "0.9.0"

[dev-dependencies]
alloy-primitives = { version = "=0.8.20" features = [ "tiny-keccak" ] }
# required for motsu
arbitrary = { version = "1.4.2", features = [ "derive" ] } 
motsu = "0.10.0"
```

> Note: Adding `motsu` to `dev-dependencies` implicitly enables the `stylus-test` feature via an [indirect dependency](https://github.com/OpenZeppelin/stylus-test-helpers/blob/4e6082ece6e0e1e45e2dac53c195ca878d0a1742/Cargo.toml#L31-L33).

### Example

The following test shows how `motsu` can be used to test contracts that accept ERC20 deposits, such as the [ERC20 Allowance example](./fungible-tokens.md#allowance-system).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use alloy_primitives::U256;
    use motsu::prelude::*;
    use openzeppelin_stylus::token::erc20::{
        ERC20InsufficientAllowance, Erc20, Error as Erc20Error, IErc20,
    };
    use stylus_sdk::call::MethodError;

    pub const TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;

    #[motsu::test]
    fn test_contract(
        stake_token: Contract<Erc20>,
        stake_contract: Contract<StakeErc20Contract>,
        alice: Address,
    ) {
        stake_token
            .sender(alice)
            ._mint(alice, U256::from(TOTAL_SUPPLY))
            .motsu_unwrap();

        stake_contract
            .sender(alice)
            .constructor(stake_token.address());

        // Verify initial state
        assert_eq!(
            stake_token.sender(alice).total_supply(),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            stake_token.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            stake_contract.sender(alice).staked_balance_of(alice),
            U256::ZERO
        );
        assert_eq!(
            stake_token
                .sender(alice)
                .balance_of(stake_contract.address()),
            U256::ZERO
        );

        // Calculate stake amount (1/2 of total supply)
        let stake_amount = U256::from(TOTAL_SUPPLY / 2);
        let remaining_balance = U256::from(TOTAL_SUPPLY / 2);

        // Give stake contract allowance to transfer 1/2 of the total supply
        stake_token
            .sender(alice)
            .approve(stake_contract.address(), stake_amount)
            .motsu_unwrap();

        // Stake 1/2 of the total supply
        stake_contract
            .sender(alice)
            .stake(stake_amount)
            .motsu_unwrap();

        // Verify balances after staking
        assert_eq!(
            stake_token.sender(alice).balance_of(alice),
            remaining_balance
        );
        assert_eq!(
            stake_contract.sender(alice).staked_balance_of(alice),
            stake_amount
        );
        assert_eq!(
            stake_token
                .sender(alice)
                .balance_of(stake_contract.address()),
            stake_amount
        );

        // Attempt to stake more than available balance - should fail
        let err = stake_contract
            .sender(alice)
            .stake(stake_amount)
            .motsu_unwrap_err();
        assert_eq!(
            err,
            Erc20Error::InsufficientAllowance(ERC20InsufficientAllowance {
                spender: stake_contract.address(),
                allowance: U256::ZERO,
                needed: stake_amount
            })
            .encode()
        );

        // Unstake the full staked amount
        stake_contract
            .sender(alice)
            .unstake(stake_amount)
            .motsu_unwrap();

        // Verify balances after unstaking
        assert_eq!(
            stake_token.sender(alice).balance_of(alice),
            U256::from(TOTAL_SUPPLY)
        );
        assert_eq!(
            stake_contract.sender(alice).staked_balance_of(alice),
            U256::ZERO
        );
        assert_eq!(
            stake_token
                .sender(alice)
                .balance_of(stake_contract.address()),
            U256::ZERO
        );

        // Attempt to unstake when no tokens are staked - should fail
        let err = stake_contract
            .sender(alice)
            .unstake(stake_amount)
            .motsu_unwrap_err();
        assert!(matches!(err, ContractError::InsufficientStakedBalance(_)));
    }
}
```

## Debugging Techniques

### Using the dbg! macro

As Stylus contracts are unit tested in the same fashion as conventional Rust code, i.e. not within a specialized VM like Solana programs, the standard library's [`dbg!` macro](https://doc.rust-lang.org/stable/std/macro.dbg.html) can be inserted into the code under test to aid in debugging. 

In Rust development, it is best practice to remove `dbg!` macro usage before committing code in version control.

### Using the console! macro

Similar to the `msg!` logging macro in Solana programs, the [`console!` macro](https://docs.rs/stylus-sdk/latest/stylus_sdk/macro.console.html) can be used to add log messages within function execution. Messages emitted with `console` will be readable in the testing node logs during integration testing.

The `console!` macro implememtation is [elided](https://github.com/OffchainLabs/stylus-sdk-rs/blob/975c8349598d2bdb3a12d91455315bd12305d132/stylus-sdk/src/debug.rs#L22-L36) unless the `debug` feature is enabled in `stylus-sdk`. This means it is safe to commit code containing `console!` usage.

The following is an example of how to conditionally enable the `debug` feature:

```toml
[package]
# ...

[features]
debug = ["stylus-sdk/debug"]

[dependencies]
# ... 
stylus-sdk = "0.9.0"
```

To build a WASM artifact with `console!` logging enabled for integration testing, the following command structure can be used:

```bash
cargo build --features debug --release
```

> Note: Additional WASM artifact size optimization may be required. Refer to the [official Stylus documentation](https://docs.arbitrum.io/stylus/how-tos/optimizing-binaries).
