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

//! This module contains the [Response] and [`UpdateStateSuccess`] structs and the [`ResponseType`] enum.
//!
//! # Examples
//!
//! ## Get response content:
//!
//! ```rust
//! # use ankaios_sdk::Response;
//! #
//! let response: Response;
//! # let response = Response::default();
//! let content = response.get_content();
//! ```
//!
//! ## Check if the `request_id` matches
//!
//! ```rust
//! # use ankaios_sdk::Response;
//! #
//! # let response = Response::default();
//! if response.get_request_id() == "1234" {
//!     println!("Request ID matches.");
//! }
//! ```
//!
//! ## Convert the update state success to a dictionary
//!
//! ```rust
//! # use ankaios_sdk::UpdateStateSuccess;
//! #
//! let update_state_success: UpdateStateSuccess;
//! # let update_state_success = UpdateStateSuccess::default();
//! let dict = update_state_success.to_dict();
//! ```

use super::workload_state_mod::WorkloadInstanceName;
use crate::components::complete_state::CompleteState;
use crate::components::log_types::LogEntry;
use crate::extensions::UnreachableOption;

use ankaios_api::ank_base;
use ankaios_api::control_api::{FromAnkaios, from_ankaios::FromAnkaiosEnum};

use core::fmt;
use std::collections::HashMap;
use std::default;

/// Enum that represents the type of responses that can be provided by the [Ankaios] cluster.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Clone, Debug, PartialEq)]
pub enum ResponseType {
    /// The complete state of the system.
    CompleteState(Box<CompleteState>),
    /// The success of an update state request.
    UpdateStateSuccess(Box<UpdateStateSuccess>),
    /// An error provided by the cluster.
    Error(String),
    /// The response indicating that the connection has been accepted.
    ControlInterfaceAccepted,
    /// The reason a connection closed was received.
    ConnectionClosedReason(String),
    /// The success of an logs request.
    LogsRequestAccepted(Vec<WorkloadInstanceName>),
    /// The success of an logs cancel request.
    LogsCancelAccepted,
    /// The response containing log entries.
    LogEntriesResponse(Vec<LogEntry>),
    /// The response indicating the stop of log entries for a specific workload.
    LogsStopResponse(WorkloadInstanceName),
}

/// Struct that represents a response from the [Ankaios] cluster.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Default, Clone, Debug)]
pub struct Response {
    /// The content of the response.
    pub content: ResponseType,
    /// The ID of the response. It should match the ID of the request.
    pub id: String,
}

/// Struct that handles the `UpdateStateSuccess` response.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateStateSuccess {
    /// The workload instance names of the workloads that were added.
    pub added_workloads: Vec<WorkloadInstanceName>,
    /// The workload instance names of the workloads that were deleted.
    pub deleted_workloads: Vec<WorkloadInstanceName>,
}

impl fmt::Display for ResponseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ResponseType::CompleteState(_) => write!(f, "CompleteState"),
            ResponseType::UpdateStateSuccess(_) => write!(f, "UpdateStateSuccess"),
            ResponseType::Error(_) => write!(f, "Error"),
            ResponseType::ControlInterfaceAccepted => write!(f, "ControlInterfaceAccepted"),
            ResponseType::ConnectionClosedReason(_) => write!(f, "ConnectionClosedReason"),
            ResponseType::LogsRequestAccepted(_) => write!(f, "LogsRequestAccepted"),
            ResponseType::LogsCancelAccepted => write!(f, "LogsCancelAccepted"),
            ResponseType::LogEntriesResponse(_) => write!(f, "LogEntriesResponse"),
            ResponseType::LogsStopResponse(_) => write!(f, "LogsStopResponse"),
        }
    }
}

impl default::Default for ResponseType {
    fn default() -> Self {
        ResponseType::Error(String::default())
    }
}

impl Response {
    /// Creates a new `Response` object.
    ///
    /// ## Arguments
    ///
    /// * `response` - The response proto message to create the [Response] from.
    ///
    /// ## Returns
    ///
    /// A new [Response] instance.
    #[must_use]
    pub fn new(response: FromAnkaios) -> Self {
        Self::from(response)
    }

