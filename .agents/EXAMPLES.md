# Running Examples

Applies when running or modifying the example workloads.

Examples are in `examples/apps/*.rs` and run as containerized Ankaios workloads (requires `ank-server`, `ank-agent`, `podman`):

```bash
cd examples && ./run_example.sh hello_ankaios
./stop_example.sh
```
