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
substreams-solana = "0.13"
```

### Protobuf Extern paths

If you have other protobuf objects that refer to the `sf.solana.type.v1` types, like the [Solana Substreams Foundational Modules](https://github.com/streamingfast/substreams-foundational-modules/tree/develop/solana-common) you must inform the Protobuf code generator to generate `subtreams_solana::pb::sf::solana::type::v1` for messages pointing to `sf.solana.type.v1`.

Add or modify the `buf.gen.yaml` file so that it has an `extern_path=...` option defined like this:

```yaml
version: v1
plugins:
- plugin: buf.build/community/neoeinstein-prost:v0.2.2 # check compatibility with your 'prost' crate
  out: ./src/pb
  opt:
    - file_descriptor_set=false
    - extern_path=.sf.solana.type.v1=::substreams_solana::pb::sf::solana::type::v1

- plugin: buf.build/community/neoeinstein-prost-crate:v0.3.1 # check compatibility with your 'prost' crate
  out: ./src/pb
  opt:
    - no_features
```

When you run `substreams protogen` or `buf generate proto`, it will probably generate links to `substreams-solana` library enabling you to leverage all helpers this library is adding.

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
