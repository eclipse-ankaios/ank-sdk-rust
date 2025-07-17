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

use crate::{
    ankaios_api, components::workload_state_mod::WorkloadInstanceName,
    std_extensions::UnreachableOption,
};

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub workload_name: WorkloadInstanceName,
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
