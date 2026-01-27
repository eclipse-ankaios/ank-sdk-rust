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

//! This module contains the definition of `Ankaios` struct, which
//! represents the main interface to the [Ankaios] cluster.
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

use std::collections::HashMap;
use std::vec;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep, timeout as tokio_timeout};

#[cfg_attr(test, mockall_double::double)]
use crate::components::control_interface::ControlInterface;
use crate::components::event_types::{EventEntry, EventsCampaignResponse};
use crate::components::log_types::{LogCampaignResponse, LogsRequest};
use crate::components::manifest::{CONFIGS_PREFIX, Manifest};
use crate::components::request::{
    AnkaiosLogsRequest, EventsCancelRequest, EventsRequest, GetStateRequest, LogsCancelRequest,
    Request, UpdateStateRequest,
};
use crate::components::response::{Response, ResponseType, UpdateStateSuccess};
use crate::components::workload_mod::{WORKLOADS_PREFIX, Workload};
use crate::components::workload_state_mod::{
    WorkloadExecutionState, WorkloadInstanceName, WorkloadStateCollection, WorkloadStateEnum,
};
use crate::{AgentAttributes, AnkaiosError, CompleteState};

/// The prefix for the agents in the state.
const AGENTS_PREFIX: &str = "agents";
/// The prefix for the workload states in the state.
const WORKLOAD_STATES_PREFIX: &str = "workloadStates";
/// The default timeout, if not manually provided.
const DEFAULT_TIMEOUT: u64 = 5; // seconds
/// The size of the channel used to receive responses.
pub(crate) const CHANNEL_SIZE: usize = 100;

/// This struct is used to interact with [Ankaios] using an intuitive API.
/// The struct automatically handles the session creation and the requests
/// and responses sent and received over the Control Interface.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
///
/// # Examples
///
/// ## Create an Ankaios object, connect and disconnect from the control interface:
///
/// ```rust,no_run
/// use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// #
/// # Runtime::new().unwrap().block_on(async {
///
/// let ankaios = Ankaios::new().await.unwrap();
/// /* */
/// drop(ankaios);
/// # })
/// ```
///
/// ## Create an Ankaios object and set the default timeout for requests:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::time::Duration;
/// # use tokio::runtime::Runtime;
/// #
/// # Runtime::new().unwrap().block_on(async {
/// #
/// let ankaios = Ankaios::new_with_timeout(Duration::from_secs(5)).await.unwrap();
/// # })
/// ```
///
/// ## Apply a manifest:
///
/// ```rust,no_run
/// # use ankaios_sdk::{Ankaios, Manifest};
/// # use tokio::runtime::Runtime;
/// #
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let manifest: Manifest;
/// # let manifest = Manifest::from_string("").unwrap();
/// let update_state_success = ankaios.apply_manifest(manifest).await.unwrap();
/// println!("{:?}", update_state_success);
/// # })
/// ```
///
/// ## Delete a manifest:
///
/// ```rust,no_run
/// # use ankaios_sdk::{Ankaios, Manifest};
/// # use tokio::runtime::Runtime;
/// #
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let manifest: Manifest;
/// # let manifest = Manifest::from_string("").unwrap();
/// let update_state_success = ankaios.delete_manifest(manifest).await.unwrap();
/// println!("{:?}", update_state_success);
/// # })
/// ```
///
/// ## Run a workload:
///
/// ```rust,no_run
/// # use ankaios_sdk::{Ankaios, Workload};
/// # use tokio::runtime::Runtime;
/// #
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let workload: Workload;
/// # let workload = Workload::builder().build().unwrap();
/// let update_state_success = ankaios.apply_workload(workload).await.unwrap();
/// println!("{:?}", update_state_success);
/// # })
/// ```
///
/// ## Get a workload:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// #
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let workload_name: String;
/// # let workload_name = String::new();
/// let workload = ankaios.get_workload(workload_name).await.unwrap();
/// println!("{:?}", workload);
/// # })
/// ```
///
/// ## Delete a workload:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let workload_name: String;
/// # let workload_name = String::new();
/// let update_state_success = ankaios.delete_workload(workload_name).await.unwrap();
/// println!("{:?}", update_state_success);
/// # })
/// ```
///
/// ## Get the state:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let state = ankaios.get_state(Vec::default()).await.unwrap();
/// println!("{:?}", state);
/// # })
/// ```
///
/// ## Get the agents:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let agents = ankaios.get_agents().await.unwrap();
/// println!("{:?}", agents);
/// # })
/// ```
///
/// ## Get the workload states:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let workload_states_collection = ankaios.get_workload_states().await.unwrap();
/// let workload_states = workload_states_collection.as_list();
/// # })
/// ```
///
/// ## Get the workload states for a specific agent:
///
/// ```rust,no_run
/// # use ankaios_sdk::Ankaios;
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let agent_name: String;
/// # let agent_name = String::new();
/// let workload_states_collection = ankaios.get_workload_states_on_agent(agent_name).await.unwrap();
/// let workload_states = workload_states_collection.as_list();
/// # })
/// ```
///
/// ## Get the workload execution state for an instance name:
///
/// ```rust,no_run
/// # use ankaios_sdk::{Ankaios, WorkloadInstanceName};
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let workload_instance_name: WorkloadInstanceName;
/// # let workload_instance_name = WorkloadInstanceName::default();
/// let workload_state = ankaios.get_execution_state_for_instance_name(&workload_instance_name).await.unwrap();
/// println!("{:?}", workload_state);
/// # })
/// ```
///
/// ## Wait for a workload to reach a state:
///
/// ```rust,no_run
/// # use ankaios_sdk::{Ankaios, AnkaiosError, WorkloadInstanceName, WorkloadStateEnum};
/// # use tokio::runtime::Runtime;
/// # Runtime::new().unwrap().block_on(async {
/// # let mut ankaios = Ankaios::new().await.unwrap();
/// #
/// let workload_instance_name: WorkloadInstanceName;
/// # let workload_instance_name = WorkloadInstanceName::default();
/// let expected_state: WorkloadStateEnum;
/// # let expected_state = WorkloadStateEnum::Running;
/// match ankaios.wait_for_workload_to_reach_state(workload_instance_name, expected_state).await {
///     Ok(_) => println!("Workload reached the expected state."),
///     Err(AnkaiosError::TimeoutError(_)) => println!("Timeout while waiting for workload to reach state."),
///     Err(err) => println!("Error while waiting for workload to reach state: {}", err),
/// }
/// # })
/// ```
pub struct Ankaios {
    /// The receiver end of the channel used to receive responses from the Control Interface.
    response_receiver: mpsc::Receiver<Response>,
    /// The control interface instance that is used to communicate with the Control Interface.
    control_interface: ControlInterface,
    /// The timeout used for the requests.
    pub timeout: Duration,
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
    /// [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if a timeout occurred when testing the connection.
    pub async fn new() -> Result<Self, AnkaiosError> {
        Self::new_with_timeout(Duration::from_secs(DEFAULT_TIMEOUT)).await
    }

