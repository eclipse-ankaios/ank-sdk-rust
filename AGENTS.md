# Agent Instructions

## 1) Understand the purpose

This file provides operational guidance for AI agents working in the `ank-sdk-rust` repository.
Use it as a decision and execution reference.

This is the **Rust SDK for Eclipse Ankaios** — a library that enables Rust workloads running inside
an Ankaios cluster to interact with the cluster: start/stop/update workloads, read state, stream logs,
subscribe to state-change events, and manage configs and agents.

The crate is published as `ankaios_sdk` on [crates.io](https://crates.io/crates/ankaios-sdk).

## 2) Follow non-negotiable rules

- Never use `unwrap()` in library code — it is linted as an error; use `?` or `expect` with context
- Never call `panic!()` directly in library code — it is linted as a deny; add a `#[allow]` with
  justification only when truly unreachable
- Never print directly to stdout in library code (`println!` is denied) — use the `log` crate macros
- Respect existing module/crate structure; do not reorganize files without explicit instruction
- Prefer the simplest solution that satisfies the request; do not add unnecessary abstraction
- Keep doc comments up to date: all public items must have `/// ...` docs (missing_docs is a warning)
- If you update `CONTRIBUTING.md`, also update `src/docs.rs` (they are kept in sync intentionally)

## 3) Understand the project structure

```text
src/
  lib.rs              # Public API surface: re-exports all public types
  ankaios.rs          # Main `Ankaios` struct — the primary user-facing API
  errors.rs           # `AnkaiosError` enum
  extensions.rs       # `UnreachableOption` / `UnreachableResult` extension traits
  docs.rs             # Contributing section (mirrored in CONTRIBUTING.md)
  components/
    complete_state.rs       # `CompleteState`, `AgentAttributes`
    control_interface.rs    # Low-level FIFO pipe I/O with the Ankaios agent
    event_types.rs          # `EventEntry`, `EventsCampaignResponse`
    log_types.rs            # `LogEntry`, `LogResponse`, `LogCampaignResponse`, `LogsRequest`
    manifest.rs             # `Manifest` (YAML parsing)
    request.rs              # Request types sent over the Control Interface
    response.rs             # `Response`, `UpdateStateSuccess`
    workload_mod/           # `Workload`, `WorkloadBuilder`, `File`
    workload_state_mod/     # `WorkloadInstanceName`, `WorkloadState`, `WorkloadStateCollection`, `WorkloadStateEnum`
  ankaios_api/        # Generated protobuf bindings (do not edit directly)
proto/                # Protobuf definitions consumed by build.rs
examples/
  apps/               # Standalone example programs (.rs files)
  app/                # Cargo project used to compile and run an example as a container
  manifest.yaml       # Ankaios manifest that starts the example workload
  run_example.sh      # Build + run an example against a live Ankaios cluster
  stop_example.sh     # Stop the running example
tools/
  check_coverage.sh   # Validate coverage meets the minimum threshold
  install_ankaios.sh  # Install Ankaios binaries (official release or CI artifact)
  update_version.sh   # Update SDK/Ankaios/API versions across the project
```

## 4) Understand the public API

The central entry point is the `Ankaios` struct in `src/ankaios.rs`. All methods are async and return
`Result<_, AnkaiosError>`.

### Connecting

```rust
let mut ank = Ankaios::new().await?;                                  // default 5s timeout
let mut ank = Ankaios::new_with_timeout(Duration::from_secs(10)).await?;
```

Connection is established by opening FIFO pipes at `/run/ankaios/control_interface/{input,output}`.
This path is mounted by the Ankaios agent when the workload has `controlInterfaceAccess` configured.

### Workload operations

```rust
// Build a workload
let workload = Workload::builder()
    .workload_name("my_app")
    .agent_name("agent_A")
    .runtime("podman")
    .restart_policy("NEVER")          // "NEVER" | "ON_FAILURE" | "ALWAYS"
    .runtime_config("image: docker.io/library/nginx")
    .add_dependency("other_wl", "ADD_COND_RUNNING")  // optional
    .add_tag("key", "value")                          // optional
    .build()?;

let success = ank.apply_workload(workload).await?;   // add or update
let workloads = ank.get_workload("my_app".to_owned()).await?;
let success = ank.delete_workload("my_app".to_owned()).await?;
```

### Manifest operations

```rust
let manifest = Manifest::from_file(Path::new("state.yaml"))?;
let manifest = Manifest::from_string("apiVersion: v1\nworkloads: ...".to_owned())?;
let success = ank.apply_manifest(manifest).await?;
let success = ank.delete_manifest(manifest).await?;
```

### State queries

```rust
// Full state or filtered by field masks
let state = ank.get_state(vec![]).await?;
let state = ank.get_state(vec!["workloadStates".to_owned()]).await?;
let state = ank.get_state(vec!["desiredState.workloads.my_app".to_owned()]).await?;

let agents: HashMap<String, AgentAttributes> = ank.get_agents().await?;
let attr = ank.get_agent("agent_A".to_owned()).await?;
ank.set_agent_tags("agent_A".to_owned(), tags).await?;

let wl_states: WorkloadStateCollection = ank.get_workload_states().await?;
let wl_states = ank.get_workload_states_on_agent("agent_A".to_owned()).await?;
let wl_states = ank.get_workload_states_for_name("my_app".to_owned()).await?;
let exec_state = ank.get_execution_state_for_instance_name(&instance_name).await?;
ank.wait_for_workload_to_reach_state(instance_name, WorkloadStateEnum::Running).await?;
```

### Config operations

```rust
ank.update_configs(configs_map).await?;
ank.add_config("my_config".to_owned(), value).await?;
let configs = ank.get_configs().await?;
let config = ank.get_config("my_config".to_owned()).await?;
ank.delete_config("my_config".to_owned()).await?;
ank.delete_all_configs().await?;
```

### Log streaming

```rust
let logs_request = LogsRequest {
    workload_names: vec![workload_instance_name.clone()],
    ..Default::default()
};
let mut log_campaign = ank.request_logs(logs_request).await?;
while let Some(log_response) = log_campaign.logs_receiver.recv().await {
    match log_response {
        LogResponse::LogEntries(entries) => { /* process entries */ }
        LogResponse::LogsStopResponse(name) => { break; }
    }
}
ank.stop_receiving_logs(log_campaign).await?;
```

Requires `LogRule` in `controlInterfaceAccess.allowRules` in the workload manifest.

### State change events

```rust
let mut events = ank
    .register_event(vec!["desiredState.workloads.my_app".to_owned()])
    .await?;
while let Some(event) = events.events_receiver.recv().await {
    // event.added_fields, event.updated_fields, event.removed_fields, event.complete_state
}
ank.unregister_event(events).await?;
```

### Error handling

All errors are variants of `AnkaiosError`:

| Variant                                        | Cause                                         |
|------------------------------------------------|-----------------------------------------------|
| `ControlInterfaceError`                        | Cannot connect or not connected               |
| `TimeoutError`                                 | Request or wait exceeded `ank.timeout`        |
| `ConnectionClosedError`                        | The control interface pipe was closed         |
| `AnkaiosResponseError`                         | Ankaios returned an application-level error   |
| `ResponseError`                                | Unexpected response type received             |
| `WorkloadFieldError` / `WorkloadBuilderError`  | Invalid workload construction                 |
| `ManifestParsingError`                         | Invalid YAML manifest                         |
| `IoError`                                      | Underlying I/O error                          |

## 5) Build and test

All commands use [just](https://github.com/casey/just). Run `just --list` for a full list.

### Build

```bash
just build             # debug build
just build-release     # release build
cargo build            # equivalent to just build
```

### Run tests

```bash
just all-tests         # unit tests + doc tests (preferred before committing)
just test              # unit tests only (uses cargo-nextest if installed, else cargo test)
just doctest           # documentation tests only
```

To run a specific test:

```bash
cargo test <test_name>
cargo test -p ankaios_sdk <test_name>
```

### Linting

```bash
just clippy            # runs clippy with --all-features and -Dclippy::all -Dclippy::pedantic
```

Clippy is configured strictly. Fix all warnings before submitting; the CI treats them as errors.

### Code coverage

```bash
just cov               # generate coverage report (requires cargo-llvm-cov)
just cov-check         # validate coverage meets the minimum threshold (tools/check_coverage.sh)
just cov-html          # generate HTML report
just cov-open          # serve HTML report on http://localhost:8000
```

### Documentation

```bash
just doc               # generate rustdoc (--all-features --document-private-items)
just doc-open          # serve docs on http://localhost:8001
```

### License check

```bash
just deny              # cargo-deny: only MIT, Apache-2.0, BSD-3-Clause are allowed
```

### Reproduce CI failures locally

The devcontainer pins a specific Rust version; CI uses the latest stable. To match CI:

```bash
rustup update stable
rustup target add --toolchain stable x86_64-unknown-linux-musl
rustup run stable just clippy
rustup run stable just all-tests
```

## 6) Run examples

Examples live in `examples/apps/*.rs` and are run as containerized Ankaios workloads.
Running an example requires Ankaios binaries (`ank-server` and `ank-agent`) and `podman`.

```bash
cd examples
./run_example.sh hello_ankaios    # build image + start cluster + apply manifest
./stop_example.sh                 # stop the cluster
```

Available examples:

| File                  | What it demonstrates                                             |
|-----------------------|------------------------------------------------------------------|
| `hello_ankaios.rs`    | Apply a workload, check state, wait for Running, delete workload |
| `get_state.rs`        | Poll and print workload states in a loop                         |
| `state_event.rs`      | Subscribe to state-change events for a workload                  |
| `get_logs.rs`         | Stream logs from a running workload                              |
| `test_workload.rs`    | Workload builder options                                         |
| `test_manifest.rs`    | Manifest API usage                                               |
| `test_configs.rs`     | Config CRUD operations                                           |
| `test_files.rs`       | Injecting files into a workload via the `File` type              |

To use a custom Ankaios binary path:

```bash
export ANK_BIN_DIR=/path/to/ankaios/executables
./run_example.sh hello_ankaios
```

To install Ankaios binaries:

```bash
tools/install_ankaios.sh --help
```

## 7) Understand the control interface

The SDK communicates with the Ankaios agent via Unix FIFO pipes at:

- `/run/ankaios/control_interface/input` — SDK writes requests
- `/run/ankaios/control_interface/output` — SDK reads responses

Messages are length-prefixed protobuf (varint + serialized proto). The proto definitions live in
`proto/` and the generated Rust code is in `src/ankaios_api/`. Do not edit generated files directly.

For the control interface to be mounted, the workload's manifest must include:

```yaml
controlInterfaceAccess:
  allowRules:
    - type: StateRule
      operation: ReadWrite   # or Read / Write
      filterMask:
        - "*"
    - type: LogRule           # only needed if using request_logs
      workloadNames:
        - "my_other_workload"
```

## 8) Versioning and release

- SDK version mirrors the Ankaios version it targets (e.g., SDK `1.0.x` works with Ankaios `1.0.x`)
- To update versions across the project: `tools/update_version.sh --help`
- Use `--release` flag when updating for a release (also updates docs URLs and badges)

## 9) Code conventions

- Use `log::debug!`, `log::info!`, `log::warn!`, `log::error!` — never `println!` in library code
- Use `thiserror` for error types; new error kinds go into `AnkaiosError` in `src/errors.rs`
- Use `mockall` / `mockall_double` for mocking in tests (see `control_interface.rs` for patterns)
- All public types must have rustdoc with `# Errors` and `# Examples` sections where applicable
- Test utilities can be gated with the `test_utils` feature flag
