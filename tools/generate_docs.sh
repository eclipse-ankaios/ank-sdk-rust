#!/bin/bash

# Ensure target directory exists
mkdir -p target

# Modify README.md
awk '/^\[Eclipse Ankaios\]/ {found=1} found' README.md > target/README.md
sed -i 's|CONTRIBUTING.md|docs/contributing/index.html|g' target/README.md

# Modify CONTRIBUTING.md
cp CONTRIBUTING.md target/CONTRIBUTING.md
sed -i 's|./CODE_OF_CONDUCT.md|../conduct/index.html|g' target/CONTRIBUTING.md

# Generate Rust documentation
cargo doc --no-deps --all-features --document-private-items