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

use std::fmt;
use std::collections::HashMap;

pub use api::ank_base;
use api::ank_base::CompleteState as AnkaiosCompleteState;
use crate::components::workload_mod::Workload;
use crate::components::workload_state_mod::WorkloadStateCollection;
use crate::components::manifest::Manifest;
use crate::AnkaiosError;

const SUPPORTED_API_VERSION: &str = "v0.1";

#[derive(Debug, Clone)]
pub struct CompleteState{
    complete_state: AnkaiosCompleteState,
    workloads: Vec<Workload>,
    workload_state_collection: WorkloadStateCollection,
    configs: HashMap<String, serde_yaml::Value>
}

impl CompleteState {
    pub fn new() -> Self {
        let mut obj = Self{
            complete_state: AnkaiosCompleteState::default(),
            workloads: Vec::new(),
            workload_state_collection: WorkloadStateCollection::new(),
            configs: HashMap::new(),
        };
        obj.set_api_version(SUPPORTED_API_VERSION.to_string());
        obj
    }

    pub fn new_from_manifest(manifest: &Manifest) -> Self {
        let dict_state = manifest.to_dict();
        let mut obj = Self::new();
        obj.set_api_version(dict_state.get("apiVersion").unwrap().as_str().unwrap());
        if let Some(workloads) = dict_state.get("workloads") {
            for (workload_name, workload) in workloads.as_mapping().unwrap().iter() {
                let workload = workload.as_mapping().unwrap();
                let workload = Workload::new_from_dict(workload_name.as_str().unwrap(), workload.clone());
                obj.add_workload(workload.unwrap());
            }
        }
        if let Some(configs) = dict_state.get("configs") {
            let mut config_map = HashMap::new();
            for (k, v) in configs.as_mapping().unwrap().iter() {
                config_map.insert(k.as_str().unwrap().to_string(), v.clone());
            }
            obj.set_configs(config_map);
        }
        obj
    }

    pub fn new_from_proto(proto: ank_base::CompleteState) -> Self {
        let mut obj = Self::new();
        obj.complete_state = proto.clone();

        fn from_config_item(config_item: &ank_base::ConfigItem) -> serde_yaml::Value {
            match &config_item.config_item {
                Some(ank_base::config_item::ConfigItem::String(val)) => serde_yaml::Value::String(
                    val.clone()
                ),
                Some(ank_base::config_item::ConfigItem::Array(val)) => serde_yaml::Value::Sequence(
                    val.values
                    .iter()
                    .map(from_config_item)
                    .collect()
                ),
                Some(ank_base::config_item::ConfigItem::Object(val)) => serde_yaml::Value::Mapping(
                    val.fields
                    .iter()
                    .map(|(k, v)| (
                        serde_yaml::Value::String(k.clone()), from_config_item(v))
                    )
                    .collect()
                ),
                None => serde_yaml::Value::Null,
            }
        }
        if proto.desired_state.is_some() {
            if let Some(configs) = proto.desired_state.as_ref().unwrap().configs.as_ref() {
                obj.configs = configs.configs.iter().map(|(k, v)| (k.clone(), from_config_item(v))).collect();
            }

            if let Some(workloads) = proto.desired_state.as_ref().unwrap().workloads.as_ref() {
                for (workload_name, workload) in workloads.workloads.iter() {
                    obj.workloads.push(Workload::new_from_proto(workload_name, workload.clone()));
                }
            }
        }

        if let Some(workload_states) = proto.workload_states.as_ref() {
            obj.workload_state_collection = WorkloadStateCollection::new_from_proto(workload_states);
        }
        obj
    }

