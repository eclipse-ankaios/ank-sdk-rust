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
use serde_yaml::Value;

use crate::ankaios_api;
use ankaios_api::ank_base;
use super::workload_state_enums::{WorkloadStateEnum, WorkloadSubStateEnum};

/// Represents the execution state of a Workload.
#[derive(Debug, Default, Clone)]
pub struct WorkloadExecutionState{
    /// The state of the workload.
    pub state: WorkloadStateEnum,
    /// The substate of the workload.
    pub substate: WorkloadSubStateEnum,
    /// Additional information about the state.
    pub additional_info: String,
}

impl WorkloadExecutionState {
    #[doc(hidden)]
    /// Creates a new `WorkloadExecutionState`` from an [ExecutionState](ank_base::ExecutionState).
    /// 
    /// ## Arguments
    /// 
    /// * `exec_state` - The [ExecutionState](ank_base::ExecutionState) to create the [`WorkloadExecutionState`] from.
    /// 
    /// ## Returns
    /// 
    /// A new [`WorkloadExecutionState`] instance.
    pub(crate) fn new(exec_state: ank_base::ExecutionState) -> WorkloadExecutionState {
        #[allow(non_snake_case)] // False positive: None is an optional, not a variable, so it's ok to not be snake_case.
        match exec_state.execution_state_enum {
            Some(execution_state_enum) => {
                let (state, substate) = WorkloadExecutionState::parse_state(&execution_state_enum);
                WorkloadExecutionState {
                    state,
                    substate,
                    additional_info: exec_state.additional_info,
                }
            },
            None => WorkloadExecutionState {
                state: WorkloadStateEnum::NotScheduled,
                substate: WorkloadSubStateEnum::NotScheduled,
                additional_info: exec_state.additional_info,
            }
        }
    }

    /// Converts the `WorkloadExecutionState` to a [String].
    /// 
    /// ## Returns
    /// 
    /// A [String] representation of the [`WorkloadExecutionState`].
    pub fn to_dict(&self) -> serde_yaml::Mapping {
        let mut map = serde_yaml::Mapping::new();
        map.insert(Value::String("state".to_owned()), Value::String(self.state.to_string()));
        map.insert(Value::String("substate".to_owned()), Value::String(self.substate.to_string()));
        map.insert(Value::String("additional_info".to_owned()), Value::String(self.additional_info.clone()));
        map
    }

    #[doc(hidden)]
    /// Helper function to parse the state and substate from the [`ExecutionStateEnum`](ank_base::execution_state::ExecutionStateEnum).
    /// 
    /// ## Arguments
    /// 
    /// * `exec_state` - The [`ExecutionStateEnum`](ank_base::execution_state::ExecutionStateEnum) to parse.
    /// 
    /// ## Returns
    /// 
    /// A tuple containing the [`WorkloadStateEnum`] and [`WorkloadSubStateEnum`] parsed
    /// from the [`ExecutionStateEnum`](ank_base::execution_state::ExecutionStateEnum).
    pub(crate) fn parse_state(exec_state: &ank_base::execution_state::ExecutionStateEnum) -> (WorkloadStateEnum, WorkloadSubStateEnum) {
        let (state, value) = match *exec_state {
            ank_base::execution_state::ExecutionStateEnum::AgentDisconnected(value) => {
                (WorkloadStateEnum::AgentDisconnected, value)
            }
            ank_base::execution_state::ExecutionStateEnum::Pending(value) => {
                (WorkloadStateEnum::Pending, value)
            }
            ank_base::execution_state::ExecutionStateEnum::Running(value) => {
                (WorkloadStateEnum::Running, value)
            }
            ank_base::execution_state::ExecutionStateEnum::Stopping(value) => {
                (WorkloadStateEnum::Stopping, value)
            }
            ank_base::execution_state::ExecutionStateEnum::Succeeded(value) => {
                (WorkloadStateEnum::Succeeded, value)
            }
            ank_base::execution_state::ExecutionStateEnum::Failed(value) => {
                (WorkloadStateEnum::Failed, value)
            }
            ank_base::execution_state::ExecutionStateEnum::NotScheduled(value) => {
                (WorkloadStateEnum::NotScheduled, value)
            }
            ank_base::execution_state::ExecutionStateEnum::Removed(value) => {
                (WorkloadStateEnum::Removed, value)
            }
        };
        (state, WorkloadSubStateEnum::new(state, value).unwrap())
    }
}

