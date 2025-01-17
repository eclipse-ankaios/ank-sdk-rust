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

use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Error, Debug)]
pub enum AnkaiosError{
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Timeout error: {0}")]
    TimeoutError(#[from] Elapsed),

    #[error("Invalid value for field {0}: {1}.")]
    WorkloadFieldError(String, String),
    #[error("Workload builder error: {0}")]
    WorkloadBuilderError(&'static str),
    #[error("Invalid manifest: {0}")]
    InvalidManifestError(String),
    #[error("Connection closed: {0}")]
    ConnectionClosedError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Response error: {0}")]
    ResponseError(String),
    #[error("Control interface error: {0}")]
    ControlInterfaceError(String),
    #[error("Ankaios error: {0}")]
    AnkaiosError(String)
}
