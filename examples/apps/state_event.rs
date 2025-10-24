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

use ankaios_sdk::{Ankaios, Workload};

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
        .runtime_config("image: ghcr.io/eclipse-ankaios/tests/alpine:latest\ncommandOptions: [ \"--entrypoint\", \"/bin/sh\" ]\ncommandArgs: [ \"-c\", \"echo -e \'1\\n2\\n3\\n4\\n5\';\" ]")
        .build()
        .expect("Failed to build workload");

    // Run the workload
    let response = ank
        .apply_workload(workload)
        .await
        .expect("Failed to apply workload");

    // Get the WorkloadInstanceName to confirm the workload was added
    let _workload_instance_name = response.added_workloads[0].clone();

    // Subscribe to changes of workload count_to_five
    let mut events_campaign_response = ank
        .register_event(vec!["desiredState.workloads.count_to_five".to_owned()])
        .await
        .expect("Failed to register events");

    // Listen for events
    while let Some(event_entry) = events_campaign_response.events_receiver.recv().await {
        println!("Received event:");
        if !event_entry.added_fields.is_empty() {
            println!("Added fields: {:?}", event_entry.added_fields);
        }
        if !event_entry.updated_fields.is_empty() {
            println!("Updated fields: {:?}", event_entry.updated_fields);
        }
        if !event_entry.removed_fields.is_empty() {
            println!("Removed fields: {:?}", event_entry.removed_fields);
        }
        println!("Current complete state: {}", event_entry.complete_state);
    }

    // Unregister events
    ank.unregister_event(events_campaign_response)
        .await
        .expect("Failed to unregister events");
}
