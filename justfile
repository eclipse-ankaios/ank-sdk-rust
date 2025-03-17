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
clippy: # --all-targets
    cargo clippy --tests --no-deps --all-features -- -Dclippy::all -Dclippy::pedantic

# Run tests using cargo nextest if installed
test:
    bash -c 'if which cargo-nextest > /dev/null 2>&1; then cargo nextest run; else cargo test; fi'

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

# Open code coverage HTML as a server, depending on the method
# cov-open method="server":
#     if [[ "{{method}}" == "server" ]]; then
#         if command -v python3 > /dev/null; then
#             python3 -m http.server -d target/llvm-cov/html 8000
#         else
#             echo "Error: python3 is not available for running the server."
#             exit 1
#         fi
#     elif [[ "{{method}}" == "xdg" ]]; then
#         if command -v xdg-open > /dev/null; then
#             xdg-open target/llvm-cov/html/index.html
#         else
#             echo "Error: xdg-open is not available."
#             exit 1
#         fi
#     elif [[ "{{method}}" == "open" ]]; then
#         if command -v open > /dev/null; then
#             open target/llvm-cov/html/index.html
#         else
#             echo "Error: open is not available."
#             exit 1
#         fi
#     else
#         echo "Error: Invalid method. Use 'server', 'xdg', or 'open'."
#         exit 1
#     fi