    /// Creates a new `Ankaios` object with a custom timeout and connects to the Control Interface.
    ///
    /// ## Arguments
    ///
    /// - `timeout`: The maximum time to wait for the requests.
    ///
    /// ## Returns
    ///
    /// A [Result] containing the [Ankaios] object if the connection was successful.
    ///
    /// ## Errors
    ///
    /// [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if an error occurred when connecting.
    pub async fn new_with_timeout(timeout: Duration) -> Result<Self, AnkaiosError> {
        let (response_sender, response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
        let mut object = Self {
            response_receiver,
            control_interface: ControlInterface::new(response_sender),
            timeout,
        };

        object.control_interface.connect(timeout).await?;
        Ok(object)
    }

    /// Sends a request to the Control Interface and waits for the response.
    ///
    /// ## Arguments
    ///
    /// - `request`: The [`Request`] to be sent.
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
    async fn send_request(
        &mut self,
        request: impl Request + 'static,
    ) -> Result<Response, AnkaiosError> {
        let request_id = request.get_id();
        self.control_interface.write_request(request).await?;
        loop {
            match tokio_timeout(self.timeout, self.response_receiver.recv()).await {
                Ok(Some(response)) => {
                    if let ResponseType::ConnectionClosedReason(reason) = response.content {
                        log::error!("Connection closed: {reason}");
                        return Err(AnkaiosError::ConnectionClosedError(reason));
                    }
                    if response.get_request_id() == request_id {
                        return Ok(response);
                    }
                    log::warn!("Received response with wrong id.");
                }
                Ok(None) => {
                    log::error!("Reading thread closed unexpectedly.");
                    return Err(AnkaiosError::ControlInterfaceError(
                        "Reading thread closed.".to_owned(),
                    ));
                }
                Err(err) => {
                    log::error!("Timeout while waiting for response.");
                    return Err(AnkaiosError::TimeoutError(err));
                }
            }
        }
    }

    /// Send a request to apply a [Manifest].
    ///
    /// ## Arguments
    ///
    /// - `manifest`: The [Manifest] to be applied.
    ///
    /// ## Returns
    ///
    /// - an [`UpdateStateSuccess`] containing the number of added and deleted workloads if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn apply_manifest(
        &mut self,
        manifest: Manifest,
    ) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let masks = manifest.calculate_masks();
        let request = UpdateStateRequest::new(&CompleteState::new_from_manifest(manifest), masks);

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!(
                    "Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(),
                    update_state_success.deleted_workloads.len()
                );
                Ok(*update_state_success)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to apply manifest: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to delete a [Manifest].
    ///
    /// ## Arguments
    ///
    /// - `manifest`: The [Manifest] to be deleted.
    ///
    /// ## Returns
    ///
    /// - an [`UpdateStateSuccess`] containing the number of added and deleted workloads if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_manifest(
        &mut self,
        manifest: Manifest,
    ) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let request =
            UpdateStateRequest::new(&CompleteState::default(), manifest.calculate_masks());

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!(
                    "Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(),
                    update_state_success.deleted_workloads.len()
                );
                Ok(*update_state_success)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete manifest: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to run a [Workload].
    ///
    /// ## Arguments
    ///
    /// - `workload`: The [Workload] to be run.
    ///
    /// ## Returns
    ///
    /// - an [`UpdateStateSuccess`] containing the number of added and deleted workloads if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn apply_workload(
        &mut self,
        workload: Workload,
    ) -> Result<UpdateStateSuccess, AnkaiosError> {
        let masks = workload.masks.clone();

        // Create CompleteState
        let complete_state = CompleteState::new_from_workloads(vec![workload]);

        // Create request
        let request = UpdateStateRequest::new(&complete_state, masks);

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!(
                    "Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(),
                    update_state_success.deleted_workloads.len()
                );
                Ok(*update_state_success)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to apply workload: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to get the [Workload] that matches the given name.
    ///
    /// ## Arguments
    ///
    /// - `workload_name`: A [String] containing the name of the workload to get.
    ///
    /// ## Returns
    ///
    /// - a [Workload] object if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload(
        &mut self,
        workload_name: String,
    ) -> Result<Vec<Workload>, AnkaiosError> {
        let complete_state = self
            .get_state(vec![format!("{WORKLOADS_PREFIX}.{workload_name}")])
            .await?;
        Ok(complete_state.get_workloads())
    }

    /// Send a request to delete a workload.
    ///
    /// ## Arguments
    ///
    /// - `workload_name`: A [String] containing the name of the workload to get.
    ///
    /// ## Returns
    ///
    /// - a [Workload] object if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_workload(
        &mut self,
        workload_name: String,
    ) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create request
        let request = UpdateStateRequest::new(
            &CompleteState::default(),
            vec![format!("{WORKLOADS_PREFIX}.{workload_name}")],
        );

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!(
                    "Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(),
                    update_state_success.deleted_workloads.len()
                );
                Ok(*update_state_success)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete workload: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to update the configs
    ///
    /// ## Arguments
    ///
    /// - `configs`: A [`HashMap`] containing the configs to be updated.
    ///
    /// ## Returns
    ///
    /// - an [`UpdateStateSuccess`] object if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn update_configs(
        &mut self,
        configs: HashMap<String, serde_yaml::Value>,
    ) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create CompleteState
        let complete_state = CompleteState::new_from_configs(configs);

        // Create request
        let request = UpdateStateRequest::new(&complete_state, vec![CONFIGS_PREFIX.to_owned()]);

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!(
                    "Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(),
                    update_state_success.deleted_workloads.len()
                );
                Ok(*update_state_success)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to update configs: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to add a config with the provided name.
    /// If the config exists, it will be replaced.
    ///
    /// ## Arguments
    ///
    /// - `name`: A [String] containing the name of the config to be added;
    /// - `configs`: A [`serde_yaml::Value`] containing the configs to be added.
    ///
    /// ## Returns
    ///
    /// - an [`UpdateStateSuccess`] object if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn add_config(
        &mut self,
        name: String,
        configs: serde_yaml::Value,
    ) -> Result<UpdateStateSuccess, AnkaiosError> {
        // Create CompleteState
        let complete_state =
            CompleteState::new_from_configs(HashMap::from([(name.clone(), configs)]));

        // Create request
        let request =
            UpdateStateRequest::new(&complete_state, vec![format!("{CONFIGS_PREFIX}.{name}")]);

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(update_state_success) => {
                log::info!(
                    "Update successful: {:?} added workloads, {:?} deleted workloads",
                    update_state_success.added_workloads.len(),
                    update_state_success.deleted_workloads.len()
                );
                Ok(*update_state_success)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to add the config: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to get all the configs.
    ///
    /// ## Returns
    ///
    /// - a [`HashMap`] containing the configs if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_configs(
        &mut self,
    ) -> Result<HashMap<String, serde_yaml::Value>, AnkaiosError> {
        let complete_state = self.get_state(vec![CONFIGS_PREFIX.to_owned()]).await?;
        Ok(complete_state.get_configs())
    }

    /// Send a request to get the config with the provided name.
    ///
    /// ## Arguments
    ///
    /// - `name`: A [String] containing the name of the config.
    ///
    /// ## Returns
    ///
    /// - a [`HashMap`] containing the config if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_config(
        &mut self,
        name: String,
    ) -> Result<HashMap<String, serde_yaml::Value>, AnkaiosError> {
        let complete_state = self
            .get_state(vec![format!("{CONFIGS_PREFIX}.{name}")])
            .await?;
        Ok(complete_state.get_configs())
    }

    /// Send a request to delete all the configs.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_all_configs(&mut self) -> Result<(), AnkaiosError> {
        // Create request
        let request =
            UpdateStateRequest::new(&CompleteState::default(), vec![CONFIGS_PREFIX.to_owned()]);

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(_) => {
                log::info!("Update successful");
                Ok(())
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete all configs: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to delete the config with the provided name.
    ///
    /// ## Arguments
    ///
    /// - `name`: A [String] containing the name of the config.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn delete_config(&mut self, name: String) -> Result<(), AnkaiosError> {
        // Create request
        let request = UpdateStateRequest::new(
            &CompleteState::default(),
            vec![format!("{CONFIGS_PREFIX}.{name}")],
        );

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(_) => {
                log::info!("Update successful");
                Ok(())
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to delete config: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to get the [complete state](CompleteState).
    ///
    /// ## Arguments
    ///
    /// - `field_masks`: A [Vec] of [String]s containing the field masks to be used in the request.
    ///
    /// ## Returns
    ///
    /// - a [`CompleteState`] object containing the state of the cluster.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_state(
        &mut self,
        field_masks: Vec<String>,
    ) -> Result<CompleteState, AnkaiosError> {
        // Create request
        let request = GetStateRequest::new(field_masks);

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::CompleteState(complete_state) => Ok(*complete_state),
            ResponseType::Error(error) => {
                log::error!("Error while trying to get the state: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to set tags for a specific agent.
    ///
    /// ## Arguments
    ///
    /// * `agent_name` - The name of the agent.
    /// * `tags` - A [`HashMap`] containing the tags to set for the agent.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn set_agent_tags(
        &mut self,
        agent_name: String,
        tags: HashMap<String, String>,
    ) -> Result<(), AnkaiosError> {
        // Create CompleteState
        let mut complete_state = CompleteState::new();
        complete_state.set_agent_tags(&agent_name, tags);

        // Create request
        let request = UpdateStateRequest::new(
            &complete_state,
            vec![format!("{AGENTS_PREFIX}.{agent_name}.tags")],
        );

        // Wait for the response
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::UpdateStateSuccess(_) => {
                log::info!("Update successful");
                Ok(())
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to set agent tags: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Send a request to get the agents.
    ///
    /// ## Returns
    ///
    /// - a [`HashMap`] containing the agents if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_agents(&mut self) -> Result<HashMap<String, AgentAttributes>, AnkaiosError> {
        let complete_state = self.get_state(vec![AGENTS_PREFIX.to_owned()]).await?;
        Ok(complete_state.get_agents())
    }

    /// Send a request to get the agents.
    ///
    /// ## Returns
    ///
    /// - the [`AgentAttributes`] of the requested agent if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_agent(&mut self, agent_name: String) -> Result<AgentAttributes, AnkaiosError> {
        let agents = self
            .get_state(vec![format!("{AGENTS_PREFIX}.{agent_name}")])
            .await?
            .get_agents();

        agents.get(&agent_name).cloned().ok_or_else(|| {
            AnkaiosError::AnkaiosResponseError(format!("Agent {agent_name} not found."))
        })
    }

    /// Send a request to get the workload states.
    ///
    /// ## Returns
    ///
    /// - a [`WorkloadStateCollection`] containing the workload states if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload_states(&mut self) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self
            .get_state(vec![WORKLOAD_STATES_PREFIX.to_owned()])
            .await?;
        Ok(complete_state.get_workload_states())
    }

    /// Send a request to get the execution state for an instance name.
    ///
    /// ## Arguments
    ///
    /// - `instance_name`: The [`WorkloadInstanceName`] to get the execution state for.
    ///
    /// ## Returns
    ///
    /// - the requested [`WorkloadExecutionState`] for the provided instance name.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_execution_state_for_instance_name(
        &mut self,
        instance_name: &WorkloadInstanceName,
    ) -> Result<WorkloadExecutionState, AnkaiosError> {
        let complete_state: CompleteState = self
            .get_state(vec![instance_name.get_filter_mask()])
            .await?;
        let workload_states = Vec::from(complete_state.get_workload_states());
        match workload_states.first() {
            Some(workload_state) => Ok(workload_state.execution_state.clone()),
            None => Err(AnkaiosError::AnkaiosResponseError(
                "No workload states found.".to_owned(),
            )),
        }
    }

    /// Send a request to get the workload states for the workloads running on a specific agent.
    ///
    /// ## Arguments
    ///
    /// - `agent_name`: A [String] containing the name of the agent to get the workload states for.
    ///
    /// ## Returns
    ///
    /// - a [`WorkloadStateCollection`] containing the workload states if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload_states_on_agent(
        &mut self,
        agent_name: String,
    ) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self
            .get_state(vec![format!("{WORKLOAD_STATES_PREFIX}.{agent_name}")])
            .await?;
        Ok(complete_state.get_workload_states())
    }

    /// Send a request to get the workload states for the workloads with a specific name.
    ///
    /// ## Arguments
    ///
    /// - `workload_name`: A [String] containing the name of the workloads to get the states for.
    ///
    /// ## Returns
    ///
    /// - a [`WorkloadStateCollection`] containing the workload states if the request was successful.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response;
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn get_workload_states_for_name(
        &mut self,
        workload_name: String,
    ) -> Result<WorkloadStateCollection, AnkaiosError> {
        let complete_state = self
            .get_state(vec![WORKLOAD_STATES_PREFIX.to_owned()])
            .await?;
        let mut workload_states_for_name = WorkloadStateCollection::new();
        for workload_state in Vec::from(complete_state.get_workload_states()) {
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
    /// - `state`: The [`WorkloadStateEnum`] to wait for.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response or waiting for the state to be reached.
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn wait_for_workload_to_reach_state(
        &mut self,
        instance_name: WorkloadInstanceName,
        state: WorkloadStateEnum,
    ) -> Result<(), AnkaiosError> {
        const CHECK_INTERVAL: Duration = Duration::from_millis(100);
        let timeout_clone = self.timeout;
        let poll_future = async {
            loop {
                let workload_exec_state = self
                    .get_execution_state_for_instance_name(&instance_name)
                    .await?;
                if workload_exec_state.state == state {
                    return Ok(());
                }

                sleep(CHECK_INTERVAL).await;
            }
        };

        match tokio_timeout(timeout_clone, poll_future).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => {
                log::error!("Error while waiting for workload to reach state: {err}");
                Err(err)
            }
            Err(err) => {
                log::error!("Timeout while waiting for workload to reach state: {err}");
                Err(AnkaiosError::TimeoutError(err))
            }
        }
    }

    /// Request logs for the specified workloads.
    ///
    /// ## Arguments
    ///
    /// - `logs_request`: A [`LogsRequest`] containing the details to request logs of workloads.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response or waiting for the state to be reached.
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn request_logs(
        &mut self,
        logs_request: LogsRequest,
    ) -> Result<LogCampaignResponse, AnkaiosError> {
        let request = AnkaiosLogsRequest::from(logs_request);
        let request_id = request.get_id();
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::LogsRequestAccepted(accepted_workload_names) => {
                log::trace!(
                    "Received LogsRequestAccepted: {accepted_workload_names:?} accepted workloads."
                );

                let (logs_sender, logs_receiver) = mpsc::channel(CHANNEL_SIZE);
                let log_campaign_response = LogCampaignResponse::new(
                    request_id.clone(),
                    accepted_workload_names,
                    logs_receiver,
                );
                self.control_interface
                    .add_log_campaign(request_id, logs_sender);
                Ok(log_campaign_response)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to request logs: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            unexpected_response => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(format!(
                    "Received unexpected response type: '{unexpected_response:?}'"
                )))
            }
        }
    }

    /// Stop receiving logs for a log campaign.
    ///
    /// ## Arguments
    ///
    /// - `log_campaign_response`: A [`LogCampaignResponse`] to stop receiving logs for;
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response or waiting for the state to be reached.
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn stop_receiving_logs(
        &mut self,
        log_campaign_response: LogCampaignResponse,
    ) -> Result<(), AnkaiosError> {
        let logs_cancel_request = LogsCancelRequest::new(log_campaign_response.get_request_id());
        self.control_interface
            .remove_log_campaign(&logs_cancel_request.get_id());
        let response = self.send_request(logs_cancel_request).await?;

        match response.content {
            ResponseType::LogsCancelAccepted => {
                log::trace!("Received LogsCancelAccepted: log campaign canceled successfully.");
                Ok(())
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to cancel log campaign: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }

    /// Register to an event campaign.
    ///
    /// ## Arguments
    ///
    /// - `field_masks`: A [Vec] of [String]s containing the field masks to be used in the request.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response or waiting for the state to be reached.
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn register_event(
        &mut self,
        field_masks: Vec<String>,
    ) -> Result<EventsCampaignResponse, AnkaiosError> {
        let request = EventsRequest::new(field_masks);
        let request_id = request.get_id();
        let response = self.send_request(request).await?;

        match response.content {
            ResponseType::CompleteState(complete_state) => {
                log::info!("Event registered successfully, state received.");

                let (events_sender, events_receiver) = mpsc::channel(CHANNEL_SIZE);
                let events_campaign_response =
                    EventsCampaignResponse::new(request_id.clone(), events_receiver);

                let event_entry = EventEntry {
                    complete_state: *complete_state,
                    ..Default::default()
                };
                events_sender.send(event_entry).await.unwrap_or_else(|err| {
                    log::error!("Error while sending initial event: '{err}'");
                });

                self.control_interface
                    .add_events_campaign(request_id, events_sender);
                Ok(events_campaign_response)
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to request events: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            unexpected_response => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(format!(
                    "Received unexpected response type: '{unexpected_response:?}'"
                )))
            }
        }
    }

    /// Unregister from an event campaign.
    ///
    /// ## Arguments
    ///
    /// - `events_campaign_response`: The [`EventsCampaignResponse`] received when registering
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`ControlInterfaceError`](AnkaiosError::ControlInterfaceError) if not connected;
    /// - [`AnkaiosError`]::[`TimeoutError`](AnkaiosError::TimeoutError) if the timeout was reached while waiting for the response or waiting for the state to be reached.
    /// - [`AnkaiosError`]::[`AnkaiosResponseError`](AnkaiosError::AnkaiosResponseError) if [Ankaios](https://eclipse-ankaios.github.io/ankaios) returned an error;
    /// - [`AnkaiosError`]::[`ResponseError`](AnkaiosError::ResponseError) if the response has the wrong type;
    /// - [`AnkaiosError`]::[`ConnectionClosedError`](AnkaiosError::ConnectionClosedError) if the connection was closed.
    pub async fn unregister_event(
        &mut self,
        events_campaign_response: EventsCampaignResponse,
    ) -> Result<(), AnkaiosError> {
        let events_cancel_request =
            EventsCancelRequest::new(events_campaign_response.get_request_id());
        self.control_interface
            .remove_events_campaign(&events_cancel_request.get_id());
        let response = self.send_request(events_cancel_request).await?;

        match response.content {
            ResponseType::EventsCancelAccepted => {
                log::trace!("Received EventsCancelAccepted: unregistered successfully.");
                Ok(())
            }
            ResponseType::Error(error) => {
                log::error!("Error while trying to unregister from the campaign: {error}");
                Err(AnkaiosError::AnkaiosResponseError(error))
            }
            _ => {
                log::error!("Received unexpected response type.");
                Err(AnkaiosError::ResponseError(
                    "Received unexpected response type.".to_owned(),
                ))
            }
        }
    }
}

