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

//! This module contains the [`CompleteState`] struct.

use serde_yaml::Value;
use std::collections::HashMap;

use crate::ankaios_api;
use crate::components::manifest::Manifest;
use crate::components::workload_mod::Workload;
use crate::components::workload_state_mod::WorkloadStateCollection;
use crate::extensions::UnreachableOption;
use ankaios_api::ank_base;

/// The API version supported by Ankaios.
const SUPPORTED_API_VERSION: &str = "v1";

/// Struct encapsulating the complete state of the [Ankaios] system.
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
///
/// # Examples
///
/// ## Create a new `CompleteState` object:
///
/// ```rust
/// # use ankaios_sdk::CompleteState;
/// #
/// let complete_state = CompleteState::new();
/// ```
///
/// ## Get the API version of the complete state:
///
/// ```rust,no_run
/// # use ankaios_sdk::CompleteState;
/// #
/// # let complete_state = CompleteState::new();
/// #
/// let api_version = complete_state.get_api_version();
/// ```
///
/// ## Get a workload from the complete state:
///
/// ```rust,no_run
/// # use ankaios_sdk::CompleteState;
/// #
/// # let complete_state = CompleteState::new();
/// #
/// let workload = complete_state.get_workload("workload_name");
/// ```
///
/// ## Get the entire list of workloads from the complete state:
///
/// ```rust,no_run
/// # use ankaios_sdk::CompleteState;
/// #
/// # let complete_state = CompleteState::new();
/// #
/// let workloads = complete_state.get_workloads();
/// ```
///
/// ## Get the connected agents:
///
/// ```rust,no_run
/// # use ankaios_sdk::CompleteState;
/// #
/// # let complete_state = CompleteState::new();
/// #
/// let agents = complete_state.get_agents();
/// ```
///
/// ## Get the workload states:
///
/// ```rust,no_run
/// # use ankaios_sdk::CompleteState;
/// #
/// # let complete_state = CompleteState::new();
/// #
/// let workload_states = complete_state.get_workload_states();
/// ```
///
/// ## Create a `CompleteState` object from a `Manifest`:
///
/// ```rust,no_run
/// # use ankaios_sdk::{CompleteState, Manifest};
/// #
/// # let complete_state = CompleteState::new();
/// #
/// let manifest: Manifest;
/// # let manifest = Manifest::from_string("").unwrap();
/// let complete_state = CompleteState::new_from_manifest(manifest);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CompleteState {
    /// The internal proto representation of the `CompleteState`.
    complete_state: ank_base::CompleteState,
}

impl CompleteState {
    /// Creates a new `CompleteState` object.
    ///
    /// ## Returns
    ///
    /// A new [`CompleteState`] instance.
    #[must_use]
    pub fn new() -> Self {
        let mut obj = Self {
            complete_state: ank_base::CompleteState::default(),
        };
        obj.set_api_version(SUPPORTED_API_VERSION.to_owned());

        if let Some(desired_state) = obj.complete_state.desired_state.as_mut() {
            desired_state.workloads = Some(ank_base::WorkloadMap {
                workloads: HashMap::default(),
            });
        }

        obj
    }

    /// Creates a new `CompleteState` object from a [Manifest].
    ///
    /// ## Arguments
    ///
    /// * `manifest` - The [Manifest] to create the [`CompleteState`] from.
    ///
    /// ## Returns
    ///
    /// A new [`CompleteState`] instance.
    #[must_use]
    pub fn new_from_manifest(manifest: Manifest) -> Self {
        let mut obj = Self::new();
        obj.complete_state.desired_state = Some(manifest.to_desired_state());
        obj
    }

    #[doc(hidden)]
    /// Creates a new `CompleteState` object from a [ank_base::CompleteState].
    ///
    /// ## Arguments
    ///
    /// * `proto` - The [ank_base::CompleteState] to create the [`CompleteState`] from.
    ///
    /// ## Returns
    ///
    /// A new [`CompleteState`] instance.
    pub(crate) fn new_from_proto(proto: ank_base::CompleteState) -> Self {
        let mut obj = Self::new();
        obj.complete_state = proto;
        obj
    }

    #[doc(hidden)]
    /// Creates a new `CompleteState` object with configs.
    ///
    /// ## Arguments
    ///
    /// * `configs` - A [`HashMap`] containing the configurations.
    ///
    /// ## Returns
    ///
    /// A new [`CompleteState`] instance.
    pub(crate) fn new_from_configs(configs: HashMap<String, Value>) -> Self {
        let mut obj = Self::new();
        obj.set_configs(configs);
        obj
    }