    /// Returns the request ID of the response.
    ///
    /// ## Returns
    ///
    /// A [String] containing the request ID of the response.
    #[must_use]
    pub fn get_request_id(&self) -> String {
        self.id.clone()
    }

    /// Returns the content of the response.
    ///
    /// ## Returns
    ///
    /// A [`ResponseType`] containing the content of the response.
    #[must_use]
    #[allow(dead_code)]
    pub fn get_content(&self) -> ResponseType {
        self.content.clone()
    }
}

impl From<FromAnkaios> for Response {
    fn from(response: FromAnkaios) -> Self {
        if let Some(response_enum) = response.from_ankaios_enum {
            match response_enum {
                FromAnkaiosEnum::Response(inner_response) => Self {
                    content: match inner_response.response_content.unwrap_or(
                        ank_base::ResponseContent::Error(ank_base::Error {
                            message: String::from("Response content is None."),
                        }),
                    ) {
                        ank_base::ResponseContent::Error(err) => ResponseType::Error(err.message),
                        ank_base::ResponseContent::CompleteState(complete_state) => {
                            ResponseType::CompleteState(Box::new(CompleteState::new_from_proto(
                                complete_state,
                            )))
                        }
                        ank_base::ResponseContent::UpdateStateSuccess(update_state_success) => {
                            ResponseType::UpdateStateSuccess(Box::new(
                                UpdateStateSuccess::new_from_proto(update_state_success),
                            ))
                        }
                        ank_base::ResponseContent::LogsRequestAccepted(logs_request_accepted) => {
                            ResponseType::LogsRequestAccepted(
                                logs_request_accepted
                                    .workload_names
                                    .into_iter()
                                    .map(WorkloadInstanceName::from)
                                    .collect(),
                            )
                        }
                        ank_base::ResponseContent::LogsCancelAccepted(_) => {
                            ResponseType::LogsCancelAccepted
                        }
                        ank_base::ResponseContent::LogEntriesResponse(log_entries_response) => {
                            let log_entries = log_entries_response
                                .log_entries
                                .into_iter()
                                .map(LogEntry::from)
                                .collect();

                            ResponseType::LogEntriesResponse(log_entries)
                        }
                        ank_base::ResponseContent::LogsStopResponse(logs_stop_response) => {
                            let instance_name = logs_stop_response
                                .workload_name
                                .map(WorkloadInstanceName::from)
                                .unwrap_or_unreachable();

                            ResponseType::LogsStopResponse(instance_name)
                        }
                    },
                    id: inner_response.request_id,
                },
                FromAnkaiosEnum::ControlInterfaceAccepted(_) => Self {
                    content: ResponseType::ControlInterfaceAccepted,
                    id: String::default(),
                },
                FromAnkaiosEnum::ConnectionClosed(connection_closed) => Self {
                    content: ResponseType::ConnectionClosedReason(connection_closed.reason),
                    id: String::default(),
                },
            }
        } else {
            Self {
                content: ResponseType::Error(String::from("Response is empty.")),
                id: String::default(),
            }
        }
    }
}

impl UpdateStateSuccess {
    #[doc(hidden)]
    /// Creates a new `UpdateStateSuccess` object from a
    /// [`ank_base::UpdateStateSuccess`] proto message.
    ///
    /// ## Arguments
    ///
    /// * `update_state_success` - The [`ank_base::UpdateStateSuccess`] to create the [`UpdateStateSuccess`] from.
    ///
    /// ## Returns
    ///
    /// A new [`UpdateStateSuccess`] instance.
    pub(crate) fn new_from_proto(update_state_success: ank_base::UpdateStateSuccess) -> Self {
        let mut added_workloads: Vec<WorkloadInstanceName> = Vec::new();
        let mut deleted_workloads: Vec<WorkloadInstanceName> = Vec::new();

        for workload in update_state_success.added_workloads {
            let parts: Vec<&str> = workload.split('.').collect();
            let [workload_name, workload_id, agent_name] = &*parts else {
                continue;
            };
            added_workloads.push(WorkloadInstanceName::new(
                (*agent_name).to_owned(),
                (*workload_name).to_owned(),
                (*workload_id).to_owned(),
            ));
        }

        for workload in update_state_success.deleted_workloads {
            let parts: Vec<&str> = workload.split('.').collect();
            let [workload_name, workload_id, agent_name] = &*parts else {
                continue;
            };
            deleted_workloads.push(WorkloadInstanceName::new(
                (*agent_name).to_owned(),
                (*workload_name).to_owned(),
                (*workload_id).to_owned(),
            ));
        }

        Self {
            added_workloads,
            deleted_workloads,
        }
    }

