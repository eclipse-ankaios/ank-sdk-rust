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

//! This module contains the definition of the `AnkaiosError` enum, which
//! represents the different types of errors that can occur in the [Ankaios]
//! application.
//! 
//! [Ankaios]: https://eclipse-ankaios.github.io/ankaios

use std::io;
use thiserror::Error;
use tokio::time::error::Elapsed;

/// An enumeration of possible errors that can occur in the Ankaios application.
///
/// This enum uses the `thiserror::Error` derive macro to automatically generate
/// implementations for the `std::error::Error` trait. Each variant represents a
/// different type of error that can occur, with associated data providing more
/// context about the error.
#[derive(Error, Debug)]
pub enum AnkaiosError{
    /// Represents an I/O error, wrapping a `std::io::Error`.
    #[error("IO Error: {0}")]
    IoError(#[from] io::Error),
    /// Represents a timeout error, wrapping a `tokio::time::error::Elapsed`.
    #[error("Timeout error: {0}")]
    TimeoutError(#[from] Elapsed),

    /// Represents an error related to an invalid value for a workload field.
    #[error("Invalid value for field {0}: {1}.")]
    WorkloadFieldError(String, String),
    /// Represents an error that occurs during the building of a workload.
    #[error("Workload builder error: {0}")]
    WorkloadBuilderError(&'static str),
    /// Represents an error that occurs when the manifest can't be parsed.
    #[error("Manifest parsing error: {0}")]
    ManifestParsingError(String),
    /// Represents an error that occurs when the connection is closed with Ankaios.
    #[error("Connection closed: {0}")]
    ConnectionClosedError(String),
    /// Represents an error related to the creation of a request.
    #[error("Request error: {0}")]
    RequestError(String),
    /// Represents an error that occurs when the response is invalid.
    #[error("Response error: {0}")]
    ResponseError(String),
    /// Represents an error related to the connection with the control interface.
    #[error("Control interface error: {0}")]
    ControlInterfaceError(String),
    /// Represents an error returned by the server in response to a distinct request.
    /// e.g. due to insufficient reading rights by the requester.
    #[error("Ankaios error: {0}")]
    AnkaiosError(String)
}
