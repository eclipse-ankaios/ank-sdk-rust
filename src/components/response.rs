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

use crate::AnkaiosError;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ResponseType {
    Error = 0,
    CompleteState = 1,
    UpdateStateSuccess = 2,
}

pub struct Response{
    // TODO
}

// ResponseEvent?

pub struct UpdateStateSuccess{
    // TODO
}

impl std::fmt::Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res_str = match self {
            ResponseType::Error => "Error",
            ResponseType::CompleteState => "CompleteState",
            ResponseType::UpdateStateSuccess => "UpdateStateSuccess",
        };
        write!(f, "{}", res_str)
    }
}

impl Response {
    pub fn new() -> Self {
        Self{}
    }

    pub fn print(&self) {
        println!("I need to be implemented!!");
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateStateSuccess {
    pub fn new() -> Self {
        Self{}
    }

    pub fn print(&self) {
        println!("I need to be implemented!!");
    }
}

impl Default for UpdateStateSuccess {
    fn default() -> Self {
        Self::new()
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
    use super::{Response, UpdateStateSuccess};

    #[test]
    fn test_response() {
        let _ = Response::new();
    }

    #[test]
    fn test_update_state_success() {
        let _ = UpdateStateSuccess::new();
    }
}