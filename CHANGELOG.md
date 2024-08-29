# Change log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

* Added `InstructionView.inner_instruction(at)` to retrieve an inner instruction at a specific index `at`. This works only if the view is currently on a `CompiledInstruction` (e.g. `InstructionView.is_root() == true`).

## 0.13.0

This is a relatively important update of the library bringing in improved version of the utilities the exist to iterate over instructions as well as access to pre-resolved Instruction's program id and accounts. The goal of this update was to make it easier to process instruction(s), reducing the overall code needed to link inner instructions and to avoid the need to implement account resolution.

Essentially, this update adds a new [InstructionView](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html) replacing the older one. This [InstructionView](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html) has [resolved progam id](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html#method.program_id), [resolved accounts](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html#method.accounts) and provides access to the transaction, compiled instruction and inner instructions (if viewing a `CompiledInstruction`).

You now get an [InstructionView](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html) when iterating over all `CompiledInstruction` (a.k.a root instruction) of a transaction via [ConfirmedTransaction::compiled_instructions](https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.compiled_instructions) (this was previously named `instructions()` directly).

And you can now also iterates over all instructions, root or inner(s), of a transaction via [ConfirmedTransaction::walk_instructions](https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.walk_instructions).

The same method are available on the [Block](https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.Block.html) directly to easily walk over all instructions found in a block.

In this update, we also added an [Address](https://docs.rs/substreams-solana/latest/substreams_solana/struct.Address.html) type that is a simply wrapper today around a `&Vec<u8>` with methods to more easily convert to `base58` string (`to_string`) as well as testing equality against various other types. For now `Address` is mainly used within the new [InstructionView](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html) struct, we plan on using it more where possible in the future.

In future updates, we plan to also attach the logs properly to `InstructionView`.

### Breaking changes

* Replaced `InstructionView` with an updated incompatible version, the 3 public fields that you had access to before has been removed in favor of methods instead, enabling future refactoring without breaking the API. Check next point for how to update.

* The methods `Block::instructions` and `ConfirmedTransaction::instructions` have been both renamed to `compiled_instructions`, they are still return an iterator to an [InstructionView](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html).

  ```rust
  for view in trx.instructions() {
    let meta = view.meta;
    let transaction = view.transaction;

    let program_id = resolve_account(view.instruction.program_id_index);
    let accounts = resolve_accounts(view.instruction.accounts);
    let data = view.instruction.data;
  }
  ```

  Becomes

  ```rust
  for view in trx.instructions() {
    let meta = view.meta();
    let transaction = view.transaction();

    // Program ID and accounts are already resolved now!
    let program_id = view.program_id();
    let accounts = view.accounts();
    let data = view.data();
  }
  ```

* The [all_instructions](https://docs.rs/substreams-solana/0.12.0/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.all_instructions) method added on `0.12.0` on both `Block` and `ConfirmedTransaction` has been renamed to `walk_instructions` to better convey the fact that we iterate over compiled (root) instruction and inner instructions.

  ```rust
  for view in trx.all_instructions() {
    // ...
  }
  ```

  Becomes

  ```rust
  for view in trx.walk_instructions() {
    // ...
  }
  ```

### Changes

* Added [account_at](https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.account_at) which returns the account at the given index.

* Added [ConfirmedTransaction::walk_instructions](https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.walk_instructions) to more easily walk over compiled and inner instructions.

* Renamed `instructions` to `compiled_instructions`.

* Improved [InstructionView](https://docs.rs/substreams-solana/latest/substreams_solana/block_view/struct.InstructionView.html) so that methods now returns resolved program ID and accounts and well as a bunch of newer helper(s).

## 0.12.0

* Added `id` helper on `pb::Transaction` and `pb::ConfirmedTransaction` that returns the transaction's hash as a
  base58 `String` value.

* Added `hash` helper on `pb::Transaction` and `pb::ConfirmedTransaction` that returns the transaction's hash as a
  byte array (`&[u8]`).

* Added `all_instructions` helper on `pb::ConfirmedTransaction` to more easily recursively walk through
  top level as well as inner instructions in one swift. See [all_instructions Docs](https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.all_instructions) for details.

* Added `resolved_accounts` for address lookup accounts, the method will take the `loaded_writable_addresses` and `loaded_readonly_addresses` from the `TransactionStatusMeta` of a `ConfirmedTransaction` and resolved the accounts.

## 0.11.1

### Changes
* Bring back module `substreams-solana` to expose modules `core` and `macro`

## 0.11.0

### Breaking changes
*  Move `substreams-solana` code to https://github.com/streamingfast/substreams-solana-program-instructions crate.

## 0.10.2

* Update `Unexpected Errors` returned to use `anyhow` crate instead of custom `UnexpectedError`

## 0.2.1

* Bump Firehose Solana Blocks with latest (v0.1.1) spkg from `firehose-solana`. This is NOT a breaking change. We have added the Address Lookup Tables to the proto definition of Message. You can read more about Address Lookup Tables [here](https://docs.solana.com/developing/lookup-tables).

## 0.2.0

* Updated to `prost` 0.11.

## 0.1.0

* StreamingFast Firehose Block generated Rust code is now included in this library directly.
