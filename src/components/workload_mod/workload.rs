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
use std::{collections::HashMap, path::Path, vec};
use serde_yaml;
pub use api::ank_base;
use crate::AnkaiosError;
use crate::WorkloadBuilder;

// Disable this from coverage
// https://github.com/rust-lang/rust/issues/84605
#[cfg(not(test))]
fn read_file_to_string(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[cfg(test)]
use crate::components::workload_mod::test_helpers::read_to_string_mock as read_file_to_string;

#[derive(Debug, Clone)]
pub struct Workload{
    pub(crate) workload: ank_base::Workload,
    pub(crate) main_mask: String,
    pub masks: Vec<String>,
    pub name: String,
}

impl Workload {
    pub fn new_from_builder<T: Into<String>>(name: T) -> Self {
        let name_str = name.into();
        Self{
            workload: ank_base::Workload::default(),
            main_mask: format!("desiredState.workloads.{}", name_str),
            masks: vec![format!("desiredState.workloads.{}", name_str)],
            name: name_str,
        }
    }

    pub fn new_from_proto<T: Into<String>>(name: T, proto: ank_base::Workload) -> Self {
        let name_str = name.into();
        Self{
            workload: proto,
            main_mask: format!("desiredState.workloads.{}", name_str),
            masks: vec![],
            name: name_str,
        }
    }

    pub fn new_from_dict<T: Into<String>>(name: T, dict_workload: serde_yaml::Mapping) -> Result<Self, AnkaiosError> {
        let mut wl_builder = Self::builder();
        wl_builder = wl_builder.workload_name(name);

        if let Some(agent) = dict_workload.get("agent") {
            wl_builder = wl_builder.agent_name(agent.as_str().unwrap())
        }
        if let Some(runtime) = dict_workload.get("runtime") {
            wl_builder = wl_builder.runtime(runtime.as_str().unwrap())
        }
        if let Some(runtime_config) = dict_workload.get("runtimeConfig") {
            wl_builder = wl_builder.runtime_config(runtime_config.as_str().unwrap())
        }
        if let Some(restart_policy) = dict_workload.get("restartPolicy") {
            wl_builder = wl_builder.restart_policy(restart_policy.as_str().unwrap())
        }
        if let Some(dependencies) = dict_workload.get("dependencies") {
            for (key, value) in dependencies.as_mapping().unwrap() {
                wl_builder = wl_builder.add_dependency(
                    key.as_str().unwrap(), 
                    value.as_str().unwrap());
            }
        }
        if let Some(tags) = dict_workload.get("tags") {
            for tag in tags.as_sequence().unwrap() {
                wl_builder = wl_builder.add_tag(
                    tag.get("key").unwrap().as_str().unwrap(), 
                    tag.get("value").unwrap().as_str().unwrap());
            }
        }
        if let Some(control_interface_access) = dict_workload.get("controlInterfaceAccess") {
            if let Some(allow_rules) = control_interface_access.get("allowRules") {
                for rule in allow_rules.as_sequence().unwrap() {
                    let operation = rule.get("operation").unwrap().as_str().unwrap();
                    let filter_masks = rule.get("filterMask")
                        .unwrap().as_sequence().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
                    wl_builder = wl_builder.add_allow_rule(operation, filter_masks);
                }
            }
            if let Some(deny_rules) = control_interface_access.get("denyRules") {
                for rule in deny_rules.as_sequence().unwrap() {
                    let operation = rule.get("operation").unwrap().as_str().unwrap();
                    let filter_masks = rule.get("filterMask")
                        .unwrap().as_sequence().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
                    wl_builder = wl_builder.add_deny_rule(operation, filter_masks);
                }
            }
        }
        if let Some(configs) = dict_workload.get("configs") {
            for (alias, name) in configs.as_mapping().unwrap() {
                wl_builder = wl_builder.add_config(
                    alias.as_str().unwrap(), 
                    name.as_str().unwrap());
            }
        }

        wl_builder.build()
    }

    pub fn to_proto(&self) -> ank_base::Workload {
        self.workload.clone()
    }

    pub fn to_dict(&self) -> serde_yaml::Mapping {
        let mut dict = serde_yaml::Mapping::new();
        if self.workload.agent.is_some() {
            dict.insert(
                serde_yaml::Value::String("agent".to_string()),
                serde_yaml::Value::String(self.workload.agent.clone().unwrap()));
        }
        if self.workload.runtime.is_some() {
            dict.insert(
                serde_yaml::Value::String("runtime".to_string()),
                serde_yaml::Value::String(self.workload.runtime.clone().unwrap()));
        }
        if self.workload.runtime_config.is_some() {
            dict.insert(
                serde_yaml::Value::String("runtimeConfig".to_string()),
                serde_yaml::Value::String(self.workload.runtime_config.clone().unwrap()));
        }
        if self.workload.restart_policy.is_some() {
            dict.insert(
                serde_yaml::Value::String("restartPolicy".to_string()),
                serde_yaml::Value::String(ank_base::RestartPolicy::from_i32(self.workload.restart_policy.unwrap()).unwrap().as_str_name().to_string()));
        }
        if self.workload.dependencies.is_some() {
            let mut deps = serde_yaml::Mapping::new();
            dict.insert(
                serde_yaml::Value::String("dependencies".to_string()),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
            for (key, value) in &self.workload.dependencies.clone().unwrap().dependencies {
                deps.insert(
                    serde_yaml::Value::String(key.clone()),
                    serde_yaml::Value::String(ank_base::AddCondition::from_i32(*value).unwrap().as_str_name().to_string()));
            }
            dict.insert(
                serde_yaml::Value::String("dependencies".to_string()),
                serde_yaml::Value::Mapping(deps));
        }
        if self.workload.tags.is_some() {
            let mut tags = serde_yaml::Sequence::new();
            for tag in &self.workload.tags.clone().unwrap().tags {
                let mut tag_dict = serde_yaml::Mapping::new();
                tag_dict.insert(
                    serde_yaml::Value::String("key".to_string()),
                    serde_yaml::Value::String(tag.key.clone()));
                tag_dict.insert(
                    serde_yaml::Value::String("value".to_string()),
                    serde_yaml::Value::String(tag.value.clone()));
                tags.push(serde_yaml::Value::Mapping(tag_dict));
            }
            dict.insert(
                serde_yaml::Value::String("tags".to_string()),
                serde_yaml::Value::Sequence(tags));
        }
        if self.workload.control_interface_access.is_some() {
            let mut control_interface_access = serde_yaml::Mapping::new();

            let mut allow_rules = serde_yaml::Sequence::new();
            for rule in &self.workload.control_interface_access.clone().unwrap().allow_rules {
                let mut rule_dict = serde_yaml::Mapping::new();
                rule_dict.insert(
                    serde_yaml::Value::String("type".to_string()),
                    serde_yaml::Value::String("StateRule".to_string()));
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum: Some(
                        ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(rule)
                    ),
                } = rule {
                    match self.access_right_rule_to_str(rule) {
                        Ok(rule) => {
                            rule_dict.insert(
                                serde_yaml::Value::String("operation".to_string()),
                                serde_yaml::Value::String(rule.0));
                            rule_dict.insert(
                                serde_yaml::Value::String("filterMask".to_string()),
                                serde_yaml::Value::Sequence(rule.1.into_iter().map(serde_yaml::Value::String).collect()));
                        },
                        Err(_) => continue,
                    };
                }
                allow_rules.push(serde_yaml::Value::Mapping(rule_dict));
            }
            if !allow_rules.is_empty() {
                control_interface_access.insert(
                    serde_yaml::Value::String("allowRules".to_string()),
                    serde_yaml::Value::Sequence(allow_rules));
            }

            let mut deny_rules = serde_yaml::Sequence::new();
            for rule in &self.workload.control_interface_access.clone().unwrap().deny_rules {
                let mut rule_dict = serde_yaml::Mapping::new();
                rule_dict.insert(
                    serde_yaml::Value::String("type".to_string()),
                    serde_yaml::Value::String("StateRule".to_string()));
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum: Some(
                        ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(rule)
                    ),
                } = rule {
                    match self.access_right_rule_to_str(rule) {
                        Ok(rule) => {
                            rule_dict.insert(
                                serde_yaml::Value::String("operation".to_string()),
                                serde_yaml::Value::String(rule.0));
                            rule_dict.insert(
                                serde_yaml::Value::String("filterMask".to_string()),
                                serde_yaml::Value::Sequence(rule.1.into_iter().map(serde_yaml::Value::String).collect()));
                        },
                        Err(_) => continue,
                    };
                }
                deny_rules.push(serde_yaml::Value::Mapping(rule_dict));
            }
            if !deny_rules.is_empty() {
                control_interface_access.insert(
                    serde_yaml::Value::String("denyRules".to_string()),
                    serde_yaml::Value::Sequence(deny_rules));
            }

            dict.insert(
                serde_yaml::Value::String("controlInterfaceAccess".to_string()),
                serde_yaml::Value::Mapping(control_interface_access));
        }
        if self.workload.configs.is_some() {
            let mut configs = serde_yaml::Mapping::new();
            for (alias, name) in &self.workload.configs.clone().unwrap().configs {
                configs.insert(
                    serde_yaml::Value::String(alias.clone()),
                    serde_yaml::Value::String(name.clone()));
            }
            dict.insert(
                serde_yaml::Value::String("configs".to_string()),
                serde_yaml::Value::Mapping(configs));
        }
        dict
    }

    pub fn builder() -> WorkloadBuilder {
        WorkloadBuilder::new()
    }

    pub fn update_workload_name<T: Into<String>>(&mut self, new_name: T) {
        self.name = new_name.into();
        self.main_mask = format!("desiredState.workloads.{}", self.name);
        self.masks = vec![format!("desiredState.workloads.{}", self.name)];
    }

    pub fn update_agent_name<T: Into<String>>(&mut self, agent_name: T) {
        self.workload.agent = Some(agent_name.into());
        self.add_mask(format!("{}.agent", self.main_mask));
    }

    pub fn update_runtime<T: Into<String>>(&mut self, runtime: T) {
        self.workload.runtime = Some(runtime.into());
        self.add_mask(format!("{}.runtime", self.main_mask));
    }

    pub fn update_runtime_config<T: Into<String>>(&mut self, runtime_config: T) {
        self.workload.runtime_config = Some(runtime_config.into());
        self.add_mask(format!("{}.runtimeConfig", self.main_mask));
    }

    pub fn update_runtime_config_from_file(&mut self, file_path: &Path) -> Result<(), AnkaiosError> {
        let runtime_config = match read_file_to_string(file_path) {
            Ok(config) => config,
            Err(err) => return Err(AnkaiosError::IoError(err))
        };
        self.update_runtime_config(runtime_config);
        Ok(())
    }

    pub fn update_restart_policy<T: Into<String>>(&mut self, restart_policy: T) -> Result<(), AnkaiosError> {
        let restart_policy = restart_policy.into();
        self.workload.restart_policy = match ank_base::RestartPolicy::from_str_name(&restart_policy.clone()) {
            Some(policy) => Some(policy as i32),
            _ => return Err(AnkaiosError::WorkloadFieldError(
                "restartPolicy".to_string(), 
                restart_policy
            ))
        };
        self.add_mask(format!("{}.restartPolicy", self.main_mask));
        Ok(())
    }

    pub fn get_dependencies(&self) -> HashMap<String, String> {
        let mut dependencies = HashMap::new();
        if let Some(deps) = &self.workload.dependencies {
            for (key, value) in &deps.dependencies {
                dependencies.insert(key.clone(), ank_base::AddCondition::from_i32(*value).unwrap().as_str_name().to_string());
            }
        }
        dependencies
    }

    pub fn update_dependencies<T: Into<String>>(&mut self, dependencies: HashMap<T, T>) -> Result<(), AnkaiosError> {
        self.workload.dependencies = Some(ank_base::Dependencies::default());
        for (workload_name, condition) in dependencies {
            let cond = condition.into();
            let add_condition = match ank_base::AddCondition::from_str_name(&cond.clone()) {
                Some(cond) => cond as i32,
                _ => return Err(AnkaiosError::WorkloadFieldError(
                    "dependency condition".to_string(), 
                    cond
                )),
            };
            if self.workload.dependencies.is_none() {
                self.workload.dependencies = Some(ank_base::Dependencies::default());
            }
            self.workload.dependencies.as_mut().unwrap().dependencies.insert(workload_name.into(), add_condition);
        }
        self.add_mask(format!("{}.dependencies", self.main_mask));
        Ok(())
    }

    pub fn add_tag<T: Into<String>>(&mut self, key: T, value: T) {
        if self.workload.tags.is_none() {
            self.workload.tags = Some(ank_base::Tags::default());
        }
        let key = key.into();
        self.workload.tags.as_mut().unwrap().tags.push(ank_base::Tag{key: key.clone(), value: value.into()});
        if !self.masks.contains(&format!("{}.tags", self.main_mask)) {
            self.add_mask(format!("{}.tags.{}", self.main_mask, key));
        }
    }

    pub fn get_tags(&self) -> Vec<Vec<String>> {
        let mut tags = vec![];
        if let Some(tags_list) = &self.workload.tags {
            for tag in &tags_list.tags {
                tags.push(vec![tag.key.clone(), tag.value.clone()]);
            }
        }
        tags
    }

    pub fn update_tags(&mut self, tags: &Vec<Vec<String>>) {
        self.workload.tags = Some(ank_base::Tags::default());
        for tag in tags {
            self.workload.tags.as_mut().unwrap().tags.push(ank_base::Tag{key: tag[0].clone(), value: tag[1].clone()});
        }
        self.masks.retain(|mask| !mask.starts_with(&format!("{}.tags", self.main_mask)));
        self.add_mask(format!("{}.tags", self.main_mask));
    }

    fn generate_access_right_rule(&self, operation: &str, filter_masks: Vec<String>) -> Result<ank_base::AccessRightsRule, AnkaiosError> {
        Ok(ank_base::AccessRightsRule {
            access_rights_rule_enum: Some(ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(
                ank_base::StateRule {
                    operation: match operation {
                        "Nothing" => ank_base::ReadWriteEnum::RwNothing as i32,
                        "Write" => ank_base::ReadWriteEnum::RwWrite as i32,
                        "Read" => ank_base::ReadWriteEnum::RwRead as i32,
                        "ReadWrite" => ank_base::ReadWriteEnum::RwReadWrite as i32,
                        _ => return Err(AnkaiosError::WorkloadFieldError(
                            "operation".to_string(), 
                            operation.to_string(),
                        )),
                    },
                    filter_masks,
                }
            )),
        })
    }

    fn access_right_rule_to_str(&self, rule: &ank_base::StateRule) -> Result<(String, Vec<String>), AnkaiosError> {
        Ok((match ank_base::ReadWriteEnum::from_i32(rule.operation) {
            Some(op) => match op.as_str_name() {
                "RW_NOTHING" => "Nothing".to_string(),
                "RW_WRITE" => "Write".to_string(),
                "RW_READ" => "Read".to_string(),
                "RW_READ_WRITE" => "ReadWrite".to_string(),
                _ => return Err(AnkaiosError::WorkloadFieldError(
                    "operation".to_string(), 
                    rule.operation.to_string(),
                ))
            },
            _ => return Err(AnkaiosError::WorkloadFieldError(
                "operation".to_string(), 
                rule.operation.to_string(),
            )),
        }, rule.filter_masks.clone()))
    }

    pub fn get_allow_rules(&self) -> Result<Vec<(String, Vec<String>)>, AnkaiosError> {
        let mut rules = vec![];
        if let Some(access) = &self.workload.control_interface_access {
            for rule in &access.allow_rules {
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum: Some(
                        ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(rule)
                    ),
                } = rule {
                    rules.push(match self.access_right_rule_to_str(rule) {
                        Ok(rule) => rule,
                        Err(err) => return Err(err),
                    });
                }
            }
        }
        Ok(rules)
    }

    pub fn update_allow_rules<T: Into<String>>(&mut self, rules: Vec<(T, Vec<T>)>) -> Result<(), AnkaiosError> {
        if self.workload.control_interface_access.is_none() {
            self.workload.control_interface_access = Some(ank_base::ControlInterfaceAccess::default());
        }
        self.workload.control_interface_access.as_mut().unwrap().allow_rules = vec![];
        for rule in rules {
            let rule = match self.generate_access_right_rule(
                rule.0.into().as_str(), 
                rule.1.into_iter().map(|x| x.into()).collect()
            ) {
                Ok(rule) => rule,
                Err(err) => return Err(err),
            };
            self.workload.control_interface_access.as_mut().unwrap().allow_rules.push(rule);
        }
        self.add_mask(format!("{}.controlInterfaceAccess.allowRules", self.main_mask));
        Ok(())
    }

    pub fn get_deny_rules(&self) -> Result<Vec<(String, Vec<String>)>, AnkaiosError> {
        let mut rules = vec![];
        if let Some(access) = &self.workload.control_interface_access {
            for rule in &access.deny_rules {
                if let ank_base::AccessRightsRule {
                    access_rights_rule_enum: Some(ank_base::access_rights_rule::AccessRightsRuleEnum::StateRule(rule)),
                } = rule {
                    rules.push(match self.access_right_rule_to_str(rule) {
                        Ok(rule) => rule,
                        Err(err) => return Err(err),
                    });
                }
            }
        }
        Ok(rules)
    }

    pub fn update_deny_rules<T: Into<String>>(&mut self, rules: Vec<(T, Vec<T>)>) -> Result<(), AnkaiosError> {
        if self.workload.control_interface_access.is_none() {
            self.workload.control_interface_access = Some(ank_base::ControlInterfaceAccess::default());
        }
        self.workload.control_interface_access.as_mut().unwrap().deny_rules = vec![];
        for rule in rules {
            let rule = match self.generate_access_right_rule(
                rule.0.into().as_str(), 
                rule.1.into_iter().map(|x| x.into()).collect()
            ){
                Ok(rule) => rule,
                Err(err) => return Err(err),
            };
            self.workload.control_interface_access.as_mut().unwrap().deny_rules.push(rule);
        }
        self.add_mask(format!("{}.controlInterfaceAccess.denyRules", self.main_mask));
        Ok(())
    }

    pub fn add_config<T: Into<String>>(&mut self, alias: T, name: T) {
        if self.workload.configs.is_none() {
            self.workload.configs = Some(ank_base::ConfigMappings{
                configs: [
                    (alias.into(), name.into()),
                ].into(),
            });
        }
        else {
            self.workload.configs.as_mut().unwrap().configs.insert(alias.into(), name.into());
        }
        self.add_mask(format!("{}.configs", self.main_mask));
    }

    pub fn get_configs(&self) -> HashMap<String, String> {
        let mut configs = HashMap::new();
        if let Some(configs_map) = &self.workload.configs {
            for (alias, name) in &configs_map.configs {
                configs.insert(alias.clone(), name.clone());
            }
        }
        configs
    }

    pub fn update_configs(&mut self, configs: HashMap<String, String>) {
        self.workload.configs = Some(ank_base::ConfigMappings{
            configs: configs.into_iter().collect(),
        });
    }

    fn add_mask(&mut self, mask: String) {
        if !self.masks.contains(&mask) && !self.masks.contains(&self.main_mask) {
            self.masks.push(mask);
        }
    }
}

