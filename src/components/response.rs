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

//! This module contains the [Response] and [UpdateStateSuccess] structs and the [ResponseType] enum.
//! 
//! # Examples
//! 
//! ## Get response content:
//! 
//! ```rust
//! let response = /* */;
//! let content = response.get_content();
//! ```
//! 
//! ## Check if the request_id matches
//! 
//! ```rust
//! if response.get_request_id() == "1234" {
//!     println!("Request ID matches.");
//! }
//! ```
//! 
//! ## Convert the update state success to a dictionary
//! 
//! ```rust
//! let update_state_success = /* */;
//! let dict = update_state_success.to_dict();
//! ```

use core::fmt;
use std::collections::HashMap;
use std::default;
use crate::ankaios_api;
use ankaios_api::ank_base::{response::ResponseContent as AnkaiosResponseContent, UpdateStateSuccess as AnkaiosUpdateStateSuccess, Error};
use ankaios_api::control_api::{FromAnkaios, from_ankaios::FromAnkaiosEnum};
use crate::components::complete_state::CompleteState;
use super::workload_state_mod::WorkloadInstanceName;

/// Enum that represents the type of responses that can be provided by the [Ankaios] cluster.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Clone, Debug)]
pub enum ResponseType {
    /// The complete state of the system.
    CompleteState(Box<CompleteState>),
    /// The success of an update state request.
    UpdateStateSuccess(Box<UpdateStateSuccess>),
    // An error provided by the cluster.
    Error(String),
    // The reason a connection closed was received.
    ConnectionClosedReason(String),
}

/// Struct that represents a response from the [Ankaios] cluster.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Default, Clone, Debug)]
pub struct Response{
    /// The content of the response.
    pub content: ResponseType,
    /// The ID of the response. It should match the ID of the request.
    pub id: String,
}

/// Struct that handles the UpdateStateSuccess response.
#[derive(Clone, Debug, Default)]
pub struct UpdateStateSuccess {
    /// The workload instance names of the workloads that were added.
    pub added_workloads: Vec<WorkloadInstanceName>,
    /// The workload instance names of the workloads that were deleted.
    pub deleted_workloads: Vec<WorkloadInstanceName>,
}

impl fmt::Display for ResponseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseType::CompleteState(_) => write!(f, "CompleteState"),
            ResponseType::UpdateStateSuccess(_) => write!(f, "UpdateStateSuccess"),
            ResponseType::Error(_) => write!(f, "Error"),
            ResponseType::ConnectionClosedReason(_) => write!(f, "ConnectionClosedReason"),
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
    pub fn new(response: FromAnkaios) -> Self {
        if let Some(response_enum) = response.from_ankaios_enum {
            match response_enum {
                FromAnkaiosEnum::Response(response) => {
                    Self{
                        content: match response.response_content.unwrap_or(
                            AnkaiosResponseContent::Error(Error{
                                message: String::from("Response content is None."),
                            })
                        )  {
                            AnkaiosResponseContent::Error(err) => ResponseType::Error(
                                err.message
                            ),
                            AnkaiosResponseContent::CompleteState(complete_state) => ResponseType::CompleteState(
                                Box::new(CompleteState::new_from_proto(complete_state)),
                            ),
                            AnkaiosResponseContent::UpdateStateSuccess(update_state_success) => ResponseType::UpdateStateSuccess(
                                Box::new(UpdateStateSuccess::new_from_proto(update_state_success)),
                            ),
                        },
                        id: response.request_id,
                    }
                },
                FromAnkaiosEnum::ConnectionClosed(connection_closed) => {
                    Self{
                        content: ResponseType::ConnectionClosedReason(connection_closed.reason),
                        id: String::from(""),
                    }
                },
            }
        } else {
            Self{
                content: ResponseType::Error(String::from("Response is empty.")),
                id: String::from(""),
            }
        }
    }

    /// Returns the request ID of the response.
    /// 
    /// ## Returns
    /// 
    /// A [String] containing the request ID of the response.
    pub fn get_request_id(&self) -> String {
        self.id.clone()
    }

    #[allow(dead_code)]
    /// Returns the content of the response.
    /// 
    /// ## Returns
    /// 
    /// A [ResponseType] containing the content of the response.
    pub fn get_content(&self) -> ResponseType {
        self.content.clone()
    }
}


