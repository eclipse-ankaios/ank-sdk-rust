# Rust Coding Rules

Applies whenever writing or editing Rust source in this crate.

- Never use `unwrap()` — linted as error; use `?` or `expect` with context
- Never call `panic!()` — linted as deny; add `#[allow]` with justification only when truly unreachable
- Never use `println!` — linted as deny; use `log` crate macros (`log::info!`, `log::debug!`, etc.)
- All public items must have `/// ...` rustdoc (`missing_docs` is a warning)
- Keep [CONTRIBUTING.md](../CONTRIBUTING.md) and [src/docs.rs](../src/docs.rs) in sync — see [DEVELOPMENT.md § Updating Documentation](../DEVELOPMENT.md#updating-documentation)
