#!/bin/bash

SOL_SPKG="https://github.com/streamingfast/firehose-solana/releases/download/v0.1.1/solana-v0.1.1.spkg"
substreams protogen "$SOL_SPKG" --exclude-paths="sf/substreams/rpc,sf/substreams/v1,google/" --output-path="./core/src/pb"
