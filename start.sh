#!/usr/bin/env bash

ulimit -n 100000
export RUST_BACKTRACE=full
nohup ./target/release/polkadot-archive -c ./archive.toml > polkadot-archive.log 2>&1 &
