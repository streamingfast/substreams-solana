#!/bin/bash

SOL_SPKG="https://github.com/streamingfast/firehose-solana/releases/download/v1.3.0/solana-v0.1.5.spkg"
substreams protogen "$SOL_SPKG" --exclude-paths="sf/substreams/rpc,sf/substreams/options.proto,sf/substreams/sink,sf/substreams/index,sf/substreams/v1,google/" --output-path="./core/src/pb"