impl fmt::Display for Workload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Workload {}: {:?}", self.name, self.to_proto())
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
    use std::collections::HashMap;
    use std::path::Path;
    use super::Workload;
    use crate::components::workload_mod::test_helpers::{
        generate_test_workload, generate_test_workload_proto, generate_test_runtime_config
    };

    #[test]
    fn utest_workload() {
        let wl_test = generate_test_workload("agent_A".to_string(), "Test".to_string(), "podman".to_string());
        let wl_proto = generate_test_workload_proto("agent_A".to_string(), "podman".to_string());
        assert_eq!(wl_test.name, "Test");
        assert_eq!(wl_test.main_mask, "desiredState.workloads.Test");
        assert_eq!(wl_test.masks, vec!["desiredState.workloads.Test".to_string()]);
        assert_eq!(wl_test.workload, wl_proto);
    }

    #[test]
    fn utest_workload_proto() {
        let workload_proto = generate_test_workload_proto("agent_A".to_string(), "podman".to_string());
        let wl = Workload::new_from_proto("Test", workload_proto.clone());
        let new_proto = wl.to_proto();
        assert_eq!(workload_proto, new_proto);
    }

    #[test]
    fn utest_workload_dict(){
        let workload = generate_test_workload("agent_A", "nginx", "podman");
        let workload_dict = workload.to_dict();
        let workload_new = Workload::new_from_dict("nginx", workload_dict);
        assert!(workload_new.is_ok());
        assert_eq!(workload.to_proto(), workload_new.unwrap().to_proto());
    }

    #[test]
    fn utest_update_fields() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        assert_eq!(wl.masks, vec!["desiredState.workloads.Test".to_string()]);

        wl.update_workload_name("TestNew");
        assert_eq!(wl.name, "TestNew");

        wl.update_agent_name("agent_B");
        assert_eq!(wl.workload.agent, Some("agent_B".to_string()));

        wl.update_runtime("podman-kube");
        assert_eq!(wl.workload.runtime, Some("podman-kube".to_string()));

        wl.update_runtime_config("config_test");
        assert_eq!(wl.workload.runtime_config, Some("config_test".to_string()));

        assert!(wl.update_restart_policy("NEVER").is_ok());
        assert_eq!(wl.workload.restart_policy, Some(0));

        assert!(wl.update_restart_policy("Dance").is_err());

        let tags = vec![vec!["key_test".to_string(), "val_test".to_string()]];
        wl.update_tags(&tags);
        assert_eq!(wl.get_tags(), tags);

        let allow_rules = vec![("Read".to_string(), vec!["desiredState.workloads.workload_A".to_string()])];
        assert!(wl.update_allow_rules(allow_rules.clone()).is_ok());
        assert_eq!(wl.get_allow_rules().unwrap(), allow_rules);

        let deny_rules = vec![("Write".to_string(), vec!["desiredState.workloads.workload_B".to_string()])];
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

        assert!(wl.update_dependencies(HashMap::from([("workload_A", "Dance")])).is_err());
    }

    #[test]
    fn utest_tags() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        let mut tags = wl.get_tags();
        assert_eq!(tags.len(), 1);

        wl.add_tag("key_test_2", "val_test_2");
        tags.push(vec!["key_test_2".to_string(), "val_test_2".to_string()]);
        assert_eq!(wl.get_tags().len(), 2);
        assert_eq!(wl.get_tags(), tags);

        tags.remove(0);
        wl.update_tags(&tags);
        assert_eq!(wl.get_tags().len(), 1);
    }

    #[test]
    fn utest_rules() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        let mut allow_rules = wl.get_allow_rules().unwrap();
        assert_eq!(allow_rules.len(), 1);

        allow_rules.push(("Write".to_string(), vec!["desiredState.workloads.workload_B".to_string()]));
        assert!(wl.update_allow_rules(allow_rules).is_ok());
        assert_eq!(wl.get_allow_rules().unwrap().len(), 2);

        assert!(wl.update_allow_rules(vec![("Dance".to_string(), vec!["desiredState.workloads.workload_A".to_string()])]).is_err());

        let mut deny_rules = wl.get_deny_rules().unwrap();
        assert_eq!(deny_rules.len(), 1);

        deny_rules.push(("Read".to_string(), vec!["desiredState.workloads.workload_A".to_string()]));
        assert!(wl.update_deny_rules(deny_rules).is_ok());
        assert_eq!(wl.get_deny_rules().unwrap().len(), 2);

        assert!(wl.update_deny_rules(vec![("Dance".to_string(), vec!["desiredState.workloads.workload_A".to_string()])]).is_err());
    }

    #[test]
    fn utest_configs() {
        let mut wl = generate_test_workload("Agent_A", "Test", "podman");
        let mut configs = wl.get_configs();
        assert_eq!(configs.len(), 1);

        wl.add_config("alias_test_2", "config_test_2");
        configs = wl.get_configs();
        assert_eq!(configs.len(), 2);

        configs.insert("alias_test_3".to_string(), "config_test_3".to_string());
        wl.update_configs(configs.clone());
        assert_eq!(wl.get_configs().len(), 3);
    }

    macro_rules! generate_test_for_mask_generation {
        ($test_name:ident, $method_name:ident, $expected_value:expr, $($args:expr),*) => {
            #[test]
            fn $test_name() {
                let mut obj = Workload {
                    workload: generate_test_workload_proto("Agent_A".to_string(), "podman".to_string()),
                    main_mask: format!("desiredState.workloads.Test"),
                    masks: vec![],
                    name: "Test".to_string(),
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

    generate_test_for_mask_generation!(utest_update_workload_name, update_workload_name,
        vec![String::from("desiredState.workloads.TestNew")], "TestNew");
    generate_test_for_mask_generation!(utest_update_agent_name, update_agent_name,
        vec![String::from("desiredState.workloads.Test.agent")], "agent_B");
    generate_test_for_mask_generation!(utest_update_runtime, update_runtime,
        vec![String::from("desiredState.workloads.Test.runtime")], "podman");
    generate_test_for_mask_generation!(utest_update_restart_policy, update_restart_policy,
        vec![String::from("desiredState.workloads.Test.restartPolicy")], "NEVER");
    generate_test_for_mask_generation!(utest_update_runtime_config, update_runtime_config,
        vec![String::from("desiredState.workloads.Test.runtimeConfig")], "config");
    generate_test_for_mask_generation!(utest_update_runtime_config_from_file, update_runtime_config_from_file,
        vec![String::from("desiredState.workloads.Test.runtimeConfig")], Path::new(""));
    generate_test_for_mask_generation!(utest_update_dependencies, update_dependencies,
        vec![String::from("desiredState.workloads.Test.dependencies")], HashMap::from([("workload_A", "ADD_COND_RUNNING")]));
    generate_test_for_mask_generation!(utest_add_tag, add_tag,
        vec![String::from("desiredState.workloads.Test.tags.key_test")], "key_test", "val_test");
        generate_test_for_mask_generation!(utest_update_tags, update_tags,
            vec![String::from("desiredState.workloads.Test.tags")], &vec![vec!["key_test".to_string(), "val_test".to_string()]]);
    generate_test_for_mask_generation!(utest_update_allow_rule, update_allow_rules,
        vec![String::from("desiredState.workloads.Test.controlInterfaceAccess.allowRules")], vec![("Read".to_string(), vec!["desiredState.workloads.workload_A".to_string()])]);
    generate_test_for_mask_generation!(utest_update_deny_rule, update_deny_rules,
        vec![String::from("desiredState.workloads.Test.controlInterfaceAccess.denyRules")], vec![("Write".to_string(), vec!["desiredState.workloads.workload_B".to_string()])]);
    generate_test_for_mask_generation!(utest_add_config, add_config,
        vec![String::from("desiredState.workloads.Test.configs")], "alias_test", "config_test");

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
        assert!(Workload::builder()
            .agent_name("agent_A")
            .runtime("podman")
            .runtime_config("config")
            .build()
            .is_err()
        );

        // No agent
        assert!(Workload::builder()
            .workload_name("Test")
            .runtime("podman")
            .runtime_config("config")
            .build()
            .is_err()
        );

        // No runtime
        assert!(Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime_config("config")
            .build()
            .is_err()
        );

        // No runtime config
        assert!(Workload::builder()
            .workload_name("Test")
            .agent_name("agent_A")
            .runtime("podman")
            .build()
            .is_err()
        );
    }
}