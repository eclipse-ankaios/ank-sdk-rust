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

use serde_yaml::Value;
use std::collections::HashMap;
use std::fmt;

use super::workload_execution_state::WorkloadExecutionState;
use super::workload_instance_name::WorkloadInstanceName;
use crate::ankaios_api;
use ankaios_api::ank_base;

/// A [`HashMap`] where the key represents the workload id and the value is of type [`WorkloadExecutionState`].
type ExecutionsStatesForId = HashMap<String, WorkloadExecutionState>;
/// A [`HashMap`] where the key represents the workload name and the value is of type [`ExecutionsStatesForId`].
type ExecutionsStatesOfWorkload = HashMap<String, ExecutionsStatesForId>;
/// A [`HashMap`] where the key represents the agent name and the value is of type [`ExecutionsStatesOfWorkload`].
type WorkloadStatesMap = HashMap<String, ExecutionsStatesOfWorkload>;

/// Struct that contains the instance name and
/// the execution state of the workload.
#[derive(Debug, Default, Clone)]
pub struct WorkloadState {
    /// The execution state of the workload.
    pub execution_state: WorkloadExecutionState,
    /// The instance name of the workload.
    pub workload_instance_name: WorkloadInstanceName,
}

/// Helper struct that specializes in managing a collection of [`WorkloadStates`](WorkloadState).
#[derive(Debug, Default, Clone)]
pub struct WorkloadStateCollection {
    /// The collection of [`WorkloadStates`](WorkloadState).
    workload_states: WorkloadStatesMap,
}

impl WorkloadState {
    #[doc(hidden)]
    /// Creates a new `WorkloadState` from an [ExecutionState](ank_base::ExecutionState).
    ///
    /// ## Arguments
    ///
    /// * `agent_name` - The name of the agent;
    /// * `workload_name` - The name of the workload;
    /// * `workload_id` - The id of the workload;
    /// * `state` - The [ExecutionState](ank_base::ExecutionState) to create the [`WorkloadState`] from.
    ///
    /// ## Returns
    ///
    /// A new [`WorkloadState`] instance.
    pub(crate) fn new_from_ank_base(
        agent_name: String,
        workload_name: String,
        workload_id: String,
        state: ank_base::ExecutionState,
    ) -> WorkloadState {
        WorkloadState {
            execution_state: WorkloadExecutionState::new(state),
            workload_instance_name: WorkloadInstanceName::new(
                agent_name,
                workload_name,
                workload_id,
            ),
        }
    }

    /// Creates a new `WorkloadState` from a [`WorkloadExecutionState`] instance.
    ///
    /// ## Arguments
    ///
    /// * `agent_name` - The name of the agent;
    /// * `workload_name` - The name of the workload;
    /// * `workload_id` - The id of the workload;
    /// * `exec_state` - The [`WorkloadExecutionState`] to create the [`WorkloadState`] from.
    ///
    /// ## Returns
    ///
    /// A new [`WorkloadState`] instance.
    #[must_use]
    pub fn new_from_exec_state(
        agent_name: String,
        workload_name: String,
        workload_id: String,
        exec_state: WorkloadExecutionState,
    ) -> WorkloadState {
        WorkloadState {
            execution_state: exec_state,
            workload_instance_name: WorkloadInstanceName::new(
                agent_name,
                workload_name,
                workload_id,
            ),
        }
    }
}

impl fmt::Display for WorkloadState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {}",
            self.workload_instance_name, self.execution_state
        )
    }
}

impl WorkloadStateCollection {
    /// Creates a new `WorkloadStateCollection` instance.
    ///
    /// ## Returns
    ///
    /// A new [`WorkloadStateCollection`] instance.
    #[must_use]
    pub fn new() -> WorkloadStateCollection {
        WorkloadStateCollection {
            workload_states: HashMap::new(),
        }
    }

