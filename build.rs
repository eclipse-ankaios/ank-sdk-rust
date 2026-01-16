// Copyright (c) 2023 Elektrobit Automotive GmbH
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

#[path = "build/mod.rs"]
mod build;
use build::setup_proto_annotations;

fn main() {
    let mut builder = tonic_prost_build::configure()
        .build_server(true)
        .type_attribute("WorkloadState", "#[allow(dead_code)]"); // Workaround until the release of the ankaios api

    // Setup the proto objects
    builder = setup_proto_annotations(builder);

    builder
        .compile_protos(&["proto/control_api.proto"], &["proto"])
        .unwrap();
}
