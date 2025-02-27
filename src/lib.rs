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
    rustdoc::missing_crate_level_docs
)]
#![deny(
    rustdoc::broken_intra_doc_links
)]
#![allow(rustdoc::private_intra_doc_links)]


#![doc(html_root_url = "https://docs.rs/ankaios_sdk/0.5.0-rc1")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/132572901?s=200&v=4")] // Icon above title in top-left
/* #![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/132572901?s=200&v=4")] */ // Icon in browser tab
#![doc(issue_tracker_base_url = "https://github.com/eclipse-ankaios/ank-sdk-rust/issues/")]

//! <div>
//! <img src="https://raw.githubusercontent.com/eclipse-ankaios/ankaios/refs/heads/main/logo/Ankaios__logo_for_dark_bgrd_clipped.png" />
//! </div>
//!
//! # Rust SDK for Eclipse Ankaios
//!  
//! [![github]](https://github.com/eclipse-ankaios/ank-sdk-rust)
//! [![crates-io]](https://crates.io/crates/ankaios-sdk)
//! [![docs-rs]](https://docs.rs/ankaios-sdk/0.5.0-rc1)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//! 
#![doc = include_str!("../target/README.md")]

mod docs;

mod errors;
pub use errors::AnkaiosError;

mod components;
pub use components::workload_mod::{Workload, WorkloadBuilder};
pub use components::workload_state_mod::{WorkloadStateCollection, WorkloadStateEnum};
pub use components::manifest::Manifest;
pub use components::complete_state::CompleteState;

mod ankaios;
pub use ankaios::Ankaios;