    pub fn to_dict(&self) -> serde_yaml::Mapping {
        let mut dict = serde_yaml::Mapping::new();
        dict.insert(serde_yaml::Value::String("apiVersion".to_string()), serde_yaml::Value::String(self.get_api_version()));
        let mut workloads = serde_yaml::Mapping::new();
        for workload in self.get_workloads() {
            workloads.insert(serde_yaml::Value::String(workload.name.clone()), serde_yaml::Value::Mapping(workload.to_dict()));
        }
        dict.insert(serde_yaml::Value::String("workloads".to_string()), serde_yaml::Value::Mapping(workloads));
        let mut configs = serde_yaml::Mapping::new();
        for (k, v) in self.get_configs() {
            configs.insert(serde_yaml::Value::String(k), v);
        }
        dict.insert(serde_yaml::Value::String("configs".to_string()), serde_yaml::Value::Mapping(configs));
        let mut agents = serde_yaml::Mapping::new();
        for (agent_name, agent) in self.get_agents() {
            let mut agent_dict = serde_yaml::Mapping::new();
            for (k, v) in agent {
                agent_dict.insert(serde_yaml::Value::String(k), serde_yaml::Value::String(v));
            }
            agents.insert(serde_yaml::Value::String(agent_name), serde_yaml::Value::Mapping(agent_dict));
        }
        dict.insert(serde_yaml::Value::String("agents".to_string()), serde_yaml::Value::Mapping(agents));
        dict.insert(serde_yaml::Value::String("workload_states".to_string()), serde_yaml::Value::Mapping(self.workload_state_collection.get_as_dict()));
        dict
    }

    pub fn to_proto(&self) -> ank_base::CompleteState {
        self.complete_state.clone()
    }

    fn set_api_version<T: Into<String>>(&mut self, api_version: T) {
        if self.complete_state.desired_state.is_none() {
            self.complete_state.desired_state = Some(ank_base::State{
                api_version: api_version.into(),
                workloads: None,
                configs: None,
            });
        } 
        else {
            self.complete_state.desired_state.as_mut().unwrap().api_version = api_version.into();
        }
    }

    pub fn get_api_version(&self) -> String {
        self.complete_state.desired_state.as_ref().unwrap().api_version.clone()
    }

    pub fn add_workload(&mut self, workload: Workload) {
        self.workloads.push(workload.clone());
        if self.complete_state.desired_state.as_mut().unwrap().workloads.is_none() {
            self.complete_state.desired_state.as_mut().unwrap().workloads = Some(ank_base::WorkloadMap{
                workloads: Default::default(),
            });
        }
        self.complete_state.desired_state.as_mut().unwrap().workloads.as_mut().unwrap().workloads.insert(workload.name.clone(), workload.to_proto());
    }

    pub fn get_workload<T: Into<String>>(&self, workload_name: T) -> Option<Workload> {
        let workload_name = workload_name.into();
        self.workloads
            .iter()
            .find(|workload| workload.name == workload_name)
            .cloned()
    }

    pub fn get_workloads(&self) -> Vec<Workload> {
        self.workloads.clone()
    }

    pub fn get_workload_states(&self) -> &WorkloadStateCollection {
        &self.workload_state_collection
    }

    pub fn get_agents(&self) -> HashMap<String, HashMap<String, String>> {
        let mut agents = HashMap::new();
        if let Some(agent_map) = &self.complete_state.agents {
            for (name, attributes) in agent_map.agents.iter() {
                let mut agent = HashMap::new();
                if let Some(cpu_usage) = &attributes.cpu_usage {
                    agent.insert("cpu_usage".to_string(), cpu_usage.cpu_usage.to_string());
                }
                if let Some(free_memory) = &attributes.free_memory {
                    agent.insert("free_memory".to_string(), free_memory.free_memory.to_string());
                }
                agents.insert(name.clone(), agent);
            }
        }
        agents
    }

    pub fn set_configs(&mut self, configs: HashMap<String, serde_yaml::Value>) {
        self.configs = configs;

        fn to_config_item(value: &serde_yaml::Value) -> ank_base::ConfigItem {
            match value {
                serde_yaml::Value::String(val) => ank_base::ConfigItem {
                    config_item: Some(ank_base::config_item::ConfigItem::String(val.clone())),
                },
                serde_yaml::Value::Sequence(val) => ank_base::ConfigItem {
                    config_item: Some(ank_base::config_item::ConfigItem::Array(ank_base::ConfigArray {
                        values: val.iter().map(to_config_item).collect(),
                    })),
                },
                serde_yaml::Value::Mapping(val) => ank_base::ConfigItem {
                    config_item: Some(ank_base::config_item::ConfigItem::Object(ank_base::ConfigObject {
                        fields: val.iter().map(|(k, v)| (k.as_str().unwrap().to_string(), to_config_item(v))).collect(),
                    })),
                },
                _ => ank_base::ConfigItem {
                    config_item: None,
                },
            }
        }

        if self.complete_state.desired_state.as_mut().unwrap().configs.is_none() {
            self.complete_state.desired_state.as_mut().unwrap().configs = Some(ank_base::ConfigMap {
                configs: Default::default(),
            });
        }
        self.complete_state.desired_state.as_mut().unwrap().configs.as_mut().unwrap().configs = self.configs.iter().map(|(k, v)| (k.clone(), to_config_item(v))).collect();
    }

