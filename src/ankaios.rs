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

use std::collections::HashMap;

use tokio::sync::mpsc;
use tokio::time::{timeout as tokio_timeout, Duration};

use crate::components::request::{Request, RequestType};
use crate::components::response::{Response, ResponseType, UpdateStateSuccess};
use crate::components::workload_mod::Workload;
use crate::{AnkaiosError, CompleteState, Manifest};
use crate::components::control_interface::{ControlInterface, ControlInterfaceState};
use crate::components::workload_state_mod::{WorkloadInstanceName, WorkloadState, WorkloadStateCollection};

const WORKLOADS_PREFIX: &str = "desiredState.workloads";
const CONFIGS_PREFIX: &str = "desiredState.configs";
const DEFAULT_TIMEOUT: u64 = 5;  // seconds

pub struct Ankaios{
    response_receiver: mpsc::Receiver<Response>,
    state_changed_receiver_handler: Option<tokio::task::JoinHandle<Result<(), AnkaiosError>>>,
    control_interface: ControlInterface,
}

impl Ankaios {
    pub async fn new() -> Result<Self, AnkaiosError> {
        let (request_sender, response_receiver) = mpsc::channel::<Response>(100);
        let (state_changed_sender, mut state_changed_receiver) = mpsc::channel::<ControlInterfaceState>(100);
        let mut object = Self{
            response_receiver,
            state_changed_receiver_handler: None,
            control_interface: ControlInterface::new(request_sender, state_changed_sender),
        };

        object.state_changed_receiver_handler = Some(tokio::spawn(async move {
            loop {
                match state_changed_receiver.recv().await {
                    Some(state) => {
                        log::info!("State changed: {:?}", state);
                    },
                    None => {
                        log::error!("State changed receiver closed unexpectedly.");
                        return Err(AnkaiosError::ControlInterfaceError("State changed receiver closed.".to_string()));
                    },
                }
            }
        }));

        match object.control_interface.connect().await {
            Ok(_) => {
                Ok(object)
            },
            Err(err) => {
                Err(err)
            }
        }
    }

    pub async fn state(&mut self) -> ControlInterfaceState {
        self.control_interface.state()
    }

