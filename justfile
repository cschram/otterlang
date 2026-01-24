set windows-shell := ["pwsh.exe", "-c"]

build:
    cargo build

test:
    cargo nextest run

fmt:
    cargo fmt

lint:
    cargo clippy

bench:
    cargo bench

example EXAMPLE: build
    ./target/debug/otter run ./examples/{{EXAMPLE}}.ot

examples: build
    ./scripts/examples.sh