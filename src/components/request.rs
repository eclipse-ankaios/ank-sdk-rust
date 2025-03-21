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
//! let _request = UpdateStateRequest::new(&complete_state);
//! ```
//! 
//! ## Create a request for getting the state:
//! 
//! ```rust
//! let mut request = GetStateRequest::new();
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
use ankaios_api::ank_base::{Request as AnkaiosRequest, request::RequestContent, UpdateStateRequest as AnkaiosUpdateStateRequest, CompleteStateRequest};
use crate::components::complete_state::CompleteState;

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

    /// Adds a mask to the request.
    /// 
    /// ## Arguments
    /// 
    /// * `mask` - A [String] containing the mask to add.
    fn add_mask(&mut self, mask: String);

    /// Sets the masks of the request.
    /// 
    /// ## Arguments
    /// 
    /// * `masks` - A [Vec] of [Strings](String) containing the masks to set.
    fn set_masks(&mut self, masks: Vec<String>);
}

/// Struct that represents a request to get the state of the [Ankaios] application.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug)]
pub struct GetStateRequest{
    /// The request proto message that will be sent to the cluster.
    #[allow(clippy::struct_field_names)]
    request: AnkaiosRequest,
    /// The unique identifier of the request.
    #[allow(clippy::struct_field_names)]
    request_id: String,
}

/// Struct that represents a request to update the state of the [Ankaios] application.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug)]
pub struct UpdateStateRequest{
    /// The request proto message that will be sent to the cluster.
    #[allow(clippy::struct_field_names)]
    request: AnkaiosRequest,
    /// The unique identifier of the request.
    #[allow(clippy::struct_field_names)]
    request_id: String,
}

impl GetStateRequest {
    /// Creates a new `GetStateRequest`.
    /// 
    /// ## Returns
    /// 
    /// A new [`GetStateRequest`] object.
    pub fn new() -> Self {
        let request_id = Uuid::new_v4().to_string();
        log::debug!("Creating new request of type GetStateRequest with id {}", request_id);

        Self{
            request: AnkaiosRequest{
                request_id: request_id.clone(),
                request_content: Some(RequestContent::CompleteStateRequest(
                    CompleteStateRequest::default()
                )),
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

    fn add_mask(&mut self, mask: String) {
        if let Some(RequestContent::CompleteStateRequest(ref mut complete_state_request)) = self.request.request_content {
            complete_state_request.field_mask.push(mask);
        }
    }

    fn set_masks(&mut self, masks: Vec<String>) {
        if let Some(RequestContent::CompleteStateRequest(ref mut complete_state_request)) = self.request.request_content {
            complete_state_request.field_mask = masks;
        }
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
    /// ## Returns
    /// 
    /// A new [`UpdateStateRequest`] object.
    pub fn new(complete_state: &CompleteState) -> Self {
        let request_id = Uuid::new_v4().to_string();
        log::debug!("Creating new request of type UpdateStateRequest with id {}", request_id);

        let update_state_request = AnkaiosUpdateStateRequest {
            new_state: Some(complete_state.to_proto()),
            update_mask: vec![],
        };

        Self{
            request: AnkaiosRequest{
                request_id: request_id.clone(),
                request_content: Some(RequestContent::UpdateStateRequest(
                    Box::new(update_state_request)
                )),
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

    fn add_mask(&mut self, mask: String) {
        if let Some(RequestContent::UpdateStateRequest(ref mut update_state_request)) = self.request.request_content {
            update_state_request.update_mask.push(mask);
        }
    }

    fn set_masks(&mut self, masks: Vec<String>) {
        if let Some(RequestContent::UpdateStateRequest(ref mut update_state_request)) = self.request.request_content {
            update_state_request.update_mask = masks;
        }
    }
}

impl fmt::Display for UpdateStateRequest {
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
    let mut req = UpdateStateRequest::new(&CompleteState::default());
    req.add_mask("test_mask".to_owned());
    req
}

#[cfg(test)]
mod tests {
    use crate::ankaios_api;
    use ankaios_api::ank_base::Request as AnkaiosRequest;

    use super::{Request, GetStateRequest, UpdateStateRequest, CompleteState};

    #[allow(clippy::shadow_unrelated)]
    #[test]
    fn test_doc_examples() {
        // Create a request for updating the state
        let complete_state = CompleteState::new();
        let _request = UpdateStateRequest::new(&complete_state);

        // Create a request for getting the state
        let mut request = GetStateRequest::new();

        // Get the request ID
        let _request_id = request.get_id();

        // Add a mask to the request
        request.add_mask("desiredState.workloads".to_owned());
    }

    #[test]
    fn utest_request_update_state() {
        let mut request = UpdateStateRequest::new(&CompleteState::default());
        let id = request.get_id();

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

        assert_eq!(format!("{request}"), format!("{:?}", request.to_proto()));
    }

    #[test]
    fn utest_request_get_state() {
        let mut request = GetStateRequest::new();
        let id = request.get_id();

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