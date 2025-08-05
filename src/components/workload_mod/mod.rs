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

//! This module contains the [Workload] and [`WorkloadBuilder`] structs, which are used to
//! define and build workloads for the [Ankaios] application.
//!
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

mod file;
mod workload;
mod workload_builder;

pub use file::{File, FileContent};
pub use workload::{WORKLOADS_PREFIX, Workload};
pub use workload_builder::WorkloadBuilder;

#[cfg(test)]
pub mod test_helpers;
