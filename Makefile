.PHONY: build test lint fmt estimate refresh tui

build:
	cargo build

test:
	cargo test --all

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

