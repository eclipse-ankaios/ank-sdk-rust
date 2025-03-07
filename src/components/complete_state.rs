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

use std::fmt;
use std::collections::HashMap;
use serde_yaml::Value;

use crate::ankaios_api;
use ankaios_api::ank_base;
use crate::components::workload_mod::Workload;
use crate::components::workload_state_mod::WorkloadStateCollection;
use crate::components::manifest::Manifest;
use crate::AnkaiosError;

/// The API version supported by Ankaios.
const SUPPORTED_API_VERSION: &str = "v0.1";

/// Struct encapsulates the complete state of the [Ankaios] system.
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
/// 
/// # Examples
/// 
/// ## Create a new `CompleteState` object:
/// 
/// ```rust
/// let complete_state = CompleteState::new();
/// ```
/// 
/// ## Get the API version of the complete state:
/// 
/// ```rust
/// let api_version = complete_state.get_api_version();
/// ```
/// ## Add a workload to the complete state:
/// 
/// ```rust
/// let workload = /* */;
/// complete_state.add_workload(workload);
/// ```
/// 
/// ## Get a workload from the complete state:
/// 
/// ```rust
/// let workload = complete_state.get_workload("workload_name");
/// ```
/// 
/// ## Get the entire list of workloads from the complete state:
/// 
/// ```rust
/// let workloads = complete_state.get_workloads();
/// ```
/// 
/// ## Get the connected agents:
/// 
/// ```rust
/// let agents = complete_state.get_agents();
/// ```
/// 
/// ## Get the workload states:
/// 
/// ```rust
/// let workload_states = complete_state.get_workload_states();
/// ```
/// 
/// ## Create a `CompleteState` object from a `Manifest`:
/// 
/// ```rust
/// let manifest = /* */;
/// let complete_state = CompleteState::try_from(&manifest).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct CompleteState{
    /// The internal proto representation of the `CompleteState`.
    complete_state: ank_base::CompleteState,
    /// The list of workloads in the `CompleteState`.
    workloads: Vec<Workload>,
    /// The workload state collection of the `CompleteState`.
    workload_state_collection: WorkloadStateCollection,
    /// The configurations of the `CompleteState`.
    configs: HashMap<String, Value>
}