impl Drop for Ankaios {
    fn drop(&mut self) {
        log::trace!("Dropping Ankaios");
        self.control_interface.disconnect().unwrap_or_else(|err| {
            log::error!("Error while disconnecting: '{err}'");
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
fn generate_test_ankaios(
    mock_control_interface: ControlInterface,
) -> (Ankaios, mpsc::Sender<Response>) {
    let (response_sender, response_receiver) = mpsc::channel::<Response>(CHANNEL_SIZE);
    (
        Ankaios {
            response_receiver,
            control_interface: mock_control_interface,
            timeout: Duration::from_millis(50),
        },
        response_sender,
    )
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::LazyLock};
    use tokio::{
        sync::{Mutex, mpsc},
        time::Duration,
    };

    use super::{
        AGENTS_PREFIX, AgentAttributes, Ankaios, AnkaiosError, CONFIGS_PREFIX, CompleteState,
        ControlInterface, DEFAULT_TIMEOUT, EventsCampaignResponse, Response,
        WORKLOAD_STATES_PREFIX, WorkloadInstanceName, WorkloadStateEnum, generate_test_ankaios,
    };
    use crate::components::{
        complete_state::generate_complete_state_proto,
        manifest::generate_test_manifest,
        request::{
            AnkaiosLogsRequest, EventsCancelRequest, EventsRequest, GetStateRequest,
            LogsCancelRequest, Request, UpdateStateRequest,
        },
        response::generate_test_response_update_state_success,
        workload_mod::{WORKLOADS_PREFIX, test_helpers::generate_test_workload},
    };
    use crate::{EventEntry, ankaios_api::ank_base::RequestContent};
    use crate::{LogCampaignResponse, LogEntry, LogResponse, LogsRequest as InputLogsRequest};

    // Used for synchronizing multiple tests that use the same mock.
    pub static MOCKALL_SYNC: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    const TEST_LOG_MESSAGE: &str = "some log message 1";
    const REQUEST_ID: &str = "request_id";
    const TEST_MASK: &str = "test.mask";

    #[tokio::test]
    async fn itest_create_ankaios() {
        let _guard = MOCKALL_SYNC.lock().await;

        let ci_new_context = ControlInterface::new_context();
        let mut ci_mock = ControlInterface::default();

        ci_mock
            .expect_connect()
            .times(1)
            .with(mockall::predicate::eq(Duration::from_millis(50)))
            .returning(|_| Ok(()));

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        ci_new_context.expect().return_once(move |_| ci_mock);

        // Create Ankaios handle
        let ankaios_handle = tokio::spawn(Ankaios::new_with_timeout(Duration::from_millis(50)));

        // Create Ankaios fully and check the connection
        let ankaios = ankaios_handle.await.unwrap();
        assert!(ankaios.is_ok());
    }

    #[tokio::test]
    async fn itest_timeout_while_connecting() {
        let _guard = MOCKALL_SYNC.lock().await;

        let ci_new_context = ControlInterface::new_context();
        let mut ci_mock = ControlInterface::default();

        ci_mock
            .expect_connect()
            .with(mockall::predicate::eq(Duration::from_secs(DEFAULT_TIMEOUT)))
            .times(1)
            .returning(|_| Err(AnkaiosError::ControlInterfaceError(String::default())));
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        ci_new_context.expect().return_once(move |_| ci_mock);

        // Create Ankaios handle
        let ankaios_handle = tokio::spawn(Ankaios::new());

        // Create Ankaios fully and check the connection
        let result = ankaios_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(AnkaiosError::ControlInterfaceError(_))
        ));
    }

    #[tokio::test]
    async fn itest_get_state_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the state
        let method_handle = tokio::spawn(async move { ank.get_state(Vec::default()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::default();
        let response = Response {
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
    async fn itest_get_state_incorrect_id_and_timeout() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the state
        let method_handle = tokio::spawn(async move { ank.get_state(Vec::default()).await });

        // Get the request from the ControlInterface
        let _request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
    async fn itest_get_state_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the state
        let method_handle = tokio::spawn(async move { ank.get_state(Vec::default()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_get_state_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the state
        let method_handle = tokio::spawn(async move { ank.get_state(Vec::default()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::UpdateStateSuccess(Box::default()),
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
    async fn itest_apply_manifest_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        // Prepare manifest
        let manifest = generate_test_manifest();
        let masks = manifest.calculate_masks();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for applying the manifest
        let method_handle = tokio::spawn(async move { ank.apply_manifest(manifest).await });

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

        // Prepare manifest
        let manifest = generate_test_manifest();
        let masks = manifest.calculate_masks();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for applying the manifest
        let method_handle = tokio::spawn(async move { ank.apply_manifest(manifest).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_apply_manifest_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        // Prepare manifest
        let manifest = generate_test_manifest();
        let masks = manifest.calculate_masks();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for applying the manifest
        let method_handle = tokio::spawn(async move { ank.apply_manifest(manifest).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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

        // Prepare manifest
        let manifest = generate_test_manifest();
        let masks = manifest.calculate_masks();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the manifest
        let method_handle = tokio::spawn(async move { ank.delete_manifest(manifest).await });

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

        // Prepare manifest
        let manifest = generate_test_manifest();
        let masks = manifest.calculate_masks();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the manifest
        let method_handle = tokio::spawn(async move { ank.delete_manifest(manifest).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_delete_manifest_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        // Prepare manifest
        let manifest = generate_test_manifest();
        let masks = manifest.calculate_masks();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the manifest
        let method_handle = tokio::spawn(async move { ank.delete_manifest(manifest).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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

        // Prepare workload
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");
        let masks = workload.masks.clone();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for applying the workload
        let method_handle = tokio::spawn(async move { ank.apply_workload(workload).await });

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

        // Prepare workload
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");
        let masks = workload.masks.clone();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for applying the workload
        let method_handle = tokio::spawn(async move { ank.apply_workload(workload).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_apply_workload_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        // Prepare workload
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");
        let masks = workload.masks.clone();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for applying the workload
        let method_handle = tokio::spawn(async move { ank.apply_workload(workload).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![format!("{WORKLOADS_PREFIX}.workload_Test")]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the workload
        let method_handle =
            tokio::spawn(async move { ank.get_workload("workload_Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let workload = generate_test_workload("agent_Test", "workload_Test", "podman");
        let complete_state = CompleteState::new_from_workloads(vec![workload.clone()]);
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the workload
        let ret_workloads = method_handle.await.unwrap().unwrap();

        assert_eq!(ret_workloads.len(), 1);
        assert_eq!(workload.workload, ret_workloads[0].workload);
    }

    #[tokio::test]
    async fn itest_delete_workload_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{WORKLOADS_PREFIX}.workload_Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle =
            tokio::spawn(async move { ank.delete_workload("workload_Test".to_owned()).await });

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
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{WORKLOADS_PREFIX}.workload_Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle =
            tokio::spawn(async move { ank.delete_workload("workload_Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_delete_workload_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{WORKLOADS_PREFIX}.workload_Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle =
            tokio::spawn(async move { ank.delete_workload("workload_Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
    async fn itest_update_configs_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![CONFIGS_PREFIX.to_owned()]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare configs
        let configs = HashMap::new();

        // Prepare handle for updating the configs
        let method_handle = tokio::spawn(async move { ank.update_configs(configs).await });

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
    async fn itest_update_configs_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![CONFIGS_PREFIX.to_owned()]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare configs
        let configs = HashMap::new();

        // Prepare handle for updating the configs
        let method_handle = tokio::spawn(async move { ank.update_configs(configs).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_update_configs_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![CONFIGS_PREFIX.to_owned()]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare configs
        let configs = HashMap::new();

        // Prepare handle for updating the configs
        let method_handle = tokio::spawn(async move { ank.update_configs(configs).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
    async fn itest_add_config_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare config
        let config = serde_yaml::Value::default();

        // Prepare handle for adding a config
        let method_handle =
            tokio::spawn(async move { ank.add_config("Test".to_owned(), config).await });

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
    async fn itest_add_config_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare config
        let config = serde_yaml::Value::default();

        // Prepare handle for adding a config
        let method_handle =
            tokio::spawn(async move { ank.add_config("Test".to_owned(), config).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_add_config_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare config
        let config = serde_yaml::Value::default();

        // Prepare handle for adding a config
        let method_handle =
            tokio::spawn(async move { ank.add_config("Test".to_owned(), config).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
    async fn itest_get_configs() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![CONFIGS_PREFIX]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the configs
        let method_handle = tokio::spawn(async move { ank.get_configs().await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let configs = HashMap::from_iter(vec![("Test".to_owned(), serde_yaml::Value::default())]);
        let complete_state = CompleteState::new_from_configs(configs.clone());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the configs
        let ret_configs = method_handle.await.unwrap().unwrap();

        assert_eq!(ret_configs, configs);
    }

    #[tokio::test]
    async fn itest_get_config() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the configs
        let method_handle = tokio::spawn(async move { ank.get_config("Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let configs = HashMap::from_iter(vec![(
            "Test".to_owned(),
            serde_yaml::Value::String("test".to_owned()),
        )]);
        let complete_state = CompleteState::new_from_configs(configs.clone());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the config
        let ret_config = method_handle.await.unwrap().unwrap();

        assert_eq!(ret_config, configs);
    }

    #[tokio::test]
    async fn itest_delete_all_configs_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![CONFIGS_PREFIX]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle = tokio::spawn(async move { ank.delete_all_configs().await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        assert!(method_handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn itest_delete_all_configs_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![CONFIGS_PREFIX]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle = tokio::spawn(async move { ank.delete_all_configs().await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_delete_all_configs_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![CONFIGS_PREFIX]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting the workload
        let method_handle = tokio::spawn(async move { ank.delete_all_configs().await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
    async fn itest_delete_config_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting a config
        let method_handle = tokio::spawn(async move { ank.delete_config("Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        assert!(method_handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn itest_delete_config_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting a config
        let method_handle = tokio::spawn(async move { ank.delete_config("Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
            content: super::ResponseType::Error("test".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_delete_config_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{CONFIGS_PREFIX}.Test")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for deleting a config
        let method_handle = tokio::spawn(async move { ank.delete_config("Test".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = Response {
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
    async fn itest_set_agent_tags_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{AGENTS_PREFIX}.agent_A.tags")]
                            && content.new_state.as_ref().is_some_and(|state| {
                                state.agents.as_ref().is_some_and(|agents| {
                                    agents.agents.get("agent_A").is_some_and(|agent| {
                                        agent.tags.as_ref().is_some_and(|tags| {
                                            tags.tags
                                                .get("environment")
                                                .is_some_and(|v| v == "production")
                                                && tags
                                                    .tags
                                                    .get("region")
                                                    .is_some_and(|v| v == "us-west")
                                        })
                                    })
                                })
                            })
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare tags
        let tags = HashMap::from([
            ("environment".to_owned(), "production".to_owned()),
            ("region".to_owned(), "us-west".to_owned()),
        ]);

        // Prepare handle for setting agent tags
        let method_handle =
            tokio::spawn(async move { ank.set_agent_tags("agent_A".to_owned(), tags).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let response = generate_test_response_update_state_success(request.get_id());

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        assert!(method_handle.await.unwrap().is_ok());
    }

    #[tokio::test]
    async fn itest_set_agent_tags_err() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{AGENTS_PREFIX}.agent_A.tags")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare tags
        let tags = HashMap::from([
            ("environment".to_owned(), "production".to_owned()),
            ("region".to_owned(), "us-west".to_owned()),
        ]);

        // Prepare handle for setting agent tags
        let method_handle =
            tokio::spawn(async move { ank.set_agent_tags("agent_A".to_owned(), tags).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate an error response
        let response = Response {
            content: super::ResponseType::Error("test error".to_owned()),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_set_agent_tags_mismatch_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                |request: &UpdateStateRequest| match &request.request.request_content {
                    Some(RequestContent::UpdateStateRequest(content)) => {
                        content.update_mask == vec![format!("{AGENTS_PREFIX}.agent_A.tags")]
                    }
                    _ => false,
                },
            )
            .return_once(|request: UpdateStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare tags
        let tags = HashMap::from([
            ("environment".to_owned(), "production".to_owned()),
            ("region".to_owned(), "us-west".to_owned()),
        ]);

        // Prepare handle for setting agent tags
        let method_handle =
            tokio::spawn(async move { ank.set_agent_tags("agent_A".to_owned(), tags).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response with wrong type
        let response = Response {
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
    async fn itest_get_agents() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![AGENTS_PREFIX]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the agents
        let method_handle = tokio::spawn(async move { ank.get_agents().await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the agents
        let ret_agents = method_handle.await.unwrap().unwrap();

        let expected_agent_attributes = AgentAttributes {
            tags: HashMap::from([("tag_key".to_owned(), "tag_value".to_owned())]),
            status: HashMap::from([
                ("free_memory".to_owned(), "1024".to_owned()),
                ("cpu_usage".to_owned(), "50".to_owned()),
            ]),
        };

        assert_eq!(
            ret_agents,
            HashMap::from([("agent_A".to_owned(), expected_agent_attributes)])
        );
    }

    #[tokio::test]
    async fn itest_get_agent_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![format!("{AGENTS_PREFIX}.agent_A")]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the agents
        let method_handle =
            tokio::spawn(async move { ank.get_agent(String::from("agent_A")).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the agents
        let ret_agents = method_handle.await.unwrap().unwrap();

        let expected_agent_attributes = AgentAttributes {
            tags: HashMap::from([("tag_key".to_owned(), "tag_value".to_owned())]),
            status: HashMap::from([
                ("free_memory".to_owned(), "1024".to_owned()),
                ("cpu_usage".to_owned(), "50".to_owned()),
            ]),
        };

        assert_eq!(ret_agents, expected_agent_attributes);
    }

    #[tokio::test]
    async fn itest_get_agent_not_found() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![format!("{AGENTS_PREFIX}.agent_not_there")]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting non-existing agent
        let method_handle =
            tokio::spawn(async move { ank.get_agent(String::from("agent_not_there")).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the result - should be an error
        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result, Err(AnkaiosError::AnkaiosResponseError(_))));
    }

    #[tokio::test]
    async fn itest_get_workload_states() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![WORKLOAD_STATES_PREFIX]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the workload states
        let method_handle = tokio::spawn(async move { ank.get_workload_states().await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the workload states
        let ret_wl_states = method_handle.await.unwrap().unwrap();

        assert_eq!(Vec::from(ret_wl_states).len(), 3);
    }

    #[tokio::test]
    async fn itest_get_execution_state_for_instance_name() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        // Prepare instance name
        let wl_instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "workload_id".to_owned(),
        );
        let masks = vec![wl_instance_name.get_filter_mask()];

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the workload execution state
        let method_handle = tokio::spawn(async move {
            ank.get_execution_state_for_instance_name(&wl_instance_name)
                .await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the workload execution state
        let ret_wl_exec_state = method_handle.await.unwrap().unwrap();

        // Cannot check the state - there are 3 workload states in the response state and all have
        // different states. Because they are saved as a hash map, the result differs. The only
        // field that is consistent is the additional info.
        assert_eq!(ret_wl_exec_state.additional_info, "Random info".to_owned());
    }

    #[tokio::test]
    async fn itest_get_workload_states_on_agent() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![format!("{WORKLOAD_STATES_PREFIX}.agent_A")]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the workload states on agent
        let method_handle =
            tokio::spawn(
                async move { ank.get_workload_states_on_agent("agent_A".to_owned()).await },
            );

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the workload states on agent
        let ret_wl_states = method_handle.await.unwrap().unwrap();

        assert_eq!(Vec::from(ret_wl_states).len(), 3);
    }

    #[tokio::test]
    async fn itest_get_workload_states_for_name() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == vec![format!("{WORKLOAD_STATES_PREFIX}")]
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the workload states for name
        let method_handle =
            tokio::spawn(async move { ank.get_workload_states_for_name("nginx".to_owned()).await });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the workload states for name
        let ret_wl_states = method_handle.await.unwrap().unwrap();

        assert_eq!(Vec::from(ret_wl_states).len(), 2);
    }

    #[tokio::test]
    async fn itest_wait_for_workload_to_reach_state_timeout() {
        let _guard = MOCKALL_SYNC.lock().await;

        // Prepare channel to intercept the request that is being
        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        // Prepare instance name
        let wl_instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "workload_id".to_owned(),
        );
        let masks = vec![wl_instance_name.get_filter_mask()];

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .withf(
                move |request: &GetStateRequest| match &request.request.request_content {
                    Some(RequestContent::CompleteStateRequest(content)) => {
                        content.field_mask == masks
                    }
                    _ => false,
                },
            )
            .return_once(move |request: GetStateRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });
        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        // Prepare handle for getting the workload states for name
        let method_handle = tokio::spawn(async move {
            ank.wait_for_workload_to_reach_state(wl_instance_name, WorkloadStateEnum::Failed)
                .await
        });

        // Get the request from the ControlInterface
        let request = request_receiver.await.unwrap();

        // Fabricate a response
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let response = Response {
            content: super::ResponseType::CompleteState(Box::new(complete_state.clone())),
            id: request.get_id(),
        };

        // Send the response
        response_sender.send(response).await.unwrap();

        // Get the workload states for name
        assert!(matches!(
            method_handle.await.unwrap(),
            Err(AnkaiosError::TimeoutError(_))
        ));
    }

    #[tokio::test]
    async fn itest_request_logs_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "1234".to_owned(),
        );

        let mut call_sequence = mockall::Sequence::new();
        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .in_sequence(&mut call_sequence)
            .return_once(move |request: AnkaiosLogsRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        let log_entries = vec![LogEntry {
            workload_name: instance_name.clone(),
            message: TEST_LOG_MESSAGE.to_owned(),
        }];
        let cloned_log_entries = log_entries.clone();
        ci_mock
            .expect_add_log_campaign()
            .times(1)
            .in_sequence(&mut call_sequence)
            .return_once(
                move |_request_id: String,
                 incoming_logs_sender: tokio::sync::mpsc::Sender<LogResponse>| {
                    incoming_logs_sender
                        .try_send(LogResponse::LogEntries(cloned_log_entries))
                        .unwrap();
                },
            );

        ci_mock
            .expect_disconnect()
            .times(1)
            .in_sequence(&mut call_sequence)
            .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let logs_request = InputLogsRequest {
            workload_names: vec![instance_name.clone()],
            ..Default::default()
        };

        let method_handle = tokio::spawn(async move { ank.request_logs(logs_request).await });

        let request = request_receiver.await.unwrap();

        let logs_accept_requested = Response {
            id: request.get_id(),
            content: super::ResponseType::LogsRequestAccepted(vec![instance_name.clone()]),
        };

        assert!(response_sender.send(logs_accept_requested).await.is_ok());

        let logs_entries_response = Response {
            id: request.get_id(),
            content: super::ResponseType::LogEntriesResponse(log_entries.clone()),
        };

        assert!(response_sender.send(logs_entries_response).await.is_ok());

        let mut log_campaign_response = method_handle.await.unwrap().unwrap();

        assert_eq!(
            log_campaign_response.accepted_workload_names,
            vec![instance_name.clone()]
        );

        assert_eq!(
            log_campaign_response.logs_receiver.recv().await.unwrap(),
            LogResponse::LogEntries(log_entries)
        );
    }

    #[tokio::test]
    async fn itest_request_logs_error() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "1234".to_owned(),
        );

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: AnkaiosLogsRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock.expect_add_log_campaign().never();

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let logs_request = InputLogsRequest {
            workload_names: vec![instance_name.clone()],
            ..Default::default()
        };

        let method_handle = tokio::spawn(async move { ank.request_logs(logs_request).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::Error("connection interruption".to_owned()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let log_campaign_response = method_handle.await.unwrap();
        assert!(log_campaign_response.is_err());
        assert_eq!(
            log_campaign_response.unwrap_err().to_string(),
            "Ankaios response error: connection interruption"
        );
    }

    #[tokio::test]
    async fn itest_request_logs_error_on_unexpected_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "1234".to_owned(),
        );

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: AnkaiosLogsRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock.expect_add_log_campaign().never();

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let logs_request = InputLogsRequest {
            workload_names: vec![instance_name.clone()],
            ..Default::default()
        };

        let method_handle = tokio::spawn(async move { ank.request_logs(logs_request).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::UpdateStateSuccess(Box::default()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let log_campaign_response = method_handle.await.unwrap();
        assert!(log_campaign_response.is_err());
        assert_eq!(
            log_campaign_response.unwrap_err().to_string(),
            "Response error: Received unexpected response type: 'UpdateStateSuccess(UpdateStateSuccess { added_workloads: [], deleted_workloads: [] })'"
        );
    }

    #[tokio::test]
    async fn itest_stop_receiving_logs_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "1234".to_owned(),
        );

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: LogsCancelRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock
            .expect_remove_log_campaign()
            .times(1)
            .return_const(());

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let accepted_workload_names = vec![instance_name.clone()];
        let (logs_sender, logs_receiver) = mpsc::channel(1);
        let log_campaign_response = LogCampaignResponse::new(
            REQUEST_ID.to_owned(),
            accepted_workload_names,
            logs_receiver,
        );

        let method_handle =
            tokio::spawn(async move { ank.stop_receiving_logs(log_campaign_response).await });

        let request = request_receiver.await.unwrap();

        let logs_cancel_accepted = Response {
            id: request.get_id(),
            content: super::ResponseType::LogsCancelAccepted,
        };

        assert!(response_sender.send(logs_cancel_accepted).await.is_ok());

        let result = method_handle.await.unwrap();
        assert!(result.is_ok());

        assert!(logs_sender.is_closed());
    }

    #[tokio::test]
    async fn itest_stop_receiving_logs_response_error() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "1234".to_owned(),
        );

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: LogsCancelRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock
            .expect_remove_log_campaign()
            .times(1)
            .return_const(());

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let accepted_workload_names = vec![instance_name.clone()];
        let (logs_sender, logs_receiver) = mpsc::channel(1);
        let log_campaign_response = LogCampaignResponse::new(
            REQUEST_ID.to_owned(),
            accepted_workload_names,
            logs_receiver,
        );

        let method_handle =
            tokio::spawn(async move { ank.stop_receiving_logs(log_campaign_response).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::Error("failed to cancel logs".to_owned()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Ankaios response error: failed to cancel logs"
        );

        assert!(logs_sender.is_closed());
    }

    #[tokio::test]
    async fn itest_stop_receiving_logs_unexpected_response() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let instance_name = WorkloadInstanceName::new(
            "agent_A".to_owned(),
            "workload_A".to_owned(),
            "1234".to_owned(),
        );

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: LogsCancelRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock
            .expect_remove_log_campaign()
            .times(1)
            .return_const(());

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let accepted_workload_names = vec![instance_name.clone()];
        let (logs_sender, logs_receiver) = mpsc::channel(1);
        let log_campaign_response = LogCampaignResponse::new(
            REQUEST_ID.to_owned(),
            accepted_workload_names,
            logs_receiver,
        );

        let method_handle =
            tokio::spawn(async move { ank.stop_receiving_logs(log_campaign_response).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::UpdateStateSuccess(Box::default()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Response error: Received unexpected response type."
        );

        assert!(logs_sender.is_closed());
    }

    #[tokio::test]
    async fn itest_register_events_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut call_sequence = mockall::Sequence::new();
        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .in_sequence(&mut call_sequence)
            .return_once(move |request: EventsRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        let event_entry = EventEntry::default();
        let cloned_event_entry = event_entry.clone();
        ci_mock
            .expect_add_events_campaign()
            .times(1)
            .in_sequence(&mut call_sequence)
            .return_once(
                move |_request_id: String,
                 incoming_events_sender: tokio::sync::mpsc::Sender<EventEntry>| {
                    incoming_events_sender
                        .try_send(cloned_event_entry)
                        .unwrap();
                },
            );

        ci_mock
            .expect_disconnect()
            .times(1)
            .in_sequence(&mut call_sequence)
            .returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let method_handle =
            tokio::spawn(async move { ank.register_event(vec![TEST_MASK.to_owned()]).await });

        let request = request_receiver.await.unwrap();

        let events_accept_requested = Response {
            id: request.get_id(),
            content: super::ResponseType::CompleteState(Box::default()),
        };

        assert!(response_sender.send(events_accept_requested).await.is_ok());

        let events_entry_response = Response {
            id: request.get_id(),
            content: super::ResponseType::EventResponse(Box::new(event_entry.clone())),
        };

        assert!(response_sender.send(events_entry_response).await.is_ok());

        let mut events_campaign_response = method_handle.await.unwrap().unwrap();

        assert_eq!(
            events_campaign_response
                .events_receiver
                .recv()
                .await
                .unwrap(),
            event_entry
        );
    }

    #[tokio::test]
    async fn itest_register_events_error() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: EventsRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock.expect_add_events_campaign().never();

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let method_handle =
            tokio::spawn(async move { ank.register_event(vec![TEST_MASK.to_owned()]).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::Error("connection interruption".to_owned()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let events_campaign_response = method_handle.await.unwrap();
        assert!(events_campaign_response.is_err());
        assert_eq!(
            events_campaign_response.unwrap_err().to_string(),
            "Ankaios response error: connection interruption"
        );
    }

    #[tokio::test]
    async fn itest_register_events_error_on_unexpected_response_type() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: EventsRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock.expect_add_events_campaign().never();

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let method_handle =
            tokio::spawn(async move { ank.register_event(vec![TEST_MASK.to_owned()]).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::UpdateStateSuccess(Box::default()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let events_campaign_response = method_handle.await.unwrap();
        assert!(events_campaign_response.is_err());
        assert_eq!(
            events_campaign_response.unwrap_err().to_string(),
            "Response error: Received unexpected response type: 'UpdateStateSuccess(UpdateStateSuccess { added_workloads: [], deleted_workloads: [] })'"
        );
    }

    #[tokio::test]
    async fn itest_unregister_events_ok() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: EventsCancelRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock
            .expect_remove_events_campaign()
            .times(1)
            .return_const(());

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let (events_sender, events_receiver) = mpsc::channel(1);
        let events_campaign_response =
            EventsCampaignResponse::new(REQUEST_ID.to_owned(), events_receiver);

        let method_handle =
            tokio::spawn(async move { ank.unregister_event(events_campaign_response).await });

        let request = request_receiver.await.unwrap();

        let events_cancel_accepted = Response {
            id: request.get_id(),
            content: super::ResponseType::EventsCancelAccepted,
        };

        assert!(response_sender.send(events_cancel_accepted).await.is_ok());

        let result = method_handle.await.unwrap();
        assert!(result.is_ok());

        assert!(events_sender.is_closed());
    }

    #[tokio::test]
    async fn itest_unregister_events_response_error() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: EventsCancelRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock
            .expect_remove_events_campaign()
            .times(1)
            .return_const(());

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let (events_sender, events_receiver) = mpsc::channel(1);
        let events_campaign_response =
            EventsCampaignResponse::new(REQUEST_ID.to_owned(), events_receiver);

        let method_handle =
            tokio::spawn(async move { ank.unregister_event(events_campaign_response).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::Error("failed to unregister".to_owned()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Ankaios response error: failed to unregister"
        );

        assert!(events_sender.is_closed());
    }

    #[tokio::test]
    async fn itest_unregister_events_unexpected_response() {
        let _guard = MOCKALL_SYNC.lock().await;

        let (request_sender, request_receiver) = tokio::sync::oneshot::channel();

        let mut ci_mock = ControlInterface::default();
        ci_mock
            .expect_write_request()
            .times(1)
            .return_once(move |request: EventsCancelRequest| {
                request_sender.send(request).unwrap();
                Ok(())
            });

        ci_mock
            .expect_remove_events_campaign()
            .times(1)
            .return_const(());

        ci_mock.expect_disconnect().times(1).returning(|| Ok(()));

        let (mut ank, response_sender) = generate_test_ankaios(ci_mock);

        let (events_sender, events_receiver) = mpsc::channel(1);
        let events_campaign_response =
            EventsCampaignResponse::new(REQUEST_ID.to_owned(), events_receiver);

        let method_handle =
            tokio::spawn(async move { ank.unregister_event(events_campaign_response).await });

        let request = request_receiver.await.unwrap();

        let response_error = Response {
            id: request.get_id(),
            content: super::ResponseType::UpdateStateSuccess(Box::default()),
        };

        assert!(response_sender.send(response_error).await.is_ok());

        let result = method_handle.await.unwrap();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Response error: Received unexpected response type."
        );

        assert!(events_sender.is_closed());
    }
}
