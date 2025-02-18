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

use std::{collections::HashMap, path::Path, vec};
use crate::AnkaiosError;
use crate::Workload;

// Disable this from coverage
// https://github.com/rust-lang/rust/issues/84605
#[cfg(not(test))]
fn read_file_to_string(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[cfg(test)]
use crate::components::workload_mod::test_helpers::read_to_string_mock as read_file_to_string;

#[derive(Debug, Default)]
pub struct WorkloadBuilder {
    pub wl_name: String,
    pub wl_agent_name: String,
    pub wl_runtime: String,
    pub wl_runtime_config: String,
    pub wl_restart_policy: Option<String>,
    pub dependencies: HashMap<String, String>,
    pub tags: Vec<Vec<String>>,
    pub allow_rules: Vec<(String, Vec<String>)>,
    pub deny_rules: Vec<(String, Vec<String>)>,
    pub configs: HashMap<String, String>,
}

impl WorkloadBuilder{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn workload_name<T: Into<String>>(mut self, name: T) -> Self {
        self.wl_name = name.into();
        self
    }

    pub fn agent_name<T: Into<String>>(mut self, name: T) -> Self {
        self.wl_agent_name = name.into();
        self
    }

    pub fn runtime<T: Into<String>>(mut self, runtime: T) -> Self {
        self.wl_runtime = runtime.into();
        self
    }

    pub fn runtime_config<T: Into<String>>(mut self, runtime_config: T) -> Self {
        self.wl_runtime_config = runtime_config.into();
        self
    }

    pub fn runtime_config_from_file(self, file_path: &Path) -> Result<Self, AnkaiosError> {
        let runtime_config = match read_file_to_string(file_path) {
            Ok(config) => config,
            Err(err) => return Err(AnkaiosError::IoError(err)),
        };
        Ok(self.runtime_config(runtime_config))
    }

    pub fn restart_policy<T: Into<String>>(mut self, restart_policy: T) -> Self {
        self.wl_restart_policy = Some(restart_policy.into());
        self
    }

    pub fn add_dependency<T: Into<String>>(mut self, workload_name: T, condition: T) -> Self {
        self.dependencies.insert(workload_name.into(), condition.into());
        self
    }

    pub fn add_tag<T: Into<String>>(mut self, key: T, value: T) -> Self {
        self.tags.push(vec![key.into(), value.into()]);
        self
    }

    pub fn add_allow_rule<T: Into<String>>(mut self, operation: T, filter_masks: Vec<String>) -> Self {
        self.allow_rules.push((operation.into(), filter_masks));
        self
    }

    pub fn add_deny_rule<T: Into<String>>(mut self, operation: T, filter_masks: Vec<String>) -> Self {
        self.deny_rules.push((operation.into(), filter_masks));
        self
    }

    pub fn add_config<T: Into<String>>(mut self, alias: T, name: T) -> Self {
        self.configs.insert(alias.into(), name.into());
        self
    }

    pub fn build(self) -> Result<Workload, AnkaiosError> {
        if self.wl_name.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError("Workload can not be built without a name."));
        }
        let mut wl = Workload::new_from_builder(self.wl_name.clone());

        if self.wl_agent_name.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError("Workload can not be built without an agent name."));
        }
        if self.wl_runtime.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError("Workload can not be built without a runtime."));
        }
        if self.wl_runtime_config.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError("Workload can not be built without a runtime config."));
        }

        wl.update_agent_name(self.wl_agent_name.clone());
        wl.update_runtime(self.wl_runtime.clone());
        wl.update_runtime_config(self.wl_runtime_config.clone());

        if let Some(restart_policy) = self.wl_restart_policy.clone() {
            wl.update_restart_policy(restart_policy)?;
        }
        if !self.dependencies.is_empty() {
            wl.update_dependencies(self.dependencies.clone())?;
        }
        if !self.tags.is_empty() {
            wl.update_tags(&self.tags);
        }
        if !self.allow_rules.is_empty() {
            wl.update_allow_rules(self.allow_rules.clone())?;
        }
        if !self.deny_rules.is_empty() {
            wl.update_deny_rules(self.deny_rules.clone())?;
        }
        if !self.configs.is_empty() {
            wl.update_configs(self.configs.clone());
        }

        Ok(wl)
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
    use std::path::Path;
    use crate::AnkaiosError;
    use super::Workload;
    use crate::components::workload_mod::test_helpers::{
        generate_test_workload_proto, generate_test_runtime_config
    };

    #[test]
    fn utest_workload_builder() {
        let wl = Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config_from_file(Path::new(generate_test_runtime_config().as_str())).unwrap()
            .restart_policy("ALWAYS")
            .add_dependency("workload_A", "ADD_COND_SUCCEEDED")
            .add_dependency("workload_C", "ADD_COND_RUNNING")
            .add_tag("key_test", "val_test")
            .add_allow_rule("Read", vec!["desiredState.workloads.workload_A".to_string()])
            .add_deny_rule("Write", vec!["desiredState.workloads.workload_B".to_string()])
            .add_config("alias_test", "config_1")
            .build();
        assert!(wl.is_ok());
        assert_eq!(wl.unwrap().to_proto(), generate_test_workload_proto("agent_A".to_string(), "podman".to_string()));
    }

    #[test]
    fn utest_build_return_err() {
        // No workload name
        assert!(matches!(
            Workload::builder()
                .agent_name("agent_A")
                .runtime("podman")
                .runtime_config("config")
                .build()
                .unwrap_err(),
            AnkaiosError::WorkloadBuilderError(msg) if msg == "Workload can not be built without a name."
        ));

        // No agent
        assert!(matches!(
            Workload::builder()
                .workload_name("Test")
                .runtime("podman")
                .runtime_config("config")
                .build()
                .unwrap_err(),
            AnkaiosError::WorkloadBuilderError(msg) if msg == "Workload can not be built without an agent name."
        ));

        // No runtime
        assert!(matches!(
            Workload::builder()
                .workload_name("Test")
                .agent_name("agent_A")
                .runtime_config("config")
                .build()
                .unwrap_err(),
            AnkaiosError::WorkloadBuilderError(msg) if msg == "Workload can not be built without a runtime."
        ));

        // No runtime config
        assert!(matches!(
            Workload::builder()
                .workload_name("Test")
                .agent_name("agent_A")
                .runtime("podman")
                .build()
                .unwrap_err(),
            AnkaiosError::WorkloadBuilderError(msg) if msg == "Workload can not be built without a runtime config."
        ));
    }
}