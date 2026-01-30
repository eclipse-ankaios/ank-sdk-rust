# Helper Scripts

This directory contains utility scripts for specific tasks. All scripts support the `--help` flag for detailed usage information.

## check_coverage.sh

Validates that code coverage meets the minimum threshold.

Recommended usage via just command: `just cov-check`

## update_version.sh

Updates SDK, Ankaios, and API versions across the project. Supports updating versions individually or all at once. Use the `--release` flag for release versions to also update documentation URLs and badges.
