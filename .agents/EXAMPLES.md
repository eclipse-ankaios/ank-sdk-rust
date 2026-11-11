# Running Examples

Applies when running or modifying the example workloads.

Examples are in `examples/control_interface/apps/*.rs` and run as containerized Ankaios workloads (requires `ank-server`, `ank-agent`, `podman`):

```bash
cd examples/control_interface && ./run_example.sh hello_ankaios
./stop_example.sh
```
