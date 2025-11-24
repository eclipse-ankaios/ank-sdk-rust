// Copyright (c) 2025 Elektrobit Automotive GmbH
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
use ankaios_api::ank_base;
use serde_yaml::{Mapping, Value};

/// Key name for mount point of workload file.
pub const FILE_MOUNT_POINT_KEY: &str = "mount_point";
/// Key name for data of workload file.
pub const FILE_DATA_KEY: &str = "data";
/// Key name for binary data of workload file.
pub const FILE_BINARY_DATA_KEY: &str = "binaryData";

/// Represents a file that can be mounted to a workload.
///
/// A `File` can contain either data or binary data (base64 encoded), but not both.
///
/// # Examples
///
/// ## Create a data file:
///
/// ```rust
/// use ankaios_sdk::File;
///
/// let data_file = File::from_data("/etc/config.txt", "Hello, World!");
/// ```
///
/// ## Create a binary file:
///
/// ```rust
/// use ankaios_sdk::File;
///
/// let binary_file = File::from_binary_data("/usr/share/app/binary_file", "iVBORw0KGgoARYANSUhEUgA...");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// The path where the file will be mounted in the container.
    pub mount_point: String,
    /// The content of the file.
    pub content: FileContent,
}

/// Represents the content type of a [`File`].
///
/// A file can contain either data or binary data (base64 encoded).
#[derive(Debug, Clone, PartialEq)]
pub enum FileContent {
    /// Data content stored as a UTF-8 string.
    Data(String),
    /// Binary content stored as a base64-encoded string.
    BinaryData(String),
}

impl File {
    /// Creates a new file with data content.
    ///
    /// ## Arguments
    ///
    /// * `mount_point` - The path where the file will be mounted in the container
    /// * `content` - The data content of the file
    ///
    /// ## Returns
    ///
    /// A new `File` instance with data content.
    pub fn from_data<T: Into<String>>(mount_point: T, content: T) -> Self {
        Self {
            mount_point: mount_point.into(),
            content: FileContent::Data(content.into()),
        }
    }

    /// Creates a new file with binary data content.
    ///
    /// ## Arguments
    ///
    /// * `mount_point` - The path where the file will be mounted in the container
    /// * `content` - The base64-encoded binary data content of the file
    ///
    /// ## Returns
    ///
    /// A new `File` instance with binary data content.
    pub fn from_binary_data<T: Into<String>>(mount_point: T, content: T) -> Self {
        Self {
            mount_point: mount_point.into(),
            content: FileContent::BinaryData(content.into()),
        }
    }

    /// Converts the file to a Mapping representation.
    ///
    /// ## Returns
    ///
    /// A [`serde_yaml::Mapping`] containing the file's mount point and content data.
    #[must_use]
    pub fn to_dict(&self) -> Mapping {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String(self.mount_point.clone()),
        );

        match &self.content {
            FileContent::Data(content) => {
                dict.insert(
                    Value::String(FILE_DATA_KEY.to_owned()),
                    Value::String(content.clone()),
                );
            }
            FileContent::BinaryData(content) => {
                dict.insert(
                    Value::String(FILE_BINARY_DATA_KEY.to_owned()),
                    Value::String(content.clone()),
                );
            }
        }

