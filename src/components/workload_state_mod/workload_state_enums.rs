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

use std::{fmt, str::FromStr};

use crate::ankaios_api;
use ankaios_api::ank_base;

/// Enum representing the state of a Workload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum WorkloadStateEnum {
    /// The agent is disconnected.
    AgentDisconnected = 0,
    /// The workload is pending.
    Pending = 1,
    /// The workload is running.
    Running = 2,
    /// The workload is stopping.
    Stopping = 3,
    /// The workload has succeeded.
    Succeeded = 4,
    /// The workload has failed.
    Failed = 5,
    /// The workload is not scheduled.
    NotScheduled = 6,
    /// The workload has been removed.
    Removed = 7,
}

/// Enum representing the substate of a Workload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum WorkloadSubStateEnum {
    /// The agent is disconnected.
    AgentDisconnected = 0,
    /// The workload is pending and in the initial state.
    PendingInitial = 1,
    /// The workload is pending and waiting to start.
    PendingWaitingToStart = 2,
    /// The workload is pending and starting.
    PendingStarting = 3,
    /// The workload is pending and starting failed.
    PendingStartingFailed = 4,
    /// The workload is running and ok.
    RunningOk = 5,
    /// The workload is stopping.
    Stopping = 6,
    /// The workload is stopping and waiting to stop.
    StoppingWaitingToStop = 7,
    /// The workload is stopping, requested at runtime.
    StoppingRequestedAtRuntime = 8,
    /// The workload is stopping, but the delete failed.
    StoppingDeleteFailed = 9,
    /// The workload has succeeded.
    SucceededOk = 10,
    /// The workload has failed, execution failed.
    FailedExecFailed = 11,
    /// The workload has failed with unknown reason.
    FailedUnknown = 12,
    /// The workload has failed and is lost.
    FailedLost = 13,
    /// The workload is not scheduled.
    NotScheduled = 14,
    /// The workload has been removed.
    Removed = 15,
}

impl WorkloadStateEnum {
    /// Creates a new `WorkloadStateEnum` from a [String] value.
    ///
    /// ## Arguments
    ///
    /// * `value` - A [String] that represents the state.
    ///
    /// ## Returns
    ///
    /// A [`WorkloadStateEnum`] instance.
    ///
    /// ## Errors
    ///
    /// If the value is not a valid state.
    pub fn new_from_str<T: Into<String>>(value: T) -> Result<WorkloadStateEnum, String> {
        match value.into().as_str() {
            "AgentDisconnected" => Ok(WorkloadStateEnum::AgentDisconnected),
            "Pending" => Ok(WorkloadStateEnum::Pending),
            "Running" => Ok(WorkloadStateEnum::Running),
            "Stopping" => Ok(WorkloadStateEnum::Stopping),
            "Succeeded" => Ok(WorkloadStateEnum::Succeeded),
            "Failed" => Ok(WorkloadStateEnum::Failed),
            "NotScheduled" => Ok(WorkloadStateEnum::NotScheduled),
            "Removed" => Ok(WorkloadStateEnum::Removed),
            _ => Err("Invalid value for WorkloadStateEnum".to_owned()),
        }
    }

    /// Converts the [`WorkloadStateEnum`] to an [i32].
    ///
    /// ## Returns
    ///
    /// An [i32] value representing the [`WorkloadStateEnum`].
    #[must_use]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

impl fmt::Display for WorkloadStateEnum {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self {
            WorkloadStateEnum::AgentDisconnected => "AgentDisconnected",
            WorkloadStateEnum::Pending => "Pending",
            WorkloadStateEnum::Running => "Running",
            WorkloadStateEnum::Stopping => "Stopping",
            WorkloadStateEnum::Succeeded => "Succeeded",
            WorkloadStateEnum::Failed => "Failed",
            WorkloadStateEnum::NotScheduled => "NotScheduled",
            WorkloadStateEnum::Removed => "Removed",
        };
        write!(f, "{state_str}")
    }
}

impl FromStr for WorkloadStateEnum {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AgentDisconnected" => Ok(WorkloadStateEnum::AgentDisconnected),
            "Pending" => Ok(WorkloadStateEnum::Pending),
            "Running" => Ok(WorkloadStateEnum::Running),
            "Stopping" => Ok(WorkloadStateEnum::Stopping),
            "Succeeded" => Ok(WorkloadStateEnum::Succeeded),
            "Failed" => Ok(WorkloadStateEnum::Failed),
            "NotScheduled" => Ok(WorkloadStateEnum::NotScheduled),
            "Removed" => Ok(WorkloadStateEnum::Removed),
            _ => Err(()),
        }
    }
}

