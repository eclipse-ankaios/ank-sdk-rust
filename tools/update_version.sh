#!/bin/bash

# Copyright (c) 2025 Elektrobit Automotive GmbH
#
# This program and the accompanying materials are made available under the
# terms of the Apache License, Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0.
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations
# under the License.
#
# SPDX-License-Identifier: Apache-2.0

set -e

script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
base_dir="$script_dir/.."
sdk_version=""
ankaios_version=""
api_version=""
release=false

usage() {
    echo "Usage: $0 [--sdk <VERSION>] [--ank <VERSION>] [--api <VERSION>] [--help]"
    echo "Update the SDK, Ankaios and API versions."
    echo "You can update all of them at once or one by one."
    echo "  --release          Set if the update is for a release version."
    echo "  --sdk <VERSION>    The new version of the SDK."
    echo "  --ank <VERSION>    The new version of Ankaios."
    echo "  --api <VERSION>    The new version for the supported API."
    echo "  --help             Display this help message and exit."
    echo ""
    echo "Example:"
    echo "  $0 --sdk 0.1.0 --ank 0.1.0 --api v0.1"
    exit 1
}

parse_arguments() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --release)
                release=true   
                ;;
            --sdk)
                shift
                sdk_version="$1"
                ;;
            --ank)
                shift
                ankaios_version="$1"
                ;;
            --api)
                shift
                api_version="$1"
                ;;
            --help|-h)
                usage
                ;;
            *)
                echo "Unknown argument: $1"
                usage
                ;;
        esac
        shift
    done
}

if [ "$#" -eq 0 ]; then
    usage
fi

parse_arguments "$@"

if [ -n "$release" ] && [ -z "$sdk_version" ]; then
    echo "Release mode is set, but no SDK version specified. Please provide a version with --sdk."
    usage
fi

if [ -z "$sdk_version" ] && [ -z "$ankaios_version" ] && [ -z "$api_version" ]; then
    echo "You must specify at least one version to update."
    usage
fi

if [ -n "$sdk_version" ]; then
    echo "Updating SDK version to $sdk_version"
    sed -i "s|^version = .*|version = \"$sdk_version\"|" "$base_dir"/Cargo.toml

    if [ "$release" = true ]; then  
        sed -i "s|documentation = \"https://docs.rs/ankaios-sdk/.*|documentation = \"https://docs.rs/ankaios-sdk/$sdk_version\"|" "$base_dir"/Cargo.toml
        
        sed -i "s|\(\[!\[Docs\.rs\](https://img\.shields\.io/badge/docs\.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs\.rs)\](https://docs\.rs/ankaios-sdk/\)[^)]*|\1$sdk_version|" "$base_dir"/README.md
        sed -i "s|\* \[Rust SDK documentation\](https://docs\.rs/ankaios-sdk/[^)]*)|* [Rust SDK documentation](https://docs.rs/ankaios-sdk/$sdk_version)|" "$base_dir"/README.md
        sed -i "s|ankaios_sdk = \"[^\"]*\"|ankaios_sdk = \"$sdk_version\"|" "$base_dir"/README.md
        
        sed -i "s|#!\[doc(html_root_url = \"https://docs\.rs/ankaios_sdk/[^\"]*\")\]|#![doc(html_root_url = \"https://docs.rs/ankaios_sdk/$sdk_version\")]|" "$base_dir"/src/lib.rs
        sed -i "s|//! \[!\[docs-rs\]\](https://docs\.rs/ankaios-sdk/[^)]*)|//! [![docs-rs]](https://docs.rs/ankaios-sdk/$sdk_version)|" "$base_dir"/src/lib.rs
        sed -i "s|//! ankaios_sdk = \"[^\"]*\"|//! ankaios_sdk = \"$sdk_version\"|" "$base_dir"/src/lib.rs
        sed -i "s|//! \* \[Rust SDK documentation\](https://docs\.rs/ankaios-sdk/[^)]*)|//! * [Rust SDK documentation](https://docs.rs/ankaios-sdk/$sdk_version)|" "$base_dir"/src/lib.rs

        echo "Please remember to update the SDK versions in the compatibility tables from README.md and src/lib.rs!"
    fi
fi

if [ -n "$ankaios_version" ]; then
    echo "Updating Ankaios version to $ankaios_version"
    sed -i "s/const ANKAIOS_VERSION: &str = .*/const ANKAIOS_VERSION: \&str = \"$ankaios_version\";/" "$base_dir"/src/components/control_interface.rs
fi

if [ -n "$api_version" ]; then
    echo "Updating API version to $api_version"
    sed -i "s/const SUPPORTED_API_VERSION: &str = .*/const SUPPORTED_API_VERSION: \&str = \"$api_version\";/" "$base_dir"/src/components/complete_state.rs
    sed -i "s/^apiVersion: .*/apiVersion: $api_version/" "$base_dir"/examples/manifest.yaml
    sed -i "s/^\/\/\/ let manifest = Manifest::from_string(\"apiVersion: .*/\/\/\/ let manifest = Manifest::from_string(\"apiVersion: $api_version\").unwrap();/" "$base_dir"/src/components/manifest.rs
fi
