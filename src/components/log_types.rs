// Copyright (c) 2025 Elektrobit Automotive GmbH
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

//! This module contains structs and enums that are used to
//! work with the components of the [Ankaios] application.
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

use crate::ankaios::CHANNEL_SIZE;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    ankaios_api, components::workload_state_mod::WorkloadInstanceName,
    std_extensions::UnreachableOption,
};

/// Struct that represents a log entry.
///
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// The name of the workload that produced the log entry.
    pub workload_name: WorkloadInstanceName,
    /// The log message.
    pub message: String,
}

impl From<ankaios_api::ank_base::LogEntry> for LogEntry {
    fn from(value: ankaios_api::ank_base::LogEntry) -> Self {
        LogEntry {
            workload_name: value.workload_name.unwrap_or_unreachable().into(),
            message: value.message,
        }
    }
}

/// Enum that represents the type of log responses that are available in a LogCampaignResponse.
///
#[derive(Debug)]
pub enum LogResponse {
    /// A response containing log entries.
    LogEntries(Vec<LogEntry>),
    /// A response indicating the stop of log entries for a specific workload.
    LogsStopResponse(WorkloadInstanceName),
}

/// Struct that represents a response of a log request.
///
#[derive(Debug)]
pub struct LogCampaignResponse {
    /// The request id as a [String] of the initial logs request.
    pub request_id: String,
    /// A vector of [WorkloadInstanceName] that were accepted for log collection.
    pub accepted_workload_names: Vec<WorkloadInstanceName>,
    /// A [Receiver] that can be used to receive log responses.
    pub logs_receiver: Receiver<LogResponse>,
}

impl LogCampaignResponse {
    #[doc(hidden)]
    /// Creates a new `LogCampaignResponse` object.
    ///
    ///
    /// ## Arguments
    ///
    /// * `request_id` - The request id as a [String] for the logs request.
    /// * `accepted_workload_names` - A vector of [WorkloadInstanceName] that were accepted for log retrieval.
    ///
    /// ## Returns
    ///
    /// A new [`(Sender<LogsResponse>, LogCampaignResponse)`] tuple.
    pub fn new(
        request_id: String,
        accepted_workload_names: Vec<WorkloadInstanceName>,
    ) -> (Sender<LogResponse>, Self) {
        let (logs_sender, logs_receiver) = channel(CHANNEL_SIZE);
        (
            logs_sender,
            LogCampaignResponse {
                request_id,
                accepted_workload_names,
                logs_receiver,
            },
        )
    }

    #[doc(hidden)]
    /// Gets the request id.
    ///
    /// ## Returns
    ///
    /// The request id as a [String].
    pub fn get_request_id(&self) -> String {
        self.request_id.clone()
    }
}
