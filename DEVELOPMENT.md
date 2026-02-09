# Development Guide

This guide provides information about the development tools and workflows available for this project.

## Project Setup

This project uses [DevContainers](https://containers.dev/) for a consistent development environment. The devcontainer includes the rust toolchain, the necessary tooling and the required VSCode extensions.

## Commands and Scripts

The project provides [just](https://github.com/casey/just) commands for common development workflows. For a complete list of available commands, check the [justfile](justfile) or directly run `just --list`.

In addition, the [tools](tools/) folder contains helper scripts for specific tasks. Check the tools [readme](tools/README.md) for more info.

## Updating Documentation

When making changes to project documentation, keep the following in mind:

**Contributing section synchronization:**

The contributing section is maintained in two locations:

- [CONTRIBUTING.md](CONTRIBUTING.md) - Used in the repository
- [src/docs.rs](src/docs.rs) - Used in the generated Rust documentation

If you update one, you must update the other to keep them synchronized. They are duplicated to ensure proper link resolution in both contexts.

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
