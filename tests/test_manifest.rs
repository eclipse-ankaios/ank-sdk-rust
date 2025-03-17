// Copyright (c) 2023 Elektrobit Automotive GmbH
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

async fn get_workloads(ank: &mut Ankaios) {
    // Request the state of the system, filtered with the workloadStates
    let complete_state = ank.get_state(Some(vec!["workloadStates".to_owned()]), Some(Duration::from_secs(5))).await.unwrap();

    // Get the workload states present in the complete state
    let workload_states_dict = complete_state.get_workload_states().get_as_list();

    // Print the states of the workloads
    for workload_state in workload_states_dict {
        println!("Workload {} on agent {} has the state {:?}", 
            workload_state.workload_instance_name.workload_name, 
            workload_state.workload_instance_name.agent_name,
            workload_state.execution_state.state
        ); 
    }
}

#[tokio::main]
async fn main() {
    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    let mut ank = Ankaios::new().await.unwrap();

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
    let manifest = Manifest::from_string(manifest_str).unwrap();

    match ank.apply_manifest(manifest.clone(), None).await {
        Ok(result) => {
            println!("Manifest applied successfully: {result:?}");
        }
        Err(err) => {
            println!("Error while applying manifest: {err:?}");
        }
    }

    sleep(Duration::from_secs(5));
    get_workloads(&mut ank).await;
    sleep(Duration::from_secs(5));

    match ank.delete_manifest(manifest, None).await {
        Ok(result) => {
            println!("Manifest deleted successfully: {result:?}");
        }
        Err(err) => {
            println!("Error while deleting manifest: {err:?}");
        }
    }

    sleep(Duration::from_secs(5));
    get_workloads(&mut ank).await;
}
