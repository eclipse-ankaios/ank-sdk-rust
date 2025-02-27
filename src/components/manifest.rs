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

//! This module contains the [Manifest] struct.

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

/// Struct represents a manifest file.
///
/// The `Manifest` struct is used to load a manifest file and
/// directly use it to modify the [Ankaios] cluster.
///
/// # Examples
///
/// ## Load a manifest from a file's [Path]:
/// 
/// ```rust
/// let manifest = Manifest::from_file(Path::new("path/to/manifest.yaml")).unwrap();
/// ```
/// 
/// ## Load a manifest from a [String]:
/// 
/// ```rust
/// let manifest = Manifest::from_string("apiVersion: v0.1").unwrap();
/// ```
/// 
/// ## Load a manifest from a [serde_yaml::Value]:
/// 
/// ```rust
/// let dict = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
/// let _manifest = Manifest::from_dict(dict).unwrap();
/// ```
/// 
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug, Clone)]
pub struct Manifest{
    manifest: serde_yaml::Value
}

impl Manifest {
    /// Create a new `Manifest` object.
    /// 
    /// ## Arguments
    /// 
    /// * `manifest` - A [serde_yaml::Value] object representing the manifest.
    /// 
    /// ## Returns
    /// 
    /// A [Manifest] object if the manifest is valid.
    /// 
    /// ## Errors
    /// 
    /// Returns an [AnkaiosError]::[InvalidManifestError](AnkaiosError::InvalidManifestError) if the manifest is not valid.
    pub fn new(manifest: serde_yaml::Value) -> Result<Manifest, AnkaiosError> {
        let obj = Self{manifest};
        if !obj.check() {
            return Err(AnkaiosError::InvalidManifestError("Manifest is not valid".to_string()));
        }
        Ok(obj)
    }

    /// Create a new `Manifest` object from a [serde_yaml::Value].
    /// 
    /// ## Arguments
    /// 
    /// * `manifest` - A [serde_yaml::Value] object representing the manifest.
    /// 
    /// ## Returns
    /// 
    /// A [Manifest] object if the manifest is valid.
    /// 
    /// ## Errors
    /// 
    /// Returns an [AnkaiosError]::[InvalidManifestError](AnkaiosError::InvalidManifestError) if the manifest is not valid.
    pub fn from_dict(manifest: serde_yaml::Value) -> Result<Manifest, AnkaiosError> {
        Self::new(manifest)
    }

    /// Create a new `Manifest` object from a [String].
    /// 
    /// ## Arguments
    /// 
    /// * `manifest` - A [String] object representing the manifest.
    /// 
    /// ## Returns
    /// 
    /// A [Manifest] object if the manifest is valid.
    /// 
    /// ## Errors
    /// 
    /// Returns an [AnkaiosError]::[InvalidManifestError](AnkaiosError::InvalidManifestError) if the manifest is not valid.
    pub fn from_string<T: Into<String>>(manifest: T) -> Result<Manifest, AnkaiosError> {
        match serde_yaml::from_str(&manifest.into()) {
            Ok(manifest) => Self::from_dict(manifest),
            Err(e) => Err(AnkaiosError::InvalidManifestError(e.to_string()))
        }
    }

    /// Create a new `Manifest` object from a file's [Path].
    /// 
    /// ## Arguments
    /// 
    /// * `path` - A [Path] object representing the manifest file.
    /// 
    /// ## Returns
    /// 
    /// A [Manifest] object if the manifest is valid.
    /// 
    /// ## Errors
    /// 
    /// Returns an [AnkaiosError]::[InvalidManifestError](AnkaiosError::InvalidManifestError) if the manifest is not valid.
    pub fn from_file(path: &Path) -> Result<Manifest, AnkaiosError> {
        match read_file_to_string(path) {
            Ok(content) => Self::from_string(content),
            Err(e) => Err(AnkaiosError::InvalidManifestError(e.to_string()))
        }
    }

    /// Check if the manifest is valid.
    /// 
    /// ## Returns
    /// 
    /// Returns `true` if the manifest is valid, `false` otherwise.
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

    /// Calculate the masks for the manifest.
    /// 
    /// ## Returns
    /// 
    /// A [vector](Vec) of [strings](String) representing the masks.
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

    /// Convert the manifest to a [serde_yaml::Value].
    /// 
    /// ## Returns
    /// 
    /// A [serde_yaml::Value] object representing the manifest.
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

#[cfg(test)]
pub fn read_to_string_mock(_path: &Path) -> Result<String, std::io::Error> {
    Ok(_path.to_str().unwrap().to_string())
}

#[cfg(test)]
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
    fn test_doc_examples() {
        // Load a manifest from a file
        let _manifest = Manifest::from_file(Path::new("apiVersion: v0.1")).unwrap();

        // Load a manifest from a string
        let _manifest = Manifest::from_string("apiVersion: v0.1").unwrap();

        // Load a manifest from a serde_yaml::Value
        let mut map = serde_yaml::Mapping::new();
        map.insert(serde_yaml::Value::String("apiVersion".to_string()), serde_yaml::Value::String("v0.1".to_string()));
        let dict = serde_yaml::Value::Mapping(map);
        let _manifest = Manifest::from_dict(dict).unwrap();
    }

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
