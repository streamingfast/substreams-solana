# Substreams Solana

Substreams development kit for Solana chains, contains Rust Firehose Block model and helpers

## Usage

* `Cargo.toml`:

```toml
[package]
name = "my-package"
version = "0.1.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
substreams-solana = "0.16.0-beta.1"
```

### Protobuf Extern paths

If you have other protobuf objects that refer to the `sf.solana.type.v1` types, like the [Solana Substreams Foundational Modules](https://github.com/streamingfast/substreams-foundational-modules/tree/develop/solana-common) you must inform the Protobuf code generator to generate `subtreams_solana::pb::sf::solana::type::v1` for messages pointing to `sf.solana.type.v1`.

Add or modify the `buf.gen.yaml` file so that it has an `extern_path=...` option defined like this:

```yaml
version: v2
plugins:
  - remote: buf.build/anthropics/buffa
    out: ./src/pb
    opt:
      - extern_path=.sf.solana.type.v1=::substreams_solana::pb::sf::solana::type::v1
```

When you run `buf generate`, it will generate links to `substreams-solana` library enabling you to leverage all helpers added by the library to Solana types.

### Decoding programs from their IDL

`Idlgen` generates Rust decoders for a program from its Anchor IDL (0.30 or legacy, Shank included). The generated module depends only on `substreams-solana`.

* `build.rs`, with `substreams-solana` and `anyhow` in `[build-dependencies]`. `Idlgen` sits behind the default `idlgen` feature, so `[dependencies]` can use `substreams-solana` with `default-features = false` to keep it out of the module build:

```rust
use substreams_solana::Idlgen;

fn main() -> Result<(), anyhow::Error> {
    Idlgen::new("pumpfun", "idls/pumpfun.json")?
        .generate()?
        .write_to_file("src/idl/pumpfun.rs")?;

    Ok(())
}
```

* `src/lib.rs`:

```rust
mod idl { pub mod pumpfun; }

use idl::pumpfun;
use substreams_solana::pb::sf::solana::r#type::v1::Block;
use substreams_solana::IdlInstruction;

fn decode(block: &Block) {
    for trx in block.transactions() {
        for ix in trx.walk_instructions() {
            // One instruction type, or any of the program's instructions.
            if let Some(Ok(buy)) = pumpfun::instructions::Buy::match_and_decode(&ix) {
                println!("buy {} for at most {}", buy.amount, buy.max_sol_cost);
            }
            if let Some(Ok(ix)) = pumpfun::Instruction::match_and_decode(&ix) {
                println!("{} with {} accounts", ix.name(), ix.account_keys().len());
            }
            // Events emitted through a self-CPI (`emit_cpi!`).
            if let Some(Ok(event)) = pumpfun::Event::match_and_decode_cpi(&ix) {
                println!("{}", event.name());
            }
        }

        // Events logged as `Program data:` (`emit!`).
        for event in pumpfun::Event::decode_logs(&trx.meta.log_messages) {
            if let Ok(pumpfun::Event::TradeEvent(trade)) = event {
                println!("trade of {} lamports", trade.sol_amount);
            }
        }
    }
}

// Account data, from an account block or an RPC response.
fn decode_account(data: &[u8]) {
    if let Some(Ok(pumpfun::Account::BondingCurve(curve))) = pumpfun::Account::decode(data) {
        println!("complete: {}", curve.complete);
    }
}
```

An instruction's named accounts are a `Result`: its arguments and raw `account_keys` decode even when its accounts cannot be named, for instance when a Shank program accepts a shorter account list than its IDL describes.

Generated code also encodes: `new` and `encode_data` build instructions, `to_account_keys` their account list, `encode_account`, `encode_log_data` and `encode_cpi_data` account and event bytes, and every IDL type implements `BorshEncode`.

## Development

We manually keep in sync the rendered Rust Firehose Block models with the actual Protocol Buffer definitions file found in [sf-solana](https://github.com/streamingfast/sf-solana/tree/develop/proto) and we commit them to Git.

This means changes to Protobuf files must be manually re-generated and commit, see below for how to do it.

### Regenerate Rust Firehose Block from Protobuf

```
buf generate
```

The schema module is declared in `buf.gen.yaml`.

## Community

Need any help? Reach out!

* [StreamingFast Discord](https://discord.gg/jZwqxJAvRs)
* [The Graph Discord](https://discord.gg/vtvv7FP)

## License

[Apache 2.0](LICENSE)
