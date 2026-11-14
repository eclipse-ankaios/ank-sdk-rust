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

use ankaios_sdk::Ankaios;
use tokio::time::{Duration, sleep};

use crate::common::grpc_config;

/// Connects, disconnects (via `Drop`) and reconnects to the same server several times in a row,
/// to check that command interface teardown doesn't leave the
/// server or the next connection attempt in a bad state.
pub async fn run(server_url: &str, _agent_name: &str) {
    for i in 1..=3 {
        println!("Ankaios connection #{i}");
        let mut ank = Ankaios::builder()
            .command_interface(grpc_config(server_url))
            .connect()
            .await
            .expect("Failed to initialize");

        let complete_state = ank
            .get_state(vec!["workloadStates".to_owned()])
            .await
            .expect("Failed to get state");
        println!(
            "Connection #{i} got {} workload state(s).",
            Vec::from(complete_state.get_workload_states()).len()
        );

        sleep(Duration::from_secs(2)).await;
        // `ank` is dropped here, closing the command interface connection.
    }
    println!("All connections opened and closed successfully.");
}
