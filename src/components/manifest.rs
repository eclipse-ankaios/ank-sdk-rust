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

use std::path::Path;
use serde_yaml;
use crate::AnkaiosError;

// Disable this from coverage
// https://github.com/rust-lang/rust/issues/84605
#[cfg(not(test))]
fn read_file_to_string(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[cfg(test)]
use self::read_to_string_mock as read_file_to_string;

#[derive(Debug, Clone)]
pub struct Manifest{
    manifest: serde_yaml::Value
}

impl Manifest {
    pub fn new(manifest: serde_yaml::Value) -> Result<Manifest, AnkaiosError> {
        let obj = Self{manifest};
        if !obj.check() {
            return Err(AnkaiosError::InvalidManifestError("Manifest is not valid".to_string()));
        }
        Ok(obj)
    }

    pub fn from_dict(manifest: serde_yaml::Value) -> Result<Manifest, AnkaiosError> {
        Self::new(manifest)
    }

    pub fn from_string<T: Into<String>>(manifest: T) -> Result<Manifest, AnkaiosError> {
        match serde_yaml::from_str(&manifest.into()) {
            Ok(manifest) => Self::from_dict(manifest),
            Err(e) => Err(AnkaiosError::InvalidManifestError(e.to_string()))
        }
    }

    pub fn from_file(path: &Path) -> Result<Manifest, AnkaiosError> {
        match read_file_to_string(path) {
            Ok(content) => Self::from_string(content),
            Err(e) => Err(AnkaiosError::InvalidManifestError(e.to_string()))
        }
    }

    pub fn check(&self) -> bool {
        if self.manifest.get("apiVersion").is_none() {
            return false;
        }
        let allowed_fields = [
            "runtime", "agent", "restartPolicy", "runtimeConfig",
            "dependencies", "tags", "controlInterfaceAccess", "configs"
            ];
        let mandatory_fields = ["runtime", "runtimeConfig", "agent"];
        for wl_name in self.manifest["workloads"].as_mapping().unwrap_or(&serde_yaml::Mapping::default()).keys() {
            for field in self.manifest["workloads"][wl_name].as_mapping().unwrap_or(&serde_yaml::Mapping::default()).keys() {
                if !allowed_fields.contains(&field.as_str().unwrap()) {
                    return false;
                }
            }
            for field in mandatory_fields.iter() {
                if self.manifest["workloads"][wl_name].get(field).is_none() {
                    return false;
                }
            }
        }
        true
    }

    pub fn calculate_masks(&self) -> Vec<String> {
        let mut masks = vec![];
        print!("{:?}", self.manifest);
        for wl_name in self.manifest["workloads"].as_mapping().unwrap_or(&serde_yaml::Mapping::default()).keys() {
            masks.push(format!("desiredState.workloads.{}", wl_name.as_str().unwrap()));
        }
        for config_name in self.manifest["configs"].as_mapping().unwrap_or(&serde_yaml::Mapping::default()).keys() {
            masks.push(format!("desiredState.configs.{}", config_name.as_str().unwrap()));
        }
        masks
    }

    pub fn to_dict(&self) -> serde_yaml::Value {
        self.manifest.clone()
    }
}

impl TryFrom<serde_yaml::Value> for Manifest {
    type Error = AnkaiosError;

    fn try_from(value: serde_yaml::Value) -> Result<Self, Self::Error> {
        Self::from_dict(value)
    }
}

impl TryFrom<String> for Manifest {
    type Error = AnkaiosError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_string(value)
    }
}

impl TryFrom<&Path> for Manifest {
    type Error = AnkaiosError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::from_file(value)
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
pub fn read_to_string_mock(_path: &Path) -> Result<String, std::io::Error> {
    Ok(_path.to_str().unwrap().to_string())
}

#[cfg(any(feature = "test_utils", test))]
static MANIFEST_CONTENT: &str = r#"apiVersion: v0.1
workloads:
    nginx_test:
        runtime: podman
        restartPolicy: NEVER
        agent: agent_A
        configs:
            c: config1
        runtimeConfig: |
            image: image/test
configs:
    config1: \"value1\"
    config2: 
        - \"value2\"
        - \"value3\"
    config3:
        field1: \"value4\"
        field2: \"value5\""#;

#[cfg(test)]
pub fn generate_test_manifest() -> Manifest {
    Manifest::from_string(MANIFEST_CONTENT).unwrap()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use serde_yaml;
    use super::{Manifest, MANIFEST_CONTENT};

    #[test]
    fn utest_creation() {
        let manifest = Manifest::from_file(Path::new(MANIFEST_CONTENT)).unwrap();
        assert_eq!(manifest.manifest["apiVersion"], "v0.1");
        assert_eq!(manifest.calculate_masks(), vec!["desiredState.workloads.nginx_test", "desiredState.configs.config1", "desiredState.configs.config2", "desiredState.configs.config3"]);

        let _ = Manifest::try_from(Path::new("path"));
        let _ = Manifest::try_from(MANIFEST_CONTENT.to_string());
        let _ = Manifest::try_from(serde_yaml::Value::default());
    }

    #[test]
    fn utest_no_workloads() {
        let manifest_result = Manifest::from_string("apiVersion: v0.1");
        assert!(manifest_result.is_ok());
        let manifest: Manifest = manifest_result.unwrap();
        assert_eq!(manifest.calculate_masks().len(), 0);
    }
}
