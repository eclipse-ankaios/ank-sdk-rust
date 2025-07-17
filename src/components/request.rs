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

//! This module contains the possible requests that can be made to the [Ankaios] application.
//! This include the [`GetStateRequest`] and [`UpdateStateRequest`] requests, that both implement the [`Request`] trait.
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios
//!
//! # Examples
//!
//! ## Create a request for updating the state:
//!
//! ```rust
//! let complete_state = CompleteState::new();
//! let _request = UpdateStateRequest::new(&complete_state, Vec::default());
//! ```
//!
//! ## Create a request for getting the state:
//!
//! ```rust
//! let mut request = GetStateRequest::new(Vec::default());
//! ```
//!
//! ## Get the request ID:
//!
//! ```rust
//! let request_id = request.get_id();
//! ```
//!
//! ## Create a request for getting the complete state filtered according to the provided field masks:
//!
//! ```rust
//! let request = GetStateRequest::new(vec!["desiredState.workloads".to_owned()]);
//! ```

use crate::components::complete_state::CompleteState;
use crate::{ankaios_api, components::workload_state_mod::WorkloadInstanceName};
use ankaios_api::ank_base::{
    request::RequestContent, CompleteStateRequest, Request as AnkaiosRequest,
    UpdateStateRequest as AnkaiosUpdateStateRequest,
};
use std::fmt;
use uuid::Uuid;

/// Trait that represents a request that can be made to the [Ankaios] application.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
pub trait Request {
    /// Returns the underlying [`AnkaiosRequest`] proto message.
    ///
    /// ## Returns
    ///
    /// The [`AnkaiosRequest`] proto message.
    fn to_proto(&self) -> AnkaiosRequest;

    /// Returns the unique identifier of the request.
    ///
    /// ## Returns
    ///
    /// A [String] containing the unique identifier of the request.
    fn get_id(&self) -> String;
}

/// Struct that represents a request to get the state of the [Ankaios] application.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug, PartialEq)]
pub struct GetStateRequest {
    /// The request proto message that will be sent to the cluster.
    #[allow(clippy::struct_field_names)]
    pub(crate) request: AnkaiosRequest,
    /// The unique identifier of the request.
    #[allow(clippy::struct_field_names)]
    request_id: String,
}

/// Struct that represents a request to update the state of the [Ankaios] application.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug, PartialEq)]
pub struct UpdateStateRequest {
    /// The request proto message that will be sent to the cluster.
    #[allow(clippy::struct_field_names)]
    pub(crate) request: AnkaiosRequest,
    /// The unique identifier of the request.
    #[allow(clippy::struct_field_names)]
    request_id: String,
}

impl GetStateRequest {
    /// Creates a new `GetStateRequest`.
    ///
    /// ## Arguments
    ///
    /// * `masks` - The field masks to be used for the request.
    ///
    /// ## Returns
    ///
    /// A new [`GetStateRequest`] object.
    pub fn new(masks: Vec<String>) -> Self {
        let request_id = Uuid::new_v4().to_string();
        log::debug!("Creating new request of type GetStateRequest with id {request_id}");

        Self {
            request: AnkaiosRequest {
                request_id: request_id.clone(),
                request_content: Some(RequestContent::CompleteStateRequest(CompleteStateRequest {
                    field_mask: masks,
                })),
            },
            request_id,
        }
    }
}

impl Request for GetStateRequest {
    fn to_proto(&self) -> AnkaiosRequest {
        self.request.clone()
    }

    fn get_id(&self) -> String {
        self.request_id.clone()
    }
}

impl fmt::Display for GetStateRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.to_proto())
    }
}

impl UpdateStateRequest {
    /// Creates a new `UpdateStateRequest`.
    ///
    /// ## Arguments
    ///
    /// * `complete_state` - The complete state to be set.
    /// * `masks` - The update masks to be used.
    ///
    /// ## Returns
    ///
    /// A new [`UpdateStateRequest`] object.
    pub fn new(complete_state: &CompleteState, masks: Vec<String>) -> Self {
        let request_id = Uuid::new_v4().to_string();
        log::debug!("Creating new request of type UpdateStateRequest with id {request_id}");

        let update_state_request = AnkaiosUpdateStateRequest {
            new_state: Some(complete_state.to_proto()),
            update_mask: masks,
        };

        Self {
            request: AnkaiosRequest {
                request_id: request_id.clone(),
                request_content: Some(RequestContent::UpdateStateRequest(Box::new(
                    update_state_request,
                ))),
            },
            request_id,
        }
    }
}

impl Request for UpdateStateRequest {
    fn to_proto(&self) -> AnkaiosRequest {
        self.request.clone()
    }

