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

use ankaios_sdk::{Ankaios, GrpcConfig};

/// Selects Unix domain sockets for `unix://` URLs and plaintext TCP otherwise.
/// HTTPS requires configuring the TCP builder with mTLS certificate material instead.
pub fn grpc_config(server_url: &str) -> GrpcConfig {
    #[cfg(unix)]
    if server_url.starts_with("unix://") {
        return GrpcConfig::unix_socket().url(server_url).into();
    }

    GrpcConfig::tcp().url(server_url).into()
}

pub async fn print_workload_states(ank: &mut Ankaios) {
    if let Ok(complete_state) = ank.get_state(vec!["workloadStates".to_owned()]).await {
        let workload_states = Vec::from(complete_state.get_workload_states());
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
