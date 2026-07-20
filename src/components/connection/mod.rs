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

//! This module contains the [`Connection`] trait, which abstracts over the different ways
//! this SDK can talk to [Ankaios]: the always-available control interface (named pipes, used
//! from inside a workload) and, behind the `grpc` feature, a direct gRPC connection to the
//! Ankaios server (used from outside a workload).
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::AnkaiosError;
use crate::ankaios_api::ank_base::Request as AnkaiosRequest;
use crate::components::event_types::EventEntry;
use crate::components::log_types::{LogEntry, LogResponse};
use crate::components::workload_state_mod::WorkloadInstanceName;

#[cfg(feature = "control_interface")]
pub mod control_interface;
#[cfg(feature = "grpc_server_interface")]
pub mod grpc_interface;

/// Version of [Ankaios](https://eclipse-ankaios.github.io/ankaios) that is compatible with this
/// SDK, sent as part of every connection handshake (control interface or gRPC), regardless of
/// transport.
pub(crate) const ANKAIOS_VERSION: &str = "1.0.0";

/// Maps request IDs to the sender half of a channel used to forward messages for an ongoing
/// log or event campaign. Shared infrastructure for any [`Connection`] implementation that
/// supports streaming campaigns (log entries, events) keyed by request ID.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub(crate) struct SynchronizedSenderMap<T> {
    /// A map of request IDs to their corresponding senders.
    pub(crate) senders_map: Arc<Mutex<HashMap<String, mpsc::Sender<T>>>>,
}

impl<T> SynchronizedSenderMap<T> {
    /// Inserts a new sender for a request ID part of a started campaign.
    ///
    /// ## Arguments
    ///
    /// * `request_id` - A [String] representing the request ID;
    /// * `sender` - A [`mpsc::Sender<T>`] to forward campaign messages.
    ///
    pub(crate) fn insert(&mut self, request_id: String, sender: mpsc::Sender<T>) {
        self.senders_map
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .insert(request_id, sender);
    }

    /// Removes a sender by its request ID.
    ///
    /// ## Arguments
    ///
    /// * `request_id` - A [String] representing the request ID.
    ///
    /// ## Returns
    ///
    /// An [`Option<mpsc::Sender<T>>`] if the request ID was found and removed, otherwise `None`.
    pub(crate) fn remove(&mut self, request_id: &str) -> Option<mpsc::Sender<T>> {
        self.senders_map
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .remove(request_id)
    }

    /// Gets a cloned sender by its request ID.
    ///
    /// ## Arguments
    ///
    /// * `request_id` - A [String] representing the request ID.
    ///
    /// ## Returns
    ///
    /// An [`Option<mpsc::Sender<T>>`] if the request ID was found, otherwise `None`.
    pub(crate) fn get_cloned(&self, request_id: &str) -> Option<mpsc::Sender<T>> {
        self.senders_map
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .get(request_id)
            .cloned()
    }
}