    fn get_id(&self) -> String {
        self.request_id.clone()
    }
}

impl fmt::Display for UpdateStateRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.to_proto())
    }
}

/// Struct that represents a request to request logs from the [Ankaios] application.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug, PartialEq)]
pub struct LogsRequest {
    /// The request proto message that will be sent to the cluster.
    #[allow(clippy::struct_field_names)]
    pub(crate) request: AnkaiosRequest,
    /// The unique identifier of the request.
    #[allow(clippy::struct_field_names)]
    request_id: String,
}

impl LogsRequest {
    /// Creates a new `GetStateRequest`.
    ///
    /// ## Arguments
    ///
    /// * `instance_names` - A [Vec] of [`WorkloadInstanceName`] for which to get logs;
    /// * `follow` - A [bool] indicating whether to continuously follow the logs;
    /// * `tail` - An [i32] indicating the number of lines to be output at the end of the logs;
    /// * `since` - An [Option<String>] to show logs after the timestamp in RFC3339 format;
    /// * `until` - An [Option<String>] to show logs before the timestamp in RFC3339 format.
    ///
    /// ## Returns
    ///
    /// A new [`LogsRequest`] object.
    pub fn new(
        instance_names: Vec<WorkloadInstanceName>,
        follow: bool,
        tail: i32,
        since: Option<String>,
        until: Option<String>,
    ) -> Self {
        let request_id = Uuid::new_v4().to_string();
        log::debug!("Creating new request of type LogsRequest with id {request_id}");

        Self {
            request: AnkaiosRequest {
                request_id: request_id.clone(),
                request_content: Some(RequestContent::LogsRequest(
                    ankaios_api::ank_base::LogsRequest {
                        workload_names: instance_names.into_iter().map(Into::into).collect(),
                        follow: Some(follow),
                        tail: Some(tail),
                        since,
                        until,
                    },
                )),
            },
            request_id,
        }
    }
}

impl Request for LogsRequest {
    fn to_proto(&self) -> AnkaiosRequest {
        self.request.clone()
    }

    fn get_id(&self) -> String {
        self.request_id.clone()
    }
}

impl fmt::Display for LogsRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.to_proto())
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
pub fn generate_test_request() -> impl Request {
    UpdateStateRequest::new(&CompleteState::default(), vec!["test_mask".to_owned()])
}

#[cfg(test)]
mod tests {
    use crate::ankaios_api;
    use ankaios_api::ank_base::Request as AnkaiosRequest;

    use super::{CompleteState, GetStateRequest, Request, UpdateStateRequest};

    #[allow(clippy::shadow_unrelated)]
    #[test]
    fn test_doc_examples() {
        // Create a request for updating the state
        let complete_state = CompleteState::new();
        let _request = UpdateStateRequest::new(&complete_state, Vec::default());

        // Create a request for getting the state
        let request = GetStateRequest::new(Vec::default());

        // Get the request ID
        let _request_id = request.get_id();

        // Create a request for getting the complete state filtered according to the provided field masks
        let _request = GetStateRequest::new(vec!["desiredState.workloads".to_owned()]);
    }

    #[test]
    fn utest_request_update_state() {
        let request = UpdateStateRequest::new(
            &CompleteState::default(),
            vec!["mask1".to_owned(), "mask2".to_owned()],
        );
        let id = request.get_id();

        assert_eq!(
            request.to_proto(),
            AnkaiosRequest {
                request_id: id,
                request_content: Some(
                    ankaios_api::ank_base::request::RequestContent::UpdateStateRequest(Box::new(
                        ankaios_api::ank_base::UpdateStateRequest {
                            new_state: Some(CompleteState::default().to_proto()),
                            update_mask: vec!["mask1".to_owned(), "mask2".to_owned()],
                        }
                    ))
                )
            }
        );

        assert_eq!(format!("{request}"), format!("{:?}", request.to_proto()));
    }

    #[test]
    fn utest_request_get_state() {
        let request = GetStateRequest::new(vec!["mask1".to_owned(), "mask2".to_owned()]);
        let id = request.get_id();

        assert_eq!(
            request.to_proto(),
            AnkaiosRequest {
                request_id: id,
                request_content: Some(
                    ankaios_api::ank_base::request::RequestContent::CompleteStateRequest(
                        ankaios_api::ank_base::CompleteStateRequest {
                            field_mask: vec!["mask1".to_owned(), "mask2".to_owned()],
                        }
                    )
                )
            }
        );

        assert_eq!(format!("{request}"), format!("{:?}", request.to_proto()));
    }
}
