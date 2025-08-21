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

//! This module contains structs and enums that are used to monitor the
//! state of workloads in the [Ankaios] application.
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios
//!
//! # Example
//!
//! ## Get all workload states:
//!
//! ```rust
//! use ankaios_sdk::WorkloadStateCollection;
//!
//! let workload_state_collection = WorkloadStateCollection::new();
//! let workload_states_list = workload_state_collection.as_list();
//! # let workload_state_collection = WorkloadStateCollection::new();
//! let workload_states_map = workload_state_collection.as_dict();
//! ```
//!
//! ## Unpack a workload state
//!
//! ```rust
//! # use ankaios_sdk::WorkloadState;
//! #
//! let workload_state: WorkloadState;
//! # let workload_state = WorkloadState::default();
//! let agent_name = workload_state.workload_instance_name.agent_name;
//! let workload_name = workload_state.workload_instance_name.workload_name;
//! let workload_id = workload_state.workload_instance_name.workload_id;
//! let state = workload_state.execution_state.state;
//! let substate = workload_state.execution_state.substate;
//! let additional_info = workload_state.execution_state.additional_info;
//! ```
//!
//! ## Get the workload instance name as a dictionary:
//!
//! ```rust
//! # use ankaios_sdk::WorkloadInstanceName;
//! #
//! let workload_instance_name: WorkloadInstanceName;
//! # let workload_instance_name = WorkloadInstanceName::default();
//! let instance_name_dict = workload_instance_name.to_dict();
//! ```

mod workload_execution_state;
mod workload_instance_name;
mod workload_state;
mod workload_state_enums;

#[allow(unused)]
pub use workload_execution_state::WorkloadExecutionState;
pub use workload_instance_name::WorkloadInstanceName;
pub use workload_state::{WorkloadState, WorkloadStateCollection};
#[allow(unused)]
pub use workload_state_enums::{WorkloadStateEnum, WorkloadSubStateEnum};

#[cfg(test)]
pub use workload_state::generate_test_workload_states_proto;

#[cfg(test)]
mod tests {
    use super::workload_state::generate_test_workload_state;
    use super::{WorkloadState, WorkloadStateCollection};

    #[allow(
        clippy::no_effect_underscore_binding,
        clippy::redundant_type_annotations
    )]
    #[test]
    fn test_doc_examples() {
        // Get all workload states
        let workload_state_collection = WorkloadStateCollection::new();
        let _workload_states_list = workload_state_collection.clone().as_list();
        let _workload_states_map = workload_state_collection.as_dict();

        // Unpack a workload state
        let workload_state: WorkloadState = generate_test_workload_state();
        let _agent_name = workload_state.workload_instance_name.agent_name;
        let _workload_name = workload_state.workload_instance_name.workload_name;
        let _workload_id = workload_state.workload_instance_name.workload_id;
        let _state = workload_state.execution_state.state;
        let _substate = workload_state.execution_state.substate;
        let _additional_info = workload_state.execution_state.additional_info;

        // Get the workload instance name as a dictionary
        let workload_instance_name = generate_test_workload_state().workload_instance_name;
        let _instance_name_dict = workload_instance_name.to_dict();
    }
}
