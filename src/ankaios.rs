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

//! This module contains the definition of the `Ankaios` struct, which
//! represents the main interface to the [Ankaios] application.
//! 
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

use std::collections::HashMap;
use std::vec;
use tokio::sync::mpsc;
use tokio::time::{timeout as tokio_timeout, Duration, sleep};

use crate::components::request::{Request, RequestType};
use crate::components::response::{Response, ResponseType, UpdateStateSuccess};
use crate::components::workload_mod::Workload;
use crate::{AnkaiosError, CompleteState, Manifest};
#[cfg_attr(test, mockall_double::double)]
use crate::components::control_interface::ControlInterface;
use crate::components::control_interface::ControlInterfaceState;
use crate::components::workload_state_mod::{WorkloadInstanceName, WorkloadState, WorkloadStateCollection, WorkloadStateEnum};

/// The prefix for the workloads in the desired state.
const WORKLOADS_PREFIX: &str = "desiredState.workloads";
/// The prefix for the configs in the desired state.
const CONFIGS_PREFIX: &str = "desiredState.configs";
/// The default timeout, if not manually provided.
const DEFAULT_TIMEOUT: u64 = 5;  // seconds

/// This struct is used to interact with the [Ankaios] using an intuitive API.
/// The struct automatically handles the session creation and the requests
/// and responses sent and received over the Control Interface.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
/// 
/// # Examples
/// 
/// ## Create an Ankaios object, connect and disconnect from the control interface:
/// 
/// ```rust
/// let ankaios = Ankaios::new().await.unwrap();
/// /* */
/// drop(ankaios);
/// ```
/// 
/// ## Apply a manifest:
/// 
/// ```rust
/// let manifest = /* */;
/// let update_state_success = ankaios.apply_manifest(manifest, None).await.unwrap();
/// println!("{:?}", update_state_success);
/// ```
/// 
/// ## Delete a manifest:
/// 
/// ```rust
/// let manifest = /* */;
/// let update_state_success = ankaios.delete_manifest(manifest, None).await.unwrap();
/// println!("{:?}", update_state_success);
/// ```
/// 
/// ## Run a workload:
/// 
/// ```rust
/// let workload = /* */;
/// let update_state_success = ankaios.apply_workload(workload, None).await.unwrap();
/// println!("{:?}", update_state_success);
/// ```
/// 
/// ## Get a workload:
/// 
/// ```rust
/// let workload_name: String = /* */;
/// let workload = ankaios.get_workload(workload_name, None).await.unwrap();
/// println!("{:?}", workload);
/// ```
/// 
/// ## Delete a workload:
/// 
/// ```rust
/// let workload_name: String = /* */;
/// let update_state_success = ankaios.delete_workload(workload_name, None).await.unwrap();
/// println!("{:?}", update_state_success);
/// ```
/// 
/// ## Get the state:
/// 
/// ```rust
/// let state = ankaios.get_state(None, None).await.unwrap();
/// println!("{:?}", state);
/// ```
/// 
/// ## Get the agents:
/// 
/// ```rust
/// let agents = ankaios.get_agents(None).await.unwrap();
/// println!("{:?}", agents);
/// ```
/// 
/// ## Get the workload states:
/// 
/// ```rust
/// let workload_states_collection = ankaios.get_workload_states(None).await.unwrap();
/// let workload_states = workload_states_collection.get_as_list();
/// ```
/// 
/// ## Get the workload states for a specific agent:
/// 
/// ```rust
/// let agent_name: String = /* */;
/// let workload_states_collection = ankaios.get_workload_states_on_agent(agent_name, None).await.unwrap();
/// let workload_states = workload_states_collection.get_as_list();
/// ```
/// 
/// ## Get the workload execution state for an instance name:
/// 
/// ```rust
/// let workload_instance_name: WorkloadInstanceName = /* */;
/// let workload_state = ankaios.get_execution_state_for_instance_name(workload_instance_name, None).await.unwrap();
/// println!("{:?}", workload_state);
/// ```
/// 
/// ## Wait for a workload to reach a state:
/// 
/// ```rust
/// let workload_instance_name: WorkloadInstanceName = /* */;
/// let expected_state: WorkloadStateEnum = /* */;
/// match ankaios.wait_for_workload_to_reach_state(workload_instance_name, expected_state, None).await {
///     Ok(_) => println!("Workload reached the expected state."),
///     Err(AnkaiosError::TimeoutError(_)) => println!("Timeout while waiting for workload to reach state."),
///     Err(err) => println!("Error while waiting for workload to reach state: {}", err),
/// }
/// ```
pub struct Ankaios{
    /// The receiver end of the channel used to receive responses from the Control Interface.
    response_receiver: mpsc::Receiver<Response>,
    /// The control interface instance that is used to communicate with the Control Interface.
    control_interface: ControlInterface,
}

