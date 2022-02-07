#!/usr/bin/env bash

ulimit -n 100000
export RUST_BACKTRACE=full
cp target/release/polkadot-archive polkadot-archive-dev
nohup ./polkadot-archive-dev -c ./archive.toml --start-block=7410000 > polkadot-archive-dev.log 2>&1 &
