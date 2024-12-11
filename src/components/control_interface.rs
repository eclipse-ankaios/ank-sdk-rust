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
pub enum ControlInterfaceState {
    Initialized = 0,
    Terminated = 1,
    AgentDisconnected = 2,
    ConnectionClosed = 3,
}

pub struct ControlInterface{
    // TODO
}

impl std::fmt::Display for ControlInterfaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res_str = match self {
            ControlInterfaceState::Initialized => "Initialized",
            ControlInterfaceState::Terminated => "Terminated",
            ControlInterfaceState::AgentDisconnected => "AgentDisconnected",
            ControlInterfaceState::ConnectionClosed => "ConnectionClosed",
        };
        write!(f, "{}", res_str)
    }
}

impl ControlInterface {
    pub fn new() -> Self {
        Self{}
    }

    pub fn print(&self) {
        println!("I need to be implemented!!");
    }
}

impl Default for ControlInterface {
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
    use super::ControlInterface;

    #[test]
    fn test_control_interface() {
        let _ = ControlInterface::new();
    }
}