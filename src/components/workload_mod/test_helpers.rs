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

use crate::Workload;
use crate::ankaios_api;
use ankaios_api::ank_base;
use std::collections::HashMap;
use std::path::Path;

#[allow(clippy::unnecessary_wraps)]
pub fn read_to_string_mock(path: &Path) -> Result<String, std::io::Error> {
    Ok(path.to_str().unwrap().to_owned())
}

pub fn generate_test_dependencies() -> HashMap<String, i32> {
    HashMap::from([
        (
            String::from("workload_C"),
            ank_base::AddCondition::AddCondRunning as i32,
        ),
        (
            String::from("workload_A"),
            ank_base::AddCondition::AddCondSucceeded as i32,
        ),
    ])
}

pub fn generate_test_runtime_config() -> String {
    String::from(
        r#"generalOptions: ["--version"]
commandOptions: ["--network=host"]
image: alpine:latest
commandArgs: ["bash"]
"#,
    )
}

pub fn generate_test_workload_proto<T: Into<String>>(
    agent_name: T,
    runtime_name: T,
) -> ank_base::Workload {
    let runtime_config = generate_test_runtime_config();
    let deps = generate_test_dependencies();

    ank_base::Workload {
        agent: Some(agent_name.into()),
        runtime: Some(runtime_name.into()),
        runtime_config: Some(runtime_config),
        restart_policy: Some(ank_base::RestartPolicy::Always as i32),
        dependencies: Some(ank_base::Dependencies { dependencies: deps }),
        tags: Some(ank_base::Tags {
            tags: HashMap::from([(String::from("key_test"), String::from("val_test"))]),
        }),
        control_interface_access: Some(ank_base::ControlInterfaceAccess {
            allow_rules: vec![ank_base::AccessRightsRule {
                access_rights_rule_enum: Some(ank_base::AccessRightsRuleEnum::StateRule(
                    ank_base::StateRule {
                        operation: ank_base::ReadWriteEnum::RwRead as i32,
                        filter_masks: vec![String::from("desiredState.workloads.workload_A")],
                    },
                )),
            }],
            deny_rules: vec![ank_base::AccessRightsRule {
                access_rights_rule_enum: Some(ank_base::AccessRightsRuleEnum::StateRule(
                    ank_base::StateRule {
                        operation: ank_base::ReadWriteEnum::RwWrite as i32,
                        filter_masks: vec![String::from("desiredState.workloads.workload_B")],
                    },
                )),
            }],
        }),
        configs: Some(ank_base::ConfigMappings {
            configs: [(String::from("alias_test"), String::from("config_1"))]
                .iter()
                .cloned()
                .collect(),
        }),
        files: Some(ank_base::Files {
            files: [ank_base::File {
                mount_point: "mount_point".to_owned(),
                file_content: Some(ank_base::FileContent::Data("Data".to_owned())),
            }]
            .to_vec(),
        }),
    }
}

pub fn generate_test_workload<T: Into<String>>(
    agent_name: T,
    workload_name: T,
    runtime_name: T,
) -> Workload {
    let name = workload_name.into();

    Workload {
        workload: generate_test_workload_proto(agent_name, runtime_name),
        main_mask: format!("desiredState.workloads.{}", name.clone()),
        masks: vec![format!("desiredState.workloads.{}", name.clone())],
        name,
    }
}
