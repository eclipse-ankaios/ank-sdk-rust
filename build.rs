// Copyright (c) 2023 Elektrobit Automotive GmbH
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

#![allow(clippy::too_many_lines)] // This file will be refactored in the ankaios_api, no need to fix this lint here
fn main() {
    let builder = tonic_prost_build::configure()
        .build_server(true)
        .boxed("Request.RequestContent.updateStateRequest")
        .boxed("FromAnkaios.FromAnkaiosEnum.response")
        .boxed("Response.ResponseContent.completeStateResponse")
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .message_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .type_attribute("WorkloadState", "#[allow(dead_code)]") // Workaround until the release of the ankaios api
        .enum_attribute(
            "AddCondition",
            "#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]",
        )
        .enum_attribute(
            "RestartPolicy",
            "#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]",
        )
        .field_attribute("ReadWriteEnum.RW_NOTHING", "#[serde(rename = \"Nothing\")]")
        .field_attribute("ReadWriteEnum.RW_READ", "#[serde(rename = \"Read\")]")
        .field_attribute("ReadWriteEnum.RW_WRITE", "#[serde(rename = \"Write\")]")
        .field_attribute(
            "ReadWriteEnum.RW_READ_WRITE",
            "#[serde(rename = \"ReadWrite\")]",
        )
        .message_attribute("ank_base.ConfigItem", "#[serde(transparent)]")
        .message_attribute("ank_base.ConfigArray", "#[serde(transparent)]")
        .message_attribute("ank_base.ConfigObject", "#[serde(transparent)]")
        .enum_attribute("ConfigItemEnum", "#[serde(untagged)]")
        .enum_attribute(
            "ExecutionStateEnum",
            "#[serde(tag = \"state\", content = \"subState\")]",
        )
        .message_attribute("Tags", "#[derive(Eq)]")
        .message_attribute("AgentAttributes", "#[derive(Eq)]")
        .message_attribute("AgentMap", "#[derive(Eq)]")


        .field_attribute("CompleteState.desiredState", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("CompleteState.workloadStates", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("CompleteState.agents", "#[serde(skip_serializing_if = \"Option::is_none\")]")

        .field_attribute("State.workloads", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("State.configs", "#[serde(skip_serializing_if = \"Option::is_none\")]")

        .field_attribute("Workload.agent", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("Workload.restartPolicy", "#[serde(default, skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("Workload.runtime", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("Workload.runtimeConfig", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("Workload.files", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("Workload.controlInterfaceAccess", "#[serde(skip_serializing_if = \"Option::is_none\")]")

        .field_attribute("Workload.tags", "#[serde(flatten)]")
        .field_attribute("Workload.configs", "#[serde(flatten)]")
        .field_attribute("Workload.dependencies", "#[serde(flatten)]")
        .field_attribute("Workload.files", "#[serde(flatten)]")
        .field_attribute("WorkloadStatesMap.agentStateMap", "#[serde(flatten)]")
        .field_attribute(
            "ExecutionsStatesOfWorkload.wlNameStateMap",
            "#[serde(flatten)]",
        )
        .field_attribute("ExecutionsStatesForId.idStateMap", "#[serde(flatten)]")
        .field_attribute("ExecutionState.additionalInfo", "#[serde(skip_serializing_if = \"Option::is_none\")]")
        .field_attribute("ExecutionState.ExecutionStateEnum", "#[serde(flatten)]")
        .field_attribute("WorkloadMap.workloads", "#[serde(flatten)]")
        .field_attribute("AgentMap.agents", "#[serde(flatten)]")
        .field_attribute("ConfigMap.configs", "#[serde(flatten)]")
        .field_attribute(
            "ControlInterfaceAccess.allowRules",
            "#[serde(default, with = \"serde_yaml::with::singleton_map_recursive\", skip_serializing_if = \"Vec::is_empty\")]",
        )
        .field_attribute(
            "ControlInterfaceAccess.denyRules",
            "#[serde(default, with = \"serde_yaml::with::singleton_map_recursive\", skip_serializing_if = \"Vec::is_empty\")]",
        )
        .field_attribute(
            "Files.files",
            "#[serde(default, skip_serializing_if = \"Vec::is_empty\")]",
        )
        // Yes, this is not a map, but this is the only way to get the desired serialization behavior without ! in the YAML and a custom serializer
        .field_attribute(
            "Files.files",
            "#[serde(with = \"serde_yaml::with::singleton_map_recursive\")]",
        )
        .field_attribute(
            "AgentStatus.cpu_usage",
            "#[serde(skip_serializing_if = \"::core::option::Option::is_none\")]",
        )
        .field_attribute("AgentStatus.cpu_usage", "#[serde(flatten)]")
        .field_attribute(
            "AgentStatus.free_memory",
            "#[serde(skip_serializing_if = \"::core::option::Option::is_none\")]",
        )
        .field_attribute("AgentStatus.free_memory", "#[serde(flatten)]")
        .field_attribute(
            "AgentAttributes.status",
            "#[serde(skip_serializing_if = \"::core::option::Option::is_none\")]",
        )
        .field_attribute(
            "AgentAttributes.tags",
            "#[serde(skip_serializing_if = \"::core::option::Option::is_none\")]",
        )
        .field_attribute(
            "AgentMap.agents",
            "#[serde(skip_serializing_if = \"::std::collections::HashMap::is_empty\")]",
        )
        .field_attribute(
            "AgentMap.agents",
            "#[serde(default, serialize_with = \"serialize_to_ordered_map\")]",
        );

    builder
        .compile_protos(&["proto/control_api.proto"], &["proto"])
        .unwrap();
}
