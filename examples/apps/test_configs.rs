// Copyright (c) 2025 Elektrobit Automotive GmbH
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
//
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, thread::sleep};

use ankaios_sdk::{Ankaios, Workload};
use tokio::time::Duration;

async fn print_workload_states(ank: &mut Ankaios) {
    if let Ok(complete_state) = ank.get_state(vec!["workloadStates".to_owned()]).await {
        // Get the workload states present in the complete state
        let workload_states = Vec::from(complete_state.get_workload_states());

        // Print the states of the workloads
        for workload_state in workload_states {
            println!(
                "Workload {} on agent {} has the state {:?}",
                workload_state.workload_instance_name.workload_name,
                workload_state.workload_instance_name.agent_name,
                workload_state.execution_state.state
            );
        }
    }
}

#[tokio::main]
async fn main() {
    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    let mut ank = Ankaios::new().await.expect("Failed to initialize");

    // Create a new workload
    let workload = Workload::builder()
        .workload_name("dynamic_nginx")
        .agent_name("agent_Rust_SDK")
        .runtime("podman")
        .restart_policy("NEVER")
        .add_config("conf", "configuration")
        .runtime_config(
            "image: docker.io/library/nginx\ncommandOptions: [\"-p\", \"8080:{{conf.port}}\"]",
        )
        .build()
        .expect("Failed to build workload");

    // Create configuration
    let mut port_map = serde_yaml::Mapping::new();
    port_map.insert(
        serde_yaml::Value::String("port".to_owned()),
        serde_yaml::Value::String("80".to_owned()),
    );
    let mut configs = HashMap::from([(
        "configuration".to_owned(),
        serde_yaml::Value::Mapping(port_map),
    )]);

    // Send configuration to Ankaios
    ank.update_configs(configs.clone())
        .await
        .expect("Failed to update configs");

    // Send workload to Ankaios
    ank.apply_workload(workload)
        .await
        .expect("Failed to apply workload");

    // Get workloads
    sleep(Duration::from_secs(5));
    print_workload_states(&mut ank).await;

    // Modify config
    configs
        .get_mut("configuration")
        .unwrap()
        .as_mapping_mut()
        .unwrap()
        .insert(
            serde_yaml::Value::String("port".to_owned()),
            serde_yaml::Value::String("81".to_owned()),
        );

    // Send updated configuration to Ankaios
    ank.update_configs(configs)
        .await
        .expect("Failed to update configs");

    // Get workloads
    sleep(Duration::from_secs(5));
    print_workload_states(&mut ank).await;

    // Delete workload
    ank.delete_workload("dynamic_nginx".to_owned())
        .await
        .expect("Failed to delete workload");

    // Get workloads
    sleep(Duration::from_secs(5));
    print_workload_states(&mut ank).await;
}
