# Architecture Notes

Non-obvious facts about the SDK's internals — useful when navigating or modifying core code.

## General

- The main API is the `Ankaios` struct in `src/ankaios.rs`. All methods are async and return `Result<_, AnkaiosError>`.
- All error types are variants of `AnkaiosError` in `src/errors.rs`.
- `src/ankaios_api/` contains generated protobuf bindings — do not edit directly.
- SDK version mirrors the Ankaios version it targets (e.g., `1.0.x` ↔ Ankaios `1.0.x`). To update versions: `tools/update_version.sh --help`.
- Test utilities are gated behind the `test_utils` feature flag.
- Use `mockall` for mocking in tests; the pattern is `#[cfg_attr(test, automock)]` on the `Connection` trait.

## Interfaces

`Ankaios` only talks to the `Connection` trait (`src/components/connection.rs`), so everything above it is the same for
all interfaces. Each implementation lives in `src/components/connection/` behind its own cargo feature, and at least one
feature must be enabled.

- **Control interface** (`control_interface`, default): for workloads running inside an Ankaios cluster. Talks to the
  agent over FIFO pipes.
- **Command interface** (`command_interface`): for tools outside the cluster. Talks gRPC directly to the Ankaios server.

### Control interface

- Workloads must declare `controlInterfaceAccess.allowRules` to use the control interface. Add a
  `LogRule` when using `request_logs`; these permissions are enforced by the Ankaios agent.

### Command interface

- The transport (TCP or Unix domain socket) is selected on `GrpcConfig`, which returns a separate builder type per
  transport. This is deliberate: options that only make sense for one transport (mTLS) do not exist on the other, so
  invalid combinations do not compile.
- The Unix socket support needs extra dependencies (`tower`, `hyper-util`, `tokio/net`), which must stay enabled only by the
  `command_interface` feature.
