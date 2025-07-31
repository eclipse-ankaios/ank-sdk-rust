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

#![warn(
    missing_docs,
    rustdoc::missing_crate_level_docs,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::str_to_string,
    clippy::redundant_type_annotations,
    clippy::absolute_paths,
    clippy::clone_on_ref_ptr,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::deref_by_slicing,
    clippy::else_if_without_else,
    clippy::unwrap_used
)]
#![deny(
    rustdoc::broken_intra_doc_links, // All links should be valid
    clippy::panic, // No panic should be invoked directly. If needed, justification is mandatory
    clippy::print_stdout, // The logger should be used instead of directly printing to stdout
)]
#![allow(
    clippy::module_name_repetitions, // Some structs have similar names with the module and this is intentional.
    clippy::struct_field_names, // Some structs have fields with the same name as the struct, but this is intentional.
    rustdoc::private_intra_doc_links, // Some links are private, but they are necessary for the documentation and solved by the "--document-private-items" flag.
)]
#![cfg_attr(
    test,
    allow(
        clippy::absolute_paths,
        clippy::panic,
        clippy::print_stdout,
        clippy::shadow_unrelated,
        clippy::unwrap_used,
    )
)]
#![doc(html_root_url = "https://docs.rs/ankaios_sdk/0.5.0")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/132572901?s=200&v=4")] // Icon above title in top-left
#![doc(issue_tracker_base_url = "https://github.com/eclipse-ankaios/ank-sdk-rust/issues/")]

