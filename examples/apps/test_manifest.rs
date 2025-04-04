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

use std::thread::sleep;

use ankaios_sdk::{Ankaios, Manifest};
use tokio::time::Duration;

async fn print_workload_states(ank: &mut Ankaios) {
    if let Ok(complete_state) = ank.get_state(vec!["workloadStates".to_owned()]).await {
        // Get the workload states present in the complete state
        let workload_states = Vec::from(complete_state.get_workload_states());

        // Print the states of the workloads
        for workload_state in workload_states {
            println!("Workload {} on agent {} has the state {:?}", 
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

    // Create manifest
    let manifest_str = r#"apiVersion: v0.1
workloads:
    dynamic_nginx:
        runtime: podman
        restartPolicy: NEVER
        agent: "{{agent.name}}"
        configs:
            agent: agent
        runtimeConfig: |
            image: image/test
            commandOptions: ["-p", "8080:80"]
configs:
    agent:
        name: agent_Rust_SDK"#;
    let manifest = Manifest::from_string(manifest_str).expect("Failed to parse manifest");

    match ank.apply_manifest(manifest.clone()).await {
        Ok(result) => {
            println!("Manifest applied successfully: {result:?}");
        }
        Err(err) => {
            println!("Error while applying manifest: {err:?}");
        }
    }

    sleep(Duration::from_secs(5));
    print_workload_states(&mut ank).await;
    sleep(Duration::from_secs(5));

    match ank.delete_manifest(manifest).await {
        Ok(result) => {
            println!("Manifest deleted successfully: {result:?}");
        }
        Err(err) => {
            println!("Error while deleting manifest: {err:?}");
        }
    }

    sleep(Duration::from_secs(5));
    print_workload_states(&mut ank).await;
}
