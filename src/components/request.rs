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
use uuid::Uuid;
use api::ank_base::{Request as AnkaiosRequest, request::RequestContent, UpdateStateRequest, CompleteStateRequest};
use crate::AnkaiosError;
use crate::components::complete_state::CompleteState;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum RequestType {
    UpdateState = 0,
    GetState = 1,
}

pub struct Request{
    request: AnkaiosRequest,
    request_id: String,
    request_type: RequestType,
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let req_str = match self {
            RequestType::UpdateState => "UpdateState",
            RequestType::GetState => "GetState",
        };
        write!(f, "{}", req_str)
    }
}

impl Request {
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
                                update_mask: Default::default(),
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

    pub fn to_proto(&self) -> AnkaiosRequest {
        self.request.clone()
    }

    pub fn get_id(&self) -> String {
        self.request_id.clone()
    }

    pub fn set_complete_state(&mut self, complete_state: CompleteState) -> Result<(), AnkaiosError> {
        if self.request_type != RequestType::UpdateState {
            return Err(AnkaiosError::RequestError("Complete state can only be set for an update state request.".to_string()));
        }

        if let Some(RequestContent::UpdateStateRequest(ref mut update_state_request)) = self.request.request_content {
            update_state_request.new_state = Some(complete_state.to_proto());
        }
        Ok(())
    }

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

#[cfg(any(feature = "test_utils", test))]
pub fn generate_test_request() -> Request {
    let mut req = Request::new(RequestType::UpdateState);
    req.add_mask("test_mask".to_string());
    req
}

#[cfg(test)]
mod tests {
    use api::ank_base::Request as AnkaiosRequest;
    use super::{RequestType, Request, CompleteState};

    #[test]
    fn utest_request_type() {
        let mut request_type = RequestType::UpdateState;
        assert_eq!(format!("{}", request_type), "UpdateState");
        request_type = RequestType::GetState;
        assert_eq!(format!("{}", request_type), "GetState");
    }

    #[test]
    fn utest_request_update_state() {
        let mut request = Request::new(RequestType::UpdateState);
        let id = request.get_id();
        assert!(request.set_complete_state(Default::default()).is_ok());

        request.set_masks(vec!["mask1".to_string()]);
        request.add_mask("mask2".to_string());
        assert_eq!(request.to_proto(), AnkaiosRequest{
            request_id: id,
            request_content: Some(api::ank_base::request::RequestContent::UpdateStateRequest(
                Box::new(api::ank_base::UpdateStateRequest{
                    new_state: Some(CompleteState::default().to_proto()),
                    update_mask: vec!["mask1".to_string(), "mask2".to_string()],
                })
            ))
        });
    }

    #[test]
    fn utest_request_get_state() {
        let mut request = Request::new(RequestType::GetState);
        let id = request.get_id();
        assert!(request.set_complete_state(Default::default()).is_err());

        request.set_masks(vec!["mask1".to_string()]);
        request.add_mask("mask2".to_string());
        assert_eq!(request.to_proto(), AnkaiosRequest{
            request_id: id,
            request_content: Some(api::ank_base::request::RequestContent::CompleteStateRequest(
                api::ank_base::CompleteStateRequest{
                    field_mask: vec!["mask1".to_string(), "mask2".to_string()],
                }
            ))
        });

        assert_eq!(format!("{}", request), format!("{:?}", request.to_proto()));
    }
}