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

# Set threshold
THRESHOLD=80

# Help function
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Validates that code coverage meets the minimum threshold."
    echo ""
    echo "Options:"
    echo "  -h, --help     Display this help message and exit"
    echo ""
    echo "Details:"
    echo "  - Requires minimum ${THRESHOLD}% coverage across regions, functions, and lines"
    echo "  - Automatically installs cargo-llvm-cov if not present"
    echo "  - Exits with error (code 1) if any coverage metric is below threshold"
    echo "  - Provides color-coded output for pass/fail status"
}

# Parse arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
    shift
done

# Check if cargo-llvm-cov is installed
if ! cargo llvm-cov --version &>/dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
    if [ $? -ne 0 ]; then
        cho -e "\033[0;31mError\033[0m: Failed to install cargo-llvm-cov. Please install it manually."
        exit 1
    fi
fi

# Extract the last coverage line with the TOTAL stats
echo "Running coverage analysis..."
TOTAL_LINE=$(cargo llvm-cov 2>/dev/null | grep "TOTAL")

# Check if we got a coverage result
if [[ -z "$TOTAL_LINE" ]]; then
    echo -e "\033[0;31mError\033[0m: Failed to calculate coverage. Check if tests pass."
    exit 1
fi

# Extract the three percentages
mapfile -t PERCENTAGES < <(echo "$TOTAL_LINE" | grep -o '[0-9]*\.[0-9]*%' | tr -d '%')

# Convert percentages to integers (remove decimal and round down)
COV_REGION=${PERCENTAGES[0]%.*}
COV_FUNC=${PERCENTAGES[1]%.*}
COV_LINES=${PERCENTAGES[2]%.*}

# Check if all percentages meet the threshold
FAIL=false
if [ "$COV_REGION" -lt "$THRESHOLD" ]; then
    echo -e "\033[0;31mRegion coverage too low: ${COV_REGION}% (Required: ${THRESHOLD}%)\033[0m"
    FAIL=true
fi
if [ "$COV_FUNC" -lt "$THRESHOLD" ]; then
    echo -e "\033[0;31mFunction coverage too low: ${COV_FUNC}% (Required: ${THRESHOLD}%)\033[0m"
    FAIL=true
fi
if [ "$COV_LINES" -lt "$THRESHOLD" ]; then
    echo -e "\033[0;31mLine coverage too low: ${COV_LINES}% (Required: ${THRESHOLD}%)\033[0m"
    FAIL=true
fi

if [ "$FAIL" = true ]; then
    exit 1
else
    echo -e "\033[0;32mCoverage is sufficient: ${COV_REGION}%, ${COV_FUNC}%, ${COV_LINES}%\033[0m"
fi