    pub fn get_configs(&self) -> HashMap<String, serde_yaml::Value> {
        self.configs.clone()
    }
}

impl Default for CompleteState {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CompleteState {
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
        Ok(Self::new_from_proto(proto))
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

#[cfg(any(feature = "test_utils", test))]
use crate::components::workload_mod::test_helpers::generate_test_workload_proto;

#[cfg(any(feature = "test_utils", test))]
use crate::components::workload_state_mod::generate_test_workload_states_proto;

#[cfg(any(feature = "test_utils", test))]
fn generate_test_configs_proto() -> ank_base::ConfigMap {
    ank_base::ConfigMap { configs: HashMap::from([
        ("config1".to_string(), ank_base::ConfigItem {
            config_item: Some(ank_base::config_item::ConfigItem::String("value1".to_string())),
        }),
        ("config2".to_string(), ank_base::ConfigItem {
            config_item: Some(ank_base::config_item::ConfigItem::Array(ank_base::ConfigArray {
                values: vec![
                    ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value2".to_string())),
                    },
                    ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value3".to_string())),
                    },
                ],
            })),
        }),
        ("config3".to_string(), ank_base::ConfigItem {
            config_item: Some(ank_base::config_item::ConfigItem::Object(ank_base::ConfigObject {
                fields: HashMap::from([
                    ("field1".to_string(), ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value4".to_string())),
                    }),
                    ("field2".to_string(), ank_base::ConfigItem {
                        config_item: Some(ank_base::config_item::ConfigItem::String("value5".to_string())),
                    }),
                ]),
            })),
        }),
    ])}
}

#[cfg(any(feature = "test_utils", test))]
fn generate_agents_proto() -> ank_base::AgentMap {
    ank_base::AgentMap { agents: HashMap::from([
        ("agent_A".to_string(), ank_base::AgentAttributes {
            cpu_usage: Some(ank_base::CpuUsage {
                cpu_usage: 50,
            }),
            free_memory: Some(ank_base::FreeMemory {
                free_memory: 1024,
            }),
        }),
    ])}
}

#[cfg(any(feature = "test_utils", test))]
fn generate_complete_state_proto() -> ank_base::CompleteState {
    ank_base::CompleteState {
        desired_state: Some(ank_base::State {
            api_version: SUPPORTED_API_VERSION.to_string(),
            workloads: Some(ank_base::WorkloadMap {
                workloads: HashMap::from([
                    ("nginx_test".to_string(), generate_test_workload_proto("agent_A", "podman")),
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
    use std::collections::HashMap;

    use super::{generate_complete_state_proto, CompleteState, SUPPORTED_API_VERSION};
    use crate::components::manifest::generate_test_manifest;
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
        configs.insert("config1".to_string(), serde_yaml::Value::Null);
        complete_state.set_configs(configs);
        assert_eq!(complete_state.get_configs().len(), 1);
        assert!(complete_state.get_configs().get("config1").unwrap().is_null());
    }

    #[test]
    fn utest_to_dict() {
        let complete_state = CompleteState::try_from(generate_complete_state_proto()).unwrap();

        // Populate the expected mapping
        let mut expected_mapping = serde_yaml::Mapping::new();
        expected_mapping.insert(serde_yaml::Value::String("apiVersion".to_string()), serde_yaml::Value::String(SUPPORTED_API_VERSION.to_string()));
        let mut workloads = serde_yaml::Mapping::new();
        workloads.insert(serde_yaml::Value::String("nginx_test".to_string()), serde_yaml::Value::Mapping(complete_state.get_workloads()[0].to_dict()));
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
        let workload_instance_name = WorkloadInstanceName::new("agent_A".to_string(), "nginx".to_string(), "1234".to_string());
        assert!(workload_states.get_for_instance_name(&workload_instance_name).is_some());
    }
}