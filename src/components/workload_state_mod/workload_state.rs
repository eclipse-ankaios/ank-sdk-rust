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

use std::fmt;
use std::collections::HashMap;

use api::ank_base;
use super::workload_execution_state::WorkloadExecutionState;
use super::workload_instance_name::WorkloadInstanceName;
use crate::AnkaiosError;

type ExecutionsStatesForId = HashMap<String, WorkloadExecutionState>;
type ExecutionsStatesOfWorkload = HashMap<String, ExecutionsStatesForId>;
type WorkloadStatesMap = HashMap<String, ExecutionsStatesOfWorkload>;

#[derive(Debug, Default)]
pub struct WorkloadState {
    pub execution_state: WorkloadExecutionState,
    pub workload_instance_name: WorkloadInstanceName,
}

#[derive(Debug, Default)]
pub struct WorkloadStateCollection {
    workload_states: WorkloadStatesMap,
}

impl WorkloadState {
    pub fn new_from_ank_base(agent_name: String, workload_name: String, workload_id: String, state: ank_base::ExecutionState) -> WorkloadState {
        WorkloadState {
            execution_state: WorkloadExecutionState::new(state),
            workload_instance_name: WorkloadInstanceName::new(agent_name, workload_name, workload_id),
        }
    }

    pub fn new_from_exec_state(agent_name: String, workload_name: String, workload_id: String, exec_state: WorkloadExecutionState) -> WorkloadState {
        WorkloadState {
            execution_state: exec_state,
            workload_instance_name: WorkloadInstanceName::new(agent_name, workload_name, workload_id),
        }
    }
}

impl fmt::Display for WorkloadState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.workload_instance_name, self.execution_state)
    }
}

impl WorkloadStateCollection {
    pub fn new() -> WorkloadStateCollection {
        WorkloadStateCollection {
            workload_states: HashMap::new(),
        }
    }

    pub fn new_from_proto(workload_states_map: &ank_base::WorkloadStatesMap) -> WorkloadStateCollection {
        let mut workload_states = WorkloadStateCollection::new();
        for (agent_name, workloads) in workload_states_map.agent_state_map.iter() {
            for (workload_name, workload_states_for_id) in workloads.wl_name_state_map.iter() {
                for (workload_id, state) in workload_states_for_id.id_state_map.iter() {
                    let workload_state = WorkloadState::new_from_ank_base(agent_name.clone(), workload_name.clone(), workload_id.clone(), state.clone());
                    workload_states.add_workload_state(workload_state);
                }
            }
        }
        workload_states
    }

    pub fn add_workload_state(&mut self, workload_state: WorkloadState) {
        let agent_name = workload_state.workload_instance_name.agent_name.clone();
        let workload_name = workload_state.workload_instance_name.workload_name.clone();
        let workload_id = workload_state.workload_instance_name.workload_id.clone();

        if !self.workload_states.contains_key(&agent_name) {
            self.workload_states.insert(agent_name.clone(), ExecutionsStatesOfWorkload::new());
        }

        if !self.workload_states.get(&agent_name).unwrap().contains_key(&workload_name) {
            self.workload_states.get_mut(&agent_name).unwrap().insert(workload_name.clone(), ExecutionsStatesForId::new());
        }

        self.workload_states.get_mut(&agent_name).unwrap().get_mut(&workload_name).unwrap().insert(workload_id, workload_state.execution_state);
    }

    pub fn get_as_dict(&self) -> serde_yaml::Mapping {
        let mut map = serde_yaml::Mapping::new();
        for (agent_name, workload_states) in self.workload_states.iter() {
            let mut agent_map = serde_yaml::Mapping::new();
            for (workload_name, workload_states_for_id) in workload_states.iter() {
                let mut workload_map = serde_yaml::Mapping::new();
                for (workload_id, workload_state) in workload_states_for_id.iter() {
                    workload_map.insert(serde_yaml::Value::String(workload_id.clone()), serde_yaml::Value::Mapping(workload_state.to_dict()));
                }
                agent_map.insert(serde_yaml::Value::String(workload_name.clone()), serde_yaml::Value::Mapping(workload_map));
            }
            map.insert(serde_yaml::Value::String(agent_name.clone()), serde_yaml::Value::Mapping(agent_map));
        }
        map
    }