    #[doc(hidden)]
    /// Creates a new `WorkloadStateCollection` from a [WorkloadStatesMap](ank_base::WorkloadStatesMap).
    ///
    /// ## Arguments
    ///
    /// * `workload_states_map` - The [WorkloadStatesMap](ank_base::WorkloadStatesMap) to create the [`WorkloadStateCollection`] from.
    ///
    /// ## Returns
    ///
    /// A new [`WorkloadStateCollection`] instance.
    pub(crate) fn new_from_proto(
        workload_states_map: &ank_base::WorkloadStatesMap,
    ) -> WorkloadStateCollection {
        let mut workload_states = WorkloadStateCollection::new();
        for (agent_name, workloads) in &workload_states_map.agent_state_map {
            for (workload_name, workload_states_for_id) in &workloads.wl_name_state_map {
                for (workload_id, state) in &workload_states_for_id.id_state_map {
                    let workload_state = WorkloadState::new_from_ank_base(
                        agent_name.clone(),
                        workload_name.clone(),
                        workload_id.clone(),
                        state.clone(),
                    );
                    workload_states.add_workload_state(workload_state);
                }
            }
        }
        workload_states
    }

    #[doc(hidden)]
    /// Adds a [`WorkloadState`] to the collection.
    ///
    /// ## Arguments
    ///
    /// * `workload_state` - The [`WorkloadState`] to add to the collection.
    pub(crate) fn add_workload_state(&mut self, workload_state: WorkloadState) {
        let agent_name = workload_state.workload_instance_name.agent_name.clone();
        let workload_name = workload_state.workload_instance_name.workload_name.clone();
        let workload_id = workload_state.workload_instance_name.workload_id.clone();

        self.workload_states
            .entry(agent_name.clone())
            .or_default()
            .entry(workload_name.clone())
            .or_default()
            .insert(workload_id.clone(), workload_state.execution_state);
    }

    /// Converts the `WorkloadStateCollection` to a [`WorkloadStatesMap`].
    ///
    /// ## Returns
    ///
    /// A [`WorkloadStatesMap`] containing the [`WorkloadStateCollection`] information.
    #[must_use]
    pub fn as_dict(self) -> WorkloadStatesMap {
        WorkloadStatesMap::from(self)
    }

    /// Converts the `WorkloadStateCollection` to a [Mapping](serde_yaml::Mapping).
    ///
    /// ## Returns
    ///
    /// A [Mapping](serde_yaml::Mapping) containing the [`WorkloadStateCollection`] information.
    #[must_use]
    pub fn as_mapping(self) -> serde_yaml::Mapping {
        serde_yaml::Mapping::from(self)
    }

    /// Converts the `WorkloadStateCollection` to a [Vec] of [`WorkloadState`].
    ///
    /// ## Returns
    ///
    /// A [Vec] of [`WorkloadStates`](WorkloadState) containing the [`WorkloadStateCollection`] information.
    #[must_use]
    pub fn as_list(self) -> Vec<WorkloadState> {
        Vec::from(self)
    }

    /// Returns the [`WorkloadState`] for a given [`WorkloadInstanceName`].
    ///
    /// ## Arguments
    ///
    /// * `instance_name` - The [`WorkloadInstanceName`] to get the [`WorkloadState`] for.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadState`] for the given [`WorkloadInstanceName`].
    /// If the [`WorkloadState`] is not found, `None` is returned.
    #[must_use]
    pub fn get_for_instance_name(
        &self,
        instance_name: &WorkloadInstanceName,
    ) -> Option<&WorkloadExecutionState> {
        self.workload_states
            .get(&instance_name.agent_name)
            .and_then(|workloads| workloads.get(&instance_name.workload_name))
            .and_then(|workload| workload.get(&instance_name.workload_id))
    }
}

impl From<ank_base::WorkloadStatesMap> for WorkloadStateCollection {
    fn from(proto: ank_base::WorkloadStatesMap) -> Self {
        Self::new_from_proto(&proto)
    }
}

