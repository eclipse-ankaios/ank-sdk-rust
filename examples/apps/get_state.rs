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

use ankaios_sdk::{Ankaios, AnkaiosError, Workload, WorkloadStateEnum};
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
    env_logger::builder().filter_level(log::LevelFilter::Debug).init();

    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    let mut ank = Ankaios::new().await.expect("Failed to initialize");

    // Create a new workload
    let workload = Workload::builder()
        .workload_name("dynamic_nginx")
        .agent_name("agent_Rust_SDK")
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config(
            "image: docker.io/library/nginx\ncommandOptions: [\"-p\", \"8080:80\"]"
        ).build().unwrap();
    
    // Run the workload
    let response = ank.apply_workload(workload).await.expect("Failed to apply workload");

    // Get the WorkloadInstanceName to check later if the workload is running
    let workload_instance_name = response.added_workloads[0].clone();

    // Wait until the workload reaches the running state
    match ank.wait_for_workload_to_reach_state(workload_instance_name.clone(), WorkloadStateEnum::Running).await {
        Ok(_) => {
            println!("Workload reached the RUNNING state.");
        }
        Err(AnkaiosError::TimeoutError(_)) => {
            println!("Workload didn't reach the required state in time.");
        }
        Err(err) => {
            println!("Error while waiting for workload to reach state: {err:?}");
        }
    }

    loop {
        // Get the state of the workloads
        print_workload_states(&mut ank).await;

        // Wait for 5 seconds
        sleep(Duration::from_secs(5));
    }
}