        dict
    }

    /// Creates a File from a Mapping representation.
    ///
    /// ## Arguments
    ///
    /// * `dict` - A [`serde_yaml::Mapping`] containing the file data with `mount_point` and either data or binaryData keys
    ///
    /// ## Returns
    ///
    /// A new `File` instance created from the Mapping data.
    ///
    /// ## Errors
    ///
    /// Returns an [`AnkaiosError`] if:
    /// - The Mapping is missing the `mount_point` key
    /// - The Mapping contains both data and binary data content
    /// - The Mapping contains neither data nor binary data content
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

        let has_data = dict.contains_key(Value::String(FILE_DATA_KEY.to_owned()));
        let has_binary_data = dict.contains_key(Value::String(FILE_BINARY_DATA_KEY.to_owned()));

        match (has_data, has_binary_data) {
            (true, false) => {
                let content = dict
                    .get(Value::String(FILE_DATA_KEY.to_owned()))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AnkaiosError::WorkloadFieldError(
                            "file".to_owned(),
                            "Invalid data content".to_owned(),
                        )
                    })?
                    .to_owned();
                Ok(Self::from_data(mount_point, content))
            }
            (false, true) => {
                let content = dict
                    .get(Value::String(FILE_BINARY_DATA_KEY.to_owned()))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AnkaiosError::WorkloadFieldError(
                            "file".to_owned(),
                            "Invalid binary data content".to_owned(),
                        )
                    })?
                    .to_owned();
                Ok(Self::from_binary_data(mount_point, content))
            }
            (true, true) => Err(AnkaiosError::WorkloadFieldError(
                "file".to_owned(),
                "File cannot have both data and binary data content".to_owned(),
            )),
            (false, false) => Err(AnkaiosError::WorkloadFieldError(
                "file".to_owned(),
                "File must have either data or binary data content".to_owned(),
            )),
        }
    }

    #[doc(hidden)]
    /// Converts the [`File`]` to its protobuf representation.
    ///
    /// ## Returns
    ///
    /// An [`ank_base`] protobuf message containing the file's mount point and content.
    pub(crate) fn into_proto(self) -> ank_base::File {
        let file_content = match self.content {
            FileContent::Data(data) => Some(ank_base::FileContent::Data(data)),
            FileContent::BinaryData(binary_data) => {
                Some(ank_base::FileContent::BinaryData(binary_data))
            }
        };

        ank_base::File {
            mount_point: self.mount_point,
            file_content,
        }
    }

    #[doc(hidden)]
    /// Converts an [`ank_base`] protobuf message to a [`File`] object.
    ///
    /// ## Returns
    ///
    /// A [`File`] object containing its mount point and content.
    pub(crate) fn from_proto(file: ank_base::File) -> Self {
        match file.file_content {
            Some(ank_base::FileContent::Data(data)) => File {
                mount_point: file.mount_point,
                content: FileContent::Data(data),
            },
            Some(ank_base::FileContent::BinaryData(binary_data)) => File {
                mount_point: file.mount_point,
                content: FileContent::BinaryData(binary_data),
            },
            None => {
                log::warn!(
                    "This case is unreachable in reality as ank_base always contains either Data or BinaryData"
                );
                File {
                    mount_point: file.mount_point,
                    content: FileContent::Data(String::new()),
                }
            }
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

#[cfg(test)]
mod tests {
    use super::{FILE_BINARY_DATA_KEY, FILE_DATA_KEY, FILE_MOUNT_POINT_KEY, File, FileContent};
    use crate::AnkaiosError;
    use ankaios_api::ank_base;
    use serde_yaml::{Mapping, Value};

    #[test]
    fn test_from_data() {
        let file = File::from_data("/etc/config.txt", "Hello, World!");

        assert_eq!(file.mount_point, "/etc/config.txt");
        assert_eq!(file.content, FileContent::Data("Hello, World!".to_owned()));
    }

    #[test]
    fn test_from_binary_data() {
        let base64_data = "iVBORw0KGgoAAAANSUhEUgA=";
        let file = File::from_binary_data("/usr/share/app/image.png", base64_data);

        assert_eq!(file.mount_point, "/usr/share/app/image.png");
        assert_eq!(
            file.content,
            FileContent::BinaryData(base64_data.to_owned())
        );
    }

    #[test]
    fn test_from_data_with_empty_content() {
        let file = File::from_data("/etc/empty.txt", "");

        assert_eq!(file.mount_point, "/etc/empty.txt");
        assert_eq!(file.content, FileContent::Data(String::new()));
    }

    #[test]
    fn test_from_binary_data_with_empty_content() {
        let file = File::from_binary_data("/usr/share/empty.bin", "");

        assert_eq!(file.mount_point, "/usr/share/empty.bin");
        assert_eq!(file.content, FileContent::BinaryData(String::new()));
    }

    #[test]
    fn test_data_file_to_dict() {
        let file = File::from_data("/etc/config.txt", "Hello, World!");
        let dict = file.to_dict();

        assert_eq!(
            dict.get(Value::String(FILE_MOUNT_POINT_KEY.to_owned())),
            Some(&Value::String("/etc/config.txt".to_owned()))
        );
        assert_eq!(
            dict.get(Value::String(FILE_DATA_KEY.to_owned())),
            Some(&Value::String("Hello, World!".to_owned()))
        );
        assert_eq!(
            dict.get(Value::String(FILE_BINARY_DATA_KEY.to_owned())),
            None
        );
    }

    #[test]
    fn test_binary_data_file_to_dict() {
        let base64_data = "iVBORw0KGgoAAAANSUhEUgA=";
        let file = File::from_binary_data("/usr/share/app/image.png", base64_data);
        let dict = file.to_dict();

        assert_eq!(
            dict.get(Value::String(FILE_MOUNT_POINT_KEY.to_owned())),
            Some(&Value::String("/usr/share/app/image.png".to_owned()))
        );
        assert_eq!(dict.get(Value::String(FILE_DATA_KEY.to_owned())), None);
        assert_eq!(
            dict.get(Value::String(FILE_BINARY_DATA_KEY.to_owned())),
            Some(&Value::String(base64_data.to_owned()))
        );
    }

    #[test]
    fn test_from_dict_with_data_content() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String("/etc/config.txt".to_owned()),
        );
        dict.insert(
            Value::String(FILE_DATA_KEY.to_owned()),
            Value::String("Hello, World!".to_owned()),
        );

        let file = File::from_dict(&dict).unwrap();

        assert_eq!(file.mount_point, "/etc/config.txt");
        assert_eq!(file.content, FileContent::Data("Hello, World!".to_owned()));
    }

    #[test]
    fn test_from_dict_with_binary_data_content() {
        let base64_data = "iVBORw0KGgoATMANSUhEUgA=";
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String("/usr/share/app/image.png".to_owned()),
        );
        dict.insert(
            Value::String(FILE_BINARY_DATA_KEY.to_owned()),
            Value::String(base64_data.to_owned()),
        );

        let file = File::from_dict(&dict).unwrap();

        assert_eq!(file.mount_point, "/usr/share/app/image.png");
        assert_eq!(
            file.content,
            FileContent::BinaryData(base64_data.to_owned())
        );
    }

    #[test]
    fn test_from_dict_missing_mount_point() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_DATA_KEY.to_owned()),
            Value::String("Hello, World!".to_owned()),
        );

        let result = File::from_dict(&dict);

        assert!(result.is_err());
        match result.unwrap_err() {
            AnkaiosError::WorkloadFieldError(field, message) => {
                assert_eq!(field, "file");
                assert_eq!(message, "Missing mount_point");
            }
            _ => panic!("Expected WorkloadFieldError"),
        }
    }

    #[test]
    fn test_from_dict_no_content() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String("/etc/config.txt".to_owned()),
        );

        let result = File::from_dict(&dict);

        assert!(result.is_err());
        match result.unwrap_err() {
            AnkaiosError::WorkloadFieldError(field, message) => {
                assert_eq!(field, "file");
                assert_eq!(message, "File must have either data or binary data content");
            }
            _ => panic!("Expected WorkloadFieldError"),
        }
    }

    #[test]
    fn test_from_dict_both_data_and_binary_data_content() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String("/etc/config.txt".to_owned()),
        );
        dict.insert(
            Value::String(FILE_DATA_KEY.to_owned()),
            Value::String("Hello, World!".to_owned()),
        );
        dict.insert(
            Value::String(FILE_BINARY_DATA_KEY.to_owned()),
            Value::String("iVBORw0KGgoATMANSUhEUgA=".to_owned()),
        );

        let result = File::from_dict(&dict);

        assert!(result.is_err());
        match result.unwrap_err() {
            AnkaiosError::WorkloadFieldError(field, message) => {
                assert_eq!(field, "file");
                assert_eq!(
                    message,
                    "File cannot have both data and binary data content"
                );
            }
            _ => panic!("Expected WorkloadFieldError"),
        }
    }

    #[test]
    fn test_from_dict_invalid_mount_point_type() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::Number(42.into()),
        );
        dict.insert(
            Value::String(FILE_DATA_KEY.to_owned()),
            Value::String("Hello, World!".to_owned()),
        );

        let result = File::from_dict(&dict);

        assert!(result.is_err());
        match result.unwrap_err() {
            AnkaiosError::WorkloadFieldError(field, message) => {
                assert_eq!(field, "file");
                assert_eq!(message, "Missing mount_point");
            }
            _ => panic!("Expected WorkloadFieldError"),
        }
    }

    #[test]
    fn test_from_dict_invalid_data_content_type() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String("/etc/config.txt".to_owned()),
        );
        dict.insert(
            Value::String(FILE_DATA_KEY.to_owned()),
            Value::Number(42.into()),
        );

        let result = File::from_dict(&dict);

        assert!(result.is_err());
        match result.unwrap_err() {
            AnkaiosError::WorkloadFieldError(field, message) => {
                assert_eq!(field, "file");
                assert_eq!(message, "Invalid data content");
            }
            _ => panic!("Expected WorkloadFieldError"),
        }
    }

    #[test]
    fn test_from_dict_invalid_binary_data_content_type() {
        let mut dict = Mapping::new();
        dict.insert(
            Value::String(FILE_MOUNT_POINT_KEY.to_owned()),
            Value::String("/usr/share/app/image.png".to_owned()),
        );
        dict.insert(
            Value::String(FILE_BINARY_DATA_KEY.to_owned()),
            Value::Number(42.into()),
        );

        let result = File::from_dict(&dict);

        assert!(result.is_err());
        match result.unwrap_err() {
            AnkaiosError::WorkloadFieldError(field, message) => {
                assert_eq!(field, "file");
                assert_eq!(message, "Invalid binary data content");
            }
            _ => panic!("Expected WorkloadFieldError"),
        }
    }

    #[test]
    fn test_data_file_round_trip_dict() {
        let original_file = File::from_data("/etc/config.txt", "Hello, World!");
        let dict = original_file.to_dict();
        let restored_file = File::from_dict(&dict).unwrap();

        assert_eq!(original_file, restored_file);
    }

    #[test]
    fn test_binary_data_file_round_trip_dict() {
        let base64_data = "iVBORw0KGgoATMANSUhEUgA=";
        let original_file = File::from_binary_data("/usr/share/app/image.png", base64_data);
        let dict = original_file.to_dict();
        let restored_file = File::from_dict(&dict).unwrap();

        assert_eq!(original_file, restored_file);
    }

    #[test]
    fn test_to_proto_data_file() {
        let file = File::from_data("/etc/config.txt", "Hello, World!");
        let proto = file.into_proto();

        assert_eq!(proto.mount_point, "/etc/config.txt");
        match proto.file_content {
            Some(ank_base::FileContent::Data(content)) => {
                assert_eq!(content, "Hello, World!");
            }
            _ => panic!("Expected data content in proto"),
        }
    }

    #[test]
    fn test_to_proto_binary_data_file() {
        let base64_data = "iVBORw0KGgoATMANSUhEUgA=";
        let file = File::from_binary_data("/usr/share/app/image.png", base64_data);
        let proto = file.into_proto();

        assert_eq!(proto.mount_point, "/usr/share/app/image.png");
        match proto.file_content {
            Some(ank_base::FileContent::BinaryData(content)) => {
                assert_eq!(content, base64_data);
            }
            _ => panic!("Expected binary data content in proto"),
        }
    }

    #[test]
    fn test_from_proto_data_file() {
        let proto = ank_base::File {
            mount_point: "/etc/config.txt".to_owned(),
            file_content: Some(ank_base::FileContent::Data("Hello, World!".to_owned())),
        };

        let file = File::from_proto(proto);

        assert_eq!(file.mount_point, "/etc/config.txt");
        assert_eq!(file.content, FileContent::Data("Hello, World!".to_owned()));
    }

    #[test]
    fn test_from_proto_binary_data_file() {
        let base64_data = "iVBORw0KGgoATMANSUhEUgA=";
        let proto = ank_base::File {
            mount_point: "/usr/share/app/image.png".to_owned(),
            file_content: Some(ank_base::FileContent::BinaryData(base64_data.to_owned())),
        };

        let file = File::from_proto(proto);

        assert_eq!(file.mount_point, "/usr/share/app/image.png");
        assert_eq!(
            file.content,
            FileContent::BinaryData(base64_data.to_owned())
        );
    }

    #[test]
    fn test_round_trip_proto_data_file() {
        let original_file = File::from_data("/etc/config.txt", "Hello, World!");
        let proto = original_file.clone().into_proto();
        let restored_file = File::from_proto(proto);

        assert_eq!(original_file, restored_file);
    }

    #[test]
    fn test_round_trip_proto_binary_data_file() {
        let base64_data = "iVBORw0KGgoATMANSUhEUgA=";
        let original_file = File::from_binary_data("/usr/share/app/image.png", base64_data);
        let proto = original_file.clone().into_proto();
        let restored_file = File::from_proto(proto);

        assert_eq!(original_file, restored_file);
    }

    #[test]
    fn test_clone() {
        let file = File::from_data("/etc/config.txt", "Hello, World!");
        let cloned_file = file.clone();

        assert_eq!(file, cloned_file);
        assert_eq!(file.mount_point, cloned_file.mount_point);
        assert_eq!(file.content, cloned_file.content);
    }

    #[test]
    fn test_file_content_equality() {
        let data_content1 = FileContent::Data("Hello".to_owned());
        let data_content2 = FileContent::Data("Hello".to_owned());
        let data_content3 = FileContent::Data("World".to_owned());
        let binary_data_content = FileContent::BinaryData("base64data".to_owned());

        assert_eq!(data_content1, data_content2);
        assert_ne!(data_content1, data_content3);
        assert_ne!(data_content1, binary_data_content);
    }
}
