#!/bin/bash

# List all available commands
help:
    just -l

# Build SDK
build:
    cargo build

# Build SDK in release mode
build-release:
    cargo build --release

# Clean the build directory
clean:
    cargo clean
    rm -rf build

# Run clippy code checks
clippy:
    cargo clippy --tests --no-deps --all-features -- -Dclippy::all -Dclippy::pedantic

# Run all tests
all-tests: test doctest

# Run tests using cargo nextest if installed
test:
    bash -c 'if which cargo-nextest > /dev/null 2>&1; then cargo nextest run; else cargo test --tests; fi'

# Run documentation tests
doctest:
    cargo test --doc --target x86_64-unknown-linux-gnu

# Run code coverage
cov:
    cargo llvm-cov

# Check coverage
cov-check:
    tools/check_coverage.sh

# Generate code coverage HTML
cov-html:
    cargo llvm-cov --html

# Open code coverage HTML
cov-open:
    python3 -m http.server -d target/llvm-cov/html 8000

# Generate documentation
doc:
    cargo doc --no-deps --all-features --document-private-items

# Open documentation
doc-open:
    python3 -m http.server -d target/x86_64-unknown-linux-musl/doc 8001
