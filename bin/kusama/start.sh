#!/usr/bin/env bash

ulimit -n 100000
export RUST_BACKTRACE=full
nohup ../../target/release/kusama-archive -c ./kusama.toml > kusama-archive.log 2>&1 &