    /// Converts the `UpdateStateSuccess` to a [`HashMap`].
    ///
    /// ## Returns
    ///
    /// A [`HashMap`] containing the [`UpdateStateSuccess`] information.
    pub fn to_dict(&self) -> HashMap<String, Vec<serde_yaml::Mapping>> {
        let mut map = HashMap::new();
        map.insert(
            "added_workloads".to_owned(),
            self.added_workloads
                .iter()
                .map(WorkloadInstanceName::to_dict)
                .collect::<Vec<_>>(),
        );
        map.insert(
            "deleted_workloads".to_owned(),
            self.deleted_workloads
                .iter()
                .map(WorkloadInstanceName::to_dict)
                .collect::<Vec<_>>(),
        );
        map
    }
}

impl fmt::Display for UpdateStateSuccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UpdateStateSuccess: added_workloads: {:?}, deleted_workloads: {:?}",
            self.added_workloads, self.deleted_workloads
        )
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
pub fn generate_test_control_interface_accepted_response() -> Response {
    Response {
        content: ResponseType::ControlInterfaceAccepted,
        id: String::default(),
    }
}

#[cfg(test)]
pub fn generate_test_proto_update_state_success(req_id: String) -> FromAnkaios {
    FromAnkaios {
        from_ankaios_enum: Some(FromAnkaiosEnum::Response(Box::new(ank_base::Response {
            request_id: req_id,
            response_content: Some(ank_base::ResponseContent::UpdateStateSuccess(
                ank_base::UpdateStateSuccess {
                    added_workloads: vec!["workload_test.1234.agent_Test".to_owned()],
                    deleted_workloads: Vec::default(),
                },
            )),
        }))),
    }
}

#[cfg(test)]
pub fn generate_test_response_update_state_success(req_id: String) -> Response {
    Response::new(generate_test_proto_update_state_success(req_id))
}

#[cfg(test)]
pub fn get_test_proto_from_ankaios_log_entries_response(
    request_id: String,
    log_entries_response: ank_base::LogEntriesResponse,
) -> FromAnkaios {
    FromAnkaios {
        from_ankaios_enum: Some(FromAnkaiosEnum::Response(Box::new(ank_base::Response {
            request_id,
            response_content: Some(ank_base::ResponseContent::LogEntriesResponse(
                log_entries_response,
            )),
        }))),
    }
}

#[cfg(test)]
pub fn generate_test_proto_log_entries_response() -> ank_base::LogEntriesResponse {
    ank_base::LogEntriesResponse {
        log_entries: vec![
            ank_base::LogEntry {
                workload_name: Some(ank_base::WorkloadInstanceName {
                    agent_name: "agent_A".to_owned(),
                    workload_name: "workload_A".to_owned(),
                    id: "id_a".to_owned(),
                }),
                message: "log message 1".to_owned(),
            },
            ank_base::LogEntry {
                workload_name: Some(ank_base::WorkloadInstanceName {
                    agent_name: "agent_B".to_owned(),
                    workload_name: "workload_B".to_owned(),
                    id: "id_b".to_owned(),
                }),
                message: "log message 2".to_owned(),
            },
        ],
    }
}

