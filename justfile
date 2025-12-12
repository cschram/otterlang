set windows-shell := ["pwsh.exe", "-c"]

build:
    cargo build

test:
    cargo test

fmt:
    cargo fmt

lint:
    cargo clippy

bench:
    cargo bench

examples: build
    ./scripts/examples.sh