    pub fn get_as_list(&self) -> Vec<WorkloadState> {
        let mut list = Vec::new();
        for (agent_name, workload_states_for_agent) in self.workload_states.iter() {
            for (workload_name, workload_states_for_id) in workload_states_for_agent.iter() {
                for (workload_id, workload_state) in workload_states_for_id.iter() {
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

    pub fn get_for_instance_name(&self, instance_name: &WorkloadInstanceName) -> Option<&WorkloadExecutionState> {
        self.workload_states.get(&instance_name.agent_name)
            .and_then(|workloads| workloads.get(&instance_name.workload_name))
            .and_then(|workload| workload.get(&instance_name.workload_id))
    }
}

impl TryFrom<ank_base::WorkloadStatesMap> for WorkloadStateCollection {
    type Error = AnkaiosError;

    fn try_from(proto: ank_base::WorkloadStatesMap) -> Result<Self, Self::Error> {
        Ok(Self::new_from_proto(&proto))
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

#[cfg(any(feature = "test_utils", test))]
pub fn generate_test_workload_states_proto() -> ank_base::WorkloadStatesMap {
    ank_base::WorkloadStatesMap { agent_state_map: HashMap::from([
        ("agent_A".to_string(), ank_base::ExecutionsStatesOfWorkload{
            wl_name_state_map: HashMap::from([
                ("nginx".to_string(), ank_base::ExecutionsStatesForId{
                    id_state_map: HashMap::from([
                        ("1234".to_string(), ank_base::ExecutionState{
                            execution_state_enum: Some(ank_base::execution_state::ExecutionStateEnum::Succeeded(ank_base::Succeeded::Ok as i32)),
                            additional_info: "Random info".to_string(),
                        }),
                    ])
                }),
            ])
        },),
        ("agent_B".to_string(), ank_base::ExecutionsStatesOfWorkload{
            wl_name_state_map: HashMap::from([
                ("nginx".to_string(), ank_base::ExecutionsStatesForId{
                    id_state_map: HashMap::from([
                        ("5678".to_string(), ank_base::ExecutionState{
                            execution_state_enum: Some(ank_base::execution_state::ExecutionStateEnum::Pending(ank_base::Pending::WaitingToStart as i32)),
                            additional_info: "Random info".to_string(),
                        }),
                    ])
                }),
                ("dyn_nginx".to_string(), ank_base::ExecutionsStatesForId{
                    id_state_map: HashMap::from([
                        ("9012".to_string(), ank_base::ExecutionState{
                            execution_state_enum: Some(ank_base::execution_state::ExecutionStateEnum::Stopping(ank_base::Stopping::WaitingToStop as i32)),
                            additional_info: "Random info".to_string(),
                        }),
                    ])
                }),
            ])
        },),
    ])}
}

#[cfg(test)]
mod tests {
    use crate::components::workload_state_mod::{WorkloadStateEnum, WorkloadSubStateEnum};

    use super::{ank_base, WorkloadExecutionState, WorkloadInstanceName, WorkloadState, WorkloadStateCollection};
    use super::generate_test_workload_states_proto;

    #[test]
    fn test_workload_state() {
        let agent_name = "agent_name".to_string();
        let workload_name = "workload_name".to_string();
        let workload_id = "workload_id".to_string();
        let state = ank_base::ExecutionState {
            execution_state_enum: Some(ank_base::execution_state::ExecutionStateEnum::Pending(ank_base::Pending::WaitingToStart as i32)),
            additional_info: "additional_info".to_string(),
        };
        let exec_state = WorkloadExecutionState::new(state.clone());

        let workload_state_ank_base = WorkloadState::new_from_ank_base(agent_name.clone(), workload_name.clone(), workload_id.clone(), state.clone());
        let workload_state_exec_state = WorkloadState::new_from_exec_state(agent_name.clone(), workload_name.clone(), workload_id.clone(), exec_state.clone());

        assert_eq!(workload_state_ank_base.to_string(), workload_state_exec_state.to_string());
        assert_eq!(workload_state_ank_base.execution_state.state, WorkloadStateEnum::Pending);
        assert_eq!(workload_state_ank_base.execution_state.substate, WorkloadSubStateEnum::PendingWaitingToStart);
        assert_eq!(workload_state_ank_base.execution_state.additional_info, "additional_info");
        assert_eq!(workload_state_ank_base.workload_instance_name.agent_name, agent_name);
        assert_eq!(workload_state_ank_base.workload_instance_name.workload_name, workload_name);
        assert_eq!(workload_state_ank_base.workload_instance_name.workload_id, workload_id);
    }

    #[test]
    fn test_workload_state_collection() {
        let state_collection = WorkloadStateCollection::new_from_proto(
            &generate_test_workload_states_proto());
        let mut state_list = state_collection.get_as_list();
        // The list comes unsorted, thus the test is not deterministic
        state_list.sort_by(|a, b| a.workload_instance_name.agent_name.cmp(&b.workload_instance_name.agent_name));
        assert_eq!(state_list.len(), 3);
        assert_eq!(state_list[0].workload_instance_name.agent_name, "agent_A");
        assert_eq!(state_list[0].workload_instance_name.workload_name, "nginx");
        assert_eq!(state_list[0].workload_instance_name.workload_id, "1234");
        assert_eq!(state_list[1].workload_instance_name.agent_name, "agent_B");
        assert_eq!(state_list[2].workload_instance_name.agent_name, "agent_B");

        let state_dict = state_collection.get_as_dict();
        assert_eq!(state_dict.len(), 2);
        assert_eq!(state_dict.get("agent_A".to_string()).unwrap().as_mapping().unwrap().len(), 1);
        assert_eq!(state_dict.get("agent_B".to_string()).unwrap().as_mapping().unwrap().len(), 2);

        let workload_instance_name = WorkloadInstanceName::new("agent_B".to_string(), "nginx".to_string(), "5678".to_string());
        let workload_state = state_collection.get_for_instance_name(&workload_instance_name).unwrap();
        assert_eq!(workload_state.state, WorkloadStateEnum::Pending);
        assert_eq!(workload_state.substate, WorkloadSubStateEnum::PendingWaitingToStart);
        assert_eq!(workload_state.additional_info, "Random info");
    }
}