impl UpdateStateSuccess{
    #[doc(hidden)]
    /// Creates a new `UpdateStateSuccess` object from a
    /// [AnkaiosUpdateStateSuccess](ank_base::UpdateStateSuccess) proto message.
    /// 
    /// ## Arguments
    /// 
    /// * `update_state_success` - The [AnkaiosUpdateStateSuccess](ank_base::UpdateStateSuccess) to create the [UpdateStateSuccess] from.
    /// 
    /// ## Returns
    /// 
    /// A new [UpdateStateSuccess] instance.
    pub(crate) fn new_from_proto(update_state_success: AnkaiosUpdateStateSuccess) -> Self {
        let mut added_workloads: Vec<WorkloadInstanceName> = Vec::new();
        let mut deleted_workloads: Vec<WorkloadInstanceName> = Vec::new();

        for workload in update_state_success.added_workloads {
            let parts: Vec<&str> = workload.split('.').collect();
            let (workload_name, workload_id, agent_name) = match &parts[..] {
                [workload_name, workload_id, agent_name] => (workload_name, workload_id, agent_name),
                _ => continue,
            };
            added_workloads.push(WorkloadInstanceName::new(agent_name.to_string(), workload_name.to_string(), workload_id.to_string()));
        }

        for workload in update_state_success.deleted_workloads {
            let parts: Vec<&str> = workload.split('.').collect();
            let (workload_name, workload_id, agent_name) = match &parts[..] {
                [workload_name, workload_id, agent_name] => (workload_name, workload_id, agent_name),
                _ => continue,
            };
            deleted_workloads.push(WorkloadInstanceName::new(agent_name.to_string(), workload_name.to_string(), workload_id.to_string()));
        }

        Self{
            added_workloads,
            deleted_workloads,
        }
    }

    /// Converts the `UpdateStateSuccess` to a [HashMap].
    /// 
    /// ## Returns
    /// 
    /// A [HashMap] containing the [UpdateStateSuccess] information.
    pub fn to_dict(&self) -> HashMap<String, Vec<serde_yaml::Mapping>> {
        let mut map = HashMap::new();
        map.insert(
            "added_workloads".to_string(),
            self.added_workloads.iter().map(|instance_name| instance_name.to_dict()).collect::<Vec<_>>(),
        );
        map.insert(
            "deleted_workloads".to_string(),
            self.deleted_workloads.iter().map(|instance_name| instance_name.to_dict()).collect::<Vec<_>>(),
        );
        map
    }
}

impl fmt::Display for UpdateStateSuccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UpdateStateSuccess: added_workloads: {:?}, deleted_workloads: {:?}", self.added_workloads, self.deleted_workloads)
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
pub fn generate_test_proto_update_state_success(req_id: String) -> FromAnkaios {
    FromAnkaios{
        from_ankaios_enum: Some(FromAnkaiosEnum::Response(
            Box::new(ankaios_api::ank_base::Response{
                request_id: req_id,
                response_content: Some(AnkaiosResponseContent::UpdateStateSuccess(
                    AnkaiosUpdateStateSuccess{
                        added_workloads: vec!["workload_test.1234.agent_Test".to_string()],
                        deleted_workloads: Default::default(),
                    }
                ))
            })
        ))
    }
}

