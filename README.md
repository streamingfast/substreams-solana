# Substreams Solana

Substreams development kit for Solana chains, contains Rust Firehose Block model and helpers

## Usage

```toml
[package]
name = "substreams-solana"
version = "0.1.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
substreams-solana = "0.1.0"
```

## Development

We manually keep in sync the rendered Rust Firehose Block models with the actual Protocol Buffer definitions file found in [sf-solana](https://github.com/streamingfast/sf-solana/tree/develop/proto) and we commit them to Git.

This means changes to Protobuf files must be manually re-generated and commit, see below for how to do it.

### Regenerate Rust Firehose Block from Protobuf

```
./gen.sh
```

## Community

Need any help? Reach out!

* [StreamingFast Discord](https://discord.gg/jZwqxJAvRs)
* [The Graph Discord](https://discord.gg/vtvv7FP)

## License

[Apache 2.0](LICENSE)
