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

use ankaios_sdk::{Ankaios, LogResponse, LogsRequest, Workload};

use crate::common::grpc_config;

/// Registers for events on a workload, applies it, streams its logs until they stop, printing
/// any events received along the way, then unregisters/deletes everything.
pub async fn run(server_url: &str, agent_name: &str) {
    let mut ank = Ankaios::builder()
        .command_interface(grpc_config(server_url))
        .connect()
        .await
        .expect("Failed to initialize");

    // Workload that prints a handful of lines then exits.
    let workload = Workload::builder()
        .workload_name("count_to_five")
        .agent_name(agent_name)
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config("image: ghcr.io/eclipse-ankaios/tests/alpine:latest\ncommandOptions: [ \"--entrypoint\", \"/bin/sh\" ]\ncommandArgs: [ \"-c\", \"echo -e '1\\n2\\n3\\n4\\n5';\" ]")
        .build()
        .expect("Failed to build workload");

    // Subscribe to changes on this workload's desired state before applying it.
    let mut events_campaign_response = ank
        .register_event(vec!["desiredState.workloads.count_to_five".to_owned()])
        .await
        .expect("Failed to register events");

    let response = ank
        .apply_workload(workload)
        .await
        .expect("Failed to apply workload");
    let workload_instance_name = response.added_workloads[0].clone();

    let logs_request = LogsRequest {
        workload_names: vec![workload_instance_name.clone()],
        ..Default::default()
    };

    let mut log_campaign_response = ank
        .request_logs(logs_request)
        .await
        .expect("Failed to request logs");

    if !log_campaign_response
        .accepted_workload_names
        .contains(&workload_instance_name)
    {
        println!("Workload '{workload_instance_name}' not accepted for log retrieval");
    }

    // Stream logs and events side by side until the logs run out.
    loop {
        tokio::select! {
            log_response = log_campaign_response.logs_receiver.recv() => {
                match log_response {
                    Some(LogResponse::LogEntries(log_entries)) => {
                        for entry in log_entries {
                            println!("[log] {}", entry.message);
                        }
                    }
                    Some(LogResponse::LogsStopResponse(name)) => {
                        println!("No more logs available for '{name}'.");
                        break;
                    }
                    None => break,
                }
            }
            Some(event_entry) = events_campaign_response.events_receiver.recv() => {
                println!(
                    "[event] added={:?} updated={:?} removed={:?}",
                    event_entry.added_fields, event_entry.updated_fields, event_entry.removed_fields
                );
            }
        }
    }

    ank.stop_receiving_logs(log_campaign_response)
        .await
        .expect("Failed to stop receiving logs");
    ank.unregister_event(events_campaign_response)
        .await
        .expect("Failed to unregister events");

    ank.delete_workload(workload_instance_name.workload_name)
        .await
        .expect("Failed to delete workload");
}
