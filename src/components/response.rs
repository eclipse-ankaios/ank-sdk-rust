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
use crate::ankaios_api::{self};
use crate::components::complete_state::CompleteState;
use crate::components::event_types::EventEntry;
use crate::components::log_types::LogEntry;
use crate::extensions::UnreachableOption;
use ankaios_api::ank_base::{
    Error, UpdateStateSuccess as AnkaiosUpdateStateSuccess,
    response::ResponseContent as AnkaiosResponseContent,
};
use ankaios_api::control_api::{FromAnkaios, from_ankaios::FromAnkaiosEnum};
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
    /// The response indicating an event notification.
    EventResponse(Box<EventEntry>),
    /// The success of an events cancel request.
    EventsCancelAccepted,
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
                        AnkaiosResponseContent::Error(Error {
                            message: String::from("Response content is None."),
                        }),
                    ) {
                        AnkaiosResponseContent::Error(err) => ResponseType::Error(err.message),
                        AnkaiosResponseContent::CompleteStateResponse(complete_state_response) => {
                            if complete_state_response.altered_fields.is_some() {
                                ResponseType::EventResponse(Box::new(EventEntry::from(
                                    *complete_state_response,
                                )))
                            } else {
                                ResponseType::CompleteState(Box::new(
                                    CompleteState::new_from_proto(
                                        complete_state_response.complete_state.expect(
                                            "Complete State response must contain Complete State.",
                                        ),
                                    ),
                                ))
                            }
                        }
                        AnkaiosResponseContent::UpdateStateSuccess(update_state_success) => {
                            ResponseType::UpdateStateSuccess(Box::new(
                                UpdateStateSuccess::new_from_proto(update_state_success),
                            ))
                        }
                        AnkaiosResponseContent::LogsRequestAccepted(logs_request_accepted) => {
                            ResponseType::LogsRequestAccepted(
                                logs_request_accepted
                                    .workload_names
                                    .into_iter()
                                    .map(WorkloadInstanceName::from)
                                    .collect(),
                            )
                        }
                        AnkaiosResponseContent::LogsCancelAccepted(_) => {
                            ResponseType::LogsCancelAccepted
                        }
                        AnkaiosResponseContent::LogEntriesResponse(log_entries_response) => {
                            let log_entries = log_entries_response
                                .log_entries
                                .into_iter()
                                .map(LogEntry::from)
                                .collect();

                            ResponseType::LogEntriesResponse(log_entries)
                        }
                        AnkaiosResponseContent::LogsStopResponse(logs_stop_response) => {
                            let instance_name = logs_stop_response
                                .workload_name
                                .map(WorkloadInstanceName::from)
                                .unwrap_or_unreachable();

                            ResponseType::LogsStopResponse(instance_name)
                        }
                        AnkaiosResponseContent::EventsCancelAccepted(_) => {
                            ResponseType::EventsCancelAccepted
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
    /// [AnkaiosUpdateStateSuccess](ank_base::UpdateStateSuccess) proto message.
    ///
    /// ## Arguments
    ///
    /// * `update_state_success` - The [AnkaiosUpdateStateSuccess](ank_base::UpdateStateSuccess) to create the [`UpdateStateSuccess`] from.
    ///
    /// ## Returns
    ///
    /// A new [`UpdateStateSuccess`] instance.
    pub(crate) fn new_from_proto(update_state_success: AnkaiosUpdateStateSuccess) -> Self {
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
        from_ankaios_enum: Some(FromAnkaiosEnum::Response(Box::new(
            ankaios_api::ank_base::Response {
                request_id: req_id,
                response_content: Some(AnkaiosResponseContent::UpdateStateSuccess(
                    AnkaiosUpdateStateSuccess {
                        added_workloads: vec!["workload_test.1234.agent_Test".to_owned()],
                        deleted_workloads: Vec::default(),
                    },
                )),
            },
        ))),
    }
}

#[cfg(test)]
pub fn generate_test_response_update_state_success(req_id: String) -> Response {
    Response::new(generate_test_proto_update_state_success(req_id))
}

#[cfg(test)]
pub fn get_test_proto_from_ankaios_log_entries_response(
    request_id: String,
    log_entries_response: ankaios_api::ank_base::LogEntriesResponse,
) -> FromAnkaios {
    FromAnkaios {
        from_ankaios_enum: Some(
            ankaios_api::control_api::from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ankaios_api::ank_base::Response {
                    request_id,
                    response_content: Some(AnkaiosResponseContent::LogEntriesResponse(
                        log_entries_response,
                    )),
                },
            )),
        ),
    }
}

