# Command Interface Example Applications for the Rust SDK

These examples show how the Ankaios Rust SDK interacts with a running cluster through
the command interface, directly from outside an Ankaios managed workload. They use
gRPC over TCP or a Unix domain socket, rather than the control interface's FIFO pipes.

## Prerequisites

Install Ankaios using the [installation guide](https://eclipse-ankaios.github.io/ankaios/latest/usage/installation/#setup-with-script).
Unix domain sockets require an Ankaios version that supports `unix://` endpoints.
The agent needs Podman to run the example workloads. Building requires Rust and
Protobuf's `protoc`; the repository's devcontainer provides the build tools.

> [!IMPORTANT]
> The examples create and delete workloads named `dynamic_nginx` (port 8080) and
> `count_to_five` on the cluster, and pull images from docker.io and ghcr.io.
> `stop_ankaios.sh` stops and removes **all** Podman containers of the current
> user, not only the ones started by the examples.

An Ankaios server and at least one agent must be running and reachable before you
run an example. The server listens on one endpoint, so choose either TCP or a Unix
domain socket.

## Starting Ankaios

From the repository root, enter the example directory:

```shell
cd examples/command_interface
```

[start_ankaios.sh](start_ankaios.sh) starts a server and an agent named `agent_A` in
the background, waits until the agent is connected and stores the logs in
`ankaios_run/`:

```shell
# TCP on 127.0.0.1:25551 (default)
./start_ankaios.sh tcp

# Unix domain socket at /tmp/ankaios.sock
./start_ankaios.sh unix
```

The Ankaios executables are taken from `/usr/local/bin`. To use another location,
set `ANK_BIN_DIR=/path/to/ankaios/executables`.

### Manual setup

Alternatively, start the server and agent yourself in separate terminals.

#### TCP

```shell
ank-server --insecure --address 127.0.0.1:25551
```

```shell
ank-agent --insecure --name agent_A --server-url http://127.0.0.1:25551
```

This uses plaintext TCP and is intended for local development.

#### Unix domain socket

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

From the example directory, build the examples:

```shell
cargo build --release
```

The package and resulting binary are named `test_app`. With the repository's
default Cargo configuration, run an example from this directory as follows:

```shell
# TCP (default server URL and agent name)
./target/x86_64-unknown-linux-musl/release/test_app basic_test

# Unix domain socket
./target/x86_64-unknown-linux-musl/release/test_app basic_test unix:///tmp/ankaios.sock agent_A
```

The signature is:

```shell
test_app <example> [server_url] [agent_name]
```

- `example`: one of `basic_test`, `logs_events_test` or `connection_lifecycle_test`
  (see [Examples](#examples)). Required; running without it prints the usage.
- `server_url`: defaults to `http://127.0.0.1:25551`. Use `http://...` for plaintext
  TCP or `unix://<absolute-path>` for a Unix domain socket.
- `agent_name`: defaults to `agent_A` and must match a running agent.

The shared helper in [src/common.rs](src/common.rs) selects `GrpcConfig::unix_socket()` or
`GrpcConfig::tcp()` from the URL. These examples do not configure mTLS; HTTPS requires
adapting the helper to use `GrpcConfig::tcp().url(...).mtls(ca_pem, crt_pem, key_pem)`.

The devcontainer targets `x86_64-unknown-linux-musl`. You can copy the resulting
static `test_app` binary to an x86_64 Linux host with Ankaios installed and run
`./test_app <example>` there with the same arguments. If you override the Cargo target or
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

## Stopping the examples

The examples exit after completing their operations. Press Ctrl+C to interrupt a
run. An interrupted or failed run can leave example workloads behind.

To clean up, run [stop_ankaios.sh](stop_ankaios.sh):

```shell
./stop_ankaios.sh
```

It stops the server and agent started by `start_ankaios.sh` (using the PIDs it
recorded in `ankaios_run/`), removes the Unix socket and stops and removes all
Podman containers. The logs remain in `ankaios_run/`. If you started Ankaios
manually, stop the processes in their terminals instead.