    #[doc(hidden)]
    /// Creates a new `CompleteState` object from a list of workloads.
    ///
    /// ## Arguments
    ///
    /// * `workloads` - A [`Vec`] of workloads to create the [`CompleteState`] from.
    ///
    /// ## Returns
    ///
    /// A new [`CompleteState`] instance.
    pub(crate) fn new_from_workloads(workloads: Vec<Workload>) -> Self {
        let mut obj = Self::new();
        if let Some(desired_state) = obj.complete_state.desired_state.as_mut() {
            if desired_state.workloads.is_none() {
                desired_state.workloads = Some(ank_base::WorkloadMap {
                    workloads: HashMap::default(),
                });
            }
            for workload in workloads {
                if let Some(workloads_internal) = desired_state.workloads.as_mut() {
                    workloads_internal
                        .workloads
                        .insert(workload.name.clone(), workload.to_proto());
                }
            }
        }
        obj
    }

    /// Converts the `CompleteState` to a [`serde_yaml::Mapping`].
    ///
    /// ## Returns
    ///
    /// A [`serde_yaml::Mapping`] containing the `CompleteState` information.
    #[must_use]
    pub fn to_dict(&self) -> serde_yaml::Mapping {
        let mut dict = serde_yaml::Mapping::new();
        dict.insert(
            Value::String("apiVersion".to_owned()),
            Value::String(self.get_api_version()),
        );
        let mut workloads = serde_yaml::Mapping::new();
        for workload in self.get_workloads() {
            workloads.insert(
                Value::String(workload.name.clone()),
                Value::Mapping(workload.to_dict()),
            );
        }
        dict.insert(
            Value::String("workloads".to_owned()),
            Value::Mapping(workloads),
        );
        let mut configs = serde_yaml::Mapping::new();
        for (k, v) in self.get_configs() {
            configs.insert(Value::String(k), v);
        }
        dict.insert(Value::String("configs".to_owned()), Value::Mapping(configs));
        let mut agents = serde_yaml::Mapping::new();
        for (agent_name, agent) in self.get_agents() {
            let mut agent_dict = serde_yaml::Mapping::new();
            for (k, v) in agent {
                agent_dict.insert(Value::String(k), Value::String(v));
            }
            agents.insert(Value::String(agent_name), Value::Mapping(agent_dict));
        }
        dict.insert(Value::String("agents".to_owned()), Value::Mapping(agents));
        dict.insert(
            Value::String("workload_states".to_owned()),
            Value::Mapping(self.get_workload_states().as_mapping()),
        );
        dict
    }

    #[doc(hidden)]
    /// Converts the `CompleteState` to a [ank_base::CompleteState].
    ///
    /// ## Returns
    ///
    /// A [ank_base::CompleteState] containing the `CompleteState` information.
    pub(crate) fn to_proto(&self) -> ank_base::CompleteState {
        self.complete_state.clone()
    }

    /// Sets the API version of the `CompleteState`.
    ///
    /// ## Arguments
    ///
    /// * `api_version` - A [String] containing the API version.
    fn set_api_version<T: Into<String>>(&mut self, api_version: T) {
        match self.complete_state.desired_state.as_mut() {
            Some(state) => state.api_version = api_version.into(),
            None => {
                self.complete_state.desired_state = Some(ank_base::State {
                    api_version: api_version.into(),
                    workloads: None,
                    configs: None,
                });
            }
        }
    }

    /// Gets the API version of the `CompleteState`.
    ///
    /// ## Returns
    ///
    /// A [String] containing the API version.
    #[must_use]
    pub fn get_api_version(&self) -> String {
        if let Some(state) = self.complete_state.desired_state.as_ref() {
            state.api_version.clone()
        } else {
            log::error!("Error: desired_state is None");
            String::new()
        }
    }

    /// Gets a workload from the `CompleteState`.
    ///
    /// ## Arguments
    ///
    /// * `workload_name` - A [String] containing the name of the workload.
    ///
    /// ## Returns
    ///
    /// A [Workload] instance if found, otherwise `None`.
    pub fn get_workload<T: Into<String>>(&self, workload_name: T) -> Option<Workload> {
        let workload_name_str = workload_name.into();
        if let Some(desired_state) = self.complete_state.desired_state.as_ref() {
            if let Some(workloads) = desired_state.workloads.as_ref() {
                for (name, workload) in &workloads.workloads {
                    if workload_name_str == *name {
                        return Some(Workload::new_from_proto(name, workload.clone()));
                    }
                }
            }
        }
        None
    }

