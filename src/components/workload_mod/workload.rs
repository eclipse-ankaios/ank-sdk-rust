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
use crate::File;
use crate::WorkloadBuilder;
use crate::ankaios_api;
use ankaios_api::ank_base;
use serde_yaml::Value;
use std::{borrow::ToOwned, collections::HashMap, convert::Into, fmt, path::Path, vec};

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

/// The prefix for the workloads in the desired state.
pub const WORKLOADS_PREFIX: &str = "desiredState.workloads";
/// The field name for the agent name.
const FIELD_AGENT_NAME: &str = "agent";
/// The field name for the runtime.
const FIELD_RUNTIME: &str = "runtime";
/// The field name for the runtime config.
const FIELD_RUNTIME_CONFIG: &str = "runtimeConfig";
/// The field name for the restart policy.
const FIELD_RESTART_POLICY: &str = "restartPolicy";
/// The field name for the dependencies.
const FIELD_DEPENDENCIES: &str = "dependencies";
/// The field name for the tags.
const FIELD_TAGS: &str = "tags";
/// The field name for the control interface access.
const FIELD_CONTROL_INTERFACE_ACCESS: &str = "controlInterfaceAccess";
/// The field name for the allow rules.
const SUBFIELD_ACCESS_ALLOW_RULES: &str = "allowRules";
/// The field name for the deny rules.
const SUBFIELD_ACCESS_DENY_RULES: &str = "denyRules";
/// The field name for the operation of a rule.
const SUBFIELD_ACCESS_OPERATION: &str = "operation";
/// The field name for the filter mask of a rule.
const SUBFIELD_ACCESS_FILTER_MASK: &str = "filterMask";
/// The field name for the access type of a rule.
const SUBFIELD_ACCESS_TYPE: &str = "type";
/// The field name for the type of a rule.
const SUBFIELD_ACCESS_STATE_RULE: &str = "StateRule";
/// The field name for the configs.
const FIELD_CONFIGS: &str = "configs";
/// The field name for files.
const FIELD_FILES: &str = "files";

/// Represents a workload with various attributes and methods to update them.
///
/// The `Workload` struct is used to store the [Ankaios] workload, allowing for
/// easy manipulation of the workload's fields and conversion to and from
/// different formats.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
///
/// # Examples
///
/// ## Create a workload using the [`WorkloadBuilder`]:
///
/// ```rust
/// use ankaios_sdk::Workload;
///
/// let workload = Workload::builder()
///     .workload_name("example_workload")
///     .agent_name("agent_A")
///     .runtime("podman")
///     .restart_policy("NEVER")
///     .runtime_config("image: docker.io/library/nginx\n
///                      commandOptions: [\"-p\", \"8080:80\"]")
///     .add_dependency("other_workload", "ADD_COND_RUNNING")
///     .add_tag("key1", "value1")
///     .add_tag("key2", "value2")
///     .build().unwrap();
/// ```
///
/// ## Update fields of the workload:
///
/// ```rust
/// # use ankaios_sdk::Workload;
/// #
/// # let mut workload = Workload::builder()
/// #   .workload_name("example_workload")
/// #   .agent_name("agent_A")
/// #   .runtime("podman")
/// #   .runtime_config("image: docker.io/library/nginx\n
/// #                    commandOptions: [\"-p\", \"8080:80\"]")
/// #   .build().unwrap();
/// workload.update_agent_name("agent_B");
/// ```
///
/// ## Update dependencies:
///
/// ```rust
/// # use ankaios_sdk::Workload;
/// #
/// # let mut workload = Workload::builder()
/// #   .workload_name("example_workload")
/// #   .agent_name("agent_A")
/// #   .runtime("podman")
/// #   .runtime_config("image: docker.io/library/nginx\n
/// #                    commandOptions: [\"-p\", \"8080:80\"]")
/// #   .build().unwrap();
/// let mut deps = workload.get_dependencies();
/// if let Some(value) = deps.get_mut("other_workload") {
///    *value = "ADD_COND_SUCCEEDED".to_owned();
/// }
/// workload.update_dependencies(deps).unwrap();
/// ```
///
/// ## Update tags:
///
/// ```rust
/// # use ankaios_sdk::Workload;
/// #
/// # let mut workload = Workload::builder()
/// #   .workload_name("example_workload")
/// #   .agent_name("agent_A")
/// #   .runtime("podman")
/// #   .runtime_config("image: docker.io/library/nginx\n
/// #                    commandOptions: [\"-p\", \"8080:80\"]")
/// #   .build().unwrap();
/// let mut tags = workload.get_tags();
/// tags.insert("key3".to_owned(), "value3".to_owned());
/// workload.update_tags(&tags);
/// ```
///
/// ## Print the updated workload:
///
/// ```rust
/// # use ankaios_sdk::Workload;
/// #
/// # let mut workload = Workload::builder()
/// #   .workload_name("example_workload")
/// #   .agent_name("agent_A")
/// #   .runtime("podman")
/// #   .runtime_config("image: docker.io/library/nginx\n
/// #                    commandOptions: [\"-p\", \"8080:80\"]")
/// #   .build().unwrap();
/// println!("{}", workload);
/// ```
#[derive(Debug, Clone)]
pub struct Workload {
    #[doc(hidden)]
    /// The underlying workload data from the proto file.
    pub(crate) workload: ank_base::Workload,
    #[doc(hidden)]
    /// The main mask of the workload.
    pub(crate) main_mask: String,
    /// A vector of strings representing the masks for the workload.
    pub masks: Vec<String>,
    /// The name of the workload.
    pub name: String,
}

impl Workload {
    #[doc(hidden)]
    /// Creates a new `Workload` instance from the builder.
    /// Must be called only from within the builder.
    ///
    /// ## Arguments
    ///
    /// - `name` - A [String] that represents the name of the workload.
    ///
    /// ## Returns
    ///
    /// A new [Workload] instance.
    pub(crate) fn new_from_builder<T: Into<String>>(name: T) -> Self {
        let name_str = name.into();
        Self {
            workload: ank_base::Workload::default(),
            main_mask: format!("{WORKLOADS_PREFIX}.{name_str}"),
            masks: vec![format!("{WORKLOADS_PREFIX}.{name_str}")],
            name: name_str,
        }
    }