impl WorkloadSubStateEnum {
    /// Creates a new `WorkloadSubStateEnum` from a [`WorkloadStateEnum`] and an [i32] value.
    ///
    /// ## Arguments
    ///
    /// * `state` - A [`WorkloadStateEnum`] that represents the state;
    /// * `value` - An [i32] value that represents the substate.
    ///
    /// ## Returns
    ///
    /// A [`WorkloadSubStateEnum`] instance.
    ///
    /// ## Errors
    ///
    /// If the value is not a valid substate for the given state.
    pub fn new(state: WorkloadStateEnum, value: i32) -> Result<WorkloadSubStateEnum, String> {
        match state {
            WorkloadStateEnum::AgentDisconnected => {
                match ank_base::AgentDisconnected::from_i32(value) {
                    Some(ank_base::AgentDisconnected::AgentDisconnected) => {
                        Ok(WorkloadSubStateEnum::AgentDisconnected)
                    }
                    None => Err("Invalid value for state AgentDisconnected".to_owned()),
                }
            }
            WorkloadStateEnum::Pending => match ank_base::Pending::from_i32(value) {
                Some(ank_base::Pending::Initial) => Ok(WorkloadSubStateEnum::PendingInitial),
                Some(ank_base::Pending::WaitingToStart) => {
                    Ok(WorkloadSubStateEnum::PendingWaitingToStart)
                }
                Some(ank_base::Pending::Starting) => Ok(WorkloadSubStateEnum::PendingStarting),
                Some(ank_base::Pending::StartingFailed) => {
                    Ok(WorkloadSubStateEnum::PendingStartingFailed)
                }
                None => Err("Invalid value for state Pending".to_owned()),
            },
            WorkloadStateEnum::Running => match ank_base::Running::from_i32(value) {
                Some(ank_base::Running::Ok) => Ok(WorkloadSubStateEnum::RunningOk),
                None => Err("Invalid value for state Running".to_owned()),
            },
            WorkloadStateEnum::Stopping => match ank_base::Stopping::from_i32(value) {
                Some(ank_base::Stopping::Stopping) => Ok(WorkloadSubStateEnum::Stopping),
                Some(ank_base::Stopping::WaitingToStop) => {
                    Ok(WorkloadSubStateEnum::StoppingWaitingToStop)
                }
                Some(ank_base::Stopping::RequestedAtRuntime) => {
                    Ok(WorkloadSubStateEnum::StoppingRequestedAtRuntime)
                }
                Some(ank_base::Stopping::DeleteFailed) => {
                    Ok(WorkloadSubStateEnum::StoppingDeleteFailed)
                }
                None => Err("Invalid value for state Stopping".to_owned()),
            },
            WorkloadStateEnum::Succeeded => match ank_base::Succeeded::from_i32(value) {
                Some(ank_base::Succeeded::Ok) => Ok(WorkloadSubStateEnum::SucceededOk),
                None => Err("Invalid value for state Succeeded".to_owned()),
            },
            WorkloadStateEnum::Failed => match ank_base::Failed::from_i32(value) {
                Some(ank_base::Failed::ExecFailed) => Ok(WorkloadSubStateEnum::FailedExecFailed),
                Some(ank_base::Failed::Unknown) => Ok(WorkloadSubStateEnum::FailedUnknown),
                Some(ank_base::Failed::Lost) => Ok(WorkloadSubStateEnum::FailedLost),
                None => Err("Invalid value for state Failed".to_owned()),
            },
            WorkloadStateEnum::NotScheduled => match ank_base::NotScheduled::from_i32(value) {
                Some(ank_base::NotScheduled::NotScheduled) => {
                    Ok(WorkloadSubStateEnum::NotScheduled)
                }
                None => Err("Invalid value for state NotScheduled".to_owned()),
            },
            WorkloadStateEnum::Removed => match ank_base::Removed::from_i32(value) {
                Some(ank_base::Removed::Removed) => Ok(WorkloadSubStateEnum::Removed),
                None => Err("Invalid value for state Removed".to_owned()),
            },
        }
    }