    /// Gets all workloads from the `CompleteState`.
    ///
    /// ## Returns
    ///
    /// A [Vec] containing all the workloads.
    #[must_use]
    pub fn get_workloads(&self) -> Vec<Workload> {
        let mut workloads_vec = Vec::new();
        if let Some(desired_state) = self.complete_state.desired_state.as_ref() {
            if let Some(workloads) = desired_state.workloads.as_ref() {
                for (workload_name, workload) in &workloads.workloads {
                    workloads_vec.push(Workload::new_from_proto(workload_name, workload.clone()));
                }
            }
        }
        workloads_vec
    }

    /// Gets the workload states from the `CompleteState`.
    ///
    /// ## Returns
    ///
    /// A [`WorkloadStateCollection`] containing the workload states.
    #[must_use]
    pub fn get_workload_states(&self) -> WorkloadStateCollection {
        if let Some(workload_states) = self.complete_state.workload_states.as_ref() {
            return WorkloadStateCollection::new_from_proto(workload_states);
        }
        WorkloadStateCollection::new()
    }

    /// Gets the connected agents from the `CompleteState`.
    ///
    /// ## Returns
    ///
    /// A [`HashMap`] containing the connected agents.
    #[must_use]
    pub fn get_agents(&self) -> HashMap<String, HashMap<String, String>> {
        let mut agents = HashMap::new();
        if let Some(agent_map) = &self.complete_state.agents {
            for (name, attributes) in &agent_map.agents {
                match &attributes.status {
                    Some(status) => {
                        let mut agent_status = HashMap::new();
                        agent_status.insert(
                            "cpu_usage".to_owned(),
                            match &status.cpu_usage {
                                Some(cpu_usage) => cpu_usage.cpu_usage.to_string(),
                                None => "N/A".to_owned(),
                            },
                        );
                        agent_status.insert(
                            "free_memory".to_owned(),
                            match &status.free_memory {
                                Some(free_memory) => free_memory.free_memory.to_string(),
                                None => "N/A".to_owned(),
                            },
                        );
                        agents.insert(name.clone(), agent_status);
                    }
                    None => {
                        agents.insert(name.clone(), HashMap::new());
                    }
                }
            }
        }
        agents
    }

    /// Sets the configurations of the `CompleteState`.
    ///
    /// ## Arguments
    ///
    /// * `configs` - A [`HashMap`] containing the configurations.
    fn set_configs(&mut self, configs: HashMap<String, Value>) {
        fn to_config_item(value: &Value) -> ank_base::ConfigItem {
            match value {
                Value::String(val) => ank_base::ConfigItem {
                    config_item_enum: Some(ank_base::ConfigItemEnum::String(val.clone())),
                },
                Value::Sequence(val) => ank_base::ConfigItem {
                    config_item_enum: Some(ank_base::ConfigItemEnum::Array(
                        ank_base::ConfigArray {
                            values: val.iter().map(to_config_item).collect(),
                        },
                    )),
                },
                Value::Mapping(val) => ank_base::ConfigItem {
                    config_item_enum: Some(ank_base::ConfigItemEnum::Object(
                        ank_base::ConfigObject {
                            fields: val
                                .iter()
                                .map(|(k, v)| {
                                    (
                                        k.as_str().unwrap_or_unreachable().to_owned(),
                                        to_config_item(v),
                                    )
                                })
                                .collect(),
                        },
                    )),
                },
                _ => ank_base::ConfigItem {
                    config_item_enum: None,
                },
            }
        }

        if let Some(desired_state) = self.complete_state.desired_state.as_mut() {
            if desired_state.configs.is_none() {
                desired_state.configs = Some(ank_base::ConfigMap {
                    configs: HashMap::default(),
                });
            }
            if let Some(state_configs) = desired_state.configs.as_mut() {
                state_configs.configs = configs
                    .iter()
                    .map(|(k, v)| (k.clone(), to_config_item(v)))
                    .collect();
                drop(configs); // Consume configs
            }
        }
    }

