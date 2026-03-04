#!/bin/bash

set -e

SOL_SPKG="https://github.com/streamingfast/firehose-solana/releases/download/v1.3.0/solana-v0.1.5.spkg"
temp_dir="$(mktemp -d)"
trap "rm -rf $temp_dir" EXIT

echo "Generating Solana Protobuf (prost) using $SOL_SPKG"
substreams protogen "$SOL_SPKG" --exclude-paths="sf/substreams/rpc,sf/substreams/options.proto,sf/substreams/sink,sf/substreams/index,sf/substreams/v1,google/" --output-path="./core/src/pb"

input="${BUF_MODULE_REF:-buf.build/streamingfast/firehose-solana}"

echo "Generating Quick Protobuf code from $input"
echo "Temporary directory: $temp_dir"

echo "Exporting proto files..."
buf export "$input" -o "$temp_dir"

echo "Exporting google protobuf Well-Known Types..."
buf export buf.build/protocolbuffers/wellknowntypes -o "$temp_dir"

echo "Generating and organizing Quick Protobuf code via build.rs..."
GENERATE_QUICK_PB=1 PROTO_DIR="$temp_dir" cargo build --manifest-path core/Cargo.toml

echo "Quick Protobuf generation complete!"