impl Ankaios {
    /// Creates a new `Ankaios` object and connects to the Control Interface.
    /// 
    /// ## Returns
    /// 
    /// A [Result] containing the [Ankaios] object if the connection was successful.
    /// 
    /// ## Errors
    /// 
    /// [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if an error occurred when connecting.
    pub async fn new() -> Result<Self, AnkaiosError> {
        let (response_sender, response_receiver) = mpsc::channel::<Response>(100);
        let mut object = Self{
            response_receiver,
            control_interface: ControlInterface::new(response_sender),
        };

        object.control_interface.connect().await?;

        // Test connection
        object.get_state(Some(vec!["desiredState.apiVersion".to_owned()]), None).await?;

        Ok(object)
    }

    /// Returns the current state of the Control Interface.
    /// 
    /// ## Returns
    /// 
    /// The [`ControlInterfaceState`] of the Control Interface.
    #[inline]
    pub fn state(&mut self) -> ControlInterfaceState {
        self.control_interface.state()
    }

    /// Sends a request to the Control Interface and waits for the response.
    /// 
    /// ## Arguments
    /// 
    /// - `request`: The [`Request`] to be sent;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - the [Response] if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    async fn send_request(&mut self, request: Request, timeout: Option<Duration>) -> Result<Response, AnkaiosError> {
        let request_id = request.get_id();
        self.control_interface.write_request(request).await?;
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT));
        loop {
            #[allow(non_snake_case)] // False positive: None is an optional, not a variable, so it's ok to not be snake_case.
            match tokio_timeout(timeout_duration, self.response_receiver.recv()).await {
                Ok(Some(response)) => {
                    if let ResponseType::ConnectionClosedReason(reason) = response.content {
                        log::error!("Connection closed: {}", reason);
                        return Err(AnkaiosError::ConnectionClosedError(reason));
                    }
                    if response.get_request_id() == request_id {
                        return Ok(response);
                    }
                    log::warn!("Received response with wrong id.");
                },
                Ok(None) => {
                    log::error!("Reading thread closed unexpectedly.");
                    return Err(AnkaiosError::ControlInterfaceError("Reading thread closed.".to_owned()));
                },
                Err(err) => {
                    log::error!("Timeout while waiting for response.");
                    return Err(AnkaiosError::TimeoutError(err));
                },
            }
        }
    }

    /// Send a request to apply a [Manifest].
    /// 
    /// ## Arguments
    /// 
    /// - `manifest`: The [Manifest] to be applied;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - an [`UpdateStateSuccess`] containing the number of added and deleted workloads if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn apply_manifest(&mut self, manifest: Manifest, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&CompleteState::new_from_manifest(&manifest)).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to delete a [Manifest].
    /// 
    /// ## Arguments
    /// 
    /// - `manifest`: The [Manifest] to be deleted;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - an [`UpdateStateSuccess`] containing the number of added and deleted workloads if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_manifest(&mut self, manifest: Manifest, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&CompleteState::default()).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to run a [Workload].
    /// 
    /// ## Arguments
    /// 
    /// - `workload`: The [Workload] to be run;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - an [`UpdateStateSuccess`] containing the number of added and deleted workloads if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn apply_workload(&mut self, workload: Workload, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        let masks = workload.masks.clone();

        // Create CompleteState
        let mut complete_state = CompleteState::default();
        complete_state.add_workload(workload);

        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&complete_state).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to get the [Workload] with the given name.
    /// If there are multiple workloads with the same name, only the first one is returned.
    /// 
    /// ## Arguments
    /// 
    /// - `workload_name`: A [String] containing the name of the workload to get;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [Workload] object if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload(&mut self, workload_name: String, timeout: Option<Duration>) -> Result<Workload, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![format!("{}.{}", WORKLOADS_PREFIX, workload_name)]), timeout).await?;
        Ok(complete_state.get_workloads()[0].clone())
    }

    /// Send a request to delete a workload.
    /// 
    /// ## Arguments
    /// 
    /// - `workload_name`: A [String] containing the name of the workload to get;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [Workload] object if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_workload(&mut self, workload_name: String, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&CompleteState::default()).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
        request.add_mask(format!("{WORKLOADS_PREFIX}.{workload_name}"));

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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to update the configs
    /// 
    /// ## Arguments
    /// 
    /// - `configs`: A [`HashMap`] containing the configs to be updated;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - an [`UpdateStateSuccess`] object if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn update_configs(&mut self, configs: HashMap<String, serde_yaml::Value>, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create CompleteState
        let mut complete_state = CompleteState::default();
        complete_state.set_configs(configs);

        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&complete_state).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
        request.add_mask(CONFIGS_PREFIX.to_owned());

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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to add a config with the provided name.
    /// If the config exists, it will be replaced.
    /// 
    /// ## Arguments
    /// 
    /// - `name`: A [String] containing the name of the config to be added;
    /// - `configs`: A [`serde_yaml::Value`] containing the configs to be added;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - an [`UpdateStateSuccess`] object if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn add_config(&mut self, name: String, configs: serde_yaml::Value, timeout: Option<Duration>) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create CompleteState
        let mut complete_state = CompleteState::default();
        complete_state.set_configs(HashMap::from([(name, configs)]));

        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&complete_state).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
        request.add_mask(CONFIGS_PREFIX.to_owned());

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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to get all the configs.
    /// 
    /// ## Arguments
    /// 
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`HashMap`] containing the configs if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_configs(&mut self, timeout: Option<Duration>) -> Result<HashMap<String, serde_yaml::Value>, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![WORKLOADS_PREFIX.to_owned()]), timeout).await?;
        Ok(complete_state.get_configs())
    }

    /// Send a request to get the config with the provided name.
    /// 
    /// ## Arguments
    /// 
    /// - `name`: A [String] containing the name of the config;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`HashMap`] containing the config if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_config(&mut self, name: String, timeout: Option<Duration>) -> Result<HashMap<String, serde_yaml::Value>, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![format!("{}.{}", CONFIGS_PREFIX, name)]), timeout).await?;
        Ok(complete_state.get_configs())
    }

    /// Send a request to delete all the configs.
    /// 
    /// ## Arguments
    /// 
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_all_configs(&mut self, timeout: Option<Duration>) -> Result<(), AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&CompleteState::default()).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
        request.add_mask(CONFIGS_PREFIX.to_owned());

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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to delete the config with the provided name.
    /// 
    /// ## Arguments
    /// 
    /// - `name`: A [String] containing the name of the config;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_config(&mut self, name: String, timeout: Option<Duration>) -> Result<(), AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::UpdateState);
        request.set_complete_state(&CompleteState::default()).unwrap_or_else(|err| {
            log::error!("Error while setting the complete state: {}", err);
        });
        request.add_mask(format!("{CONFIGS_PREFIX}.{name}"));

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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to get the [complete state](CompleteState).
    /// 
    /// ## Arguments
    /// 
    /// - `field_masks`: A [Vec] of [String]s containing the field masks to be used in the request;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`CompleteState`] object containing the state of the cluster.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_state(&mut self, field_masks: Option<Vec<String>>, timeout: Option<Duration>) -> Result<CompleteState, AnkaiosError> {
        // Create request
        let mut request = Request::new(RequestType::GetState);
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
                Err(AnkaiosError::ResponseError("Received wrong response type.".to_owned()))
            }
        }
    }

    /// Send a request to get the agents.
    /// 
    /// ## Arguments
    /// 
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`HashMap`] containing the agents if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_agents(&mut self, timeout: Option<Duration>) -> Result<HashMap<String, HashMap<String, String>>, AnkaiosError> {
        let complete_state = self.get_state(None, timeout).await?;
        Ok(complete_state.get_agents())
    }

    /// Send a request to get the workload states
    /// 
    /// ## Arguments
    /// 
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`WorkloadStateCollection`] containing the workload states if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload_states(&mut self, timeout: Option<Duration>) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self.get_state(None, timeout).await?;
        Ok(complete_state.get_workload_states().clone())
    }

    /// Send a request to get the execution state for an instance name.
    /// 
    /// ## Arguments
    /// 
    /// - `instance_name`: The [`WorkloadInstanceName`] to get the execution state for;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - the requested [`WorkloadState`] for the provided instance name.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_execution_state_for_instance_name(&mut self, instance_name: WorkloadInstanceName, timeout: Option<Duration>) -> Result<WorkloadState, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![instance_name.get_filter_mask()]), timeout).await?;
        let workload_states = complete_state.get_workload_states().get_as_list();
        #[allow(non_snake_case)] // False positive: None is an optional, not a variable, so it's ok to not be snake_case.
        match workload_states.first() {
            Some(workload_state) => Ok(workload_state.clone()),
            None => Err(AnkaiosError::AnkaiosError("No workload states found.".to_owned()))
        }
    }

    /// Send a request to get the workload states for the workloads running on a specific agent.
    /// 
    /// ## Arguments
    /// 
    /// - `agent_name`: A [String] containing the name of the agent to get the workload states for;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`WorkloadStateCollection`] containing the workload states if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload_states_on_agent(&mut self, agent_name: String, timeout: Option<Duration>) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self.get_state(Some(vec![format!("workloadStates.{}", agent_name)]), timeout).await?;
        Ok(complete_state.get_workload_states().clone())
    }

    /// Send a request to get the workload states for the workloads with a specific name.
    /// 
    /// ## Arguments
    /// 
    /// - `workload_name`: A [String] containing the name of the workloads to get the states for;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Returns
    /// 
    /// - a [`WorkloadStateCollection`] containing the workload states if the request was successful.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload_states_for_name(&mut self, workload_name: String, timeout: Option<Duration>) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self.get_state(Some(vec!["workloadStates".to_owned()]), timeout).await?;
        let mut workload_states_for_name = WorkloadStateCollection::new();
        for workload_state in complete_state.get_workload_states().get_as_list() {
            if workload_state.workload_instance_name.workload_name == workload_name {
                workload_states_for_name.add_workload_state(workload_state.clone());
            }
        }
        Ok(workload_states_for_name)
    }

    /// Waits for the workload to reach the specified state.
    /// 
    /// ## Arguments
    /// 
    /// - `instance_name`: The [`WorkloadInstanceName`] to wait for;
    /// - `state`: The [`WorkloadStateEnum`] to wait for;
    /// - `timeout`: The maximum time to wait for the response. If `None`, the default timeout is used.
    /// 
    /// ## Errors
    /// 
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response or waiting for the state to be reached.
    /// - [`AnkaiosError`]::[`AnkaiosError`](AnkaiosError::AnkaiosError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn wait_for_workload_to_reach_state(&mut self, instance_name: WorkloadInstanceName, state: WorkloadStateEnum, timeout: Option<Duration>) -> Result<(), AnkaiosError> {
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT));

        let poll_future = async {
            loop {
                let workload_state = self.get_execution_state_for_instance_name(instance_name.clone(), None).await?;
                if workload_state.execution_state.state == state {
                    return Ok(());
                }

                sleep(Duration::from_millis(100)).await;
            }
        };

        match tokio_timeout(timeout_duration, poll_future).await {
            Ok(Ok(())) => {
                Ok(())
            },
            Ok(Err(err)) => {
                log::error!("Error while waiting for workload to reach state: {}", err);
                Err(err)
            },
            Err(err) => {
                log::error!("Timeout while waiting for workload to reach state: {}", err);
                Err(AnkaiosError::TimeoutError(err))
            },
        }
    }
}

