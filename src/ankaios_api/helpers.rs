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

use crate::ankaios_api::ank_base::ControlInterfaceAccess;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};

pub fn serialize_to_ordered_map<S, T: Serialize>(
    value: &HashMap<String, T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

#[allow(clippy::ref_option)] // &Option<Type> is used by the serde::Serialize trait for optional fields, and this must match
pub fn should_skip_control_interface_serialization(
    control_interface_access: &Option<ControlInterfaceAccess>,
) -> bool {
    match control_interface_access {
        None => true,
        Some(access) => access.allow_rules.is_empty() && access.deny_rules.is_empty(),
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
    use super::*;
    use crate::components::workload_mod::test_helpers::generate_test_control_interface_access;

    #[test]
    fn utest_serialize_to_ordered_map() {
        let mut serializer = serde_yaml::Serializer::new(Vec::new());

        let mut map = HashMap::new();
        map.insert("b".to_string(), 2);
        map.insert("a".to_string(), 1);
        map.insert("c".to_string(), 3);

        let result = serialize_to_ordered_map(&map, &mut serializer);
        assert!(result.is_ok());
        assert_eq!(serializer.into_inner().unwrap(), b"a: 1\nb: 2\nc: 3\n");
    }

    #[test]
    fn utest_should_skip_control_interface_serialization() {
        let empty_access: Option<ControlInterfaceAccess> = None;
        assert!(should_skip_control_interface_serialization(&empty_access));

        let access_with_no_rules = Some(ControlInterfaceAccess {
            allow_rules: vec![],
            deny_rules: vec![],
        });
        assert!(should_skip_control_interface_serialization(
            &access_with_no_rules
        ));

        let mut access_with_allow_rules = generate_test_control_interface_access();
        access_with_allow_rules.deny_rules = vec![];

        assert!(!should_skip_control_interface_serialization(&Some(
            access_with_allow_rules
        )));

        let mut access_with_deny_rules = generate_test_control_interface_access();
        access_with_deny_rules.allow_rules = vec![];
        assert!(!should_skip_control_interface_serialization(&Some(
            access_with_deny_rules
        )));
    }
}
