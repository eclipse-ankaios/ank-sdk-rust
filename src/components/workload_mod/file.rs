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

use crate::ankaios_api::ank_base;
use crate::AnkaiosError;
use serde_yaml::{Mapping, Value};
use std::fmt;

/// Key name for mount point of workload file.
pub const FILE_MOUNT_POINT_KEY: &str = "mount_point";
/// Key name for data of workload file.
pub const FILE_DATA_KEY: &str = "data";
/// Key name for binary data of workload file.
pub const FILE_BINARY_DATA_KEY: &str = "binaryData";

/// Represents a file that can be mounted to a workload.
///
/// A `File` can contain either text data or binary data (base64 encoded), but not both.
///
/// # Examples
///
/// ## Create a text file:
///
/// ```rust
/// use ankaios_sdk::File;
///
/// let text_file = File::from_text("/etc/config.txt", "Hello, World!");
/// ```
///
/// ## Create a binary file:
///
/// ```rust
/// use ankaios_sdk::File;
///
/// let binary_file = File::from_binary("/usr/share/app/binary_file", "iVBORw0KGgoARYANSUhEUgA...");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// The path where the file will be mounted in the container.
    pub mount_point: String,
    content: FileContent,
}

/// Represents the content type of a [`File`].
///
/// A file can contain either text data or binary data (base64 encoded).
#[derive(Debug, Clone, PartialEq)]
pub enum FileContent {
    /// Text content stored as a UTF-8 string.
    Text(String),
    /// Binary content stored as a base64-encoded string.
    Binary(String),
}

impl File {
    /// Creates a new file with text content.
    ///
    /// ## Arguments
    ///
    /// * `mount_point` - The path where the file will be mounted in the container
    /// * `content` - The text content of the file
    ///
    /// ## Returns
    ///
    /// A new `File` instance with text content.
    pub fn from_text<T: Into<String>>(mount_point: T, content: T) -> Self {
        Self {
            mount_point: mount_point.into(),
            content: FileContent::Text(content.into()),
        }
    }

    /// Creates a new file with binary content.
    ///
    /// ## Arguments
    ///
    /// * `mount_point` - The path where the file will be mounted in the container
    /// * `content` - The base64-encoded binary content of the file
    ///
    /// ## Returns
    ///
    /// A new `File` instance with binary content.
    pub fn from_binary<T: Into<String>>(mount_point: T, content: T) -> Self {
        Self {
            mount_point: mount_point.into(),
            content: FileContent::Binary(content.into()),
        }
    }

    /// Returns the mount point of the file.
    ///
    /// ## Returns
    ///
    /// A string slice containing the mount point path.
    #[must_use]
    pub fn mount_point(&self) -> &str {
        &self.mount_point
    }

    /// Returns the text content if the file contains text data.
    ///
    /// ## Returns
    ///
    /// `Some(&str)` if the file contains text data, `None` if it contains binary data.
    #[must_use]
    pub fn text_content(&self) -> Option<&str> {
        match &self.content {
            FileContent::Text(content) => Some(content),
            FileContent::Binary(_) => None,
        }
    }

    /// Returns the binary content if the file contains binary data.
    ///
    /// ## Returns
    ///
    /// `Some(&str)` if the file contains binary data, `None` if it contains text data.
    #[must_use]
    pub fn binary_content(&self) -> Option<&str> {
        match &self.content {
            FileContent::Binary(content) => Some(content),
            FileContent::Text(_) => None,
        }
    }

