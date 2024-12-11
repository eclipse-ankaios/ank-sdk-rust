// Copyright (c) 2024 Elektrobit Automotive GmbH
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

mod workload_state;
mod workload_state_enums;
mod workload_execution_state;
mod workload_instance_name;

pub use workload_state::{WorkloadState, WorkloadStateCollection};
pub use workload_state_enums::{WorkloadStateEnum, WorkloadSubStateEnum};
pub use workload_execution_state::WorkloadExecutionState;
pub use workload_instance_name::WorkloadInstanceName;

#[cfg(test)]
pub use workload_state::generate_test_workload_states_proto;