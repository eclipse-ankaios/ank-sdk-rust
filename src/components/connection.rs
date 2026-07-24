// Copyright (c) 2026 Elektrobit Automotive GmbH
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

//! This module contains the [`Connection`] trait, implemented by the different interfaces the
//! SDK can connect to [Ankaios] through: the Control Interface (Unix FIFO pipes an agent mounts
//! into a workload's own container, for use from inside that workload; behind the
//! `control_interface` feature) and the Command Interface (a direct gRPC connection to the
//! Ankaios server, for use from outside the cluster; behind the `grpc_server_interface` feature).
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::AnkaiosError;
use crate::ankaios_api::ank_base::Request as AnkaiosRequest;
use crate::components::event_types::EventEntry;
use crate::components::log_types::LogResponse;

#[cfg(feature = "control_interface")]
pub mod control_interface;
#[cfg(feature = "grpc_server_interface")]
pub mod grpc_interface;
mod helpers;

pub(crate) use helpers::{
    RequestId, SynchronizedSenderMap, forward_event_response, forward_log_entries,
    forward_logs_stop_response,
};

/// Version of [Ankaios](https://eclipse-ankaios.github.io/ankaios) that is compatible with this
/// SDK, sent as part of every connection handshake (control interface or gRPC), regardless of
/// transport.
pub(crate) const ANKAIOS_VERSION: &str = "1.0.0";

/// Abstracts over the transport used to exchange [`AnkaiosRequest`]/[`Response`](crate::Response)
/// messages with Ankaios, so that [`Ankaios`](crate::Ankaios) can work identically regardless of
/// whether it is connected via the control interface or via gRPC.
#[cfg_attr(test, automock)]
#[async_trait]
pub(crate) trait Connection: Send {
    /// Establishes the connection, waiting up to `timeout` for it to be accepted.
    async fn connect(&mut self, timeout: Duration) -> Result<(), AnkaiosError>;

    /// Tears down the connection.
    fn disconnect(&mut self) -> Result<(), AnkaiosError>;

    /// Sends a request. The request must already have been converted to its proto
    /// representation via [`Request::to_proto`](crate::Request::to_proto).
    async fn write_request(&mut self, request: AnkaiosRequest) -> Result<(), AnkaiosError>;

    /// Registers a channel to receive log entries for an ongoing log campaign.
    fn add_log_campaign(&mut self, request_id: RequestId, logs_sender: mpsc::Sender<LogResponse>);

    /// Unregisters a log campaign started via [`add_log_campaign`](Connection::add_log_campaign).
    fn remove_log_campaign(&mut self, request_id: &str);

    /// Registers a channel to receive events for an ongoing event campaign.
    fn add_events_campaign(
        &mut self,
        request_id: RequestId,
        events_sender: mpsc::Sender<EventEntry>,
    );

    /// Unregisters an event campaign started via [`add_events_campaign`](Connection::add_events_campaign).
    fn remove_events_campaign(&mut self, request_id: &str);
}