#[cfg(test)]
pub fn generate_test_proto_log_entries_response() -> ankaios_api::ank_base::LogEntriesResponse {
    ankaios_api::ank_base::LogEntriesResponse {
        log_entries: vec![
            ankaios_api::ank_base::LogEntry {
                workload_name: Some(ankaios_api::ank_base::WorkloadInstanceName {
                    agent_name: "agent_A".to_owned(),
                    workload_name: "workload_A".to_owned(),
                    id: "id_a".to_owned(),
                }),
                message: "log message 1".to_owned(),
            },
            ankaios_api::ank_base::LogEntry {
                workload_name: Some(ankaios_api::ank_base::WorkloadInstanceName {
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
pub fn generate_test_response_event_entry(request_id: String) -> Response {
    Response::new(FromAnkaios {
        from_ankaios_enum: Some(
            ankaios_api::control_api::from_ankaios::FromAnkaiosEnum::Response(Box::new(
                ankaios_api::ank_base::Response {
                    request_id,
                    response_content: Some(AnkaiosResponseContent::CompleteStateResponse(
                        Box::new(ankaios_api::ank_base::CompleteStateResponse {
                            complete_state: Some(ankaios_api::ank_base::CompleteState::default()),
                            altered_fields: Some(ankaios_api::ank_base::AlteredFields {
                                added_fields: vec!["field1".to_owned()],
                                updated_fields: vec!["field2".to_owned()],
                                removed_fields: vec!["field3".to_owned()],
                            }),
                        }),
                    )),
                },
            )),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{Response, ResponseType, UpdateStateSuccess};
    use crate::components::response::{
        generate_test_proto_log_entries_response, generate_test_response_event_entry,
        get_test_proto_from_ankaios_log_entries_response,
    };
    use crate::{EventEntry, ankaios_api};
    use ankaios_api::ank_base::{
        Response as AnkaiosResponse, UpdateStateSuccess as AnkaiosUpdateStateSuccess,
        response::ResponseContent as AnkaiosResponseContent,
    };
    use ankaios_api::control_api::{FromAnkaios, from_ankaios};
    use std::collections::HashMap;

    #[test]
    fn utest_response_type() {
        let mut response_type = ResponseType::default();
        assert_eq!(format!("{response_type:?}"), "Error(\"\")");
        response_type = ResponseType::CompleteState(Box::default());
        assert_eq!(
            format!("{response_type:?}"),
            "CompleteState(CompleteState { complete_state: CompleteState { desired_state: Some(State { api_version: \"v1\", workloads: Some(WorkloadMap { workloads: {} }), configs: None }), workload_states: None, agents: None } })"
        );
        response_type = ResponseType::UpdateStateSuccess(Box::default());
        assert_eq!(
            format!("{response_type:?}"),
            "UpdateStateSuccess(UpdateStateSuccess { added_workloads: [], deleted_workloads: [] })"
        );
        response_type = ResponseType::ConnectionClosedReason(String::default());
        assert_eq!(format!("{response_type:?}"), "ConnectionClosedReason(\"\")");
    }

    #[test]
    fn utest_response_error() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                AnkaiosResponse {
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::Error(
                        ankaios_api::ank_base::Error::default(),
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::Error(String::default())
        );
    }

    #[test]
    fn utest_response_complete_state() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                AnkaiosResponse {
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::CompleteStateResponse(
                        Box::new(ankaios_api::ank_base::CompleteStateResponse {
                            complete_state: Some(ankaios_api::ank_base::CompleteState {
                                desired_state: Some(ankaios_api::ank_base::State {
                                    api_version: "v1".to_owned(),
                                    workloads: Some(Default::default()),
                                    configs: None,
                                }),
                                workload_states: None,
                                agents: None,
                            }),
                            altered_fields: None,
                        }),
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::CompleteState(Box::default())
        );
    }

    #[test]
    fn utest_response_update_state_success() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                AnkaiosResponse {
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::UpdateStateSuccess(
                        ankaios_api::ank_base::UpdateStateSuccess::default(),
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::UpdateStateSuccess(Box::default())
        );
    }

    #[test]
    fn utest_response_control_interface_accepted() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::ControlInterfaceAccepted(
                ankaios_api::control_api::ControlInterfaceAccepted::default(),
            )),
        });
        assert_eq!(response.get_request_id(), String::default());
        assert_eq!(
            response.get_content(),
            ResponseType::ControlInterfaceAccepted
        );
    }

    #[test]
    fn utest_response_connection_closed() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::ConnectionClosed(
                ankaios_api::control_api::ConnectionClosed::default(),
            )),
        });
        assert_eq!(response.get_request_id(), String::default());
        assert_eq!(
            response.get_content(),
            ResponseType::ConnectionClosedReason(String::default())
        );
    }

    #[test]
    fn utest_response_empty() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: None,
        });
        assert_eq!(response.get_request_id(), String::default());
        assert_eq!(
            response.get_content(),
            ResponseType::Error(String::from("Response is empty."))
        );
    }

    #[test]
    fn utest_update_state_success() {
        let update_state_success = UpdateStateSuccess::new_from_proto(AnkaiosUpdateStateSuccess {
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
            format!("{update_state_success:?}"),
            "UpdateStateSuccess { added_workloads: [WorkloadInstanceName { agent_name: \"agent_Test\", workload_name: \"workload_new\", workload_id: \"1234\" }], deleted_workloads: [WorkloadInstanceName { agent_name: \"agent_Test\", workload_name: \"workload_old\", workload_id: \"5678\" }] }"
        );
    }

    #[test]
    fn utest_response_logs_request_accepted() {
        let workload_names = vec![
            ankaios_api::ank_base::WorkloadInstanceName {
                agent_name: "agent_A".to_owned(),
                workload_name: "workload_A".to_owned(),
                id: "id_a".to_owned(),
            },
            ankaios_api::ank_base::WorkloadInstanceName {
                agent_name: "agent_B".to_owned(),
                workload_name: "workload_B".to_owned(),
                id: "id_b".to_owned(),
            },
        ];

        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                AnkaiosResponse {
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::LogsRequestAccepted(
                        ankaios_api::ank_base::LogsRequestAccepted {
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
                AnkaiosResponse {
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::LogsCancelAccepted(
                        ankaios_api::ank_base::LogsCancelAccepted {},
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
        let expected_instance_name = ankaios_api::ank_base::WorkloadInstanceName {
            agent_name: "agent_A".to_owned(),
            workload_name: "workload_A".to_owned(),
            id: "id_a".to_owned(),
        };

        let from_ankaios_response = FromAnkaios {
            from_ankaios_enum: Some(
                ankaios_api::control_api::from_ankaios::FromAnkaiosEnum::Response(Box::new(
                    ankaios_api::ank_base::Response {
                        request_id: "123".to_owned(),
                        response_content: Some(AnkaiosResponseContent::LogsStopResponse(
                            ankaios_api::ank_base::LogsStopResponse {
                                workload_name: Some(expected_instance_name.clone()),
                            },
                        )),
                    },
                )),
            ),
        };

        let response = Response::new(from_ankaios_response);

        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::LogsStopResponse(expected_instance_name.into())
        );
    }

    #[test]
    fn utest_response_event_entry() {
        let response = generate_test_response_event_entry("123".to_owned());
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(
            response.get_content(),
            ResponseType::EventResponse(Box::new(EventEntry::from(
                ankaios_api::ank_base::CompleteStateResponse {
                    complete_state: Some(ankaios_api::ank_base::CompleteState::default()),
                    altered_fields: Some(ankaios_api::ank_base::AlteredFields {
                        added_fields: vec!["field1".to_owned()],
                        updated_fields: vec!["field2".to_owned()],
                        removed_fields: vec!["field3".to_owned()],
                    }),
                }
            )))
        );
    }

    #[test]
    fn utest_response_events_cancel_accepted() {
        let response = Response::new(FromAnkaios {
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(Box::new(
                AnkaiosResponse {
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::EventsCancelAccepted(
                        ankaios_api::ank_base::EventsCancelAccepted {},
                    )),
                },
            ))),
        });
        assert_eq!(response.get_request_id(), "123".to_owned());
        assert_eq!(response.get_content(), ResponseType::EventsCancelAccepted);
    }
}
