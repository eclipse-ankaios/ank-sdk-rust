#!/bin/bash

# Copyright (c) 2026 Elektrobit Automotive GmbH
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

usage() {
    echo "Usage: $0 <ref> [-o|--output <dir>] [-h|--help]"
    echo "Fetch the proto files from the Ankaios repository."
    echo ""
    echo "  <ref>                  The branch or tag of the Ankaios repository to fetch from."
    echo "  -o / --output <dir>    The directory to write the proto files to."
    echo "                         Defaults to the proto directory of this repository."
    echo "  -h / --help            Display this help message and exit."
    echo ""
    echo "Example:"
    echo "  $0 main"
    echo "  $0 v1.0.0 --output /tmp/protos"
    exit 0
}

if [[ $# -lt 1 ]]; then
    echo "Error: Missing required argument <ref>" >&2
    usage
fi

if [[ "$1" = "--help" ]] || [[ "$1" = "-h" ]]; then
    usage
fi

REF="$1"
shift

PROTO_DIR="$script_dir/../proto"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output|-o)
            if [[ -z "$2" ]]; then
                echo "Error: output flag requires a value" >&2
                exit 1
            fi
            PROTO_DIR="$2"
            shift 2
            ;;
        --help|-h)
            usage
            ;;
        *)
            echo "Error: Unknown argument: $1" >&2
            usage
            ;;
    esac
done

BASE_URL="https://raw.githubusercontent.com/eclipse-ankaios/ankaios/${REF}"
# Proto files and their location in the Ankaios repository
declare -A PROTO_FILES=(
    ["ank_base.proto"]="ankaios_api/proto/ank_base.proto"
    ["control_api.proto"]="ankaios_api/proto/control_api.proto"
    ["grpc_api.proto"]="grpc/proto/grpc_api.proto"
)

mkdir -p "$PROTO_DIR"

# Download everything first, so a failure does not leave a partially updated directory
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

for PROTO_FILE in "${!PROTO_FILES[@]}"; do
    URL="${BASE_URL}/${PROTO_FILES[$PROTO_FILE]}"
    TMP_PATH="${TMP_DIR}/${PROTO_FILE}"

    HTTP_CODE=$(curl -s -w "%{http_code}" -o "$TMP_PATH" "$URL") || HTTP_CODE="000"

    if [[ "$HTTP_CODE" -ne 200 ]]; then
        echo "Error: Failed to download $PROTO_FILE (HTTP $HTTP_CODE)" >&2
        echo "URL: $URL" >&2
        exit 1
    fi

    if [[ ! -s "$TMP_PATH" ]]; then
        echo "Error: Downloaded $PROTO_FILE is empty" >&2
        exit 1
    fi

    echo "Successfully downloaded $PROTO_FILE"
done

for PROTO_FILE in "${!PROTO_FILES[@]}"; do
    mv "${TMP_DIR}/${PROTO_FILE}" "${PROTO_DIR}/${PROTO_FILE}"
done

echo "Done. Proto files written to $PROTO_DIR"
