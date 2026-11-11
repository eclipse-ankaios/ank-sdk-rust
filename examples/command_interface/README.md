# Command Interface Example Applications for the Rust SDK

These examples show how the Ankaios Rust SDK interacts with a running cluster through
the command interface, directly from outside an Ankaios managed workload. They use
gRPC over TCP or a Unix domain socket, rather than the control interface's FIFO pipes.

## Prerequisites

Install Ankaios using the [installation guide](https://eclipse-ankaios.github.io/ankaios/latest/usage/installation/#setup-with-script).
Unix domain sockets require an Ankaios version that supports `unix://` endpoints.
The agent needs Podman to run the example workloads. Building requires Rust and
Protobuf's `protoc`; the repository's devcontainer provides the build tools.

An Ankaios server and at least one agent must be running and reachable before you
run an example. A server listens on one endpoint, so choose one of the following
options. Start the server and agent in separate terminals.

### Option 1: TCP

```shell
ank-server --insecure --address 127.0.0.1:25551
```

```shell
ank-agent --insecure --name agent_A --server-url http://127.0.0.1:25551
```

This uses plaintext TCP and is intended for local development.

### Option 2: Unix domain socket

This requires a Unix system and an Ankaios version with Unix socket support.
No TLS is used, so do not pass `--insecure` to the server or agent.

```shell
ank-server --address unix:///tmp/ankaios.sock
```

```shell
ank-agent --name agent_A --server-url unix:///tmp/ankaios.sock
```

The socket path must be absolute. The server creates the socket with mode `0660`,
so the agent and example processes need access as the socket owner or members of
its group. To assign a group to the socket, pass `--socket-group <group>` to the server.

If the agent cannot create its default run folder (`/run/ankaios/`), create a directory
that only you can access and pass it with `--run-folder <dir>`.

## Building and running

From the repository root, enter the example directory:

```shell
cd examples/command_interface
```

The example to run is selected in [src/main.rs](src/main.rs) by
commenting/uncommenting the calls. Only `basic_test` is enabled by default.
Rebuild after changing the selection:

```shell
cargo build --release
```

The package and resulting binary are named `test_app`. With the repository's
default Cargo configuration, run it from this directory as follows:

```shell
# TCP (default server URL and agent name)
./target/x86_64-unknown-linux-musl/release/test_app

# Unix domain socket
./target/x86_64-unknown-linux-musl/release/test_app unix:///tmp/ankaios.sock agent_A
```

The signature is:

```shell
test_app [server_url] [agent_name]
```

- `server_url`: defaults to `http://127.0.0.1:25551`. Use `http://...` for plaintext
  TCP or `unix://<absolute-path>` for a Unix domain socket.
- `agent_name`: defaults to `agent_A` and must match a running agent.

The shared helper in [src/common.rs](src/common.rs) selects `GrpcConfig::unix_socket()` or
`GrpcConfig::tcp()` from the URL. These examples do not configure mTLS; HTTPS requires
adapting the helper to use `GrpcConfig::tcp().url(...).mtls(ca_pem, crt_pem, key_pem)`.

The devcontainer targets `x86_64-unknown-linux-musl`. You can copy the resulting
static `test_app` binary to an x86_64 Linux host with Ankaios installed and run
`./test_app` there with the same arguments. If you override the Cargo target or
target directory, adjust the binary path accordingly.

## Examples

This folder contains the following manually run examples; running `cargo test`
does not execute them against a cluster.

### `basic_test`

Applies an nginx workload, waits for it to reach `RUNNING`, updates its restart
policy, then deletes it. Prints workload states after each change.

### `logs_events_test`

Registers for desired-state events on a workload, starts it, then streams its logs
and prints events until logging stops. Finally, unregisters the campaigns and
deletes the workload.

### `connection_lifecycle_test`

Opens and drops three sequential command interface connections, requesting and
printing workload states each time. Uses the server URL but does not create
workloads or use the agent name.

### `common`

Shared transport-selection and workload-state printing helpers.

## Stopping the examples

The examples exit after completing their operations. Press Ctrl+C to interrupt a
run. An interrupted or failed run can leave example workloads behind; inspect and
remove those workloads manually.

The examples do not stop the Ankaios server or agent. Stop the processes you
started in their respective terminals when you finish. There is no cleanup script
that stops unrelated instances or removes all Podman containers.