    /// Converts the `WorkloadSubStateEnum` to an [i32].
    ///
    /// ## Returns
    ///
    /// An [i32] value representing the [`WorkloadSubStateEnum`].
    pub fn to_i32(self) -> i32 {
        match self {
            WorkloadSubStateEnum::AgentDisconnected => {
                ank_base::AgentDisconnected::AgentDisconnected as i32
            }
            WorkloadSubStateEnum::PendingInitial => ank_base::Pending::Initial as i32,
            WorkloadSubStateEnum::PendingWaitingToStart => ank_base::Pending::WaitingToStart as i32,
            WorkloadSubStateEnum::PendingStarting => ank_base::Pending::Starting as i32,
            WorkloadSubStateEnum::PendingStartingFailed => ank_base::Pending::StartingFailed as i32,
            WorkloadSubStateEnum::RunningOk => ank_base::Running::Ok as i32,
            WorkloadSubStateEnum::Stopping => ank_base::Stopping::Stopping as i32,
            WorkloadSubStateEnum::StoppingWaitingToStop => ank_base::Stopping::WaitingToStop as i32,
            WorkloadSubStateEnum::StoppingRequestedAtRuntime => {
                ank_base::Stopping::RequestedAtRuntime as i32
            }
            WorkloadSubStateEnum::StoppingDeleteFailed => ank_base::Stopping::DeleteFailed as i32,
            WorkloadSubStateEnum::SucceededOk => ank_base::Succeeded::Ok as i32,
            WorkloadSubStateEnum::FailedExecFailed => ank_base::Failed::ExecFailed as i32,
            WorkloadSubStateEnum::FailedUnknown => ank_base::Failed::Unknown as i32,
            WorkloadSubStateEnum::FailedLost => ank_base::Failed::Lost as i32,
            WorkloadSubStateEnum::NotScheduled => ank_base::NotScheduled::NotScheduled as i32,
            WorkloadSubStateEnum::Removed => ank_base::Removed::Removed as i32,
        }
    }
}

impl fmt::Display for WorkloadSubStateEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let substate_str = match self {
            WorkloadSubStateEnum::AgentDisconnected => "AgentDisconnected",
            WorkloadSubStateEnum::PendingInitial => "PendingInitial",
            WorkloadSubStateEnum::PendingWaitingToStart => "PendingWaitingToStart",
            WorkloadSubStateEnum::PendingStarting => "PendingStarting",
            WorkloadSubStateEnum::PendingStartingFailed => "PendingStartingFailed",
            WorkloadSubStateEnum::RunningOk => "RunningOk",
            WorkloadSubStateEnum::Stopping => "Stopping",
            WorkloadSubStateEnum::StoppingWaitingToStop => "StoppingWaitingToStop",
            WorkloadSubStateEnum::StoppingRequestedAtRuntime => "StoppingRequestedAtRuntime",
            WorkloadSubStateEnum::StoppingDeleteFailed => "StoppingDeleteFailed",
            WorkloadSubStateEnum::SucceededOk => "SucceededOk",
            WorkloadSubStateEnum::FailedExecFailed => "FailedExecFailed",
            WorkloadSubStateEnum::FailedUnknown => "FailedUnknown",
            WorkloadSubStateEnum::FailedLost => "FailedLost",
            WorkloadSubStateEnum::NotScheduled => "NotScheduled",
            WorkloadSubStateEnum::Removed => "Removed",
        };
        write!(f, "{substate_str}")
    }
}

