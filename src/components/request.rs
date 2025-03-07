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

//! This module contains the [`Request`] struct and the [`RequestType`] enum.
//! 
//! # Examples
//! 
//! ## Create a `Request` for updating the state:
//! 
//! ```rust
//! let complete_state = /* */;
//! let mut request = Request::new(RequestType::UpdateState);
//! request.set_complete_state(complete_state).unwrap();
//! ```
//! 
//! ## Create a `Request` for getting the state:
//! 
//! ```rust
//! let mut request = Request::new(RequestType::GetState);
//! ```
//! 
//! ## Get the request ID:
//! 
//! ```rust
//! let request_id = request.get_id();
//! ```
//!
//! ## Add a mask to the request:
//!
//! ```rust
//! request.add_mask("desiredState.workloads".to_owned());
//! ```

use std::fmt;
use uuid::Uuid;
use crate::ankaios_api;
use ankaios_api::ank_base::{Request as AnkaiosRequest, request::RequestContent, UpdateStateRequest, CompleteStateRequest};
use crate::AnkaiosError;
use crate::components::complete_state::CompleteState;

/// Enum that represents the type of request that can be made to the [Ankaios] application.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum RequestType {
    /// Request that updates the state of the cluster.
    UpdateState = 0,
    /// Request that gets the state of the cluster.
    GetState = 1,
}

/// Struct that represents a request that can be made to the [Ankaios] application.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug)]
pub struct Request{
    /// The request proto message that will be sent to the cluster.
    #[allow(clippy::struct_field_names)]
    request: AnkaiosRequest,
    /// The unique identifier of the request.
    #[allow(clippy::struct_field_names)]
    request_id: String,
    /// The type of request.
    #[allow(clippy::struct_field_names)]
    request_type: RequestType,
}

impl fmt::Display for RequestType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let req_str = match *self {
            RequestType::UpdateState => "UpdateState",
            RequestType::GetState => "GetState",
        };
        write!(f, "{req_str}")
    }
}

impl Request {
    /// Creates a new `Request` object.
    /// 
    /// ## Arguments
    /// 
    /// * `request_type` - The type of request to create.
    /// 
    /// ## Returns
    /// 
    /// A new [`Request`] object.
    pub fn new(request_type: RequestType) -> Self {
        let request_id = Uuid::new_v4().to_string();
        log::debug!("Creating new request of type {} with id {}", request_type, request_id);
        match request_type {
            RequestType::UpdateState => {
                Self{
                    request: AnkaiosRequest{
                        request_id: request_id.clone(),
                        request_content: Some(RequestContent::UpdateStateRequest(
                            Box::new(UpdateStateRequest{
                                new_state: None,
                                update_mask: Vec::default(),
                            })
                        )),
                    },
                    request_id,
                    request_type,
                }
            },
            RequestType::GetState => {
                Self{
                    request: AnkaiosRequest{
                        request_id: request_id.clone(),
                        request_content: Some(RequestContent::CompleteStateRequest(
                            CompleteStateRequest::default()
                        )),
                    },
                    request_id,
                    request_type,
                }
            },
        }
    }

    /// Returns the underlying [`AnkaiosRequest`] proto message.
    /// 
    /// ## Returns
    /// 
    /// The [`AnkaiosRequest`] proto message.
    pub fn to_proto(&self) -> AnkaiosRequest {
        self.request.clone()
    }

    /// Returns the unique identifier of the request.
    /// 
    /// ## Returns
    /// 
    /// A [String] containing the unique identifier of the request.
    pub fn get_id(&self) -> String {
        self.request_id.clone()
    }

    /// Sets the complete state of the request.
    /// 
    /// ## Arguments
    /// 
    /// * `complete_state` - The complete state to set.
    /// 
    /// ## Returns
    /// 
    /// An [`AnkaiosError`]::[`RequestError`](AnkaiosError::RequestError) if the
    /// request type is not [`RequestType::UpdateState`].
    pub fn set_complete_state(&mut self, complete_state: &CompleteState) -> Result<(), AnkaiosError> {
        if self.request_type != RequestType::UpdateState {
            return Err(AnkaiosError::RequestError("Complete state can only be set for an update state request.".to_owned()));
        }

        if let Some(RequestContent::UpdateStateRequest(ref mut update_state_request)) = self.request.request_content {
            update_state_request.new_state = Some(complete_state.to_proto());
        }
        Ok(())
    }

