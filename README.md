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

* If you have other protobuf objects that refer to the `sf.solana.type.v1` types, create a `buf.gen.yaml` file like this before you run `substreams protogen` or `buf generate`:

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