    /// Gets the configurations of the `CompleteState`.
    ///
    /// ## Returns
    ///
    /// A [`HashMap`] containing the configurations.
    #[must_use]
    pub fn get_configs(&self) -> HashMap<String, Value> {
        fn from_config_item(config_item: &ank_base::ConfigItem) -> Value {
            match &config_item.config_item_enum {
                Some(ank_base::ConfigItemEnum::String(val)) => Value::String(val.clone()),
                Some(ank_base::ConfigItemEnum::Array(val)) => {
                    Value::Sequence(val.values.iter().map(from_config_item).collect())
                }
                Some(ank_base::ConfigItemEnum::Object(val)) => Value::Mapping(
                    val.fields
                        .iter()
                        .map(|(k, v)| (Value::String(k.clone()), from_config_item(v)))
                        .collect(),
                ),
                None => Value::Null,
            }
        }
        if let Some(desired_state) = self.complete_state.desired_state.as_ref() {
            if let Some(configs) = desired_state.configs.as_ref() {
                return configs
                    .configs
                    .iter()
                    .map(|(k, v)| (k.clone(), from_config_item(v)))
                    .collect();
            }
        }
        HashMap::new()
    }
}

impl Default for CompleteState {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Manifest> for CompleteState {
    fn from(manifest: Manifest) -> Self {
        Self::new_from_manifest(manifest)
    }
}

impl From<ank_base::CompleteState> for CompleteState {
    fn from(proto: ank_base::CompleteState) -> Self {
        Self::new_from_proto(proto)
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
use crate::components::workload_mod::test_helpers::generate_test_workload_proto;

#[cfg(test)]
use crate::components::workload_state_mod::generate_test_workload_states_proto;

#[cfg(test)]
pub fn generate_test_configs_proto() -> ank_base::ConfigMap {
    ank_base::ConfigMap {
        configs: HashMap::from([
            (
                "config1".to_owned(),
                ank_base::ConfigItem {
                    config_item_enum: Some(ank_base::ConfigItemEnum::String("value1".to_owned())),
                },
            ),
            (
                "config2".to_owned(),
                ank_base::ConfigItem {
                    config_item_enum: Some(ank_base::ConfigItemEnum::Array(
                        ank_base::ConfigArray {
                            values: vec![
                                ank_base::ConfigItem {
                                    config_item_enum: Some(ank_base::ConfigItemEnum::String(
                                        "value2".to_owned(),
                                    )),
                                },
                                ank_base::ConfigItem {
                                    config_item_enum: Some(ank_base::ConfigItemEnum::String(
                                        "value3".to_owned(),
                                    )),
                                },
                            ],
                        },
                    )),
                },
            ),
            (
                "config3".to_owned(),
                ank_base::ConfigItem {
                    config_item_enum: Some(ank_base::ConfigItemEnum::Object(
                        ank_base::ConfigObject {
                            fields: HashMap::from([
                                (
                                    "field1".to_owned(),
                                    ank_base::ConfigItem {
                                        config_item_enum: Some(ank_base::ConfigItemEnum::String(
                                            "value4".to_owned(),
                                        )),
                                    },
                                ),
                                (
                                    "field2".to_owned(),
                                    ank_base::ConfigItem {
                                        config_item_enum: Some(ank_base::ConfigItemEnum::String(
                                            "value5".to_owned(),
                                        )),
                                    },
                                ),
                            ]),
                        },
                    )),
                },
            ),
        ]),
    }
}

#[cfg(test)]
fn generate_agents_proto() -> ank_base::AgentMap {
    ank_base::AgentMap {
        agents: HashMap::from([(
            "agent_A".to_owned(),
            ank_base::AgentAttributes {
                status: Some(ank_base::AgentStatus {
                    cpu_usage: Some(ank_base::CpuUsage { cpu_usage: 50 }),
                    free_memory: Some(ank_base::FreeMemory { free_memory: 1024 }),
                }),
                ..Default::default()
            },
        )]),
    }
}

#[cfg(test)]
pub fn generate_complete_state_proto() -> ank_base::CompleteState {
    ank_base::CompleteState {
        desired_state: Some(ank_base::State {
            api_version: SUPPORTED_API_VERSION.to_owned(),
            workloads: Some(ank_base::WorkloadMap {
                workloads: HashMap::from([(
                    "nginx_test".to_owned(),
                    generate_test_workload_proto("agent_A", "podman"),
                )]),
            }),
            configs: Some(generate_test_configs_proto()),
        }),
        workload_states: Some(generate_test_workload_states_proto()),
        agents: Some(generate_agents_proto()),
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml::Value;
    use std::collections::HashMap;

    use super::{CompleteState, SUPPORTED_API_VERSION, generate_complete_state_proto};
    use crate::components::manifest::generate_test_manifest;
    use crate::components::workload_mod::test_helpers::generate_test_workload;
    use crate::components::workload_state_mod::WorkloadInstanceName;

    #[test]
    fn utest_api_version() {
        let mut complete_state = CompleteState::default();
        assert_eq!(complete_state.get_api_version(), SUPPORTED_API_VERSION);
        complete_state.set_api_version("v0.2");
        assert_eq!(complete_state.get_api_version(), "v0.2");
    }

    #[test]
    fn utest_proto() {
        let complete_state = CompleteState::new_from_proto(generate_complete_state_proto());
        let other_complete_state = CompleteState::new_from_proto(complete_state.to_proto());
        assert_eq!(complete_state, other_complete_state);
    }

    #[test]
    fn utest_from_manifest() {
        let manifest = generate_test_manifest();
        let complete_state = CompleteState::from(manifest.clone());
        assert_eq!(complete_state.get_workloads().len(), 1);
        assert_eq!(complete_state.get_configs().len(), 3);
        assert_eq!(
            complete_state.complete_state.desired_state.unwrap(),
            manifest.to_desired_state()
        );
    }

    #[test]
    fn utest_from_configs() {
        let configs = HashMap::from([
            ("config1".to_owned(), Value::String("value1".to_owned())),
            (
                "config2".to_owned(),
                Value::Sequence(vec![
                    Value::String("value2".to_owned()),
                    Value::String("value3".to_owned()),
                ]),
            ),
        ]);
        let complete_state = CompleteState::new_from_configs(configs.clone());
        assert_eq!(complete_state.get_configs(), configs);
    }

    #[test]
    fn utest_from_workloads() {
        let workloads = vec![
            generate_test_workload("agent_A", "nginx", "podman"),
            generate_test_workload("agent_B", "apache", "docker"),
        ];
        let complete_state = CompleteState::new_from_workloads(workloads.clone());
        assert_eq!(complete_state.get_workloads().len(), workloads.len());
    }

    #[test]
    fn utest_invalid_value_config() {
        let mut complete_state = CompleteState::default();
        let mut configs = HashMap::new();
        configs.insert("config1".to_owned(), Value::Null);
        complete_state.set_configs(configs);
        assert_eq!(complete_state.get_configs().len(), 1);
        assert!(complete_state.get_configs()["config1"].is_null());
    }

    #[test]
    fn utest_to_dict() {
        let complete_state = CompleteState::from(generate_complete_state_proto());
        let complete_state_dict = complete_state.to_dict();
        assert_eq!(
            complete_state_dict
                .get(Value::String("apiVersion".to_owned()))
                .unwrap(),
            &Value::String(SUPPORTED_API_VERSION.to_owned())
        );

        let workloads = complete_state_dict
            .get(Value::String("workloads".to_owned()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(workloads.len(), 1);

        assert_eq!(
            workloads
                .get(Value::String("nginx_test".to_owned()))
                .unwrap()
                .as_mapping()
                .unwrap()
                .len(),
            9
        );

        let configs = complete_state_dict
            .get(Value::String("configs".to_owned()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(configs.len(), 3);

        let agents = complete_state_dict
            .get(Value::String("agents".to_owned()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(agents.len(), 1);

        let workload_states = complete_state_dict
            .get(Value::String("workload_states".to_owned()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(workload_states.len(), 2);
        let workload_states_agent_b = workload_states
            .get(Value::String("agent_B".to_owned()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(workload_states_agent_b.len(), 2);
    }

    #[test]
    fn utest_get_workload() {
        let complete_state = CompleteState::from(generate_complete_state_proto());
        let workload = complete_state.get_workload("nginx_test").unwrap();
        assert_eq!(workload.name, "nginx_test");
    }

    #[test]
    fn utest_get_workload_states() {
        let complete_state = CompleteState::from(generate_complete_state_proto());
        let workload_states = complete_state.get_workload_states();
        let workload_instance_name =
            WorkloadInstanceName::new("agent_A".to_owned(), "nginx".to_owned(), "1234".to_owned());
        assert!(
            workload_states
                .get_for_instance_name(&workload_instance_name)
                .is_some()
        );
    }
}
