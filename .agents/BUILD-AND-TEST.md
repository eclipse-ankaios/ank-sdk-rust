# Build and Test

Applies before building, testing, or linting.

All commands via [just](https://github.com/casey/just). Run `just --list` for the full list.

> **Note:** The devcontainer pins a specific Rust version; CI uses the latest stable. If something passes locally but fails in CI (or vice versa), see [DEVELOPMENT.md § Troubleshooting](../DEVELOPMENT.md#troubleshooting) to reproduce the CI environment.

```bash
just build          # debug build
just all-tests      # unit + doc tests (run before committing)
just clippy         # strict clippy (all warnings are CI errors)
just deny           # license check (MIT, Apache-2.0, BSD-3-Clause only)
```
