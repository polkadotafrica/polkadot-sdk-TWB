# Pallet Auto Tasks

A Substrate pallet that demonstrates how to implement automated, state-driven tasks within a blockchain runtime. This pallet enables the execution of predefined logic without direct user interaction.

## Overview

The `pallet-auto-tasks` provides a framework for defining autonomous tasks that are triggered by blockchain state conditions rather than explicit user transactions. This opens up countless possibilities for blockchain automation including:

- Storage cleanup mechanisms
- Data aggregation services
- Automated workflow processing
- Scheduled operations

## Features

- Fully on-chain automated task execution
- State-driven task triggering
- Trustless execution via inherent transactions
- Configurable task conditions and weights
- Example implementations for common use cases

## Usage

### Basic Integration

Add this pallet to your runtime's `Cargo.toml`:

```toml
pallet-auto-tasks = { version = "0.1.0", default-features = false }
```

Configure it in your runtime:

```rust
parameter_types! {
    pub const MaxNumbersPerBlock: u32 = 50;
}

impl pallet_auto_tasks::Config for Runtime {
    type RuntimeTask = RuntimeTask;
    type WeightInfo = pallet_auto_tasks::weights::SubstrateWeight<Runtime>;
}
```

### Automated Tasks

The pallet demonstrates a simple automated task that:

1. Identifies numbers stored in a specific storage map
2. Adds them to a running total
3. Removes them from the original storage

Additionally, two real-world examples are provided:

1. Automated message cleanup - Removes expired messages based on block number
2. Data aggregation service - Calculates daily averages of transaction values

## Experimental Feature Flag

This pallet requires the `experimental` feature flag to be enabled to access the task-related macros:

```
cargo build --features experimental
```

## Examples

Check the `examples` directory for real-world examples of implementing automated tasks for:

- Storage cleanup
- Data aggregation
- Scheduled operations

## Testing

Run tests with:

```
cargo test
```

## Documentation

For more detailed documentation, check the inline code comments or build the documentation:

```
cargo doc --open
```

## License

This project is licensed under the Apache License, Version 2.0.

## Resources

- [Substrate Documentation](https://docs.substrate.io/)
- [Task Implementation PR: paritytech/polkadot-sdk#4545](https://github.com/paritytech/polkadot-sdk/pull/4545)
- [Task Implementation PR: paritytech/polkadot-sdk#5163](https://github.com/paritytech/polkadot-sdk/pull/5163)
- [Example Implementation: substrate/frame/examples/tasks](https://github.com/paritytech/polkadot-sdk/blob/master/substrate/frame/examples/tasks/src/lib.rs)