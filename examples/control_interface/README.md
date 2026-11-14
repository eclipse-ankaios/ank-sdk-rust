# Control Interface Example Applications for the Rust SDK

This example application demonstrates how the Ankaios Rust SDK can be used to interact with a running cluster by using the control interface from an Ankaios managed workload.

## Prerequisites

In order to run the examples, Ankaios must be installed and available on the system. To install Ankaios, take a look at the [installation guide](https://eclipse-ankaios.github.io/ankaios/latest/usage/installation/#setup-with-script).

Alternatively you can compile from source and provide the path to the executables to the run scripts with the `ANK_BIN_DIR` environment variable.

> **IMPORTANT**  
> Make sure that no other Ankaios instance is running on the local machine. If there is an instance running, the example could change its configuration. The stop script would even stop all running instances and **delete all podman containers**.

## Building and running

To build and run an example, just execute the `run_example.sh` script. The signature is as follows:

```shell
./run_example.sh <example-name> [extra-build-args]
```

For example, running the default example:

```shell
./run_example.sh hello_ankaios
```

You can also provide additional arguments to the build command if needed:

```shell
./run_example.sh hello_ankaios --no-cache
```

Of course, you are free to build manually if needed by calling the following command from the repository root:

```shell
podman build -t app:0.1 -f examples/control_interface/app/Dockerfile .
```

## Stopping the example

To stop the example just call

```shell
./stop_example.sh
```

> **WARNING**  
> The stop script terminates all running ankaios instances, both server and agents and **deletes all podman containers** on the system. If you don't want this, you would need to clean up manually.

## Examples

This folder currently contains the following examples:

### `hello_ankaios`

This example executes a workload that prints a hello message to demonstrate the basic usage of the Rust SDK.

Feel free to update the code in the example and test it with the `run_example.sh` script.

To view the logs of an example like the `hello_ankaios` workload just use the convenience script:

```shell
./ank-logs.sh
```

### `get_logs`

The `get_logs` example workload starts another workload that prints log lines and then it requests and prints them.

### `get_state`

The `get_state` example demonstrates how to retrieve the current state of the Ankaios cluster.

### `state_event`

The `state_event` example workload registers for events of a specific workload, starts it, and showcases the basic functionality of the event subscription and their format.

### `test_workload`

`test_workload` demonstrates workload lifecycle operations (start, stop, update).

### `test_manifest`

`test_manifest` shows how to apply and manage workload manifests through the control interface.

### `test_configs`

`test_configs` demonstrates configuration management operations through the Ankaios SDK.

### `test_files`

`test_files` exemplifies how files mounted to a workload can be manipulated.
