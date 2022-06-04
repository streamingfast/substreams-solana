#!/bin/bash

SOL_SPKG="https://github.com/streamingfast/sf-solana/releases/download/v0.1.0/solana-v0.1.0.spkg"
substreams protogen "$SOL_SPKG" --exclude-paths="sf/substreams/v1,google/" --output-path="./core/src/pb"
