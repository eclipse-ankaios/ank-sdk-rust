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

use super::workload_mod::WORKLOADS_PREFIX;
use crate::ankaios_api;
use crate::{AnkaiosError, Workload};
use ankaios_api::ank_base;
use std::{collections::HashMap, path::Path};

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
use self::read_to_string_mock as read_file_to_string;

/// The prefix for the api version in the desired state.
pub const API_VERSION_PREFIX: &str = "desiredState.apiVersion";
/// The prefix for the configs in the desired state.
pub const CONFIGS_PREFIX: &str = "desiredState.configs";

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
/// ## Load a manifest from a [`serde_yaml::Value`]:
///
/// ```rust
/// let dict = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
/// let _manifest = Manifest::from_dict(dict).unwrap();
/// ```
///
/// [Ankaios]: https://eclipse-ankaios.github.io/ankaios
#[derive(Debug, Clone)]
pub struct Manifest {
    /// The desired state.
    desired_state: ank_base::State,
}

impl Manifest {
    /// Create a new `Manifest` object from a [`serde_yaml::Value`].
    ///
    /// ## Arguments
    ///
    /// * `manifest` - A [`serde_yaml::Value`] object representing the manifest.
    ///
    /// ## Returns
    ///
    /// A [Manifest] object if the manifest is valid.
    ///
    /// ## Errors
    ///
    /// Returns an [`AnkaiosError`]::[`ManifestParsingError`](AnkaiosError::ManifestParsingError) if the manifest is not valid.
    pub fn from_dict(manifest: serde_yaml::Value) -> Result<Manifest, AnkaiosError> {
        Manifest::try_from(manifest)
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
    /// Returns an [`AnkaiosError`]::[`ManifestParsingError`](AnkaiosError::ManifestParsingError) if the manifest is not valid.
    pub fn from_string<T: Into<String>>(manifest: T) -> Result<Manifest, AnkaiosError> {
        Manifest::try_from(manifest.into())
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
    /// Returns an [`AnkaiosError`]::[`ManifestParsingError`](AnkaiosError::ManifestParsingError) if the manifest is not valid.
    pub fn from_file(path: &Path) -> Result<Manifest, AnkaiosError> {
        Manifest::try_from(path)
    }

    /// Calculate the masks for the manifest.
    ///
    /// ## Returns
    ///
    /// A [vector](Vec) of [strings](String) representing the masks.
    #[must_use]
    pub fn calculate_masks(&self) -> Vec<String> {
        let mut masks = vec![];
        if let Some(workloads) = self.desired_state.workloads.as_ref() {
            for wl_name in workloads.workloads.keys() {
                masks.push(format!("{WORKLOADS_PREFIX}.{wl_name}"));
            }
        }
        if let Some(configs) = self.desired_state.configs.as_ref() {
            for config_name in configs.configs.keys() {
                masks.push(format!("{CONFIGS_PREFIX}.{config_name}"));
            }
        }
        masks
    }

    /// Get the manifest as a [`ank_base::State`].
    ///
    /// ## Returns
    ///
    /// A [`ank_base::State`] object representing the manifest.
    #[allow(clippy::wrong_self_convention)] // This function must consume self in order to not clone the desired state when returning it.
    pub(crate) fn to_desired_state(self) -> ank_base::State {
        self.desired_state
    }
}

impl TryFrom<serde_yaml::Value> for Manifest {
    type Error = AnkaiosError;

    fn try_from(manifest: serde_yaml::Value) -> Result<Self, Self::Error> {
        // Extract api version
        let api_version = match manifest.get("apiVersion") {
            Some(value) => match value.as_str() {
                Some(version) => version.to_owned(),
                None => {
                    return Err(AnkaiosError::ManifestParsingError(
                        "Invalid apiVersion".to_owned(),
                    ))
                }
            },
            None => {
                return Err(AnkaiosError::ManifestParsingError(
                    "Missing apiVersion".to_owned(),
                ))
            }
        };

        // Extract workloads
        let mut workloads: ank_base::WorkloadMap = ank_base::WorkloadMap {
            workloads: HashMap::new(),
        };
        if let Some(workloads_value) = manifest.get("workloads") {
            if let Some(workloads_mapping) = workloads_value.as_mapping() {
                for (key, value) in workloads_mapping {
                    if let Some(key_str) = key.as_str() {
                        if let Some(value_mapping) = value.as_mapping() {
                            let workload = Workload::new_from_dict(
                                key_str.to_owned(),
                                &value_mapping.clone(),
                            )?;
                            workloads
                                .workloads
                                .insert(key_str.to_owned(), workload.to_proto());
                        } else {
                            return Err(AnkaiosError::ManifestParsingError(
                                "Invalid workload mapping".to_owned(),
                            ));
                        }
                    } else {
                        return Err(AnkaiosError::ManifestParsingError(
                            "Invalid workload key".to_owned(),
                        ));
                    }
                }
            } else {
                return Err(AnkaiosError::ManifestParsingError(
                    "Invalid workloads mapping".to_owned(),
                ));
            }
        }

        // Extract configs
        let configs = match manifest.get("configs") {
            Some(configs_value) => {
                match serde_yaml::from_value::<ank_base::ConfigMap>(configs_value.clone()) {
                    Ok(configs) => Some(configs),
                    Err(e) => return Err(AnkaiosError::ManifestParsingError(e.to_string())),
                }
            }
            None => None,
        };

        Ok(Self {
            desired_state: ank_base::State {
                api_version,
                workloads: if workloads.workloads.is_empty() {
                    None
                } else {
                    Some(workloads)
                },
                configs,
            },
        })
    }
}

impl TryFrom<String> for Manifest {
    type Error = AnkaiosError;

    fn try_from(manifest: String) -> Result<Self, Self::Error> {
        match serde_yaml::from_str(&manifest) {
            Ok(man) => Self::from_dict(man),
            Err(e) => Err(AnkaiosError::ManifestParsingError(e.to_string())),
        }
    }
}

impl TryFrom<&Path> for Manifest {
    type Error = AnkaiosError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        match read_file_to_string(path) {
            Ok(content) => Self::from_string(content),
            Err(e) => Err(AnkaiosError::ManifestParsingError(e.to_string())),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
//                 ########  #######    #########  #########                //
//                    ##     ##        ##             ##                    //
//                    ##     #####     #########      ##                    //
//                    ##     ##                ##     ##                    //
//                    ##     #######   #########      ##                    //
//////////////////////////////////////////////////////////////////////////////

/// Helper function to read a file to a string.
#[allow(clippy::unnecessary_wraps)]
#[cfg(test)]
pub fn read_to_string_mock(path: &Path) -> Result<String, std::io::Error> {
    Ok(path.to_str().unwrap().to_owned())
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
    use super::{Manifest, MANIFEST_CONTENT};
    use serde_yaml;
    use std::path::Path;

    #[test]
    fn test_doc_examples() {
        // Load a manifest from a file
        let _manifest = Manifest::from_file(Path::new("apiVersion: v0.1")).unwrap();

        // Load a manifest from a string
        let _manifest = Manifest::from_string("apiVersion: v0.1").unwrap();

        // Load a manifest from a serde_yaml::Value
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("apiVersion".to_owned()),
            serde_yaml::Value::String("v0.1".to_owned()),
        );
        let dict = serde_yaml::Value::Mapping(map);
        let _manifest = Manifest::from_dict(dict).unwrap();
    }

    #[test]
    fn utest_creation() {
        let manifest = Manifest::from_file(Path::new(MANIFEST_CONTENT)).unwrap();
        assert_eq!(manifest.desired_state.api_version, "v0.1");
        let masks = manifest.calculate_masks();
        assert!(masks.contains(&"desiredState.workloads.nginx_test".to_owned()));
        assert!(masks.contains(&"desiredState.configs.config1".to_owned()));
        assert!(masks.contains(&"desiredState.configs.config2".to_owned()));
        assert!(masks.contains(&"desiredState.configs.config3".to_owned()));

        let _ = Manifest::try_from(Path::new("path"));
        let _ = Manifest::try_from(MANIFEST_CONTENT.to_owned());
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
