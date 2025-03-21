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

#![allow(
    clippy::doc_markdown,
    clippy::trivially_copy_pass_by_ref,
    clippy::enum_variant_names,
    clippy::needless_pass_by_value,
    clippy::str_to_string,
    clippy::absolute_paths,
    clippy::shadow_reuse,
)]

pub mod ank_base {
    tonic::include_proto!("ank_base");
}

pub mod control_api {
    tonic::include_proto!("control_api");
}

mod convert;
