.PHONY: build test lint clean fmt check deny

build:
	cargo build

test:
	cargo test

lint: fmt-check clippy deny

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy -- -D warnings

deny:
	cargo deny check

clean:
	cargo clean

check: fmt-check clippy test

all: fmt clippy deny test build
