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

use ankaios_sdk::Ankaios;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    {
        println!("Ankaios 1");
        let mut _ank = Ankaios::new().await.expect("Failed to initialize");
        sleep(Duration::from_secs(5)).await;
    }
    println!("Pause");
    sleep(Duration::from_secs(5)).await;
    {
        println!("Ankaios 2");
        let mut _ank = Ankaios::new().await.expect("Failed to initialize");
        sleep(Duration::from_secs(5)).await;
    }
    println!("End");
}
