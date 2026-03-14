# Agent Instructions

## Purpose

Rust SDK for Eclipse Ankaios — enables workloads inside an Ankaios cluster to start/stop/update
workloads, read state, stream logs, subscribe to events, and manage configs.
Crate: `ankaios_sdk` on crates.io.

## Non-negotiable rules

- Never use `unwrap()` — linted as error; use `?` or `expect` with context
- Never call `panic!()` — linted as deny; add `#[allow]` with justification only when truly unreachable
- Never use `println!` — linted as deny; use `log` crate macros (`log::info!`, `log::debug!`, etc.)
- Do not reorganize files without explicit instruction
- Prefer the simplest solution; no unnecessary abstraction
- All public items must have `/// ...` rustdoc (`missing_docs` is a warning)
- If you update `CONTRIBUTING.md`, also update `src/docs.rs` — they are kept in sync

## Build and test

All commands via [just](https://github.com/casey/just). Run `just --list` for the full list.

> **Note:** The devcontainer pins a specific Rust version; CI uses the latest stable. If something passes locally but fails in CI (or vice versa), run with `rustup run stable just clippy` / `rustup run stable just all-tests` to reproduce the CI environment.

```bash
just build          # debug build
just all-tests      # unit + doc tests (run before committing)
just clippy         # strict clippy (all warnings are CI errors)
just deny           # license check (MIT, Apache-2.0, BSD-3-Clause only)
```

## Key non-obvious facts

- The main API is the `Ankaios` struct in `src/ankaios.rs`. All methods are async and return `Result<_, AnkaiosError>`.
- All error types are variants of `AnkaiosError` in `src/errors.rs`.
- Communication uses Unix FIFO pipes at `/run/ankaios/control_interface/{input,output}` (length-prefixed protobuf).
- For the pipes to be mounted, the workload manifest must declare `controlInterfaceAccess.allowRules` (add a `LogRule` entry when using `request_logs`).
- `src/ankaios_api/` contains generated protobuf bindings — do not edit directly.
- Use `mockall` / `mockall_double` for mocking in tests; see `src/components/control_interface.rs` for the established pattern.
- Test utilities are gated behind the `test_utils` feature flag.
- SDK version mirrors the Ankaios version it targets (e.g., `1.0.x` ↔ Ankaios `1.0.x`). To update versions: `tools/update_version.sh --help`.

## Run examples

Examples are in `examples/apps/*.rs` and run as containerized Ankaios workloads (requires `ank-server`, `ank-agent`, `podman`):

```bash
cd examples && ./run_example.sh hello_ankaios
./stop_example.sh
```