impl CompleteState {
    /// Creates a new `CompleteState` object.
    /// 
    /// ## Returns
    /// 
    /// A new [`CompleteState`] instance.
    #[must_use]
    pub fn new() -> Self {
        let mut obj = Self{
            complete_state: ank_base::CompleteState::default(),
            workloads: Vec::new(),
            workload_state_collection: WorkloadStateCollection::new(),
            configs: HashMap::new(),
        };
        obj.set_api_version(SUPPORTED_API_VERSION.to_owned());
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
    /// 
    /// ## Panics
    /// 
    /// Panics if the [Manifest] is not valid.
    #[must_use]
    pub fn new_from_manifest(manifest: &Manifest) -> Self {
        let dict_state = manifest.to_dict();
        let mut obj = Self::new();
        obj.set_api_version(dict_state.get("apiVersion").unwrap().as_str().unwrap());
        if let Some(workloads) = dict_state.get("workloads") {
            for (workload_name, workload) in workloads.as_mapping().unwrap() {
                let workload_map = workload.as_mapping().unwrap();
                let workload_object = Workload::new_from_dict(workload_name.as_str().unwrap(), &workload_map.clone());
                obj.add_workload(workload_object.unwrap());
            }
        }
        if let Some(configs) = dict_state.get("configs") {
            let mut config_map = HashMap::new();
            for (k, v) in configs.as_mapping().unwrap() {
                config_map.insert(k.as_str().unwrap().to_owned(), v.clone());
            }
            obj.set_configs(config_map);
        }
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
    pub(crate) fn new_from_proto(proto: &ank_base::CompleteState) -> Self {
        fn from_config_item(config_item: &ank_base::ConfigItem) -> Value {
            #[allow(non_snake_case)] // False positive: None is an optional, not a variable, so it's ok to not be snake_case.
            match &config_item.config_item {
                Some(ank_base::config_item::ConfigItem::String(val)) => Value::String(
                    val.clone()
                ),
                Some(ank_base::config_item::ConfigItem::Array(val)) => Value::Sequence(
                    val.values
                    .iter()
                    .map(from_config_item)
                    .collect()
                ),
                Some(ank_base::config_item::ConfigItem::Object(val)) => Value::Mapping(
                    val.fields
                    .iter()
                    .map(|(k, v)| (
                        Value::String(k.clone()), from_config_item(v))
                    )
                    .collect()
                ),
                None => Value::Null,
            }
        }

        let mut obj = Self::new();
        obj.complete_state = proto.clone();

        if proto.desired_state.is_some() {
            if let Some(configs) = proto.desired_state.as_ref().unwrap().configs.as_ref() {
                obj.configs = configs.configs.iter().map(|(k, v)| (k.clone(), from_config_item(v))).collect();
            }

            if let Some(workloads) = proto.desired_state.as_ref().unwrap().workloads.as_ref() {
                for (workload_name, workload) in &workloads.workloads {
                    obj.workloads.push(Workload::new_from_proto(workload_name, workload.clone()));
                }
            }
        }

        if let Some(workload_states) = proto.workload_states.as_ref() {
            obj.workload_state_collection = WorkloadStateCollection::new_from_proto(workload_states);
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
        dict.insert(Value::String("apiVersion".to_owned()), Value::String(self.get_api_version()));
        let mut workloads = serde_yaml::Mapping::new();
        for workload in self.get_workloads() {
            workloads.insert(Value::String(workload.name.clone()), Value::Mapping(workload.to_dict()));
        }
        dict.insert(Value::String("workloads".to_owned()), Value::Mapping(workloads));
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
        dict.insert(Value::String("workload_states".to_owned()), Value::Mapping(self.workload_state_collection.get_as_dict()));
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
        #[allow(non_snake_case)] // False positive: None is an optional, not a variable, so it's ok to not be snake_case.
        match self.complete_state.desired_state.as_mut() {
            Some(state) => state.api_version = api_version.into(),
            None => {
                self.complete_state.desired_state = Some(ank_base::State{
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

    /// Adds a workload to the `CompleteState`.
    /// 
    /// ## Arguments
    /// 
    /// * `workload` - The [Workload] to add.
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_workload(&mut self, workload: Workload) {
        self.workloads.push(workload.clone());

        if let Some(desired_state) = self.complete_state.desired_state.as_mut() {
            if desired_state.workloads.is_none() {
                desired_state.workloads = Some(ank_base::WorkloadMap{
                    workloads: HashMap::default(),
                });
            }
            // desired_state.workloads.as_mut().unwrap().workloads.insert(workload.name.clone(), workload.to_proto());
            if let Some(workloads) = desired_state.workloads.as_mut() {
                workloads.workloads.insert(workload.name.clone(), workload.to_proto());
            }
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
        self.workloads
            .iter()
            .find(|workload| workload.name == workload_name_str)
            .cloned()
    }

    /// Gets all workloads from the `CompleteState`.
    /// 
    /// ## Returns
    /// 
    /// A [Vec] containing all the workloads.
    #[must_use]
    pub fn get_workloads(&self) -> Vec<Workload> {
        self.workloads.clone()
    }

    /// Gets the workload states from the `CompleteState`.
    /// 
    /// ## Returns
    /// 
    /// A [`WorkloadStateCollection`] containing the workload states.
    #[must_use]
    pub fn get_workload_states(&self) -> &WorkloadStateCollection {
        &self.workload_state_collection
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
                let mut agent = HashMap::new();
                if let Some(cpu_usage) = &attributes.cpu_usage {
                    agent.insert("cpu_usage".to_owned(), cpu_usage.cpu_usage.to_string());
                }
                if let Some(free_memory) = &attributes.free_memory {
                    agent.insert("free_memory".to_owned(), free_memory.free_memory.to_string());
                }
                agents.insert(name.clone(), agent);
            }
        }
        agents
    }

    /// Sets the configurations of the `CompleteState`.
    /// 
    /// ## Arguments
    /// 
    /// * `configs` - A [`HashMap`] containing the configurations.
    pub fn set_configs(&mut self, configs: HashMap<String, Value>) {
        fn to_config_item(value: &Value) -> ank_base::ConfigItem {
            match value {
                Value::String(val) => ank_base::ConfigItem {
                    config_item: Some(ank_base::config_item::ConfigItem::String(val.clone())),
                },
                Value::Sequence(val) => ank_base::ConfigItem {
                    config_item: Some(ank_base::config_item::ConfigItem::Array(ank_base::ConfigArray {
                        values: val.iter().map(to_config_item).collect(),
                    })),
                },
                Value::Mapping(val) => ank_base::ConfigItem {
                    config_item: Some(ank_base::config_item::ConfigItem::Object(ank_base::ConfigObject {
                        fields: val.iter().map(|(k, v)| (k.as_str().unwrap().to_owned(), to_config_item(v))).collect(),
                    })),
                },
                _ => ank_base::ConfigItem {
                    config_item: None,
                },
            }
        }

        self.configs = configs;
        if let Some(desired_state) = self.complete_state.desired_state.as_mut() {
            if desired_state.configs.is_none() {
                desired_state.configs = Some(ank_base::ConfigMap {
                    configs: HashMap::default(),
                });
            }
            if let Some(state_configs) = desired_state.configs.as_mut() {
                state_configs.configs = self.configs.iter().map(|(k, v)| (k.clone(), to_config_item(v))).collect();
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
        self.configs.clone()
    }
}

impl Default for CompleteState {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CompleteState {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_proto())
    }
}

impl TryFrom<&Manifest> for CompleteState {
    type Error = AnkaiosError;

    fn try_from(manifest: &Manifest) -> Result<Self, Self::Error> {
        Ok(Self::new_from_manifest(manifest))
    }
}

impl TryFrom<ank_base::CompleteState> for CompleteState {
    type Error = AnkaiosError;

    fn try_from(proto: ank_base::CompleteState) -> Result<Self, Self::Error> {
        Ok(Self::new_from_proto(&proto))
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
fn generate_test_configs_proto() -> ank_base::ConfigMap {
    ank_base::ConfigMap { configs: HashMap::from([
        ("config1".to_owned(), ank_base::ConfigItem {
            config_item: Some(ank_base::config_item::ConfigItem::String("value1".to_owned())),
        }),
        ("config2".to_owned(), ank_base::ConfigItem {
            config_item: Some(ank_base::config_item::ConfigItem::Array(ank_base::ConfigArray {
                values: vec![
                    ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value2".to_owned())),
                    },
                    ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value3".to_owned())),
                    },
                ],
            })),
        }),
        ("config3".to_owned(), ank_base::ConfigItem {
            config_item: Some(ank_base::config_item::ConfigItem::Object(ank_base::ConfigObject {
                fields: HashMap::from([
                    ("field1".to_owned(), ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value4".to_owned())),
                    }),
                    ("field2".to_owned(), ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value5".to_owned())),
                    }),
                ]),
            })),
        }),
    ])}
}

#[cfg(test)]
fn generate_agents_proto() -> ank_base::AgentMap {
    ank_base::AgentMap { agents: HashMap::from([
        ("agent_A".to_owned(), ank_base::AgentAttributes {
            cpu_usage: Some(ank_base::CpuUsage {
                cpu_usage: 50,
            }),
            free_memory: Some(ank_base::FreeMemory {
                free_memory: 1024,
            }),
        }),
    ])}
}

#[cfg(test)]
fn generate_complete_state_proto() -> ank_base::CompleteState {
    ank_base::CompleteState {
        desired_state: Some(ank_base::State {
            api_version: SUPPORTED_API_VERSION.to_owned(),
            workloads: Some(ank_base::WorkloadMap {
                workloads: HashMap::from([
                    ("nginx_test".to_owned(), generate_test_workload_proto("agent_A", "podman")),
                ]),
            }),
            configs: Some(generate_test_configs_proto()),
        }),
        workload_states: Some(generate_test_workload_states_proto()),
        agents: Some(generate_agents_proto()),
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use serde_yaml::Value;
    use std::collections::HashMap;

    use super::{generate_complete_state_proto, CompleteState, SUPPORTED_API_VERSION};
    use crate::components::manifest::generate_test_manifest;
    use crate::components::workload_mod::test_helpers::generate_test_workload;
    use crate::components::workload_state_mod::WorkloadInstanceName;

    #[test]
    fn test_doc_examples() {
        // Create a new `CompleteState` object
        let mut complete_state = CompleteState::new();

        // Get the API version of the complete state
        let _api_version = complete_state.get_api_version();

        // Add a workload to the complete state
        let workload = generate_test_workload("agent_test", "workload_test", "podman");
        complete_state.add_workload(workload);

        // Get a workload from the complete state
        let _workload = complete_state.get_workload("workload_test");

        // Get the entire list of workloads from the complete state
        let _workloads = complete_state.get_workloads();

        // Get the connected agents
        let _agents = complete_state.get_agents();

        // Get the workload states
        let _workload_states = complete_state.get_workload_states();

        // Create a `CompleteState` object from a `Manifest`
        let manifest = generate_test_manifest();
        let _complete_state = CompleteState::try_from(&manifest).unwrap();
    }

    #[test]
    fn utest_api_version() {
        let mut complete_state = CompleteState::default();
        assert_eq!(complete_state.get_api_version(), SUPPORTED_API_VERSION);
        complete_state.set_api_version("v0.2");
        assert_eq!(complete_state.get_api_version(), "v0.2");
    }

    #[test]
    fn utest_proto() {
        let complete_state = CompleteState::new_from_proto(&generate_complete_state_proto());
        let other_complete_state = CompleteState::new_from_proto(&complete_state.to_proto());
        assert_eq!(complete_state.to_string(), other_complete_state.to_string());
    }

    #[test]
    fn utest_from_manifest() {
        let manifest = generate_test_manifest();
        let complete_state = CompleteState::try_from(&manifest).unwrap();
        assert_eq!(complete_state.get_workloads().len(), 1);
        assert_eq!(complete_state.get_configs().len(), 3);
        assert_eq!(manifest.to_dict().get("workloads").unwrap(), complete_state.to_dict().get("workloads").unwrap());
        assert_eq!(manifest.to_dict().get("configs").unwrap(), complete_state.to_dict().get("configs").unwrap());
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
        let complete_state = CompleteState::try_from(generate_complete_state_proto()).unwrap();

        // Populate the expected mapping
        let mut expected_mapping = serde_yaml::Mapping::new();
        expected_mapping.insert(Value::String("apiVersion".to_owned()), Value::String(SUPPORTED_API_VERSION.to_owned()));
        let mut workloads = serde_yaml::Mapping::new();
        workloads.insert(Value::String("nginx_test".to_owned()), Value::Mapping(complete_state.get_workloads()[0].to_dict()));
        // TODO

        assert_eq!(complete_state.to_dict().type_id(), expected_mapping.type_id());
        //assert_eq!(complete_state.to_dict(), expected_mapping);
    }

    #[test]
    fn utest_get_workload() {
        let complete_state = CompleteState::try_from(generate_complete_state_proto()).unwrap();
        let workload = complete_state.get_workload("nginx_test").unwrap();
        assert_eq!(workload.name, "nginx_test");
    }

    #[test]
    fn utest_get_workload_states() {
        let complete_state = CompleteState::try_from(generate_complete_state_proto()).unwrap();
        let workload_states = complete_state.get_workload_states();
        let workload_instance_name = WorkloadInstanceName::new("agent_A".to_owned(), "nginx".to_owned(), "1234".to_owned());
        assert!(workload_states.get_for_instance_name(&workload_instance_name).is_some());
    }
}