#[cfg(test)]
pub fn generate_test_logs_stop_response(
    request_id: String,
    workload_name: WorkloadInstanceName,
) -> Response {
    Response {
        content: ResponseType::LogsStopResponse(workload_name),
        id: request_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Response, ResponseType, UpdateStateSuccess, generate_test_response_update_state_success,
    };
    use crate::components::response::{
        generate_test_proto_log_entries_response, get_test_proto_from_ankaios_log_entries_response,
    };

    use ankaios_api::ank_base;
    use ankaios_api::control_api::{
        ConnectionClosed, ControlInterfaceAccepted, FromAnkaios, from_ankaios,
        from_ankaios::FromAnkaiosEnum,
    };

    use std::collections::HashMap;

    #[test]
    fn test_doc_examples() {
        // Get response content
        let response = generate_test_response_update_state_success("1234".to_owned());
        let _content = response.get_content();

        // Check if the request_id matches
        if response.get_request_id() == "1234" {
            println!("Request ID matches.");
        }

        // Convert the update state success to a dictionary
        let update_state_success =
            UpdateStateSuccess::new_from_proto(ank_base::UpdateStateSuccess {
                added_workloads: vec!["workload_test.1234.agent_Test".to_owned()],
                deleted_workloads: Vec::default(),
            });
        let _dict = update_state_success.to_dict();
    }

    #[test]
    fn utest_response_type() {
        let mut response_type = ResponseType::default();
        assert_eq!(format!("{response_type}"), "Error");
        response_type = ResponseType::CompleteState(Box::default());
        assert_eq!(format!("{response_type}"), "CompleteState");
        response_type = ResponseType::UpdateStateSuccess(Box::default());
        assert_eq!(format!("{response_type}"), "UpdateStateSuccess");
        response_type = ResponseType::ConnectionClosedReason(String::default());
        assert_eq!(format!("{response_type}"), "ConnectionClosedReason");
    }

    #[test]
    fn utest_response_error() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ank_base::Response {
                    request_id: String::from("123"),
                    response_content: Some(ank_base::ResponseContent::Error(
                        ank_base::Error::default(),
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            format!("{}", response.get_content()),
            format!("{}", ResponseType::Error(String::default()))
        );
    }

    #[test]
    fn utest_response_complete_state() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ank_base::Response {
                    request_id: String::from("123"),
                    response_content: Some(ank_base::ResponseContent::CompleteState(
                        ank_base::CompleteState::default(),
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            format!("{}", response.get_content()),
            format!("{}", ResponseType::CompleteState(Box::default()))
        );
    }

    #[test]
    fn utest_response_update_state_success() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ank_base::Response {
                    request_id: String::from("123"),
                    response_content: Some(ank_base::ResponseContent::UpdateStateSuccess(
                        ank_base::UpdateStateSuccess::default(),
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            format!("{}", response.get_content()),
            format!("{}", ResponseType::UpdateStateSuccess(Box::default()))
        );
    }

    #[test]
    fn utest_response_control_interface_accepted() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::ControlInterfaceAccepted(
                ControlInterfaceAccepted::default(),
            )),
        });
        assert_eq!(response.get_request_id(), String::default());
        assert_eq!(
            format!("{}", response.get_content()),
            format!("{}", ResponseType::ControlInterfaceAccepted)
        );
    }

    #[test]
    fn utest_response_connection_closed() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::ConnectionClosed(
                ConnectionClosed::default(),
            )),
        });
        assert_eq!(response.get_request_id(), String::default());
        assert_eq!(
            format!("{}", response.get_content()),
            format!(
                "{}",
                ResponseType::ConnectionClosedReason(String::default())
            )
        );
    }

    #[test]
    fn utest_response_empty() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: None,
        });
        assert_eq!(response.get_request_id(), String::default());
        assert_eq!(
            format!("{}", response.get_content()),
            format!(
                "{}",
                ResponseType::Error(String::from("Response is empty."))
            )
        );
    }

    #[test]
    fn utest_update_state_success() {
        let update_state_success =
            UpdateStateSuccess::new_from_proto(ank_base::UpdateStateSuccess {
                added_workloads: vec!["workload_new.1234.agent_Test".to_owned()],
                deleted_workloads: vec!["workload_old.5678.agent_Test".to_owned()],
            });

        assert_eq!(update_state_success.added_workloads.len(), 1);
        assert_eq!(update_state_success.deleted_workloads.len(), 1);
        assert_eq!(
            update_state_success.to_dict(),
            HashMap::from([
                (
                    "added_workloads".to_owned(),
                    vec![serde_yaml::Mapping::from_iter([
                        (
                            serde_yaml::Value::String("agent_name".to_owned()),
                            serde_yaml::Value::String("agent_Test".to_owned())
                        ),
                        (
                            serde_yaml::Value::String("workload_name".to_owned()),
                            serde_yaml::Value::String("workload_new".to_owned())
                        ),
                        (
                            serde_yaml::Value::String("workload_id".to_owned()),
                            serde_yaml::Value::String("1234".to_owned())
                        ),
                    ])]
                ),
                (
                    "deleted_workloads".to_owned(),
                    vec![serde_yaml::Mapping::from_iter([
                        (
                            serde_yaml::Value::String("agent_name".to_owned()),
                            serde_yaml::Value::String("agent_Test".to_owned())
                        ),
                        (
                            serde_yaml::Value::String("workload_name".to_owned()),
                            serde_yaml::Value::String("workload_old".to_owned())
                        ),
                        (
                            serde_yaml::Value::String("workload_id".to_owned()),
                            serde_yaml::Value::String("5678".to_owned())
                        ),
                    ])]
                ),
            ])
        );

        assert_eq!(
            format!("{update_state_success}"),
            "UpdateStateSuccess: added_workloads: [WorkloadInstanceName { agent_name: \"agent_Test\", workload_name: \"workload_new\", workload_id: \"1234\" }], deleted_workloads: [WorkloadInstanceName { agent_name: \"agent_Test\", workload_name: \"workload_old\", workload_id: \"5678\" }]"
        );
    }

    #[test]
    fn utest_response_logs_request_accepted() {
        let workload_names = vec![
            ank_base::WorkloadInstanceName {
                agent_name: "agent_A".to_owned(),
                workload_name: "workload_A".to_owned(),
                id: "id_a".to_owned(),
            },
            ank_base::WorkloadInstanceName {
                agent_name: "agent_B".to_owned(),
                workload_name: "workload_B".to_owned(),
                id: "id_b".to_owned(),
            },
        ];

        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ank_base::Response {
                    request_id: String::from("123"),
                    response_content: Some(ank_base::ResponseContent::LogsRequestAccepted(
                        ank_base::LogsRequestAccepted {
                            workload_names: workload_names.clone(),
                        },
                    )),
                },
            ))),
        });

        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::LogsRequestAccepted(
                workload_names
                    .into_iter()
                    .map(std::convert::Into::into)
                    .collect()
            )
        );
    }

    #[test]
    fn utest_response_logs_cancel_accepted() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ank_base::Response {
                    request_id: String::from("123"),
                    response_content: Some(ank_base::ResponseContent::LogsCancelAccepted(
                        ank_base::LogsCancelAccepted {},
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(response.get_content(), ResponseType::LogsCancelAccepted);
    }

    #[test]
    fn utest_response_log_entries_response() {
        let log_entries_response = generate_test_proto_log_entries_response();
        let log_entries = log_entries_response.log_entries.clone();
        let response = Response::new(get_test_proto_from_ankaios_log_entries_response(
            "123".to_owned(),
            log_entries_response.clone(),
        ));
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::LogEntriesResponse(
                log_entries
                    .into_iter()
                    .map(std::convert::Into::into)
                    .collect()
            )
        );
    }

    #[test]
    fn utest_response_logs_stop_response() {
        let expected_instance_name = ank_base::WorkloadInstanceName {
            agent_name: "agent_A".to_owned(),
            workload_name: "workload_A".to_owned(),
            id: "id_a".to_owned(),
        };

        let from_ankaios_response = FromAnkaios {
            from_ankaios_enum: Some(FromAnkaiosEnum::Response(Box::new(ank_base::Response {
                request_id: "123".to_owned(),
                response_content: Some(ank_base::ResponseContent::LogsStopResponse(
                    ank_base::LogsStopResponse {
                        workload_name: Some(expected_instance_name.clone()),
                    },
                )),
            }))),
        };

        let response = Response::new(from_ankaios_response);

        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::LogsStopResponse(expected_instance_name.into())
        );
    }
}
