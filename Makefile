.PHONY: upgrade-substrate build build-polkadot build-kusama fmt-check fmt clean

upgrade-substrate:
	sed -i 's/${from}/${to}/g' `rg "${from}" -t toml -T lock -l`
	cargo update

build: build-polkadot

build-polkadot:
	cargo build --release --bin polkadot-archive --features polkadot

build-kusama:
	cargo build --release --bin kusama-archive --features kusama

fmt-check:
	cargo +nightly fmt --all -- --check

fmt:
	cargo +nightly fmt --all

clean:
	cargo clean
