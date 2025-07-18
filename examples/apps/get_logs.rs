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

use ankaios_sdk::{Ankaios, AnkaiosError, LogResponse, Workload, WorkloadStateEnum};
use tokio::time::Duration;

#[tokio::main]
async fn main() {
    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    let mut ank = Ankaios::new().await.expect("Failed to initialize");

    // Create a new workload outputting test logs.
    let workload = Workload::builder()
        .workload_name("count_to_five")
        .agent_name("agent_Rust_SDK")
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config("image: ghcr.io/eclipse-ankaios/tests/alpine:latest\ncommandOptions: [ \"--entrypoint\", \"/bin/sh\" ]\ncommandArgs: [ \"-c\", \"echo -e \'1\\n2\\n3\\n4\\n5\'\" ]")
        .build()
        .expect("Failed to build workload");

    // Run the workload
    let response = ank
        .apply_workload(workload)
        .await
        .expect("Failed to apply workload");

    // Get the WorkloadInstanceName to check later if the workload is running
    let workload_instance_name = response.added_workloads[0].clone();

    // Request the logs from the new workload
    let mut log_campaign_response = ank
        .request_logs(vec![workload_instance_name.clone()], false, -1, None, None)
        .await
        .expect("Failed to request logs");

    // Check if the workload was accepted for log retrieval
    if !log_campaign_response
        .accepted_workload_names
        .contains(&workload_instance_name)
    {
        println!(
            "Workload '{}' not accepted for log retrieval",
            workload_instance_name
        );

        std::process::exit(1);
    }

    // Listen for log entries until stop
    while let Some(log_response) = log_campaign_response.logs_receiver.recv().await {
        match log_response {
            LogResponse::LogEntries(log_entries) => {
                for entry in log_entries {
                    println!("{}", entry.message);
                }
            }
            LogResponse::LogsStopResponse(workload_name) => {
                println!(
                    "No more logs available for workload '{}'. Stopping log retrieval.",
                    workload_name
                );
                break;
            }
        }
    }

    // Stop receiving logs for the workload
    ank.stop_receiving_logs(log_campaign_response)
        .await
        .expect("Failed to stop receiving logs");
    println!(
        "Stopped log retrieval for workload '{}'.",
        workload_instance_name
    );
}