impl From<WorkloadStateCollection> for WorkloadStatesMap {
    fn from(collection: WorkloadStateCollection) -> Self {
        collection.workload_states
    }
}

impl From<WorkloadStateCollection> for serde_yaml::Mapping {
    fn from(collection: WorkloadStateCollection) -> Self {
        let mut map = serde_yaml::Mapping::new();
        for (agent_name, workload_states) in &collection.workload_states {
            let mut agent_map = serde_yaml::Mapping::new();
            for (workload_name, workload_states_for_id) in workload_states {
                let mut workload_map = serde_yaml::Mapping::new();
                for (workload_id, workload_state) in workload_states_for_id {
                    workload_map.insert(
                        Value::String(workload_id.clone()),
                        Value::Mapping(workload_state.to_dict()),
                    );
                }
                agent_map.insert(
                    Value::String(workload_name.clone()),
                    Value::Mapping(workload_map),
                );
            }
            map.insert(Value::String(agent_name.clone()), Value::Mapping(agent_map));
        }
        map
    }
}

impl From<WorkloadStateCollection> for Vec<WorkloadState> {
    fn from(collection: WorkloadStateCollection) -> Self {
        let mut list = Vec::new();
        for (agent_name, workload_states_for_agent) in &collection.workload_states {
            for (workload_name, workload_states_for_id) in workload_states_for_agent {
                for (workload_id, workload_state) in workload_states_for_id {
                    let workload_instance_name = WorkloadInstanceName::new(
                        agent_name.clone(),
                        workload_name.clone(),
                        workload_id.clone(),
                    );
                    list.push(WorkloadState {
                        execution_state: workload_state.clone(),
                        workload_instance_name,
                    });
                }
            }
        }
        list
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
pub fn generate_test_workload_states_proto() -> ank_base::WorkloadStatesMap {
    ank_base::WorkloadStatesMap {
        agent_state_map: HashMap::from([
            (
                "agent_A".to_owned(),
                ank_base::ExecutionsStatesOfWorkload {
                    wl_name_state_map: HashMap::from([(
                        "nginx".to_owned(),
                        ank_base::ExecutionsStatesForId {
                            id_state_map: HashMap::from([(
                                "1234".to_owned(),
                                ank_base::ExecutionState {
                                    execution_state_enum: Some(
                                        ank_base::execution_state::ExecutionStateEnum::Succeeded(
                                            ank_base::Succeeded::Ok as i32,
                                        ),
                                    ),
                                    additional_info: "Random info".to_owned(),
                                },
                            )]),
                        },
                    )]),
                },
            ),
            (
                "agent_B".to_owned(),
                ank_base::ExecutionsStatesOfWorkload {
                    wl_name_state_map: HashMap::from([
                        (
                            "nginx".to_owned(),
                            ank_base::ExecutionsStatesForId {
                                id_state_map: HashMap::from([(
                                    "5678".to_owned(),
                                    ank_base::ExecutionState {
                                        execution_state_enum: Some(
                                            ank_base::execution_state::ExecutionStateEnum::Pending(
                                                ank_base::Pending::WaitingToStart as i32,
                                            ),
                                        ),
                                        additional_info: "Random info".to_owned(),
                                    },
                                )]),
                            },
                        ),
                        (
                            "dyn_nginx".to_owned(),
                            ank_base::ExecutionsStatesForId {
                                id_state_map: HashMap::from([(
                                    "9012".to_owned(),
                                    ank_base::ExecutionState {
                                        execution_state_enum: Some(
                                            ank_base::execution_state::ExecutionStateEnum::Stopping(
                                                ank_base::Stopping::WaitingToStop as i32,
                                            ),
                                        ),
                                        additional_info: "Random info".to_owned(),
                                    },
                                )]),
                            },
                        ),
                    ]),
                },
            ),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use crate::components::workload_state_mod::{WorkloadStateEnum, WorkloadSubStateEnum};

    use super::generate_test_workload_states_proto;
    use super::{
        WorkloadExecutionState, WorkloadInstanceName, WorkloadState, WorkloadStateCollection,
        ank_base,
    };

    #[test]
    fn utest_workload_state() {
        let agent_name = "agent_name".to_owned();
        let workload_name = "workload_name".to_owned();
        let workload_id = "workload_id".to_owned();
        let state = ank_base::ExecutionState {
            execution_state_enum: Some(ank_base::execution_state::ExecutionStateEnum::Pending(
                ank_base::Pending::WaitingToStart as i32,
            )),
            additional_info: "additional_info".to_owned(),
        };
        let exec_state = WorkloadExecutionState::new(state.clone());

        let workload_state_ank_base = WorkloadState::new_from_ank_base(
            agent_name.clone(),
            workload_name.clone(),
            workload_id.clone(),
            state.clone(),
        );
        let workload_state_exec_state = WorkloadState::new_from_exec_state(
            agent_name.clone(),
            workload_name.clone(),
            workload_id.clone(),
            exec_state.clone(),
        );

        assert_eq!(
            workload_state_ank_base.to_string(),
            workload_state_exec_state.to_string()
        );
        assert_eq!(
            workload_state_ank_base.execution_state.state,
            WorkloadStateEnum::Pending
        );
        assert_eq!(
            workload_state_ank_base.execution_state.substate,
            WorkloadSubStateEnum::PendingWaitingToStart
        );
        assert_eq!(
            workload_state_ank_base.execution_state.additional_info,
            "additional_info"
        );
        assert_eq!(
            workload_state_ank_base.workload_instance_name.agent_name,
            agent_name
        );
        assert_eq!(
            workload_state_ank_base.workload_instance_name.workload_name,
            workload_name
        );
        assert_eq!(
            workload_state_ank_base.workload_instance_name.workload_id,
            workload_id
        );
    }

    #[test]
    fn utest_workload_state_collection() {
        let state_collection = WorkloadStateCollection::from(generate_test_workload_states_proto());
        let mut state_list = state_collection.clone().as_list();
        // The list comes unsorted, thus the test is not deterministic
        state_list.sort_by(|a, b| {
            a.workload_instance_name
                .agent_name
                .cmp(&b.workload_instance_name.agent_name)
        });
        assert_eq!(state_list.len(), 3);
        assert_eq!(state_list[0].workload_instance_name.agent_name, "agent_A");
        assert_eq!(state_list[0].workload_instance_name.workload_name, "nginx");
        assert_eq!(state_list[0].workload_instance_name.workload_id, "1234");
        assert_eq!(state_list[1].workload_instance_name.agent_name, "agent_B");
        assert_eq!(state_list[2].workload_instance_name.agent_name, "agent_B");

        let state_dict = state_collection.clone().as_dict();
        assert_eq!(state_dict.len(), 2);
        assert_eq!(state_dict.get("agent_A").unwrap().len(), 1);
        assert_eq!(state_dict.get("agent_B").unwrap().len(), 2);

        let state_dict = state_collection.clone().as_mapping();
        assert_eq!(state_dict.len(), 2);
        assert_eq!(
            state_dict
                .get("agent_A".to_owned())
                .unwrap()
                .as_mapping()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            state_dict
                .get("agent_B".to_owned())
                .unwrap()
                .as_mapping()
                .unwrap()
                .len(),
            2
        );

        let workload_instance_name =
            WorkloadInstanceName::new("agent_B".to_owned(), "nginx".to_owned(), "5678".to_owned());
        let workload_state = state_collection
            .get_for_instance_name(&workload_instance_name)
            .unwrap();
        assert_eq!(workload_state.state, WorkloadStateEnum::Pending);
        assert_eq!(
            workload_state.substate,
            WorkloadSubStateEnum::PendingWaitingToStart
        );
        assert_eq!(workload_state.additional_info, "Random info");
    }
}
