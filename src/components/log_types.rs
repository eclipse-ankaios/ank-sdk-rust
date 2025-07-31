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

use tokio::sync::mpsc::Receiver;

use crate::{
    ankaios_api, components::workload_state_mod::WorkloadInstanceName,
    std_extensions::UnreachableOption,
};

/// Struct that represents a logs request.
///
#[derive(Debug, Clone)]
pub struct LogsRequest {
    /// The names of the workloads for which logs are requested.
    pub workload_names: Vec<WorkloadInstanceName>,
    /// Enable or disable whether to continuously follow the logs
    pub follow: bool,
    /// The number of lines to be output at the end of the logs (default: -1, which means all lines).
    pub tail: i32,
    /// Show logs after the timestamp in RFC3339 format (default: None).
    pub since: Option<String>,
    /// Show logs before the timestamp in RFC3339 format (default: None).
    pub until: Option<String>,
}

impl Default for LogsRequest {
    #[doc(hidden)]
    /// Creates a new default `LogsRequest` object.
    ///
    /// ## Returns
    ///
    /// A new [LogsRequest] with default parameters.
    fn default() -> Self {
        LogsRequest {
            workload_names: vec![],
            follow: false,
            tail: -1,
            since: None,
            until: None,
        }
    }
}

/// Struct that represents a log entry.
///
#[derive(Debug, Clone, PartialEq)]
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

/// Enum that represents the type of log responses that are available in a `LogCampaignResponse`.
///
#[derive(Debug, PartialEq)]
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
    request_id: String,
    /// A vector of [`WorkloadInstanceName`] that were accepted for log collection.
    pub accepted_workload_names: Vec<WorkloadInstanceName>,
    /// A [Receiver] that can be used to receive log responses.
    pub logs_receiver: Receiver<LogResponse>,
}

impl LogCampaignResponse {
    #[doc(hidden)]
    /// Creates a new `LogCampaignResponse` object.
    ///
    /// ## Arguments
    ///
    /// * `request_id` - The request id as a [String] for the logs request.
    /// * `accepted_workload_names` - A vector of [WorkloadInstanceName] that were accepted for log retrieval.
    /// * `logs_receiver` - A [Receiver<LogResponse>] that can be used to receive log responses.
    ///
    /// ## Returns
    ///
    /// A new [`LogCampaignResponse`] object.
    #[must_use]
    pub fn new(
        request_id: String,
        accepted_workload_names: Vec<WorkloadInstanceName>,
        logs_receiver: Receiver<LogResponse>,
    ) -> Self {
        LogCampaignResponse {
            request_id,
            accepted_workload_names,
            logs_receiver,
        }
    }

    #[doc(hidden)]
    /// Gets the request id.
    ///
    /// ## Returns
    ///
    /// The request id as a [String].
    #[must_use]
    pub fn get_request_id(&self) -> String {
        self.request_id.clone()
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
    use super::{ankaios_api, LogCampaignResponse, LogEntry, WorkloadInstanceName};
    use tokio::sync::mpsc;

    const REQUEST_ID: &str = "test_request_id";
    const AGENT_A: &str = "agent_A";
    const WORKLOAD_NAME: &str = "workload_A";
    const WORKLOAD_ID: &str = "id_a";
    const TEST_LOG_MESSAGE: &str = "test_log_message";

    #[test]
    fn utest_log_entry_proto_to_sdk_object() {
        let proto_entry = ankaios_api::ank_base::LogEntry {
            workload_name: Some(ankaios_api::ank_base::WorkloadInstanceName {
                agent_name: AGENT_A.to_owned(),
                workload_name: WORKLOAD_NAME.to_owned(),
                id: WORKLOAD_ID.to_owned(),
            }),
            message: TEST_LOG_MESSAGE.to_owned(),
        };
        let sdk_entry = LogEntry::from(proto_entry);
        assert_eq!(
            sdk_entry.workload_name,
            WorkloadInstanceName::new(
                AGENT_A.to_owned(),
                WORKLOAD_NAME.to_owned(),
                WORKLOAD_ID.to_owned()
            )
        );
        assert_eq!(sdk_entry.message, TEST_LOG_MESSAGE.to_owned());
    }

    #[test]
    fn utest_log_campaign_response_get_request_id() {
        let (_logs_sender, logs_receiver) = mpsc::channel(1);
        let log_campaign_response =
            LogCampaignResponse::new(REQUEST_ID.to_owned(), Vec::default(), logs_receiver);
        assert_eq!(log_campaign_response.get_request_id(), REQUEST_ID);
    }
}