impl Drop for Ankaios {
    fn drop(&mut self) {
        log::trace!("Dropping Ankaios");
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
pub fn generate_test_ankaios(mock_control_interface: ControlInterface) -> (Ankaios, mpsc::Sender<Response>) {
    let (response_sender, response_receiver) = mpsc::channel::<Response>(100);
    (Ankaios{
        response_receiver,
        control_interface: mock_control_interface,
    },
    response_sender)
}

#[cfg(test)]
mod tests {
    use mockall::lazy_static;
    use tokio::{
        time::Duration,
        sync::Mutex
    };

    use super::{
        Ankaios, ControlInterface, ControlInterfaceState, generate_test_ankaios,
        AnkaiosError, Response, CompleteState,
    };
    use crate::components::{
        manifest::generate_test_manifest,
        response::generate_test_response_update_state_success,
        workload_mod::test_helpers::generate_test_workload,
    };

    lazy_static! {
        // USed for synchronizing multiple tests that use the same Mock.
        pub static ref MOCKALL_SYNC: Mutex<()> = Mutex::new(());
    }

    #[tokio::test]
    async fn test_ankaios_state() {
        let _guard = MOCKALL_SYNC.lock().await;

        let mut mock_control_interface = ControlInterface::default();
        mock_control_interface.expect_state().returning(|| ControlInterfaceState::Initialized);
        mock_control_interface.expect_disconnect().returning(|| Ok(()));

        let (mut ank, _response_sender) = generate_test_ankaios(mock_control_interface);

        assert!(ank.state() == ControlInterfaceState::Initialized);
    }

    #[tokio::test]
    async fn itest_create_ankaios() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to send the created response sender from the ControlInterface
        let (response_sender_store, response_sender_recv) = tokio::sync::oneshot::channel();
        // Prepare channel to intercept the request that is being created to check the connection
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let ci_new_context = ControlInterface::new_context();
        let mut ci_mock = ControlInterface::default();

        ci_mock.expect_connect()
            .times(1)
            .returning(|| Ok(()));
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect()
            .times(1)
            .returning(|| Ok(()));

        ci_new_context.expect()
            .return_once(move |sender| {
                response_sender_store.send(sender).unwrap();
                ci_mock
            });

        // Create Ankaios handle
        let ankaios_handle = tokio::spawn(Ankaios::new());

        // Get the response sender from the ControlInterface creation
        let response_sender = response_sender_recv.await.unwrap();

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::CompleteState(Box::default()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Create Ankaios fully and check the connection
        let _ankaios = ankaios_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn itest_connection_closed() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to send the created response sender from the ControlInterface
        let (response_sender_store, response_sender_recv) = tokio::sync::oneshot::channel();
        // Prepare channel to intercept the request that is being created to check the connection
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let ci_new_context = ControlInterface::new_context();
        let mut ci_mock = ControlInterface::default();

        ci_mock.expect_connect()
            .times(1)
            .returning(|| Ok(()));
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect()
            .times(1)
            .returning(|| Ok(()));

        ci_new_context.expect()
            .return_once(move |sender| {
                response_sender_store.send(sender).unwrap();
                ci_mock
            });

        // Create Ankaios handle
        let ankaios_handle = tokio::spawn(Ankaios::new());

        // Get the response sender from the ControlInterface creation
        let response_sender = response_sender_recv.await.unwrap();

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::ConnectionClosedReason(String::default()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Create Ankaios fully and check the connection
        let result = ankaios_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ConnectionClosedError(_))));
    }

