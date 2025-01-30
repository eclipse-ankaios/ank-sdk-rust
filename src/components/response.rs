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

use core::fmt;
use std::collections::HashMap;
use std::default;
use api::ank_base::{response::ResponseContent as AnkaiosResponseContent, UpdateStateSuccess as AnkaiosUpdateStateSuccess, Error};
use api::control_api::{FromAnkaios, from_ankaios::FromAnkaiosEnum};
use crate::components::complete_state::CompleteState;

use super::workload_state_mod::WorkloadInstanceName;


#[derive(Clone, Debug)]
pub enum ResponseType {
    CompleteState(Box<CompleteState>),
    UpdateStateSuccess(Box<UpdateStateSuccess>),
    Error(String),
    ConnectionClosedReason(String),
}

#[derive(Default, Clone, Debug)]
pub struct Response{
    pub content: ResponseType,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct UpdateStateSuccess {
    pub added_workloads: Vec<WorkloadInstanceName>,
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

    pub fn get_request_id(&self) -> String {
        self.id.clone()
    }

    #[allow(dead_code)]
    pub fn get_content(&self) -> ResponseType {
        self.content.clone()
    }
}


impl UpdateStateSuccess{
    pub fn new_from_proto(update_state_success: AnkaiosUpdateStateSuccess) -> Self {
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
mod tests {
    use super::{Response, UpdateStateSuccess};
    use api::ank_base::UpdateStateSuccess as AnkaiosUpdateStateSuccess;
    use api::control_api::FromAnkaios;

    #[test]
    fn test_response() {
        let _ = Response::new(FromAnkaios::default());
    }

    #[test]
    fn test_update_state_success() {
        let _ = UpdateStateSuccess::new_from_proto(AnkaiosUpdateStateSuccess::default());
    }
}