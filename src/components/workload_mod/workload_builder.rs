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

use crate::AnkaiosError;
use crate::Workload;
use std::{collections::HashMap, path::Path, vec};

// Disable this from coverage
// https://github.com/rust-lang/rust/issues/84605
#[cfg(not(test))]
use std::{fs, io};
/// Helper function to read a file to a string.
#[cfg(not(test))]
fn read_file_to_string(path: &Path) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

#[cfg(test)]
use crate::components::workload_mod::test_helpers::read_to_string_mock as read_file_to_string;

use super::file::File;

/// A builder struct for the [Workload] struct.
///
/// # Example
///
/// ## Create a workload using the [`WorkloadBuilder`]:
///
/// ```rust
/// use ankaios_sdk::{Workload, WorkloadBuilder, File};
///
/// let workload: Workload = WorkloadBuilder::new()
///     .workload_name("example_workload")
///     .agent_name("agent_A")
///     .runtime("podman")
///     .restart_policy("NEVER")
///     .runtime_config("image: docker.io/library/nginx\n
///                      commandOptions: [\"-p\", \"8080:80\"]")
///     .add_dependency("other_workload", "ADD_COND_RUNNING")
///     .add_tag("key1", "value1")
///     .add_tag("key2", "value2")
///     .add_file_object(File::text("/etc/config.yaml", "debug: true"))
///     .build().unwrap();
/// ```
#[must_use] // Added to ensure that the returned Self from the methods is used.
#[derive(Debug, Default)]
pub struct WorkloadBuilder {
    /// The name of the workload.
    pub wl_name: String,
    /// The name of the agent.
    pub wl_agent_name: String,
    /// The runtime.
    pub wl_runtime: String,
    /// The runtime config.
    pub wl_runtime_config: String,
    /// The restart policy. Allowed values: "`ALWAYS`", "`ON_FAILURE`", "`NEVER`".
    pub wl_restart_policy: Option<String>,
    /// The dependencies. Allowed values: "`ADD_COND_SUCCEEDED`", "`ADD_COND_FAILED`", "`ADD_COND_RUNNING`".
    pub dependencies: HashMap<String, String>,
    /// The tags.
    pub tags: Vec<Vec<String>>,
    /// The allow rules. Allowed values: "`Nothing`", "`Write`", "`Read`", "`ReadWrite`".
    pub allow_rules: Vec<(String, Vec<String>)>,
    /// The deny rules. Allowed values: "`Nothing`", "`Write`", "`Read`", "`ReadWrite`".
    pub deny_rules: Vec<(String, Vec<String>)>,
    /// The config aliases.
    pub configs: HashMap<String, String>,
    /// The workload files.
    pub files: Vec<File>,
}

