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

tonic::include_proto!("ank_base"); // The string specified here must match the proto package name

pub use crate::ankaios_api::helpers::serialize_to_ordered_map;

pub use access_rights_rule::AccessRightsRuleEnum;
pub use config_item::ConfigItemEnum;
pub use execution_state::ExecutionStateEnum;
pub use file::FileContent;
pub use request::RequestContent;
