// Copyright (c) 2026 Elektrobit Automotive GmbH
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

use ankaios_sdk::{Ankaios, AnkaiosError, Workload, WorkloadStateEnum};
use tokio::time::Duration;

use crate::common::{grpc_config, print_workload_states};

/// Applies a workload, updates it, deletes it, printing the workload states after every change.
pub async fn run(server_url: &str, agent_name: &str) {
    let mut ank = Ankaios::builder()
        .command_interface(grpc_config(server_url))
        .connect()
        .await
        .expect("Failed to initialize");

    // Create a new workload
    let workload = Workload::builder()
        .workload_name("dynamic_nginx")
        .agent_name(agent_name)
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config("image: docker.io/library/nginx\ncommandOptions: [\"-p\", \"8080:80\"]")
        .build()
        .expect("Failed to build workload");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Run the workload
    let response = ank
        .apply_workload(workload)
        .await
        .expect("Failed to apply workload");
    let workload_instance_name = response.added_workloads[0].clone();

    match ank
        .wait_for_workload_to_reach_state(
            workload_instance_name.clone(),
            WorkloadStateEnum::Running,
        )
        .await
    {
        Ok(()) => println!("Workload reached the RUNNING state."),
        Err(AnkaiosError::TimeoutError(_)) => {
            println!("Workload didn't reach the required state in time.");
        }
        Err(err) => println!("Error while waiting for workload to reach state: {err:?}"),
    }
    print_workload_states(&mut ank).await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Update the workload
    let workloads = ank
        .get_workload(workload_instance_name.clone().workload_name)
        .await
        .expect("Failed to get workload");
    let mut workload = workloads[0].clone();
    workload
        .update_restart_policy("ALWAYS")
        .expect("Failed to update restart policy");

    ank.apply_workload(workload)
        .await
        .expect("Failed to update workload");

    match ank
        .wait_for_workload_to_reach_state(
            workload_instance_name.clone(),
            WorkloadStateEnum::Running,
        )
        .await
    {
        Ok(()) => println!("Workload reached the RUNNING state after update."),
        Err(AnkaiosError::TimeoutError(_)) => {
            println!("Workload didn't reach the required state in time.");
        }
        Err(err) => println!("Error while waiting for workload to reach state: {err:?}"),
    }
    print_workload_states(&mut ank).await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Delete the workload
    ank.delete_workload(workload_instance_name.workload_name)
        .await
        .expect("Failed to delete workload");
    println!("Workload deleted, closing in 5 seconds...");

    tokio::time::sleep(Duration::from_secs(5)).await;
    print_workload_states(&mut ank).await;
}