    #[tokio::test]
    async fn itest_get_state_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the state
        let method_handle = tokio::spawn(async move {
            ank.get_state(None, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::default();
        let response = Response{
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the state
        let state = method_handle.await.unwrap().unwrap();

        assert_eq!(state.get_api_version(), complete_state.get_api_version());
    }

    #[tokio::test]
    async fn itest_get_state_incorrect_id_timeout() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the state
        let method_handle = tokio::spawn(async move {
            ank.get_state(None, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let _request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::CompleteState(Box::default()),
            id: "incorrect_id".to_owned(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the state
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::TimeoutError(_))));
    }

    #[tokio::test]
    async fn itest_apply_manifest_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare manifest
        let manifest = generate_test_manifest();

        // Prepare handle for applying the manifest
        let method_handle = tokio::spawn(async move {
            ank.apply_manifest(manifest, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let ret = method_handle.await.unwrap().unwrap();
        assert!(ret.added_workloads.len() == 1);
        assert!(ret.deleted_workloads.is_empty());
    }

    #[tokio::test]
    async fn itest_apply_manifest_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare manifest
        let manifest = generate_test_manifest();

        // Prepare handle for applying the manifest
        let method_handle = tokio::spawn(async move {
            ank.apply_manifest(manifest, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosError(_))));
    }

    #[tokio::test]
    async fn itest_apply_manifest_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare manifest
        let manifest = generate_test_manifest();

        // Prepare handle for applying the manifest
        let method_handle = tokio::spawn(async move {
            ank.apply_manifest(manifest, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::CompleteState(Box::default()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ResponseError(_))));
    }

    #[tokio::test]
    async fn itest_delete_manifest_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare manifest
        let manifest = generate_test_manifest();

        // Prepare handle for deleting the manifest
        let method_handle = tokio::spawn(async move {
            ank.delete_manifest(manifest, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let ret = method_handle.await.unwrap().unwrap();
        assert!(ret.added_workloads.len() == 1);
        assert!(ret.deleted_workloads.is_empty());
    }

    #[tokio::test]
    async fn itest_delete_manifest_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare manifest
        let manifest = generate_test_manifest();

        // Prepare handle for deleting the manifest
        let method_handle = tokio::spawn(async move {
            ank.delete_manifest(manifest, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosError(_))));
    }

    #[tokio::test]
    async fn itest_delete_manifest_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare manifest
        let manifest = generate_test_manifest();

        // Prepare handle for deleting the manifest
        let method_handle = tokio::spawn(async move {
            ank.delete_manifest(manifest, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::CompleteState(Box::default()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ResponseError(_))));
    }

    #[tokio::test]
    async fn itest_apply_workload_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare workload
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");

        // Prepare handle for applying the workload
        let method_handle = tokio::spawn(async move {
            ank.apply_workload(workload, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let ret = method_handle.await.unwrap().unwrap();
        assert!(ret.added_workloads.len() == 1);
        assert!(ret.deleted_workloads.is_empty());
    }

    #[tokio::test]
    async fn itest_apply_workload_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare workload
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");

        // Prepare handle for applying the workload
        let method_handle = tokio::spawn(async move {
            ank.apply_workload(workload, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosError(_))));
    }

    #[tokio::test]
    async fn itest_apply_workload_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare workload
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");

        // Prepare handle for applying the workload
        let method_handle = tokio::spawn(async move {
            ank.apply_workload(workload, Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::CompleteState(Box::default()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ResponseError(_))));
    }

    #[tokio::test]
    async fn itest_get_workload() {
        // TODO
    }

    #[tokio::test]
    async fn itest_delete_workload_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle = tokio::spawn(async move {
            ank.delete_workload("workload_Test".to_owned(), Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let ret = method_handle.await.unwrap().unwrap();
        assert!(ret.added_workloads.len() == 1);
        assert!(ret.deleted_workloads.is_empty());
    }

    #[tokio::test]
    async fn itest_delete_workload_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle = tokio::spawn(async move {
            ank.delete_workload("workload_Test".to_owned(), Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosError(_))));
    }

    #[tokio::test]
    async fn itest_delete_workload_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock.expect_write_request()
            .times(1)
            .return_once(move |request| {
                request_sender.send(request).unwrap();
                Ok(())
            });
            ci_mock.expect_disconnect()
                .times(1)
                .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle = tokio::spawn(async move {
            ank.delete_workload("workload_Test".to_owned(), Some(Duration::from_millis(50))).await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response{
            content: super::ResponseType::CompleteState(Box::default()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::ResponseError(_))));
    }
}