impl FromStr for WorkloadSubStateEnum {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AgentDisconnected" => Ok(WorkloadSubStateEnum::AgentDisconnected),
            "PendingInitial" => Ok(WorkloadSubStateEnum::PendingInitial),
            "PendingWaitingToStart" => Ok(WorkloadSubStateEnum::PendingWaitingToStart),
            "PendingStarting" => Ok(WorkloadSubStateEnum::PendingStarting),
            "PendingStartingFailed" => Ok(WorkloadSubStateEnum::PendingStartingFailed),
            "RunningOk" => Ok(WorkloadSubStateEnum::RunningOk),
            "Stopping" => Ok(WorkloadSubStateEnum::Stopping),
            "StoppingWaitingToStop" => Ok(WorkloadSubStateEnum::StoppingWaitingToStop),
            "StoppingRequestedAtRuntime" => Ok(WorkloadSubStateEnum::StoppingRequestedAtRuntime),
            "StoppingDeleteFailed" => Ok(WorkloadSubStateEnum::StoppingDeleteFailed),
            "SucceededOk" => Ok(WorkloadSubStateEnum::SucceededOk),
            "FailedExecFailed" => Ok(WorkloadSubStateEnum::FailedExecFailed),
            "FailedUnknown" => Ok(WorkloadSubStateEnum::FailedUnknown),
            "FailedLost" => Ok(WorkloadSubStateEnum::FailedLost),
            "NotScheduled" => Ok(WorkloadSubStateEnum::NotScheduled),
            "Removed" => Ok(WorkloadSubStateEnum::Removed),
            _ => Err(()),
        }
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
    use std::str::FromStr;

    use crate::ankaios_api;
    use ankaios_api::ank_base;

    use super::{WorkloadStateEnum, WorkloadSubStateEnum};

    #[test]
    fn utest_workload_state_enum_helpers() {
        let state = WorkloadStateEnum::default();
        assert!(WorkloadStateEnum::is_valid(0));
        assert_eq!(WorkloadStateEnum::from_i32(0).unwrap(), state);
    }

    macro_rules! generate_test_for_workload_state_enum {
        ($test_name:ident, $enum_val:ident, $idx:expr) => {
            #[test]
            fn $test_name() {
                let state = WorkloadStateEnum::$enum_val;
                assert_eq!(state.as_i32(), $idx);
                assert_eq!(state.to_string(), stringify!($enum_val));
                assert_eq!(
                    state,
                    WorkloadStateEnum::new_from_str(stringify!($enum_val)).unwrap()
                );
                assert_eq!(state, stringify!($enum_val).parse().unwrap());
            }
        };
    }

    generate_test_for_workload_state_enum!(
        utest_workload_state_enum_agent_disconnected,
        AgentDisconnected,
        0
    );
    generate_test_for_workload_state_enum!(utest_workload_state_enum_pending, Pending, 1);
    generate_test_for_workload_state_enum!(utest_workload_state_enum_running, Running, 2);
    generate_test_for_workload_state_enum!(utest_workload_state_enum_stopping, Stopping, 3);
    generate_test_for_workload_state_enum!(utest_workload_state_enum_succeeded, Succeeded, 4);
    generate_test_for_workload_state_enum!(utest_workload_state_enum_failed, Failed, 5);
    generate_test_for_workload_state_enum!(
        utest_workload_state_enum_not_scheduled,
        NotScheduled,
        6
    );
    generate_test_for_workload_state_enum!(utest_workload_state_enum_removed, Removed, 7);

    #[test]
    fn utest_workload_state_str_invalid() {
        assert!(WorkloadStateEnum::from_str(stringify!(Invalid)).is_err());
        assert!(WorkloadStateEnum::new_from_str("Invalid").is_err());
    }

    #[test]
    fn utest_workload_sub_state_enum_helpers() {
        let substate = WorkloadSubStateEnum::default();
        assert_eq!(substate.to_i32(), 0i32);
        assert_eq!(WorkloadSubStateEnum::from_i32(0).unwrap(), substate);
    }

    macro_rules! generate_test_for_workload_state_enum {
        ($test_name:ident, $enum_val:ident, $state_val:ident, $idx:expr) => {
            #[test]
            fn $test_name() {
                let substate =
                    WorkloadSubStateEnum::new(WorkloadStateEnum::$state_val, $idx).unwrap();
                assert_eq!(substate.to_i32(), $idx);
                assert_eq!(substate.to_string(), stringify!($enum_val));
                assert_eq!(substate, stringify!($enum_val).parse().unwrap());
            }
        };
    }

    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_agent_disconnected,
        AgentDisconnected,
        AgentDisconnected,
        ank_base::AgentDisconnected::AgentDisconnected as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_pending_initial,
        PendingInitial,
        Pending,
        ank_base::Pending::Initial as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_pending_waiting_to_start,
        PendingWaitingToStart,
        Pending,
        ank_base::Pending::WaitingToStart as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_pending_starting,
        PendingStarting,
        Pending,
        ank_base::Pending::Starting as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_pending_starting_failed,
        PendingStartingFailed,
        Pending,
        ank_base::Pending::StartingFailed as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_running_ok,
        RunningOk,
        Running,
        ank_base::Running::Ok as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_stopping,
        Stopping,
        Stopping,
        ank_base::Stopping::Stopping as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_stopping_waiting_to_stop,
        StoppingWaitingToStop,
        Stopping,
        ank_base::Stopping::WaitingToStop as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_stopping_requested_at_runtime,
        StoppingRequestedAtRuntime,
        Stopping,
        ank_base::Stopping::RequestedAtRuntime as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_stopping_delete_failed,
        StoppingDeleteFailed,
        Stopping,
        ank_base::Stopping::DeleteFailed as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_succeeded_ok,
        SucceededOk,
        Succeeded,
        ank_base::Succeeded::Ok as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_failed_exec_failed,
        FailedExecFailed,
        Failed,
        ank_base::Failed::ExecFailed as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_failed_unknown,
        FailedUnknown,
        Failed,
        ank_base::Failed::Unknown as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_failed_lost,
        FailedLost,
        Failed,
        ank_base::Failed::Lost as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_not_scheduled,
        NotScheduled,
        NotScheduled,
        ank_base::NotScheduled::NotScheduled as i32
    );
    generate_test_for_workload_state_enum!(
        utest_workload_substate_enum_removed,
        Removed,
        Removed,
        ank_base::Removed::Removed as i32
    );

    #[test]
    fn utest_workload_substate_enum_err() {
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::AgentDisconnected, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::Pending, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::Running, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::Stopping, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::Succeeded, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::Failed, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::NotScheduled, 20).is_err());
        assert!(WorkloadSubStateEnum::new(WorkloadStateEnum::Removed, 20).is_err());
    }

    #[test]
    fn utest_workload_substate_str_invalid() {
        assert!(WorkloadSubStateEnum::from_str(stringify!(Invalid)).is_err());
    }
}