    #[doc(hidden)]
    /// Creates a new `Workload` instance from a proto.
    ///
    /// ## Arguments
    ///
    /// - `name` - A [String] that represents the name of the workload;
    /// - `proto` - A proto instance of [`ank_base::Workload`].
    ///
    /// ## Returns
    ///
    /// A new [Workload] instance.
    pub(crate) fn new_from_proto<T: Into<String>>(name: T, proto: ank_base::Workload) -> Self {
        let name_str = name.into();
        Self {
            workload: proto,
            main_mask: format!("{WORKLOADS_PREFIX}.{name_str}"),
            masks: vec![],
            name: name_str,
        }
    }

    #[doc(hidden)]
    /// Creates a new `Workload` instance from a Map.
    ///
    /// ## Arguments
    ///
    /// - `name` - A [String] that represents the name of the workload;
    /// - `dict_workload` - An instance of [`serde_yaml::Mapping`] that represents the workload.
    ///
    /// ## Returns
    ///
    /// A new [Workload] instance.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadBuilderError`](AnkaiosError::WorkloadBuilderError) - If the builder fails.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn new_from_dict<T: Into<String>>(
        name: T,
        dict_workload: &serde_yaml::Mapping,
    ) -> Result<Self, AnkaiosError> {
        let mut wl_builder = Self::builder();
        wl_builder = wl_builder.workload_name(name);

        if let Some(agent) = dict_workload.get(FIELD_AGENT_NAME) {
            let agent_str = agent.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                FIELD_AGENT_NAME.to_owned(),
                "Should be a string".to_owned(),
            ))?;
            wl_builder = wl_builder.agent_name(agent_str);
        }
        if let Some(runtime) = dict_workload.get(FIELD_RUNTIME) {
            let runtime_str = runtime.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                FIELD_RUNTIME.to_owned(),
                "Should be a string".to_owned(),
            ))?;
            wl_builder = wl_builder.runtime(runtime_str);
        }
        if let Some(runtime_config) = dict_workload.get(FIELD_RUNTIME_CONFIG) {
            let runtime_config_str =
                runtime_config
                    .as_str()
                    .ok_or(AnkaiosError::WorkloadFieldError(
                        FIELD_RUNTIME_CONFIG.to_owned(),
                        "Should be a string".to_owned(),
                    ))?;
            wl_builder = wl_builder.runtime_config(runtime_config_str);
        }
        if let Some(restart_policy) = dict_workload.get(FIELD_RESTART_POLICY) {
            let restart_policy_str =
                restart_policy
                    .as_str()
                    .ok_or(AnkaiosError::WorkloadFieldError(
                        FIELD_RESTART_POLICY.to_owned(),
                        "Should be a string".to_owned(),
                    ))?;
            wl_builder = wl_builder.restart_policy(restart_policy_str);
        }
        if let Some(dependencies) = dict_workload.get(FIELD_DEPENDENCIES) {
            let dependencies_map =
                dependencies
                    .as_mapping()
                    .ok_or(AnkaiosError::WorkloadFieldError(
                        FIELD_DEPENDENCIES.to_owned(),
                        "Should be a mapping".to_owned(),
                    ))?;
            for (key, value) in dependencies_map {
                let key_str = key.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                    FIELD_DEPENDENCIES.to_owned(),
                    "Key should be a string".to_owned(),
                ))?;
                let value_str = value.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                    FIELD_DEPENDENCIES.to_owned(),
                    "Value should be a string".to_owned(),
                ))?;
                wl_builder = wl_builder.add_dependency(key_str, value_str);
            }
        }
        if let Some(tags) = dict_workload.get(FIELD_TAGS) {
            let tags_map = tags.as_mapping().ok_or(AnkaiosError::WorkloadFieldError(
                FIELD_TAGS.to_owned(),
                "Should be a mapping".to_owned(),
            ))?;

            for (key, value) in tags_map {
                let key_str = key.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                    FIELD_TAGS.to_owned(),
                    "Tag key should be a string".to_owned(),
                ))?;
                let value_str = value.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                    FIELD_TAGS.to_owned(),
                    "Tag value should be a string".to_owned(),
                ))?;
                wl_builder = wl_builder.add_tag(key_str, value_str);
            }
        }
        if let Some(control_interface_access) = dict_workload.get(FIELD_CONTROL_INTERFACE_ACCESS) {
            let control_interface_access_map =
                control_interface_access
                    .as_mapping()
                    .ok_or(AnkaiosError::WorkloadFieldError(
                        FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                        "Should be a mapping".to_owned(),
                    ))?;
            if let Some(allow_rules) = control_interface_access_map.get(SUBFIELD_ACCESS_ALLOW_RULES)
            {
                let allow_rules_seq =
                    allow_rules
                        .as_sequence()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Allow rules should be a sequence".to_owned(),
                        ))?;
                for rule in allow_rules_seq {
                    let rule_map = rule.as_mapping().ok_or(AnkaiosError::WorkloadFieldError(
                        FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                        "Allow rule should be a mapping".to_owned(),
                    ))?;
                    let operation = rule_map
                        .get(SUBFIELD_ACCESS_OPERATION)
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Allow rule should have an operation".to_owned(),
                        ))?
                        .as_str()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Allow rule operation should be a string".to_owned(),
                        ))?;
                    let filter_masks = rule_map
                        .get(SUBFIELD_ACCESS_FILTER_MASK)
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Allow rule should have a filter mask".to_owned(),
                        ))?
                        .as_sequence()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Allow rule filter mask should be a sequence".to_owned(),
                        ))?
                        .iter()
                        .map(|x| match x.as_str() {
                            Some(s) => Ok(s.to_owned()),
                            None => Err(AnkaiosError::WorkloadFieldError(
                                FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                                "Allow rule filter mask value should be a string".to_owned(),
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    wl_builder = wl_builder.add_allow_rule(operation, filter_masks);
                }
            }
            if let Some(deny_rules) = control_interface_access_map.get(SUBFIELD_ACCESS_DENY_RULES) {
                let deny_rules_seq =
                    deny_rules
                        .as_sequence()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Deny rules should be a sequence".to_owned(),
                        ))?;
                for rule in deny_rules_seq {
                    let rule_map = rule.as_mapping().ok_or(AnkaiosError::WorkloadFieldError(
                        FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                        "Deny rule should be a mapping".to_owned(),
                    ))?;
                    let operation = rule_map
                        .get(SUBFIELD_ACCESS_OPERATION)
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Deny rule should have an operation".to_owned(),
                        ))?
                        .as_str()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Deny rule operation should be a string".to_owned(),
                        ))?;
                    let filter_masks = rule_map
                        .get(SUBFIELD_ACCESS_FILTER_MASK)
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Deny rule should have a filter mask".to_owned(),
                        ))?
                        .as_sequence()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                            "Deny rule filter mask should be a sequence".to_owned(),
                        ))?
                        .iter()
                        .map(|x| match x.as_str() {
                            Some(s) => Ok(s.to_owned()),
                            None => Err(AnkaiosError::WorkloadFieldError(
                                FIELD_CONTROL_INTERFACE_ACCESS.to_owned(),
                                "Deny rule filter mask value should be a string".to_owned(),
                            )),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    wl_builder = wl_builder.add_deny_rule(operation, filter_masks);
                }
            }
        }
        if let Some(configs) = dict_workload.get(FIELD_CONFIGS) {
            let configs_map = configs
                .as_mapping()
                .ok_or(AnkaiosError::WorkloadFieldError(
                    FIELD_CONFIGS.to_owned(),
                    "Should be a mapping".to_owned(),
                ))?;
            for (alias, config_name) in configs_map {
                let alias_str = alias.as_str().ok_or(AnkaiosError::WorkloadFieldError(
                    FIELD_CONFIGS.to_owned(),
                    "Alias should be a string".to_owned(),
                ))?;
                let config_name_str =
                    config_name
                        .as_str()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_CONFIGS.to_owned(),
                            "Name should be a string".to_owned(),
                        ))?;
                wl_builder = wl_builder.add_config(alias_str, config_name_str);
            }
        }
        if let Some(files) = dict_workload.get(FIELD_FILES) {
            let files_vec = files.as_sequence().ok_or(AnkaiosError::WorkloadFieldError(
                FIELD_FILES.to_owned(),
                "should be a sequence".to_owned(),
            ))?;

            for file_value in files_vec {
                let file_mapping =
                    file_value
                        .as_mapping()
                        .ok_or(AnkaiosError::WorkloadFieldError(
                            FIELD_FILES.to_owned(),
                            "file should be a mapping".to_owned(),
                        ))?;
                let file = File::from_dict(file_mapping)?;
                wl_builder = wl_builder.add_file(file);
            }
        }

        wl_builder.build()
    }

    /// Converts the `Workload` instance to a proto message.
    ///
    /// ## Returns
    ///
    /// A [`ank_base::Workload`] instance.
    #[must_use]
    #[inline]
    pub fn to_proto(self) -> ank_base::Workload {
        self.workload
    }

    /// Converts the `Workload` instance to a [`serde_yaml::Mapping`].
    ///
    /// ## Returns
    ///
    /// A [`serde_yaml::Mapping`] instance.
    #[allow(clippy::too_many_lines)]
    pub fn to_dict(&self) -> serde_yaml::Mapping {
        let mut dict = serde_yaml::Mapping::new();
        if let Some(agent) = self.workload.agent.clone() {
            dict.insert(
                Value::String(FIELD_AGENT_NAME.to_owned()),
                Value::String(agent),
            );
        }
        if let Some(runtime) = self.workload.runtime.clone() {
            dict.insert(
                Value::String(FIELD_RUNTIME.to_owned()),
                Value::String(runtime),
            );
        }
        if let Some(runtime_config) = self.workload.runtime_config.clone() {
            dict.insert(
                Value::String(FIELD_RUNTIME_CONFIG.to_owned()),
                Value::String(runtime_config),
            );
        }
        if let Some(restart_policy) = self.workload.restart_policy
            && let Some(ank_restart_policy) = ank_base::RestartPolicy::from_i32(restart_policy)
        {
            dict.insert(
                Value::String(FIELD_RESTART_POLICY.to_owned()),
                Value::String(ank_restart_policy.as_str_name().to_owned()),
            );
        }
        if let Some(dependencies) = self.workload.dependencies.clone() {
            let mut deps = serde_yaml::Mapping::new();
            dict.insert(
                Value::String(FIELD_DEPENDENCIES.to_owned()),
                Value::Mapping(serde_yaml::Mapping::new()),
            );
            for (key, value) in &dependencies.dependencies {
                if let Some(cond) = ank_base::AddCondition::from_i32(*value) {
                    deps.insert(
                        Value::String(key.clone()),
                        Value::String(cond.as_str_name().to_owned()),
                    );
                }
            }
            dict.insert(
                Value::String(FIELD_DEPENDENCIES.to_owned()),
                Value::Mapping(deps),
            );
        }
        if let Some(wl_tags) = self.workload.tags.clone() {
            let mut tags = serde_yaml::Mapping::new();
            for (key, value) in &wl_tags.tags {
                tags.insert(Value::String(key.clone()), Value::String(value.clone()));
            }
            dict.insert(Value::String(FIELD_TAGS.to_owned()), Value::Mapping(tags));
        }
        if let Some(ci_access) = self.workload.control_interface_access.clone() {
            let mut control_interface_access = serde_yaml::Mapping::new();

            let mut allow_rules = serde_yaml::Sequence::new();
            for rule in &ci_access.allow_rules {
                let mut rule_dict = serde_yaml::Mapping::new();
                rule_dict.insert(
                    Value::String(SUBFIELD_ACCESS_TYPE.to_owned()),
                    Value::String(SUBFIELD_ACCESS_STATE_RULE.to_owned()),
                );
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum:
                        Some(ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(inner_rule)),
                } = rule
                {
                    match Self::access_right_rule_to_str(inner_rule) {
                        Ok(rule_ok) => {
                            rule_dict.insert(
                                Value::String(SUBFIELD_ACCESS_OPERATION.to_owned()),
                                Value::String(rule_ok.0),
                            );
                            rule_dict.insert(
                                Value::String(SUBFIELD_ACCESS_FILTER_MASK.to_owned()),
                                Value::Sequence(rule_ok.1.into_iter().map(Value::String).collect()),
                            );
                        }
                        Err(_) => continue,
                    }
                }
                allow_rules.push(Value::Mapping(rule_dict));
            }
            if !allow_rules.is_empty() {
                control_interface_access.insert(
                    Value::String(SUBFIELD_ACCESS_ALLOW_RULES.to_owned()),
                    Value::Sequence(allow_rules),
                );
            }

            let mut deny_rules = serde_yaml::Sequence::new();
            for rule in &ci_access.deny_rules {
                let mut rule_dict = serde_yaml::Mapping::new();
                rule_dict.insert(
                    Value::String(SUBFIELD_ACCESS_TYPE.to_owned()),
                    Value::String(SUBFIELD_ACCESS_STATE_RULE.to_owned()),
                );
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum:
                        Some(ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(inner_rule)),
                } = rule
                {
                    match Self::access_right_rule_to_str(inner_rule) {
                        Ok(rule_ok) => {
                            rule_dict.insert(
                                Value::String(SUBFIELD_ACCESS_OPERATION.to_owned()),
                                Value::String(rule_ok.0),
                            );
                            rule_dict.insert(
                                Value::String(SUBFIELD_ACCESS_FILTER_MASK.to_owned()),
                                Value::Sequence(rule_ok.1.into_iter().map(Value::String).collect()),
                            );
                        }
                        Err(_) => continue,
                    }
                }
                deny_rules.push(Value::Mapping(rule_dict));
            }
            if !deny_rules.is_empty() {
                control_interface_access.insert(
                    Value::String(SUBFIELD_ACCESS_DENY_RULES.to_owned()),
                    Value::Sequence(deny_rules),
                );
            }

            dict.insert(
                Value::String(FIELD_CONTROL_INTERFACE_ACCESS.to_owned()),
                Value::Mapping(control_interface_access),
            );
        }
        if let Some(wl_configs) = self.workload.configs.clone() {
            let mut configs = serde_yaml::Mapping::new();
            for (alias, name) in &wl_configs.configs {
                configs.insert(Value::String(alias.clone()), Value::String(name.clone()));
            }
            dict.insert(
                Value::String(FIELD_CONFIGS.to_owned()),
                Value::Mapping(configs),
            );
        }
        if let Some(wl_files) = self.workload.files.clone() {
            let mut files = serde_yaml::Sequence::new();
            for file in &wl_files.files {
                let file_mapping = File::from_proto(file.clone()).to_dict();

                files.push(Value::Mapping(file_mapping));
            }
            dict.insert(
                Value::String(FIELD_FILES.to_owned()),
                Value::Sequence(files),
            );
        }

        dict
    }

    /// Creates a new [`WorkloadBuilder`] instance.
    ///
    /// ## Returns
    ///
    /// A new [`WorkloadBuilder`] instance.
    #[inline]
    pub fn builder() -> WorkloadBuilder {
        WorkloadBuilder::new()
    }

    /// Updates the name of the workload.
    ///
    /// ## Arguments
    ///
    /// - `new_name` - A [String] that represents the new name of the workload.
    pub fn update_workload_name<T: Into<String>>(&mut self, new_name: T) {
        self.name = new_name.into();
        self.main_mask = format!("{WORKLOADS_PREFIX}.{}", self.name);
        self.masks = vec![format!("{WORKLOADS_PREFIX}.{}", self.name)];
    }

    /// Updates the agent name of the workload.
    ///
    /// ## Arguments
    ///
    /// - `agent_name` - A [String] that represents the new [agent name](ank_base::Workload).
    pub fn update_agent_name<T: Into<String>>(&mut self, agent_name: T) {
        self.workload.agent = Some(agent_name.into());
        self.add_mask(format!("{}.{FIELD_AGENT_NAME}", self.main_mask));
    }

    /// Updates the runtime of the workload.
    ///
    /// ## Arguments
    ///
    /// - `runtime` - A [String] that represents the new [runtime](ank_base::Workload).
    pub fn update_runtime<T: Into<String>>(&mut self, runtime: T) {
        self.workload.runtime = Some(runtime.into());
        self.add_mask(format!("{}.{FIELD_RUNTIME}", self.main_mask));
    }

    /// Updates the runtime config of the workload.
    ///
    /// ## Arguments
    ///
    /// - `runtime_config` - A [String] that represents the new [runtime config](ank_base::Workload).
    pub fn update_runtime_config<T: Into<String>>(&mut self, runtime_config: T) {
        self.workload.runtime_config = Some(runtime_config.into());
        self.add_mask(format!("{}.{FIELD_RUNTIME_CONFIG}", self.main_mask));
    }

    /// Updates the runtime config of the workload using a file.
    ///
    /// ## Arguments
    ///
    /// - `file_path` - A [Path] towards the [runtime config](ank_base::Workload) file.
    ///
    /// ## Errors
    ///
    /// An [`AnkaiosError`]::[`IoError`](AnkaiosError::IoError) if the file cannot be read.
    pub fn update_runtime_config_from_file(
        &mut self,
        file_path: &Path,
    ) -> Result<(), AnkaiosError> {
        let runtime_config = read_file_to_string(file_path)?;
        self.update_runtime_config(runtime_config);
        Ok(())
    }

    /// Updates the restart policy of the workload.
    /// Allowed values are "`NEVER`", "`ON_FAILURE`" and "`ALWAYS`".
    ///
    /// ## Arguments
    ///
    /// - `restart_policy` - A [String] that represents the new [restart policy](ank_base::Workload).
    ///
    /// ## Errors
    ///
    /// An [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) if the value is not a valid restart policy.
    pub fn update_restart_policy<T: Into<String>>(
        &mut self,
        restart_policy: T,
    ) -> Result<(), AnkaiosError> {
        let restart_policy_str = restart_policy.into();
        self.workload.restart_policy =
            match ank_base::RestartPolicy::from_str_name(&restart_policy_str.clone()) {
                Some(policy) => Some(policy as i32),
                _ => {
                    return Err(AnkaiosError::WorkloadFieldError(
                        FIELD_RESTART_POLICY.to_owned(),
                        restart_policy_str,
                    ));
                }
            };
        self.add_mask(format!("{}.{FIELD_RESTART_POLICY}", self.main_mask));
        Ok(())
    }

    /// Getter for the dependencies of the workload.
    ///
    /// ## Returns
    ///
    /// A [`HashMap`] containing the [dependencies](ank_base::Workload) of the workload.
    #[must_use]
    pub fn get_dependencies(&self) -> HashMap<String, String> {
        let mut dependencies = HashMap::new();
        if let Some(deps) = &self.workload.dependencies {
            for (key, value) in &deps.dependencies {
                if let Some(add_cond) = ank_base::AddCondition::from_i32(*value) {
                    dependencies.insert(key.clone(), add_cond.as_str_name().to_owned());
                }
            }
        }
        dependencies
    }

    /// Updates the dependencies of the workload.
    /// Allowed values for the conditions are "`ADD_COND_RUNNING`", "`ADD_COND_SUCCEEDED`" and "`ADD_COND_FAILED`".
    ///
    /// ## Arguments
    ///
    /// - `dependencies` - A [`HashMap`] containing the [dependencies](ank_base::Workload) of the workload.
    ///
    /// ## Errors
    ///
    /// An [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) if the values are not valid dependency conditions.
    pub fn update_dependencies<T: Into<String>>(
        &mut self,
        dependencies: HashMap<T, T>,
    ) -> Result<(), AnkaiosError> {
        self.workload.dependencies = Some(ank_base::Dependencies::default());
        for (workload_name, condition) in dependencies {
            let cond = condition.into();
            let add_condition = match ank_base::AddCondition::from_str_name(&cond.clone()) {
                Some(add_cond) => add_cond as i32,
                _ => {
                    return Err(AnkaiosError::WorkloadFieldError(
                        "dependency condition".to_owned(),
                        cond,
                    ));
                }
            };
            if let Some(deps) = self.workload.dependencies.as_mut() {
                deps.dependencies
                    .insert(workload_name.into(), add_condition);
            }
        }
        self.add_mask(format!("{}.{FIELD_DEPENDENCIES}", self.main_mask));
        Ok(())
    }

    /// Adds a tag to the workload.
    ///
    /// ## Arguments
    ///
    /// - `key` - A [String] containing the [tag](ank_base::Workload) key;
    /// - `value` - A [String] containing the [tag](ank_base::Workload) value.
    pub fn add_tag<T: Into<String>>(&mut self, key: T, value: T) {
        if self.workload.tags.is_none() {
            self.workload.tags = Some(ank_base::Tags::default());
        }
        let key_str = key.into();
        if let Some(tags) = self.workload.tags.as_mut() {
            tags.tags.insert(key_str.clone(), value.into());
        }

        if !self
            .masks
            .contains(&format!("{}.{FIELD_TAGS}", self.main_mask))
        {
            self.add_mask(format!("{}.{FIELD_TAGS}.{key_str}", self.main_mask));
        }
    }

    /// Getter for the tags of the workload.
    ///
    /// ## Returns
    ///
    /// A [`HashMap`] containing the [tags](ank_base::Workload) of the workload.
    #[must_use]
    pub fn get_tags(&self) -> HashMap<String, String> {
        self.workload
            .tags
            .as_ref()
            .map_or_else(HashMap::new, |tags_list| {
                tags_list
                    .tags
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
    }

    /// Updates the tags of the workload.
    ///
    /// ## Arguments
    ///
    /// - `tags` - A [`HashMap`] containing the [tags](ank_base::Workload) of the workload.
    pub fn update_tags(&mut self, tags: &HashMap<String, String>) {
        self.workload.tags = Some({
            let mut ank_tags = ank_base::Tags::default();
            for (key, value) in tags {
                ank_tags.tags.insert(key.clone(), value.to_owned());
            }
            ank_tags
        });
        self.masks
            .retain(|mask| !mask.starts_with(&format!("{}.{FIELD_TAGS}", self.main_mask)));
        self.add_mask(format!("{}.{FIELD_TAGS}", self.main_mask));
    }

    /// Given an operation and a list of filter masks, generates an [`AccessRightsRule`](ank_base::AccessRightsRule).
    ///
    /// ## Arguments
    ///
    /// - `operation` - A [String] containing the operation;
    /// - `filter_masks` - A [Vec] containing the filter masks.
    ///
    /// ## Returns
    ///
    /// An [`AccessRightsRule`](ank_base::AccessRightsRule) instance.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) - If the operation is not a valid operation.
    fn generate_access_right_rule(
        operation: &str,
        filter_masks: Vec<String>,
    ) -> Result<ank_base::AccessRightsRule, AnkaiosError> {
        Ok(ank_base::AccessRightsRule {
            access_rights_rule_enum: Some(
                ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(
                    ank_base::StateRule {
                        operation: match operation {
                            "Nothing" => ank_base::ReadWriteEnum::RwNothing as i32,
                            "Write" => ank_base::ReadWriteEnum::RwWrite as i32,
                            "Read" => ank_base::ReadWriteEnum::RwRead as i32,
                            "ReadWrite" => ank_base::ReadWriteEnum::RwReadWrite as i32,
                            _ => {
                                return Err(AnkaiosError::WorkloadFieldError(
                                    SUBFIELD_ACCESS_OPERATION.to_owned(),
                                    operation.to_owned(),
                                ));
                            }
                        },
                        filter_masks,
                    },
                ),
            ),
        })
    }

    /// Converts an [`AccessRightsRule`](ank_base::AccessRightsRule) to a tuple of [Strings](String).
    ///     
    /// ## Arguments
    ///
    /// - `rule` - An [`AccessRightsRule`](ank_base::AccessRightsRule) instance.
    ///
    /// ## Returns
    ///
    /// A tuple containing the operation and the filter masks.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) - If the operation is not a valid operation.
    fn access_right_rule_to_str(
        rule: &ank_base::StateRule,
    ) -> Result<(String, Vec<String>), AnkaiosError> {
        Ok((
            match ank_base::ReadWriteEnum::from_i32(rule.operation) {
                Some(op) => match op.as_str_name() {
                    "RW_NOTHING" => "Nothing".to_owned(),
                    "RW_WRITE" => "Write".to_owned(),
                    "RW_READ" => "Read".to_owned(),
                    "RW_READ_WRITE" => "ReadWrite".to_owned(),
                    _ => {
                        return Err(AnkaiosError::WorkloadFieldError(
                            SUBFIELD_ACCESS_OPERATION.to_owned(),
                            rule.operation.to_string(),
                        ));
                    }
                },
                _ => {
                    return Err(AnkaiosError::WorkloadFieldError(
                        SUBFIELD_ACCESS_OPERATION.to_owned(),
                        rule.operation.to_string(),
                    ));
                }
            },
            rule.filter_masks.clone(),
        ))
    }

    /// Getter for the [allow rules](ank_base::Workload) of the workload.
    ///
    /// ## Returns
    ///
    /// A [Vec] containing the allow rules of the workload.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) - If the operation is not a valid operation.
    pub fn get_allow_rules(&self) -> Result<Vec<(String, Vec<String>)>, AnkaiosError> {
        let mut rules = vec![];
        if let Some(access) = &self.workload.control_interface_access {
            for rule in &access.allow_rules {
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum:
                        Some(ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(inner_rule)),
                } = rule
                {
                    rules.push(Self::access_right_rule_to_str(inner_rule)?);
                }
            }
        }
        Ok(rules)
    }

    /// Updates the [allow rules](ank_base::Workload) of the workload.
    ///
    /// ## Arguments
    ///
    /// - `rules` - A [Vec] containing the allow rules of the workload.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) - If the operation is not a valid operation.
    pub fn update_allow_rules<T: Into<String>>(
        &mut self,
        rules: Vec<(T, Vec<T>)>,
    ) -> Result<(), AnkaiosError> {
        if self.workload.control_interface_access.is_none() {
            self.workload.control_interface_access =
                Some(ank_base::ControlInterfaceAccess::default());
        }
        if let Some(access) = &mut self.workload.control_interface_access {
            access.allow_rules = vec![];
        }
        for rule in rules {
            let access_rule = Self::generate_access_right_rule(
                rule.0.into().as_str(),
                rule.1.into_iter().map(Into::into).collect(),
            )?;
            if let Some(access) = &mut self.workload.control_interface_access {
                access.allow_rules.push(access_rule);
            }
        }
        self.add_mask(format!(
            "{}.{FIELD_CONTROL_INTERFACE_ACCESS}.{SUBFIELD_ACCESS_ALLOW_RULES}",
            self.main_mask
        ));
        Ok(())
    }

    /// Getter for the [deny rules](ank_base::Workload) of the workload.
    ///
    /// ## Returns
    ///
    /// A [Vec] containing the deny rules of the workload.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) - If the operation is not a valid operation.
    pub fn get_deny_rules(&self) -> Result<Vec<(String, Vec<String>)>, AnkaiosError> {
        let mut rules = vec![];
        if let Some(access) = &self.workload.control_interface_access {
            for rule in &access.deny_rules {
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum:
                        Some(ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(inner_rule)),
                } = rule
                {
                    rules.push(Self::access_right_rule_to_str(inner_rule)?);
                }
            }
        }
        Ok(rules)
    }

    /// Updates the [deny rules](ank_base::Workload) of the workload.
    ///
    /// ## Arguments
    ///
    /// - `rules` - A [Vec] containing the deny rules of the workload.
    ///
    /// ## Errors
    ///
    /// - [`AnkaiosError`]::[`WorkloadFieldError`](AnkaiosError::WorkloadFieldError) - If the operation is not a valid operation.
    pub fn update_deny_rules<T: Into<String>>(
        &mut self,
        rules: Vec<(T, Vec<T>)>,
    ) -> Result<(), AnkaiosError> {
        if self.workload.control_interface_access.is_none() {
            self.workload.control_interface_access =
                Some(ank_base::ControlInterfaceAccess::default());
        }
        if let Some(access) = self.workload.control_interface_access.as_mut() {
            access.deny_rules = vec![];
        }
        for rule in rules {
            let access_rule = Self::generate_access_right_rule(
                rule.0.into().as_str(),
                rule.1.into_iter().map(Into::into).collect(),
            )?;
            if let Some(access) = &mut self.workload.control_interface_access {
                access.deny_rules.push(access_rule);
            }
        }
        self.add_mask(format!(
            "{}.{FIELD_CONTROL_INTERFACE_ACCESS}.{SUBFIELD_ACCESS_DENY_RULES}",
            self.main_mask
        ));
        Ok(())
    }

    /// Adds a [config alias](ank_base::Workload) to the workload.
    ///
    /// ## Arguments
    ///
    /// - `alias` - A [String] containing the alias of the config;
    /// - `name` - A [String] containing the name of the config it refers to.
    pub fn add_config<T: Into<String>>(&mut self, alias: T, name: T) {
        let alias_str = alias.into();
        match self.workload.configs {
            Some(ref mut configs_map) => {
                configs_map.configs.insert(alias_str.clone(), name.into());
            }
            None => {
                self.workload.configs = Some(ank_base::ConfigMappings {
                    configs: [(alias_str.clone(), name.into())].iter().cloned().collect(),
                });
            }
        }
        self.add_mask(format!("{}.{FIELD_CONFIGS}.{alias_str}", self.main_mask));
    }

    /// Getter for the [configs](ank_base::Workload) of the workload.
    ///
    /// ## Returns
    ///
    /// A [`HashMap`] containing the configs of the workload.
    #[must_use]
    pub fn get_configs(&self) -> HashMap<String, String> {
        let mut configs = HashMap::new();
        if let Some(configs_map) = &self.workload.configs {
            for (alias, name) in &configs_map.configs {
                configs.insert(alias.clone(), name.clone());
            }
        }
        configs
    }

    /// Updates the [configs](ank_base::Workload) of the workload.
    ///
    /// ## Arguments
    ///
    /// - `configs` - A [`HashMap`] containing the configs of the workload.
    pub fn update_configs(&mut self, configs: HashMap<String, String>) {
        self.workload.configs = Some(ank_base::ConfigMappings {
            configs: configs.into_iter().collect(),
        });
        self.add_mask(format!("{}.{FIELD_CONFIGS}", self.main_mask));
    }

    /// Adds a file to the workload.
    ///
    /// ## Arguments
    ///
    /// - `file` - A [File] object representing the file to add.
    pub fn add_file(&mut self, file: File) {
        if self.workload.files.is_none() {
            self.workload.files = Some(ank_base::Files::default());
            self.add_mask(format!("{}.{FIELD_FILES}", self.main_mask));
        }

        if let Some(files) = self.workload.files.as_mut() {
            files.files.push(file.into_proto());
        }
    }

    /// Retrieves the files associated with the workload as File objects.
    ///
    /// ## Returns
    ///
    /// A [Vec] of [File] objects representing the files in the workload.
    #[must_use]
    pub fn get_files(&self) -> Vec<File> {
        if let Some(files) = &self.workload.files {
            files
                .files
                .clone()
                .into_iter()
                .map(File::from_proto)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Updates the files associated with the workload using File objects.
    ///
    /// This method replaces all existing files with the provided File objects.
    ///
    /// ## Arguments
    ///
    /// - `files` - A vector of [File] objects to set as the workload's files.
    pub fn update_files(&mut self, files: Vec<File>) {
        if files.is_empty() {
            self.workload.files = None;
        } else {
            self.workload.files = Some(ank_base::Files {
                files: files
                    .into_iter()
                    .map(super::file::File::into_proto)
                    .collect(),
            });
            self.add_mask(format!("{}.{FIELD_FILES}", self.main_mask));
        }
    }

    /// Adds a mask to the workload.
    ///
    /// ## Arguments
    ///
    /// - `mask` - A [String] containing the mask to be added.
    fn add_mask(&mut self, mask: String) {
        if !self.masks.contains(&mask) && !self.masks.contains(&self.main_mask) {
            let configs_mask = format!("{}.{}", self.main_mask.as_str(), FIELD_CONFIGS);
            if mask == configs_mask {
                let masks_to_remove: Vec<String> = self
                    .masks
                    .iter()
                    .filter(|mask_| mask_.starts_with(&configs_mask))
                    .cloned()
                    .collect();
                for mask_ in masks_to_remove {
                    if let Some(pos) = self.masks.iter().position(|m| m == &mask_) {
                        self.masks.remove(pos);
                    }
                }
                self.masks.push(configs_mask);
            } else if mask.starts_with(&configs_mask) && self.masks.contains(&configs_mask) {
            } else {
                self.masks.push(mask);
            }
        }
    }
}

impl fmt::Display for Workload {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Workload {}: {:?}", self.name, self.clone().to_proto())
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
        generate_test_runtime_config, generate_test_workload, generate_test_workload_proto,
    };
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn utest_workload() {
        let wl_test =
            generate_test_workload("agent_A".to_owned(), "Test".to_owned(), "podman".to_owned());
        let wl_proto = generate_test_workload_proto("agent_A".to_owned(), "podman".to_owned());
        assert_eq!(wl_test.name, "Test");
        assert_eq!(wl_test.main_mask, "desiredState.workloads.Test");
        assert_eq!(
            wl_test.masks,
            vec!["desiredState.workloads.Test".to_owned()]
        );
        assert_eq!(wl_test.workload, wl_proto);
    }

    #[test]
    fn utest_workload_proto() {
        let workload_proto =
            generate_test_workload_proto("agent_A".to_owned(), "podman".to_owned());
        let wl = Workload::new_from_proto("Test", workload_proto.clone());
        let new_proto = wl.to_proto();
        assert_eq!(workload_proto, new_proto);
    }

    #[test]
    fn utest_workload_dict() {
        let workload = generate_test_workload("agent_A", "nginx", "podman");
        let workload_dict = workload.to_dict();
        let workload_new = Workload::new_from_dict("nginx", &workload_dict);
        assert!(workload_new.is_ok());
        assert_eq!(workload.to_proto(), workload_new.unwrap().to_proto());
    }

    #[test]
    fn utest_update_fields() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        assert_eq!(wl.masks, vec!["desiredState.workloads.Test".to_owned()]);

        wl.update_workload_name("TestNew");
        assert_eq!(wl.name, "TestNew");

        wl.update_agent_name("agent_B");
        assert_eq!(wl.workload.agent, Some("agent_B".to_owned()));

        wl.update_runtime("podman-kube");
        assert_eq!(wl.workload.runtime, Some("podman-kube".to_owned()));

        wl.update_runtime_config("config_test");
        assert_eq!(wl.workload.runtime_config, Some("config_test".to_owned()));

        assert!(wl.update_restart_policy("NEVER").is_ok());
        assert_eq!(wl.workload.restart_policy, Some(0));

        assert!(wl.update_restart_policy("Dance").is_err());

        let tags = HashMap::from([("key_test".to_owned(), "val_test".to_owned())]);
        wl.update_tags(&tags);
        assert_eq!(wl.get_tags(), tags);

        let allow_rules = vec![(
            "Read".to_owned(),
            vec!["desiredState.workloads.workload_A".to_owned()],
        )];
        assert!(wl.update_allow_rules(allow_rules.clone()).is_ok());
        assert_eq!(wl.get_allow_rules().unwrap(), allow_rules);

        let deny_rules = vec![(
            "Write".to_owned(),
            vec!["desiredState.workloads.workload_B".to_owned()],
        )];
        assert!(wl.update_deny_rules(deny_rules.clone()).is_ok());
        assert_eq!(wl.get_deny_rules().unwrap(), deny_rules);
    }

    #[test]
    fn utest_dependencies() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        let mut deps = wl.get_dependencies();
        assert_eq!(deps.len(), 2);

        deps.remove("workload_A");
        assert!(wl.update_dependencies(deps).is_ok());
        assert_eq!(wl.get_dependencies().len(), 1);

        assert!(
            wl.update_dependencies(HashMap::from([("workload_A", "Dance")]))
                .is_err()
        );
    }

    #[test]
    fn utest_tags() {
        let mut wl = Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config("config")
            .build()
            .unwrap();
        wl.add_tag("key_test_1", "val_test_1");
        let mut tags = wl.get_tags();
        assert_eq!(tags.len(), 1);

        wl.add_tag("key_test_2", "val_test_2");
        tags.insert("key_test_2".to_owned(), "val_test_2".to_owned());
        assert_eq!(wl.get_tags().len(), 2);
        assert_eq!(wl.get_tags(), tags);

        if let Some(key) = tags.keys().next().cloned() {
            tags.remove(&key);
        }
        wl.update_tags(&tags);
        assert_eq!(wl.get_tags().len(), 1);
    }

    #[test]
    fn utest_rules() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        let mut allow_rules = wl.get_allow_rules().unwrap();
        assert_eq!(allow_rules.len(), 1);

        allow_rules.push((
            "Write".to_owned(),
            vec!["desiredState.workloads.workload_B".to_owned()],
        ));
        assert!(wl.update_allow_rules(allow_rules).is_ok());
        assert_eq!(wl.get_allow_rules().unwrap().len(), 2);

        assert!(
            wl.update_allow_rules(vec![(
                "Dance".to_owned(),
                vec!["desiredState.workloads.workload_A".to_owned()]
            )])
            .is_err()
        );

        let mut deny_rules = wl.get_deny_rules().unwrap();
        assert_eq!(deny_rules.len(), 1);

        deny_rules.push((
            "Read".to_owned(),
            vec!["desiredState.workloads.workload_A".to_owned()],
        ));
        assert!(wl.update_deny_rules(deny_rules).is_ok());
        assert_eq!(wl.get_deny_rules().unwrap().len(), 2);

        assert!(
            wl.update_deny_rules(vec![(
                "Dance".to_owned(),
                vec!["desiredState.workloads.workload_A".to_owned()]
            )])
            .is_err()
        );
    }

    #[test]
    fn utest_configs() {
        let mut wl = Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config("config")
            .build()
            .unwrap();
        wl.masks = Vec::default();
        wl.add_config("alias_test_1", "config_test_1");
        let mut configs = wl.get_configs();
        assert_eq!(configs.len(), 1);
        assert_eq!(
            wl.masks,
            vec!["desiredState.workloads.Test.configs.alias_test_1".to_owned()]
        );

        wl.add_config("alias_test_2", "config_test_2");
        configs = wl.get_configs();
        assert_eq!(configs.len(), 2);
        assert_eq!(
            wl.masks,
            vec![
                "desiredState.workloads.Test.configs.alias_test_1".to_owned(),
                "desiredState.workloads.Test.configs.alias_test_2".to_owned()
            ]
        );

        configs.insert("alias_test_3".to_owned(), "config_test_3".to_owned());
        wl.update_configs(configs.clone());
        assert_eq!(wl.get_configs().len(), 3);
        assert_eq!(configs.len(), 3);
        assert_eq!(
            wl.masks,
            vec!["desiredState.workloads.Test.configs".to_owned()]
        );

        wl.add_config("alias_test_4", "config_test_2");
        configs = wl.get_configs();
        assert_eq!(configs.len(), 4);
        assert_eq!(
            wl.masks,
            vec!["desiredState.workloads.Test.configs".to_owned()]
        );
    }

    #[test]
    fn utest_files() {
        let mut wl = Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config("config")
            .build()
            .unwrap();

        let config_file = File::from_data("/etc/app/config.yaml", "debug: true");
        let icon_file =
            File::from_binary_data("/usr/share/app/icon.png", "iVBORw0KGgoAAAANSUhEUgA...");

        wl.add_file(config_file);
        wl.add_file(icon_file);

        let files = wl.get_files();
        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .any(|f| f.mount_point == "/etc/app/config.yaml")
        );
        assert!(
            files
                .iter()
                .any(|f| f.mount_point == "/usr/share/app/icon.png")
        );

        // Test updating file objects
        let new_files = vec![
            File::from_data("/etc/new_config.yaml", "production: true"),
            File::from_binary_data("/usr/share/binary_data", "AAABAAEAEBAAAAEAIABoBAAAFgAAA..."),
        ];

        wl.update_files(new_files);
        let updated_files = wl.get_files();
        assert_eq!(updated_files.len(), 2);
        assert!(
            updated_files
                .iter()
                .any(|f| f.mount_point == "/etc/new_config.yaml")
        );
        assert!(
            updated_files
                .iter()
                .any(|f| f.mount_point == "/usr/share/binary_data")
        );
    }

    macro_rules! generate_test_for_mask_generation {
        ($test_name:ident, $method_name:ident, $expected_value:expr, $($args:expr),*) => {
            #[test]
            fn $test_name() {
                let mut obj = Workload {
                    workload: generate_test_workload_proto("Agent_A".to_owned(), "podman".to_owned()),
                    main_mask: format!("desiredState.workloads.Test"),
                    masks: vec![],
                    name: "Test".to_owned(),
                };
                // Call function and assert the mask has been added
                let _ = obj.$method_name($($args),*);
                assert_eq!(obj.masks.len(), 1);
                assert_eq!(obj.masks, $expected_value);

                // Adding again should not add another identical mask
                let _ = obj.$method_name($($args),*);
                assert_eq!(obj.masks.len(), 1);
            }
        };
    }

    generate_test_for_mask_generation!(
        utest_update_workload_name,
        update_workload_name,
        vec![String::from("desiredState.workloads.TestNew")],
        "TestNew"
    );
    generate_test_for_mask_generation!(
        utest_update_agent_name,
        update_agent_name,
        vec![String::from("desiredState.workloads.Test.agent")],
        "agent_B"
    );
    generate_test_for_mask_generation!(
        utest_update_runtime,
        update_runtime,
        vec![String::from("desiredState.workloads.Test.runtime")],
        "podman"
    );
    generate_test_for_mask_generation!(
        utest_update_restart_policy,
        update_restart_policy,
        vec![String::from("desiredState.workloads.Test.restartPolicy")],
        "NEVER"
    );
    generate_test_for_mask_generation!(
        utest_update_runtime_config,
        update_runtime_config,
        vec![String::from("desiredState.workloads.Test.runtimeConfig")],
        "config"
    );
    generate_test_for_mask_generation!(
        utest_update_runtime_config_from_file,
        update_runtime_config_from_file,
        vec![String::from("desiredState.workloads.Test.runtimeConfig")],
        Path::new("")
    );
    generate_test_for_mask_generation!(
        utest_update_dependencies,
        update_dependencies,
        vec![String::from("desiredState.workloads.Test.dependencies")],
        HashMap::from([("workload_A", "ADD_COND_RUNNING")])
    );
    generate_test_for_mask_generation!(
        utest_add_tag,
        add_tag,
        vec![String::from("desiredState.workloads.Test.tags.key_test")],
        "key_test",
        "val_test"
    );
    generate_test_for_mask_generation!(
        utest_update_tags,
        update_tags,
        vec![String::from("desiredState.workloads.Test.tags")],
        &HashMap::from([("key_test".to_owned(), "val_test".to_owned())])
    );
    generate_test_for_mask_generation!(
        utest_update_allow_rule,
        update_allow_rules,
        vec![String::from(
            "desiredState.workloads.Test.controlInterfaceAccess.allowRules"
        )],
        vec![(
            "Read".to_owned(),
            vec!["desiredState.workloads.workload_A".to_owned()]
        )]
    );
    generate_test_for_mask_generation!(
        utest_update_deny_rule,
        update_deny_rules,
        vec![String::from(
            "desiredState.workloads.Test.controlInterfaceAccess.denyRules"
        )],
        vec![(
            "Write".to_owned(),
            vec!["desiredState.workloads.workload_B".to_owned()]
        )]
    );
    generate_test_for_mask_generation!(
        utest_add_config,
        add_config,
        vec![String::from(
            "desiredState.workloads.Test.configs.alias_test"
        )],
        "alias_test",
        "config_test"
    );

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
        assert!(
            Workload::builder()
                .agent_name("agent_A")
                .runtime("podman")
                .runtime_config("config")
                .build()
                .is_err()
        );

        // No agent
        assert!(
            Workload::builder()
                .workload_name("Test")
                .runtime("podman")
                .runtime_config("config")
                .build()
                .is_err()
        );

        // No runtime
        assert!(
            Workload::builder()
                .workload_name("Test")
                .agent_name("agent_A")
                .runtime_config("config")
                .build()
                .is_err()
        );

        // No runtime config
        assert!(
            Workload::builder()
                .workload_name("Test")
                .agent_name("agent_A")
                .runtime("podman")
                .build()
                .is_err()
        );
    }

    #[test]
    fn utest_display() {
        let wl = Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config("config")
            .build()
            .unwrap();
        assert_eq!(
            format!("{wl}"),
            "Workload Test: Workload { agent: Some(\"agent_A\"), restart_policy: None, dependencies: None, tags: None, runtime: Some(\"podman\"), runtime_config: Some(\"config\"), control_interface_access: None, configs: None, files: None }"
        );
    }
}
