#!/bin/bash

SOL_SPKG="https://github.com/streamingfast/firehose-solana/releases/download/v1.1.3/solana-v0.1.4.spkg"
substreams protogen "$SOL_SPKG" --exclude-paths="sf/substreams/rpc,sf/substreams/v1,google/" --output-path="./core/src/pb"