impl<T> Default for SynchronizedSenderMap<T> {
    fn default() -> Self {
        SynchronizedSenderMap {
            senders_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Forwards the log entries to the appropriate log campaign receiver. Shared by every
/// [`Connection`] implementation's response dispatch, since it operates purely on the already
/// envelope-decoded [`LogEntry`] payload.
///
/// ## Arguments
///
/// * `request_id` - A [String] representing the request ID of the initial logs request of the log campaign;
/// * `log_entries` - A [`Vec<LogEntry>`] containing the log entries of workload to be forwarded;
/// * `logs_sender_map` - A [`SynchronizedSenderMap<LogResponse>`] to forward log entries and stop responses for a log campaign.
///
pub(crate) async fn forward_log_entries(
    request_id: String,
    log_entries: Vec<LogEntry>,
    logs_sender_map: &SynchronizedSenderMap<LogResponse>,
) {
    let log_entries_sender = logs_sender_map.get_cloned(&request_id);

    if let Some(sender) = log_entries_sender {
        log::trace!(
            "Forwarding log entries for request id '{request_id}' to log campaign receiver."
        );
        sender
            .send(LogResponse::LogEntries(log_entries))
            .await
            .unwrap_or_else(|err| {
                log::error!("Error while sending log entries: '{err}'");
            });
    } else {
        log::debug!(
            "Received log entries response for request id '{request_id}', but no log campaign found."
        );
    }
}

/// Forwards the logs stop response for a workload instance to the appropriate log campaign
/// receiver. Shared by every [`Connection`] implementation's response dispatch.
///
/// ## Arguments
///
/// * `request_id` - A [String] representing the request ID of the initial logs request of the log campaign;
/// * `instance_name` - A [`WorkloadInstanceName`] for which the logs stop response is sent;
/// * `logs_sender_map` - A [`SynchronizedSenderMap<LogResponse>`] to forward log entries and stop responses for a log campaign.
///
pub(crate) async fn forward_logs_stop_response(
    request_id: String,
    instance_name: WorkloadInstanceName,
    logs_sender_map: &mut SynchronizedSenderMap<LogResponse>,
) {
    let log_entries_sender = logs_sender_map.get_cloned(&request_id);
    if let Some(sender) = log_entries_sender {
        log::trace!(
            "Forwarding logs stop response for workload '{instance_name:?}' of request id '{request_id}' to log campaign receiver."
        );
        sender
            .send(LogResponse::LogsStopResponse(instance_name))
            .await
            .unwrap_or_else(|err| {
                log::error!("Error while sending log stop message: '{err}'");
            });
    } else {
        log::debug!(
            "Received logs stop response for request id '{request_id}', but no log campaign found."
        );
    }
}

/// Forwards the event entries to the appropriate receiver. Shared by every [`Connection`]
/// implementation's response dispatch.
///
/// ## Arguments
///
/// * `request_id` - A [String] representing the request ID of the initial event request of the events campaign;
/// * `event_entry` - A [`EventEntry`] representing the event to be forwarded;
/// * `event_sender_map` - A [`SynchronizedSenderMap<EventEntry>`] to forward an event for an event campaign.
///
pub(crate) async fn forward_event_response(
    request_id: String,
    event_entry: Box<EventEntry>,
    event_sender_map: &SynchronizedSenderMap<EventEntry>,
) {
    let event_sender = event_sender_map.get_cloned(&request_id);

    if let Some(sender) = event_sender {
        log::trace!("Forwarding event entry for request id '{request_id}' to receiver.");
        sender.send(*event_entry).await.unwrap_or_else(|err| {
            log::error!("Error while sending event entry: '{err}'");
        });
    } else {
        log::debug!(
            "Received event entry for request id '{request_id}', but no event campaign found."
        );
    }
}

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
    fn add_log_campaign(&mut self, request_id: String, logs_sender: mpsc::Sender<LogResponse>);

    /// Unregisters a log campaign started via [`add_log_campaign`](Connection::add_log_campaign).
    fn remove_log_campaign(&mut self, request_id: &str);

    /// Registers a channel to receive events for an ongoing event campaign.
    fn add_events_campaign(&mut self, request_id: String, events_sender: mpsc::Sender<EventEntry>);

    /// Unregisters an event campaign started via [`add_events_campaign`](Connection::add_events_campaign).
    fn remove_events_campaign(&mut self, request_id: &str);
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
    use tokio::sync::mpsc;

    use super::{
        SynchronizedSenderMap, forward_event_response, forward_log_entries,
        forward_logs_stop_response,
    };
    use crate::components::event_types::EventEntry;
    use crate::components::log_types::LogResponse;
    use crate::components::workload_state_mod::WorkloadInstanceName;

    const REQUEST_ID: &str = "request_id_1";

    // A dropped receiver makes the forwarding send() fail; these tests only check that the
    // failure is swallowed (logged) instead of panicking.

    #[tokio::test]
    async fn utest_forward_log_entries_survives_dropped_receiver() {
        let mut map = SynchronizedSenderMap::<LogResponse>::default();
        let (sender, receiver) = mpsc::channel(1);
        map.insert(REQUEST_ID.to_owned(), sender);
        drop(receiver);

        forward_log_entries(REQUEST_ID.to_owned(), Vec::default(), &map).await;
    }

    #[tokio::test]
    async fn utest_forward_logs_stop_response_survives_dropped_receiver() {
        let mut map = SynchronizedSenderMap::<LogResponse>::default();
        let (sender, receiver) = mpsc::channel(1);
        map.insert(REQUEST_ID.to_owned(), sender);
        drop(receiver);

        forward_logs_stop_response(
            REQUEST_ID.to_owned(),
            WorkloadInstanceName::new(
                "agent_A".to_owned(),
                "workload_A".to_owned(),
                "id_a".to_owned(),
            ),
            &mut map,
        )
        .await;
    }

    #[tokio::test]
    async fn utest_forward_event_response_survives_dropped_receiver() {
        let mut map = SynchronizedSenderMap::<EventEntry>::default();
        let (sender, receiver) = mpsc::channel(1);
        map.insert(REQUEST_ID.to_owned(), sender);
        drop(receiver);

        forward_event_response(REQUEST_ID.to_owned(), Box::default(), &map).await;
    }
}