impl fmt::Display for WorkloadExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({}): {}", self.state, self.substate, self.additional_info)
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
mod tests {
    use serde_yaml::Value;
    use super::ank_base;
    use super::{WorkloadExecutionState, WorkloadStateEnum, WorkloadSubStateEnum};

    #[test]
    fn utest_default_functionality() {
        let default_exec_state = WorkloadExecutionState::new(
            ank_base::ExecutionState {
                execution_state_enum: None,
                additional_info: "No state present".to_owned(),
            }
        );
        assert_eq!(default_exec_state.state, WorkloadStateEnum::NotScheduled);
        assert_eq!(default_exec_state.substate, WorkloadSubStateEnum::NotScheduled);
        assert_eq!(default_exec_state.additional_info, "No state present");
        assert_eq!(default_exec_state.to_string(), "NotScheduled (NotScheduled): No state present");

        let mut expected_dict = serde_yaml::Mapping::new();
        expected_dict.insert(Value::String("state".to_owned()), Value::String("NotScheduled".to_owned()));
        expected_dict.insert(Value::String("substate".to_owned()), Value::String("NotScheduled".to_owned()));
        expected_dict.insert(Value::String("additional_info".to_owned()), Value::String("No state present".to_owned()));

        assert_eq!(default_exec_state.to_dict(), expected_dict);
    }

    macro_rules! generate_test_for_workload_execution_state {
        ($test_name:ident, $state:ident, $substate:ident, $ank_base_state:expr) => {
            #[test]
            fn $test_name() {
                let exec_state = WorkloadExecutionState::new(
                    ank_base::ExecutionState {
                        execution_state_enum: Some($ank_base_state),
                        additional_info: "Additional info".to_owned(),
                    }
                );
                assert_eq!(exec_state.state, WorkloadStateEnum::$state);
                assert_eq!(exec_state.substate, WorkloadSubStateEnum::$substate);
                assert_eq!(exec_state.additional_info, "Additional info");
            }
        };
    }

    generate_test_for_workload_execution_state!(utest_agent_disconnected, AgentDisconnected, AgentDisconnected,
        ank_base::execution_state::ExecutionStateEnum::AgentDisconnected(ank_base::AgentDisconnected::AgentDisconnected as i32));
    generate_test_for_workload_execution_state!(utest_pending, Pending, PendingWaitingToStart,
        ank_base::execution_state::ExecutionStateEnum::Pending(ank_base::Pending::WaitingToStart as i32));
    generate_test_for_workload_execution_state!(utest_running, Running, RunningOk,
        ank_base::execution_state::ExecutionStateEnum::Running(ank_base::Running::Ok as i32));
    generate_test_for_workload_execution_state!(utest_stopping, Stopping, StoppingWaitingToStop,
        ank_base::execution_state::ExecutionStateEnum::Stopping(ank_base::Stopping::WaitingToStop as i32));
    generate_test_for_workload_execution_state!(utest_succeeded, Succeeded, SucceededOk,
        ank_base::execution_state::ExecutionStateEnum::Succeeded(ank_base::Succeeded::Ok as i32));
    generate_test_for_workload_execution_state!(utest_failed, Failed, FailedExecFailed,
        ank_base::execution_state::ExecutionStateEnum::Failed(ank_base::Failed::ExecFailed as i32));
    generate_test_for_workload_execution_state!(utest_not_scheduled, NotScheduled, NotScheduled,
        ank_base::execution_state::ExecutionStateEnum::NotScheduled(ank_base::NotScheduled::NotScheduled as i32));
    generate_test_for_workload_execution_state!(utest_removed, Removed, Removed,
        ank_base::execution_state::ExecutionStateEnum::Removed(ank_base::Removed::Removed as i32));
}