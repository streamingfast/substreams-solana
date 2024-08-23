## Next

### Unreleased

* Added `id` helper on `pb::ConfirmedTransaction` that returns the transaction's hash as a
  base58 `String` value.

* Added `hash` helper on `pb::ConfirmedTransaction` that returns the transaction's hash as a
  byte array (`&[u8]`).

* Added `all_instructions` helper on `pb::ConfirmedTransaction` to more easily recursively walk through
  top level as well as inner instructions in one swift. See https://docs.rs/substreams-solana/latest/substreams_solana/pb/sf/solana/type/v1/struct.ConfirmedTransaction.html#method.all_instructions for details.

* Added `resolved_accounts` and `resolved_accounts_as_strings` for address lookup accounts
  * Both methods will take the `loaded_writable_addresses and `loaded_readonly_addresses` from the `TransactionStatusMeta` of a `ConfirmedTransaction` and resolved the accounts

## [v0.11.1](https://github.com/streamingfast/substreams-solana/releases/tag/0.11.0)
### Changes
* Bring back module substreams-solana to expose modules `core` and `macro`

## [v0.11.0](https://github.com/streamingfast/substreams-solana/releases/tag/0.11.0)
### Breaking changes
*  Move `substreams-solana` code to https://github.com/streamingfast/substreams-solana-program-instructions crate.

## [v0.10.2](https://github.com/streamingfast/substreams-solana/releases/tag/0.10.2)

* Update `Unexpected Errors` returned to use `anyhow` crate instead of custom `UnexpectedError`

## [v0.2.1](https://github.com/streamingfast/substreams-solana/releases/tag/0.2.1)

* Bump Firehose Solana Blocks with latest (v0.1.1) spkg from `firehose-solana`. This is NOT a breaking change. We have added the Address Lookup Tables to the proto definition of Message. You can read more about Address Lookup Tables [here](https://docs.solana.com/developing/lookup-tables).

## [v0.2.0](https://github.com/streamingfast/substreams-solana/releases/tag/0.2.0)

* Updated to `prost` 0.11.

## [v0.1.0](https://github.com/streamingfast/substreams-solana/releases/tag/0.1.0)

* StreamingFast Firehose Block generated Rust code is now included in this library directly.
