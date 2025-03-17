<picture style="padding-bottom: 1em;">
  <source media="(prefers-color-scheme: dark)" srcset="https://github.com/eclipse-ankaios/ankaios/blob/main/logo/Ankaios__logo_for_dark_bgrd_clipped.png">
  <source media="(prefers-color-scheme: light)" srcset="https://github.com/eclipse-ankaios/ankaios/blob/main/logo/Ankaios__logo_for_light_bgrd_clipped.png">
  <img alt="Shows Ankaios logo" src="https://github.com/eclipse-ankaios/ankaios/blob/main/logo/Ankaios__logo_for_light_bgrd_clipped.png">
</picture>

# Rust SDK for Eclipse Ankaios

[![Github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/eclipse-ankaios/ank-sdk-rust)
[![Crates.io](https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust)](https://crates.io/crates/ankaios-sdk)
[![Docs.rs](https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs)](https://docs.rs/ankaios-sdk/0.5.0-rc1)

[Eclipse Ankaios](https://github.com/eclipse-ankaios/ankaios) provides workload and
container orchestration for automotive High Performance Computing Platforms (HPCs).
While it can be used for various fields of applications, it is developed from scratch
for automotive use cases and provides a slim yet powerful solution to manage
containerized applications.

The Rust SDK provides easy access from the container (workload) point-of-view
to manage the Ankaios system. A workload can use the Rust SDK to start, stop and
update other workloads and configs and get the state of the Ankaios system.

## Setup

### Setup via crates.io

Add the following to your `Cargo.toml`:

```toml
[dependencies]
ankaios_sdk = "0.5.0-rc1"
```

### Clone and link as vendor

Create a `vendor` folder and clone the crate inside:

```sh
mkdir -p vendor
git clone git@github.com:eclipse-ankaios/ank-sdk-rust.git vendor/ankaios_sdk
```

Then add it to your `Cargo.toml`:

```toml
[dependencies]
ankaios_sdk = { path = "vendor/ankaios_sdk" }
```

## Compatibility

Please make sure the Rust SDK is compatible with the version of Ankaios you
are using. For information regarding versioning, please refer to this table:

| Ankaios    | Rust SDK |
| -------- | ------- |
| 0.4.x and below | No Rust SDK available. Please update Ankaios. |
| 0.5.x | 0.5.x     |

## Usage

After setup, you can use the Ankaios SDK to configure and run workloads
and request the state of the Ankaios system and the connected agents.

The following example assumes that the code is running in a managed by
Ankaios workload with configured control interface access:

```rust
use ankaios_sdk::{Ankaios, AnkaiosError, Workload, WorkloadStateEnum};
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    let mut ank = Ankaios::new().await.unwrap();

    // Create a new workload
    let workload = Workload::builder()
        .workload_name("dynamic_nginx")
        .agent_name("agent_A")
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config(
            "image: docker.io/library/nginx\ncommandOptions: [\"-p\", \"8080:80\"]"
        ).build().unwrap();
    
    // Run the workload
    let response = ank.apply_workload(workload, None).await.unwrap();

    // Get the WorkloadInstanceName to check later if the workload is running
    let workload_instance_name = response.added_workloads[0].clone();

    // Request the execution state based on the workload instance name
    match ank.get_execution_state_for_instance_name(workload_instance_name.clone(), None).await {
        Ok(state) => {
            let exec_state = state.execution_state;
            println!("State: {:?}, substate: {:?}, info: {:?}", exec_state.state, exec_state.substate, exec_state.additional_info);
        }
        Err(err) => {
            println!("Error while getting workload state: {:?}", err);
        }
    }

    // Wait until the workload reaches the running state
    match ank.wait_for_workload_to_reach_state(workload_instance_name, WorkloadStateEnum::Running, None).await {
        Ok(_) => {
            println!("Workload reached the RUNNING state.");
        }
        Err(err) => match err {
            AnkaiosError::TimeoutError(_) => {
                println!("Workload didn't reach the required state in time.");
            }
            _ => println!("Error while waiting for workload to reach state: {:?}", err),
        }
    }

    // Request the state of the system, filtered with the workloadStates
    let complete_state = ank.get_state(Some(vec!["workloadStates".to_owned()]), Some(Duration::from_secs(5))).await.unwrap();

    // Get the workload states present in the complete state
    let workload_states_dict = complete_state.get_workload_states().get_as_dict();

    // Print the states of the workloads
    for (agent_name, workload_states) in workload_states_dict.iter() {
        for (workload_name, workload_states) in workload_states.as_mapping().unwrap().iter() {
            for (_workload_id, workload_state) in workload_states.as_mapping().unwrap().iter() {
                println!("Workload {} on agent {} has the state {:?}", 
                    workload_name.as_str().unwrap(), agent_name.as_str().unwrap(), workload_state.get("state").unwrap().as_str().unwrap().to_string());
            }
        }
    }
}
```

For more details, please visit:
* [Ankaios documentation](https://eclipse-ankaios.github.io/ankaios/latest/)
* [Rust SDK documentation](https://docs.rs/ankaios-sdk/0.5.0-rc1)

## Contributing

This project welcomes contributions and suggestions. Before contributing, make sure to read the
[contribution guideline](CONTRIBUTING.md).

## License

Ankaios Rust SDK is licensed using the Apache License Version 2.0.