#[cfg(test)]
pub fn generate_test_response_update_state_success(req_id: String) -> Response {
    Response::new(generate_test_proto_update_state_success(req_id))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::{ResponseType, Response, UpdateStateSuccess,
        generate_test_response_update_state_success};
    use crate::ankaios_api;
    use ankaios_api::ank_base::{
        Response as AnkaiosResponse,
        response::ResponseContent as AnkaiosResponseContent,
        UpdateStateSuccess as AnkaiosUpdateStateSuccess,
    };
    use ankaios_api::control_api::{from_ankaios, FromAnkaios};

    #[test]
    fn test_doc_examples() {
        // Get response content
        let response = generate_test_response_update_state_success("1234".to_string());
        let _content = response.get_content();

        // Check if the request_id matches
        if response.get_request_id() == "1234" {
            println!("Request ID matches.");
        }

        // Convert the update state success to a dictionary
        let update_state_success = UpdateStateSuccess::new_from_proto(AnkaiosUpdateStateSuccess{
            added_workloads: vec!["workload_test.1234.agent_Test".to_string()],
            deleted_workloads: Default::default(),
        });
        let _dict = update_state_success.to_dict();
    }

    #[test]
    fn utest_response_type() {
        let mut response_type = ResponseType::default();
        assert_eq!(format!("{}", response_type), "Error");
        response_type = ResponseType::CompleteState(Box::default());
        assert_eq!(format!("{}", response_type), "CompleteState");
        response_type = ResponseType::UpdateStateSuccess(Box::default());
        assert_eq!(format!("{}", response_type), "UpdateStateSuccess");
        response_type = ResponseType::ConnectionClosedReason(String::default());
        assert_eq!(format!("{}", response_type), "ConnectionClosedReason");
    }

    #[test]
    fn utest_response_error() {
        let response = Response::new(FromAnkaios{
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(
                Box::new(AnkaiosResponse{
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::Error(
                        Default::default()
                    ))
                })
            ))
        });
        assert_eq!(response.get_request_id(), "123".to_string());
        assert_eq!(format!("{}", response.get_content()), format!("{}", ResponseType::Error(String::default())));
    }

    #[test]
    fn utest_response_complete_state() {
        let response = Response::new(FromAnkaios{
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(
                Box::new(AnkaiosResponse{
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::CompleteState(
                        Default::default()
                    ))
                })
            ))
        });
        assert_eq!(response.get_request_id(), "123".to_string());
        assert_eq!(format!("{}", response.get_content()), format!("{}", ResponseType::CompleteState(Box::default())));
    }

    #[test]
    fn utest_response_update_state_success() {
        let response = Response::new(FromAnkaios{
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::Response(
                Box::new(AnkaiosResponse{
                    request_id: String::from("123"),
                    response_content: Some(AnkaiosResponseContent::UpdateStateSuccess(
                        Default::default()
                    ))
                })
            ))
        });
        assert_eq!(response.get_request_id(), "123".to_string());
        assert_eq!(format!("{}", response.get_content()), format!("{}", ResponseType::UpdateStateSuccess(Box::default())));
    }

    #[test]
    fn utest_response_connection_closed() {
        let response = Response::new(FromAnkaios{
            from_ankaios_enum: Some(from_ankaios::FromAnkaiosEnum::ConnectionClosed(
                Default::default()
            ))
        });
        assert_eq!(response.get_request_id(), "".to_string());
        assert_eq!(format!("{}", response.get_content()), format!("{}", ResponseType::ConnectionClosedReason(String::default())));
    }

    #[test]
    fn utest_response_empty() {
        let response = Response::new(FromAnkaios{
            from_ankaios_enum: None
        });
        assert_eq!(response.get_request_id(), "".to_string());
        assert_eq!(format!("{}", response.get_content()), format!("{}", ResponseType::Error(String::from("Response is empty."))));
    }

    #[test]
    fn utest_update_state_success() {
        let update_state_success = UpdateStateSuccess::new_from_proto(AnkaiosUpdateStateSuccess{
            added_workloads: vec!["workload_new.1234.agent_Test".to_string()],
            deleted_workloads: vec!["workload_old.5678.agent_Test".to_string()],
        });

        assert_eq!(update_state_success.added_workloads.len(), 1);
        assert_eq!(update_state_success.deleted_workloads.len(), 1);
        assert_eq!(update_state_success.to_dict(), HashMap::from([
            ("added_workloads".to_string(), vec![serde_yaml::Mapping::from_iter([
                (serde_yaml::Value::String("agent_name".to_string()), serde_yaml::Value::String("agent_Test".to_string())),
                (serde_yaml::Value::String("workload_name".to_string()), serde_yaml::Value::String("workload_new".to_string())),
                (serde_yaml::Value::String("workload_id".to_string()), serde_yaml::Value::String("1234".to_string())),
            ])]),
            ("deleted_workloads".to_string(), vec![serde_yaml::Mapping::from_iter([
                (serde_yaml::Value::String("agent_name".to_string()), serde_yaml::Value::String("agent_Test".to_string())),
                (serde_yaml::Value::String("workload_name".to_string()), serde_yaml::Value::String("workload_old".to_string())),
                (serde_yaml::Value::String("workload_id".to_string()), serde_yaml::Value::String("5678".to_string())),
            ])]),
        ]));

        assert_eq!(format!("{}", update_state_success), "UpdateStateSuccess: added_workloads: [WorkloadInstanceName { agent_name: \"agent_Test\", workload_name: \"workload_new\", workload_id: \"1234\" }], deleted_workloads: [WorkloadInstanceName { agent_name: \"agent_Test\", workload_name: \"workload_old\", workload_id: \"5678\" }]");
    }
}