    /// Adds a mask to the request.
    /// 
    /// ## Arguments
    /// 
    /// * `mask` - A [String] containing the mask to add.
    pub fn add_mask(&mut self, mask: String) {
        match self.request_type {
            RequestType::UpdateState => {
                if let Some(RequestContent::UpdateStateRequest(ref mut update_state_request)) = self.request.request_content {
                    update_state_request.update_mask.push(mask);
                }
            },
            RequestType::GetState => {
                if let Some(RequestContent::CompleteStateRequest(ref mut complete_state_request)) = self.request.request_content {
                    complete_state_request.field_mask.push(mask);
                }
            },
        }
    }

    /// Sets the masks of the request.
    /// 
    /// ## Arguments
    /// 
    /// * `masks` - A [Vec] of [Strings](String) containing the masks to set.
    pub fn set_masks(&mut self, masks: Vec<String>) {
        match self.request_type {
            RequestType::UpdateState => {
                if let Some(RequestContent::UpdateStateRequest(ref mut update_state_request)) = self.request.request_content {
                    update_state_request.update_mask = masks;
                }
            },
            RequestType::GetState => {
                if let Some(RequestContent::CompleteStateRequest(ref mut complete_state_request)) = self.request.request_content {
                    complete_state_request.field_mask = masks;
                }
            },
        }
    }
}

impl fmt::Display for Request {
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
pub fn generate_test_request() -> Request {
    let mut req = Request::new(RequestType::UpdateState);
    req.add_mask("test_mask".to_owned());
    req
}

#[cfg(test)]
mod tests {
    use crate::ankaios_api;
    use ankaios_api::ank_base::Request as AnkaiosRequest;

    use super::{RequestType, Request, CompleteState};

    #[allow(clippy::shadow_unrelated)]
    #[test]
    fn test_doc_examples() {
        // Create a `Request` for updating the state
        let complete_state = CompleteState::new();
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&complete_state).unwrap();

        // Create a `Request` for getting the state
        let mut request = Request::new(RequestType::GetState);

        // Get the request ID
        let _request_id = request.get_id();

        // Add a mask to the request
        request.add_mask("desiredState.workloads".to_owned());
    }

    #[test]
    fn utest_request_type() {
        let mut request_type = RequestType::UpdateState;
        assert_eq!(format!("{request_type}"), "UpdateState");
        request_type = RequestType::GetState;
        assert_eq!(format!("{request_type}"), "GetState");
    }

    #[test]
    fn utest_request_update_state() {
        let mut request = Request::new(RequestType::UpdateState);
        let id = request.get_id();
        assert!(request.set_complete_state(&CompleteState::default()).is_ok());

        request.set_masks(vec!["mask1".to_owned()]);
        request.add_mask("mask2".to_owned());
        assert_eq!(request.to_proto(), AnkaiosRequest{
            request_id: id,
            request_content: Some(ankaios_api::ank_base::request::RequestContent::UpdateStateRequest(
                Box::new(ankaios_api::ank_base::UpdateStateRequest{
                    new_state: Some(CompleteState::default().to_proto()),
                    update_mask: vec!["mask1".to_owned(), "mask2".to_owned()],
                })
            ))
        });
    }

    #[test]
    fn utest_request_get_state() {
        let mut request = Request::new(RequestType::GetState);
        let id = request.get_id();
        assert!(request.set_complete_state(&CompleteState::default()).is_err());

        request.set_masks(vec!["mask1".to_owned()]);
        request.add_mask("mask2".to_owned());
        assert_eq!(request.to_proto(), AnkaiosRequest{
            request_id: id,
            request_content: Some(ankaios_api::ank_base::request::RequestContent::CompleteStateRequest(
                ankaios_api::ank_base::CompleteStateRequest{
                    field_mask: vec!["mask1".to_owned(), "mask2".to_owned()],
                }
            ))
        });

        assert_eq!(format!("{request}"), format!("{:?}", request.to_proto()));
    }
}