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
use serde_yaml::Value;

/// Helper struct that contains information about a Workload instance.
/// 
/// # Example
/// 
/// ## Create a Workload Instance Name object
/// 
/// ```rust
/// let workload_instance_name = WorkloadInstanceName::new(
///     "agent_Test".to_owned(),
///     "workload_Test".to_owned(),
///     "1234".to_owned()
/// );
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WorkloadInstanceName{
    /// The name of the agent.
    pub agent_name: String,
    /// The name of the workload.
    pub workload_name: String,
    /// The id of the workload.
    pub workload_id: String,
}

impl WorkloadInstanceName {
    /// Creates a new `WorkloadInstanceName` object.
    /// 
    /// ## Arguments
    /// 
    /// * `agent_name` - A [String] containing the name of the agent;
    /// * `workload_name` - A [String] containing the name of the workload;
    /// * `workload_id` - A [String] containing the id of the workload.
    /// 
    /// ## Returns
    /// 
    /// A new [`WorkloadInstanceName`] object.
    pub fn new(agent_name: String, workload_name: String, workload_id: String) -> WorkloadInstanceName {
        WorkloadInstanceName {
            agent_name,
            workload_name,
            workload_id,
        }
    }

    /// Converts the `WorkloadInstanceName` to a [Mapping](serde_yaml::Mapping).
    /// 
    /// ## Returns
    /// 
    /// A [Mapping](serde_yaml::Mapping) containing the `WorkloadInstanceName` information.
    pub fn to_dict(&self) -> serde_yaml::Mapping {
        let mut map = serde_yaml::Mapping::new();
        map.insert(Value::String("agent_name".to_owned()), Value::String(self.agent_name.clone()));
        map.insert(Value::String("workload_name".to_owned()), Value::String(self.workload_name.clone()));
        map.insert(Value::String("workload_id".to_owned()), Value::String(self.workload_id.clone()));
        map
    }

    /// Returns the filter mask of the Workload Instance Name.
    /// 
    /// ## Returns
    /// 
    /// A [String] that represents the filter mask.
    pub fn get_filter_mask(&self) -> String {
        format!("workloadStates.{}.{}.{}", self.agent_name, self.workload_name, self.workload_id)
    }
}

impl fmt::Display for WorkloadInstanceName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.workload_name, self.workload_id, self.agent_name)
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
    use serde_yaml::Value;
    use super::WorkloadInstanceName;

    #[test]
    fn utest_instance_name() {
        let instance_name = WorkloadInstanceName::new(
            "agent_Test".to_owned(), "workload_Test".to_owned(), "1234".to_owned()
        );
        assert_eq!(instance_name.agent_name, "agent_Test");
        assert_eq!(instance_name.workload_name, "workload_Test");
        assert_eq!(instance_name.workload_id, "1234");

        assert_eq!(instance_name.to_string(), "workload_Test.1234.agent_Test");
        assert_eq!(instance_name.get_filter_mask(), "workloadStates.agent_Test.workload_Test.1234");
        assert_eq!(instance_name.to_dict(), serde_yaml::Mapping::from_iter([
            (Value::String("agent_name".to_owned()), Value::String("agent_Test".to_owned())),
            (Value::String("workload_name".to_owned()), Value::String("workload_Test".to_owned())),
            (Value::String("workload_id".to_owned()), Value::String("1234".to_owned())),
        ]));

        let mut another_instance_name = WorkloadInstanceName::new(
            "agent_Test".to_owned(), "workload_Test".to_owned(), "1234".to_owned()
        );
        assert_eq!(instance_name, another_instance_name);
        "agent_Test2".clone_into(&mut another_instance_name.agent_name);
        assert_ne!(instance_name, another_instance_name);
    }
}
