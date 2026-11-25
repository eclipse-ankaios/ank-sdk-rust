# Running Examples

Applies when running or modifying the example workloads. Both example sets need `ank-server`, `ank-agent` and `podman`.
`stop_*.sh` stops and removes all podman containers.

## Control interface

Examples are in `examples/control_interface/apps/*.rs` and run as containerized Ankaios workloads (FIFO pipes). See [README](../examples/control_interface/README.md).

```bash
cd examples/control_interface && ./run_example.sh hello_ankaios
./stop_example.sh
```

## Command interface

A standalone crate (`test_app`) in `examples/command_interface/` that connects from outside the cluster over gRPC (TCP or Unix socket). See [README](../examples/command_interface/README.md).

```bash
cd examples/command_interface && ./start_ankaios.sh tcp   # or: unix
cargo build --release
./target/x86_64-unknown-linux-musl/release/test_app basic_test   # see README for examples and arguments
./stop_ankaios.sh
```