    async fn send_request(&mut self, request: Request, timeout: Option<Duration>) -> Result<Response, AnkaiosError> {
        let request_id = request.get_id();
        self.control_interface.write_request(request);
        // Check if the response is correct
        // check id: request_id
        // check if the response is connection closed
        // if not correct id => warn
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT));
        loop {
            match tokio_timeout(timeout_duration, self.response_receiver.recv()).await {
                Ok(Some(response)) => {
                    if let ResponseType::ConnectionClosedReason(reason) = response.content {
                        log::error!("Connection closed: {}", reason);
                        return Err(AnkaiosError::ConnectionClosedError(reason));
                    }
                    if response.get_request_id() == request_id {
                        return Ok(response);
                    } else {
                        log::warn!("Received response with wrong id.");
                    }
                },
                Ok(None) => {
                    log::error!("Reading thread closed unexpectedly.");
                    return Err(AnkaiosError::ControlInterfaceError("Reading thread closed.".to_string()));
                },
                Err(err) => {
                    log::error!("Timeout while waiting for response.");
                    return Err(AnkaiosError::TimeoutError(err));
                },
            }
        }
    }

    pub async fn apply_manifest(&mut self, manifest:Manifest, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let mut request: Request = Request::new(RequestType::UpdateState);
        request.set_complete_state(CompleteState::new_from_manifest(&manifest)).unwrap();
        request.set_masks(manifest.calculate_masks());

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!("Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(), update_state_success.deleted_workloads.len());
                Ok(*update_state_success)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to apply manifest: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn delete_manifest(&mut self, manifest:Manifest, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let mut request: Request = Request::new(RequestType::UpdateState);
        request.set_complete_state(CompleteState::default()).unwrap();
        request.set_masks(manifest.calculate_masks());

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!("Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(), update_state_success.deleted_workloads.len());
                Ok(*update_state_success)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete manifest: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn apply_workload(&mut self, workload:Workload, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        let masks = workload.masks.clone();

        // Create CompleteState
        let mut complete_state = CompleteState::default();
        complete_state.add_workload(workload);

        // Create request
        let mut request: Request = Request::new(RequestType::UpdateState);
        request.set_complete_state(complete_state).unwrap();
        request.set_masks(masks);

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!("Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(), update_state_success.deleted_workloads.len());
                Ok(*update_state_success)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to apply workload: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn get_workload(&mut self, workload_name: String, timeout: Option<Duration>) -> Result<Workload, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![format!("{}.{}", WORKLOADS_PREFIX, workload_name)]), timeout).await?;
        Ok(complete_state.get_workloads()[0].clone())
    }

    pub async fn delete_workload(&mut self, workload:Workload, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let mut request: Request = Request::new(RequestType::UpdateState);
        request.set_complete_state(CompleteState::default()).unwrap();
        request.add_mask(format!("{}.{}",WORKLOADS_PREFIX, workload.name));

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!("Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(), update_state_success.deleted_workloads.len());
                Ok(*update_state_success)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete workload: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn update_configs(&mut self, configs: HashMap<String, serde_yaml::Value>, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create CompleteState
        let mut complete_state = CompleteState::default();
        complete_state.set_configs(configs);

        // Create request
        let mut request: Request = Request::new(RequestType::UpdateState);
        request.set_complete_state(complete_state).unwrap();
        request.add_mask(CONFIGS_PREFIX.to_string());

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!("Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(), update_state_success.deleted_workloads.len());
                Ok(*update_state_success)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to update configs: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn add_config(&mut self, name: String, configs: serde_yaml::Value, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create CompleteState
        let mut complete_state = CompleteState::default();
        complete_state.set_configs(HashMap::from([(name, configs)]));

        // Create request
        let mut request: Request = Request::new(RequestType::UpdateState);
        request.set_complete_state(complete_state).unwrap();
        request.add_mask(CONFIGS_PREFIX.to_string());

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!("Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(), update_state_success.deleted_workloads.len());
                Ok(*update_state_success)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to add the config: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn get_configs(&mut self, timeout: Option<Duration>) -> Result<HashMap<String, serde_yaml::Value>, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![WORKLOADS_PREFIX.to_string()]), timeout).await?;
        Ok(complete_state.get_configs())
    }

    pub async fn get_config(&mut self, name: String, timeout: Option<Duration>) -> Result<HashMap<String, serde_yaml::Value>, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![format!("{}.{}", CONFIGS_PREFIX, name)]), timeout).await?;
        Ok(complete_state.get_configs())
    }

    pub async fn delete_all_configs(&mut self, timeout: Option<Duration>) -> Result<(), AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(CompleteState::default()).unwrap();
        request.add_mask(CONFIGS_PREFIX.to_string());

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(_) => {
                log::info!("Update successful");
                Ok(())
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete all configs: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn delete_config(&mut self, name: String, timeout: Option<Duration>) -> Result<(), AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(CompleteState::default()).unwrap();
        request.add_mask(format!("{}.{}", CONFIGS_PREFIX, name));

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(_) => {
                log::info!("Update successful");
                Ok(())
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete config: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn get_state(&mut self, field_masks: Option<Vec<String>>, timeout: Option<Duration>) -> Result<CompleteState, AnkaiosError> {
        // Create request
        let mut request: Request = Request::new(RequestType::GetState);
        if let Some(masks) = field_masks {
            request.set_masks(masks);
        }

        // Wait for the response
        let response = self.send_request(request, timeout).await?;

        match response.content {
            ResponseType::CompleteState(complete_state) => {
                Ok(*complete_state)
            },
            ResponseType::Error(error) => {
                log::error!("Error while trying to get the state: {}", error);
                Err(AnkaiosError::AnkaiosError(error))
            },
            _ => {
                log::error!("Received wrong response type.");
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_string()))
            }
        }
    }

    pub async fn get_agents(&mut self, timeout: Option<Duration>) -> Result<HashMap<String, HashMap<String, String>>, AnkaiosError> {
        let complete_state = self.get_state(None, timeout).await?;
        Ok(complete_state.get_agents())
    }

    pub async fn get_workload_states(&mut self, timeout: Option<Duration>) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self.get_state(None, timeout).await?;
        Ok(complete_state.get_workload_states().clone())
    }

    pub async fn get_execution_state_for_instance_name(&mut self, instance_name: WorkloadInstanceName, timeout: Option<Duration>) -> Result<WorkloadState, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![instance_name.get_filter_mask()]), timeout).await?;
        let workload_states = complete_state.get_workload_states().get_as_list();
        if workload_states.is_empty() {
            return Err(AnkaiosError::AnkaiosError("No workload states found.".to_string()));
        }
        Ok(workload_states[0].clone())
    }

    pub async fn get_workload_states_on_agent(&mut self, agent_name: String, timeout: Option<Duration>) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![format!("workloadStates.{}", agent_name)]), timeout).await?;
        Ok(complete_state.get_workload_states().clone())
    }

    pub async fn get_workload_states_for_name(&mut self, workload_name: String, timeout: Option<Duration>) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self.get_state(Some(vec!["workloadStates".to_string()]), timeout).await?;
        let mut workload_states_for_name = WorkloadStateCollection::new();
        for workload_state in complete_state.get_workload_states().get_as_list() {
            if workload_state.workload_instance_name.workload_name == workload_name {
                workload_states_for_name.add_workload_state(workload_state.clone());
            }
        }
        Ok(workload_states_for_name)
    }
}

impl Drop for Ankaios {
    fn drop(&mut self) {
        self.control_interface.disconnect().unwrap_or_else(|err| {
            log::error!("Error while disconnecting: '{}'", err);
        });
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
}