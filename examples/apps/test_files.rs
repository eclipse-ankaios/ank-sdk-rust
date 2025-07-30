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

use ankaios_sdk::{Ankaios, File, Workload};
use tokio::time::Duration;

async fn print_workload_states(ank: &mut Ankaios) {
    if let Ok(workload_states) = ank.get_workload_states().await {
        for workload_state in workload_states.as_list() {
            println!(
                "Workload {} on agent {} has the state {:?}",
                workload_state.workload_instance_name.workload_name,
                workload_state.workload_instance_name.agent_name,
                workload_state.execution_state.state
            );
        }
    }
}

#[tokio::main]
async fn main() {
    // Create a new Ankaios object.
    // The connection to the control interface is automatically done at this step.
    println!("PLEASE PROVIDE VALID FILES' PATHS IF YOU WANT TO FULLY USE THIS EXAMPLE");

    let mut ank = Ankaios::new().await.expect("Failed to initialize");

    // Create a workload with text file
    let workload_with_text_file = Workload::builder()
        .workload_name("nginx_with_text_file")
        .agent_name("agent_Rust_SDK")
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config(
            "image: docker.io/library/nginx:latest\ncommandOptions: [\"-p\", \"8081:80\"]"
        )
        .add_file(File::from_data("/usr/share/nginx/html/index.html", "<html><body><h1>Hello from Ankaios with text file!</h1></body></html>"))
        .add_file(File::from_data("/etc/nginx/conf.d/custom.conf", "server {\n    listen 80;\n    server_name localhost;\n    location / {\n        root /usr/share/nginx/html;\n        index index.html;\n    }\n}"))
        .build().expect("Failed to build workload with text file");

    // Create a workload with binary file (base64 encoded)
    let workload_with_binary_file = Workload::builder()
        .workload_name("nginx_with_binary_file")
        .agent_name("agent_Rust_SDK")
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config(
            "image: docker.io/library/nginx:latest\ncommandOptions: [\"-p\", \"8082:80\"]"
        )
        .add_file(File::from_binary_data("/usr/share/nginx/html/favicon.ico", "AAABAAEAEBAAAAEAIABoBAAAFgAAACgAAAAQAAAAIAAAAAEAIAAAAAAAAAQAABILAAASCwAAAAAAAAAAAAD///8A////AP///wD///8A////AP///wD///8A////AP///wD///8A////AP///wD///8A////AP///wD///8A"))
        .build().expect("Failed to build workload with binary file");

    println!("Testing workload files functionality...");

    // Send first workload with text files to Ankaios
    println!("Applying workload with text files...");
    ank.apply_workload(workload_with_text_file)
        .await
        .expect("Failed to apply workload with text files");

    // Wait and check states
    tokio::time::sleep(Duration::from_secs(5)).await;
    print_workload_states(&mut ank).await;

    // Send second workload with binary file to Ankaios
    println!("Applying workload with binary file...");
    ank.apply_workload(workload_with_binary_file)
        .await
        .expect("Failed to apply workload with binary file");

    // Wait and check states
    tokio::time::sleep(Duration::from_secs(5)).await;
    print_workload_states(&mut ank).await;

    // Test file manipulation - create a workload and then update its files
    println!("Creating workload with initial file...");
    let mut dynamic_workload = Workload::builder()
        .workload_name("dynamic_file_workload")
        .agent_name("agent_Rust_SDK")
        .runtime("podman")
        .restart_policy("NEVER")
        .runtime_config(
            "image: docker.io/library/nginx:latest\ncommandOptions: [\"-p\", \"8083:80\"]",
        )
        .build()
        .expect("Failed to build dynamic workload");

    // Add initial file
    dynamic_workload
        .add_file(File::from_data(
            "/usr/share/nginx/html/index.html",
            "<html><body><h1>Initial content</h1></body></html>",
        ));

    ank.apply_workload(dynamic_workload.clone())
        .await
        .expect("Failed to apply dynamic workload");

    tokio::time::sleep(Duration::from_secs(5)).await;
    print_workload_states(&mut ank).await;

    // Update the workload with additional files
    println!("Updating workload with additional files...");
    dynamic_workload.update_files(vec![File::from_data("/usr/share/nginx/html/about.html", "<html><body><h1>About page</h1><p>This is a dynamically added file!</p></body></html>"), File::from_data("/usr/share/nginx/html/config.json", "{\"version\": \"1.0\", \"environment\": \"test\", \"features\": [\"files\", \"dynamic_updates\"]}")]);

    ank.apply_workload(dynamic_workload)
        .await
        .expect("Failed to update dynamic workload");

    tokio::time::sleep(Duration::from_secs(5)).await;
    print_workload_states(&mut ank).await;

    // Test retrieving and displaying file information
    println!("Retrieving file information from workloads...");
    if let Ok(complete_state) = ank
        .get_state(vec!["desiredState.workloads".to_owned()])
        .await
    {
        let workloads = complete_state.get_workloads();

        for workload in workloads {
            println!(
                "The following files are associated with workload {:?}:",
                workload.name
            );
            let wl_files = workload.get_files();
            for file in wl_files {
                println!("{:?}", file);
            }
        }
    }

    // Clean up - delete workloads
    println!("\nCleaning up workloads...");
    ank.delete_workload("nginx_with_text_file".to_owned())
        .await
        .expect("Failed to delete workload with text file");
    ank.delete_workload("nginx_with_binary_file".to_owned())
        .await
        .expect("Failed to delete workload with binary file");
    ank.delete_workload("dynamic_file_workload".to_owned())
        .await
        .expect("Failed to delete dynamic workload");

    // Final state check
    tokio::time::sleep(Duration::from_secs(5)).await;
    print_workload_states(&mut ank).await;
}
