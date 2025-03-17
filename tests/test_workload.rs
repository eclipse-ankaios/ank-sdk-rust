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

use ankaios_sdk::{Ankaios, AnkaiosError, Workload, WorkloadStateEnum};
use tokio::time::Duration;

async fn get_workloads(ank: &mut Ankaios) {
    // Request the state of the system, filtered with the workloadStates
    let complete_state = ank.get_state(Some(vec!["workloadStates".to_owned()]), Some(Duration::from_secs(5))).await.unwrap();

    // Get the workload states present in the complete state
    let workload_states_dict = complete_state.get_workload_states().get_as_dict();

    // Print the states of the workloads
    for (agent_name, workload_states) in workload_states_dict {
        for (workload_name, workload_states) in workload_states.as_mapping().unwrap() {
            for (_workload_id, workload_state) in workload_states.as_mapping().unwrap() {
                println!("Workload {} on agent {} has the state {:?}", 
                    workload_name.as_str().unwrap(), agent_name.as_str().unwrap(), workload_state.get("state").unwrap().as_str().unwrap().to_string());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    let mut ank = Ankaios::new().await.unwrap();

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
    let response = ank.apply_workload(workload, None).await.unwrap();

    // Get the WorkloadInstanceName to check later if the workload is running
    let workload_instance_name = response.added_workloads[0].clone();

    // Request the execution state based on the workload instance name
    match ank.get_execution_state_for_instance_name(workload_instance_name.clone(), None).await {
        Ok(exec_state) => {
            println!("State: {:?}, substate: {:?}, info: {:?}", exec_state.state, exec_state.substate, exec_state.additional_info);
        }
        Err(err) => {
            println!("Error while getting workload state: {err:?}"); // ##########
        }
    }

    // Wait until the workload reaches the running state
    match ank.wait_for_workload_to_reach_state(workload_instance_name.clone(), WorkloadStateEnum::Running, None).await {
        Ok(()) => {
            println!("Workload reached the RUNNING state.");
        }
        Err(err) => match err {
            AnkaiosError::TimeoutError(_) => {
                println!("Workload didn't reach the required state in time.");
            }
            _ => println!("Error while waiting for workload to reach state: {err:?}"),
        }
    }

    // Get the workload
    let mut workload = ank.get_workload(workload_instance_name.clone().workload_name, None).await.unwrap();

    // Modify workload
    workload.update_restart_policy("ALWAYS").unwrap();

    // Update workload
    match ank.apply_workload(workload.clone(), None).await {
        Ok(response) => {
            println!("Workload updated: {response:?}");
        }
        Err(err) => {
            println!("Error while updating workload: {err:?}");
        }
    }

    // Wait until the workload reaches the running state
    match ank.wait_for_workload_to_reach_state(workload_instance_name.clone(), WorkloadStateEnum::Running, None).await {
        Ok(()) => {
            println!("Workload reached the RUNNING state.");
        }
        Err(err) => match err {
            AnkaiosError::TimeoutError(_) => {
                println!("Workload didn't reach the required state in time.");
            }
            _ => println!("Error while waiting for workload to reach state: {err:?}"),
        }
    }

    // Delete workload
    match ank.delete_workload(workload_instance_name.workload_name, None).await {
        Ok(response) => {
            println!("Workload deleted: {response:?}");
        }
        Err(err) => {
            println!("Error while deleting workload: {err:?}");
        }
    }

    // Wait for the workload to stop
    sleep(Duration::from_secs(5));
    get_workloads(&mut ank).await;
}
