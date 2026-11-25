// Copyright (c) 2026 Elektrobit Automotive GmbH
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

mod basic_test;
mod common;
mod connection_lifecycle_test;
mod logs_events_test;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut args = std::env::args().skip(1);
    let example = args.next().unwrap_or_default();
    let server_url = args
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:25551".to_owned());
    let agent_name = args.next().unwrap_or_else(|| "agent_A".to_owned());

    match example.as_str() {
        "basic_test" => basic_test::run(&server_url, &agent_name).await,
        "logs_events_test" => logs_events_test::run(&server_url, &agent_name).await,
        "connection_lifecycle_test" => {
            connection_lifecycle_test::run(&server_url, &agent_name).await;
        }
        _ => {
            eprintln!(
                "Usage: test_app <example> [server_url] [agent_name]\n\n\
                 Examples:\n  basic_test\n  logs_events_test\n  connection_lifecycle_test"
            );
            std::process::exit(2);
        }
    }
}
