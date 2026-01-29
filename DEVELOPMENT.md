# Development Guide

This guide provides information about the development tools and workflows available for this project.

## Quick Start

### Project Setup

This project uses [DevContainers](https://containers.dev/) for a consistent development environment. The devcontainer includes the rust toolchain, the necessary tooling and the required VSCode extensions.

The project provides [just](https://github.com/casey/just) commands for common development workflows and helper scripts in the [tools](tools/) directory for specific tasks. See the sections below for details.

## Just Commands

Common development commands via [just](https://github.com/casey/just). Run `just help` for a complete list.

### Building

| Command              | Description                                   |
| -------------------- | --------------------------------------------- |
| `just build`         | Build the SDK in debug mode                   |
| `just build-release` | Build the SDK in release mode                 |
| `just clean`         | Clean the build directory and cargo artifacts |

### Testing

| Command          | Description                                                       |
| ---------------- | ----------------------------------------------------------------- |
| `just test`      | Run tests (uses cargo-nextest if available, otherwise cargo test) |
| `just doctest`   | Run documentation tests                                           |
| `just all-tests` | Run all available tests                                           |

### Code Quality

| Command       | Description                                      |
| ------------- | ------------------------------------------------ |
| `just clippy` | Run clippy linter with strict checks             |
| `just deny`   | Check licenses and dependencies using cargo-deny |

### Code Coverage

| Command          | Description                                                   |
| ---------------- | ------------------------------------------------------------  |
| `just cov`       | Generate code coverage report                                 |
| `just cov-check` | Verify coverage meets the set threshold using a custom script |
| `just cov-html`  | Generate HTML coverage report                                 |
| `just cov-open`  | Open HTML coverage report in browser (serves on port 8000)    |

### Documentation

| Command          | Description                                                   |
| ---------------- | ------------------------------------------------------------- |
| `just doc`       | Generate project documentation                                |
| `just doc-open`  | Open generated documentation in browser (serves on port 8001) |

### Tooling

| Command          | Description                                                             |
| ---------------- | ----------------------------------------------------------------------- |
| `just msrv-find` | Find the minimum supported Rust version (installs cargo-msrv if needed) |

## Helper Scripts

The [tools](tools/) directory contains utility scripts for specific tasks. All scripts support the `--help` flag for detailed usage information.

### check_coverage.sh

Validates that code coverage meets the minimum threshold.

Recommended usage via just command: `just cov-check`

### update_version.sh

Updates SDK, Ankaios, and API versions across the project. Supports updating versions individually or all at once. Use the `--release` flag for release versions to also update documentation URLs and badges.

## Development Workflow

### Before Committing

Always run these checks before creating a pull request:

```bash
just clippy          # Lint check
just all-tests       # All tests pass
just cov-check       # Coverage threshold met
just deny            # License compliance
```

### Updating Documentation

When making changes to project documentation, keep the following in mind:

**Contributing section synchronization:**

The contributing section is maintained in two locations:

- [CONTRIBUTING.md](CONTRIBUTING.md) - Used in the repository
- [src/docs.rs](src/docs.rs) - Used in the generated Rust documentation

If you update one, you must update the other to keep them synchronized. They are duplicated to ensure proper link resolution in both contexts.

**Adding new tools:**

When adding a new tool or script to the [tools](tools/) directory:

1. Update this DEVELOPMENT.md file with the tool's description and usage
2. Ensure the script includes a `--help` option with detailed usage information
3. Follow the pattern established by existing scripts.

## Troubleshooting

### Checks pass locally but fail in the pipeline

The devcontainer uses a fixed Rust version, while the CI pipeline uses the latest stable release. This can cause checks to pass locally but fail in the pipeline.

To reproduce pipeline failures locally, update to the latest stable and add the required target:

```bash
rustup update stable
rustup target add --toolchain stable x86_64-unknown-linux-musl
```

Then run checks with the latest version:

```bash
rustup run stable just clippy
rustup run stable just all-tests
```

Or set stable as default and add the target:

```bash
rustup default stable
rustup target add x86_64-unknown-linux-musl
```

## Additional Resources

- [Contributing Guidelines](CONTRIBUTING.md)
- [Rust Coding Guidelines](https://eclipse-ankaios.github.io/ankaios/main/development/rust-coding-guidelines/)
- [Unit Verification Strategy](https://eclipse-ankaios.github.io/ankaios/main/development/unit-verification/)
