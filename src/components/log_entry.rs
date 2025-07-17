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

use crate::ankaios_api;

use super::workload_state_mod::WorkloadInstanceName;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub workload_name: WorkloadInstanceName,
    pub message: String,
}

impl TryFrom<ankaios_api::ank_base::LogEntry> for LogEntry {
    type Error = String;
    fn try_from(value: ankaios_api::ank_base::LogEntry) -> Result<Self, Self::Error> {
        Ok(LogEntry {
            workload_name: value
                .workload_name
                .ok_or_else(|| format!("LogEntry has no workload instance name."))?
                .into(), // TODO: switch to try_from
            message: value.message,
        })
    }
}