impl WorkloadBuilder {
    /// Creates a new [`WorkloadBuilder`] instance.
    ///
    /// ## Returns
    ///
    /// A new [`WorkloadBuilder`] instance.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the name of the workload.
    ///
    /// ## Arguments
    ///
    /// * `name` - A [String] that represents the name of the workload.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn workload_name<T: Into<String>>(mut self, name: T) -> Self {
        self.wl_name = name.into();
        self
    }

    /// Sets the name of the agent.
    ///
    /// ## Arguments
    ///
    /// * `name` - A [String] that represents the name of the agent.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn agent_name<T: Into<String>>(mut self, name: T) -> Self {
        self.wl_agent_name = name.into();
        self
    }

    /// Sets the runtime.
    ///
    /// ## Arguments
    ///
    /// * `runtime` - A [String] that represents the runtime.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn runtime<T: Into<String>>(mut self, runtime: T) -> Self {
        self.wl_runtime = runtime.into();
        self
    }

    /// Sets the runtime config.
    ///
    /// ## Arguments
    ///
    /// * `runtime_config` - A [String] that represents the runtime config.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn runtime_config<T: Into<String>>(mut self, runtime_config: T) -> Self {
        self.wl_runtime_config = runtime_config.into();
        self
    }

    /// Sets the runtime config from a file.
    ///
    /// ## Arguments
    ///
    /// * `file_path` - A [Path] object that represents the path to the file.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    ///
    /// ## Errors
    ///
    /// Returns an [`AnkaiosError`]::[`IoError`](AnkaiosError::IoError) if the file can not be read.
    pub fn runtime_config_from_file(self, file_path: &Path) -> Result<Self, AnkaiosError> {
        let runtime_config = read_file_to_string(file_path)?;
        Ok(self.runtime_config(runtime_config))
    }

    /// Sets the restart policy.
    ///
    /// ## Arguments
    ///
    /// * `restart_policy` - A [String] that represents the restart policy.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn restart_policy<T: Into<String>>(mut self, restart_policy: T) -> Self {
        self.wl_restart_policy = Some(restart_policy.into());
        self
    }

    /// Adds a dependency.
    ///
    /// ## Arguments
    ///
    /// * `workload_name` - A [String] that represents the name of the workload;
    /// * `condition` - A [String] that represents the condition.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn add_dependency<T: Into<String>>(mut self, workload_name: T, condition: T) -> Self {
        self.dependencies
            .insert(workload_name.into(), condition.into());
        self
    }

    /// Adds a tag.
    ///
    /// ## Arguments
    ///
    /// * `key` - A [String] that represents the key of the tag;
    /// * `value` - A [String] that represents the value of the tag.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn add_tag<T: Into<String>>(mut self, key: T, value: T) -> Self {
        self.tags.push(vec![key.into(), value.into()]);
        self
    }

    /// Adds an allow rule.
    ///
    /// ## Arguments
    ///
    /// * `operation` - A [String] that represents the operation;
    /// * `filter_masks` - A [vector](Vec) of [strings](String) that represents the filter masks.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn add_allow_rule<T: Into<String>>(
        mut self,
        operation: T,
        filter_masks: Vec<String>,
    ) -> Self {
        self.allow_rules.push((operation.into(), filter_masks));
        self
    }

    /// Adds a deny rule.
    ///
    /// ## Arguments
    ///
    /// * `operation` - A [String] that represents the operation;
    /// * `filter_masks` - A [vector](Vec) of [strings](String) that represents the filter masks.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn add_deny_rule<T: Into<String>>(
        mut self,
        operation: T,
        filter_masks: Vec<String>,
    ) -> Self {
        self.deny_rules.push((operation.into(), filter_masks));
        self
    }

    /// Adds a config alias.
    ///
    /// ## Arguments
    ///
    /// * `alias` - A [String] that represents the alias of the config;
    /// * `name` - A [String] that represents the name of the config it refers to.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn add_config<T: Into<String>>(mut self, alias: T, name: T) -> Self {
        self.configs.insert(alias.into(), name.into());
        self
    }

    /// Adds a [`File`] object to the workload.
    ///
    /// ## Arguments
    ///
    /// * `file` - A [`File`] object that represents the file to be added.
    ///
    /// ## Returns
    ///
    /// The [`WorkloadBuilder`] instance.
    pub fn add_file(mut self, file: File) -> Self {
        self.files.push(file);

        self
    }

    /// Creates a new `Workload` instance from a Map.
    ///
    /// # Arguments
    ///
    /// * `name` - A [String] that represents the name of the workload;
    /// * `dict_workload` - An instance of [`serde_yaml::Mapping`] that represents the workload.
    ///
    /// # Returns
    ///
    /// A new [Workload] instance.
    ///
    /// # Errors
    ///
    /// Returns an [`AnkaiosError`]::[`WorkloadBuilderError`](AnkaiosError::WorkloadBuilderError) if the builder fails to build the workload.
    pub fn build(self) -> Result<Workload, AnkaiosError> {
        if self.wl_name.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError(
                "Workload can not be built without a name.",
            ));
        }
        let mut wl = Workload::new_from_builder(self.wl_name.clone());

        if self.wl_agent_name.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError(
                "Workload can not be built without an agent name.",
            ));
        }
        if self.wl_runtime.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError(
                "Workload can not be built without a runtime.",
            ));
        }
        if self.wl_runtime_config.is_empty() {
            return Err(AnkaiosError::WorkloadBuilderError(
                "Workload can not be built without a runtime config.",
            ));
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
        if !self.files.is_empty() {
            wl.update_files(self.files.clone());
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
    use super::Workload;
    use crate::components::workload_mod::file::File;
    use crate::components::workload_mod::test_helpers::{
        generate_test_runtime_config, generate_test_workload_proto,
    };
    use crate::AnkaiosError;
    use std::path::Path;

    #[test]
    fn utest_workload_builder() {
        let wl = Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config_from_file(Path::new(generate_test_runtime_config().as_str()))
            .unwrap()
            .restart_policy("ALWAYS")
            .add_dependency("workload_A", "ADD_COND_SUCCEEDED")
            .add_dependency("workload_C", "ADD_COND_RUNNING")
            .add_tag("key_test", "val_test")
            .add_allow_rule("Read", vec!["desiredState.workloads.workload_A".to_owned()])
            .add_deny_rule(
                "Write",
                vec!["desiredState.workloads.workload_B".to_owned()],
            )
            .add_config("alias_test", "config_1")
            .add_file(File::from_data("mount_point", "Data"))
            .build();

        assert!(wl.is_ok());
        assert_eq!(
            wl.unwrap().to_proto(),
            generate_test_workload_proto("agent_A".to_owned(), "podman".to_owned())
        );
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
