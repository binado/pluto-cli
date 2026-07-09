# https://just.systems

test:
    cargo test --all-features

lint:
    cargo clippy --all-targets --all-features -- -D warnings

format:
    cargo fmt --all

format-check:
    cargo fmt --all -- --check

check:
    cargo check --all-targets --all-features

build:
    cargo build --release --all-features

# Matches CI jobs (read-only formatting check).
ci: format-check lint check test build

# Quick static checks without tests or release build.
check-all: format-check lint check