    /// Returns whether this file contains text content.
    ///
    /// ## Returns
    ///
    /// `true` if the file contains text content, `false` otherwise.
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self.content, FileContent::Text(_))
    }

    /// Returns whether this file contains binary content.
    ///
    /// ## Returns
    ///
    /// `true` if the file contains binary content, `false` otherwise.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        matches!(self.content, FileContent::Binary(_))
    }

    /// Converts the file to a Mapping representation for backward compatibility.
    ///
    /// ## Returns
    ///
    /// A [`serde_yaml::Mapping`] containing the file's mount point and content data.
    pub fn to_dict(&self) -> Mapping {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String(self.mount_point.clone()),
        );

        match &self.content {
            FileContent::Text(content) => {
                dict.insert(
                    Value::String(FILE_DATA_KEY.to_owned()),
                    Value::String(content.clone()),
                );
            }
            FileContent::Binary(content) => {
                dict.insert(
                    Value::String(FILE_BINARY_DATA_KEY.to_owned()),
                    Value::String(content.clone()),
                );
            }
        }

        dict
    }

    /// Creates a File from a Mapping representation for backward compatibility.
    ///
    /// ## Arguments
    ///
    /// * `dict` - A [`serde_yaml::Mapping`] containing the file data with mount_point and either data or binaryData keys
    ///
    /// ## Returns
    ///
    /// A new `File` instance created from the Mapping data.
    ///
    /// ## Errors
    ///
    /// Returns an [`AnkaiosError`] if:
    /// - The Mapping is missing the mount_point key
    /// - The Mapping contains both text and binary content
    /// - The Mapping contains neither text nor binary content
    pub fn from_dict(dict: &Mapping) -> Result<Self, AnkaiosError> {
        let mount_point = dict
            .get(Value::String(FILE_MOUNT_POINT_KEY.to_owned()))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AnkaiosError::WorkloadFieldError(
                    "file".to_owned(),
                    "Missing mount_point".to_owned(),
                )
            })?
            .to_owned();

        let has_text = dict.contains_key(Value::String(FILE_DATA_KEY.to_owned()));
        let has_binary = dict.contains_key(Value::String(FILE_BINARY_DATA_KEY.to_owned()));

        match (has_text, has_binary) {
            (true, false) => {
                let content = dict
                    .get(Value::String(FILE_DATA_KEY.to_owned()))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AnkaiosError::WorkloadFieldError(
                            "file".to_owned(),
                            "Invalid text content".to_owned(),
                        )
                    })?
                    .to_owned();
                Ok(Self::from_text(mount_point, content))
            }
            (false, true) => {
                let content = dict
                    .get(Value::String(FILE_BINARY_DATA_KEY.to_owned()))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AnkaiosError::WorkloadFieldError(
                            "file".to_owned(),
                            "Invalid binary content".to_owned(),
                        )
                    })?
                    .to_owned();
                Ok(Self::from_binary(mount_point, content))
            }
            (true, true) => Err(AnkaiosError::WorkloadFieldError(
                "file".to_owned(),
                "File cannot have both text and binary content".to_owned(),
            )),
            (false, false) => Err(AnkaiosError::WorkloadFieldError(
                "file".to_owned(),
                "File must have either text or binary content".to_owned(),
            )),
        }
    }

    #[doc(hidden)]
    /// Converts the [`File`]` to its protobuf representation.
    ///
    /// ## Returns
    ///
    /// An [`ank_base::File`] protobuf message containing the file's mount point and content.
    pub(crate) fn to_proto(&self) -> ank_base::File {
        let file_content = match &self.content {
            FileContent::Text(data) => Some(ank_base::file::FileContent::Data(data.to_owned())),
            FileContent::Binary(binary_data) => {
                Some(ank_base::file::FileContent::BinaryData(binary_data.to_owned()))
            }
        };

        ank_base::File {
            mount_point: self.mount_point.clone(),
            file_content,
        }
    }

    #[doc(hidden)]
    /// Converts an [`ank_base::File`] protobuf message to a [`File`] object.
    /// 
    /// ## Returns
    /// 
    /// A [`File`] object containing its mount point and content.
    pub(crate) fn from_proto(file: ank_base::File) -> Self {
        match file.file_content {
            Some(ank_base::file::FileContent::Data(data)) => File {
                mount_point: file.mount_point,
                content: FileContent::Text(data),
            },
            Some(ank_base::file::FileContent::BinaryData(binary_data)) => File {
                mount_point: file.mount_point,
                content: FileContent::Binary(binary_data),
            },
            None => {
                // This case is unreachable in reality as ank_base::File always contains either Data or BinaryData
                File {
                    mount_point: file.mount_point,
                    content: FileContent::Text(String::new()),
                }
            }
        }
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.content {
            FileContent::Text(content) => {
                write!(
                    f,
                    "File(text): {} -> {} ({} bytes)",
                    self.mount_point,
                    if content.len() > 50 {
                        format!("{}...", &content[..50])
                    } else {
                        content.clone()
                    },
                    content.len()
                )
            }
            FileContent::Binary(content) => {
                write!(
                    f,
                    "File(binary): {} -> base64 data ({} bytes)",
                    self.mount_point,
                    content.len()
                )
            }
        }
    }
}