//! <picture style="padding-bottom: 1em;">
//!   <source media="(prefers-color-scheme: dark)" srcset="https://github.com/eclipse-ankaios/ankaios/raw/main/logo/Ankaios__logo_for_dark_bgrd_clipped.png">
//!   <source media="(prefers-color-scheme: light)" srcset="https://github.com/eclipse-ankaios/ankaios/raw/main/logo/Ankaios__logo_for_light_bgrd_clipped.png">
//!   <img alt="Shows Ankaios logo" src="https://github.com/eclipse-ankaios/ankaios/raw/main/logo/Ankaios__logo_for_light_bgrd_clipped.png">
//! </picture>
//!
//! # Rust SDK for Eclipse Ankaios
//!  
//! [![github]](https://github.com/eclipse-ankaios/ank-sdk-rust)
//! [![crates-io]](https://crates.io/crates/ankaios-sdk)
//! [![docs-rs]](https://docs.rs/ankaios-sdk/0.5.0)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! [Eclipse Ankaios](https://github.com/eclipse-ankaios/ankaios) provides workload and
//! container orchestration for automotive High Performance Computing Platforms (HPCs).
//! While it can be used for various fields of applications, it is developed from scratch
//! for automotive use cases and provides a slim yet powerful solution to manage
//! containerized applications.
//!
//! The Rust SDK provides easy access from the container (workload) point-of-view
//! to manage the Ankaios system. A workload can use the Rust SDK to start, stop and
//! update other workloads and configs and get the state of the Ankaios system.
//!
//! ## Setup
//!
//! ### Setup via crates.io
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! ankaios_sdk = "0.5.0"
//! ```
//!
//! ### Clone and link as vendor
//!
//! Create a `vendor` folder and clone the crate inside:
//!
//! ```sh
//! mkdir -p vendor
//! git clone git@github.com:eclipse-ankaios/ank-sdk-rust.git vendor/ankaios_sdk
//! ```
//!
//! Then add it to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! ankaios_sdk = { path = "vendor/ankaios_sdk" }
//! ```
//!
//! ## Compatibility
//!
//! Please make sure the Rust SDK is compatible with the version of Ankaios you
//! are using. For information regarding versioning, please refer to this table:
//!
//! | Ankaios    | Rust SDK |
//! | -------- | ------- |
//! | 0.4.x and below | No Rust SDK available. Please update Ankaios. |
//! | 0.5.x | 0.5.x     |
//!
//! ## Usage
//!
//! After setup, you can use the Ankaios SDK to configure and run workloads
//! and request the state of the Ankaios system and the connected agents.
//!
//! The following example assumes that the code is running in a workload managed by
//! Ankaios with configured control interface access. This can also be tested from the
//! examples folder by running `./run_example.sh hello_ankaios`.
//!
//! ```rust
//! use ankaios_sdk::{Ankaios, AnkaiosError, Workload, WorkloadStateEnum};
//! use tokio::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new Ankaios object.
//!     // The connection to the control interface is automatically done at this step.
//!     let mut ank = Ankaios::new().await.expect("Failed to initialize");
//!
//!     // Create a new workload
//!     let workload = Workload::builder()
//!         .workload_name("dynamic_nginx")
//!         .agent_name("agent_A")
//!         .runtime("podman")
//!         .restart_policy("NEVER")
//!         .runtime_config(
//!             "image: docker.io/library/nginx\ncommandOptions: [\"-p\", \"8080:80\"]"
//!         ).build().expect("Failed to build workload");
//!     
//!     // Run the workload
//!     let response = ank.apply_workload(workload).await.expect("Failed to apply workload");
//!
//!     // Get the WorkloadInstanceName to check later if the workload is running
//!     let workload_instance_name = response.added_workloads[0].clone();
//!
//!     // Request the execution state based on the workload instance name
//!     match ank.get_execution_state_for_instance_name(&workload_instance_name).await {
//!         Ok(state) => {
//!             let exec_state = state.execution_state;
//!             println!("State: {:?}, substate: {:?}, info: {:?}", exec_state.state, exec_state.substate, exec_state.additional_info);
//!         }
//!         Err(err) => {
//!             println!("Error while getting workload state: {err:?}");
//!         }
//!     }
//!
//!     // Wait until the workload reaches the running state
//!     match ank.wait_for_workload_to_reach_state(workload_instance_name, WorkloadStateEnum::Running).await {
//!         Ok(_) => {
//!             println!("Workload reached the RUNNING state.");
//!         }
//!         Err(AnkaiosError::TimeoutError(_)) => {
//!             println!("Workload didn't reach the required state in time.");
//!         }
//!         Err(err) => {
//!             println!("Error while waiting for workload to reach state: {err:?}");
//!         }
//!     }
//!
//!     // Request the state of the system, filtered with the workloadStates
//!     let complete_state = ank.get_state(vec!["workloadStates".to_owned()]).await.expect("Failed to get the state");
//!
//!     // Get the workload states present in the complete state
//!     let workload_states_dict = complete_state.get_workload_states().get_as_list();
//!
//!     // Print the states of the workloads
//!     for workload_state in workload_states_dict {
//!         println!("Workload {} on agent {} has the state {:?}",
//!             workload_state.workload_instance_name.workload_name,
//!             workload_state.workload_instance_name.agent_name,
//!             workload_state.execution_state.state
//!         );
//!     }
//! }
//! ```
//!
//! For more details, please visit:
//!
//! * [Ankaios documentation](https://eclipse-ankaios.github.io/ankaios/latest/)
//! * [Rust SDK documentation](https://docs.rs/ankaios-sdk/0.5.0)
//!
//! ## Contributing
//!
//! This project welcomes contributions and suggestions. Before contributing, make sure to read the
//! [contribution guideline](docs/contributing/index.html).
//!
//! ## License
//!
//! Ankaios Rust SDK is licensed using the Apache License Version 2.0.
//!

mod ankaios_api;
mod docs;
mod std_extensions;

mod errors;
pub use errors::AnkaiosError;

mod components;

pub use components::complete_state::CompleteState;
pub use components::control_interface::ControlInterfaceState;
pub use components::log_types::{LogCampaignResponse, LogEntry, LogResponse, LogsRequest};
pub use components::manifest::Manifest;
pub use components::workload_mod::{Workload, WorkloadBuilder, File, FileContent};
pub use components::workload_state_mod::{
    WorkloadState, WorkloadStateCollection, WorkloadStateEnum,
};

mod ankaios;
pub use ankaios::Ankaios;
