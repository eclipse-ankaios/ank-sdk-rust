#!/bin/bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
ANKAIOS_SERVER_SOCKET="0.0.0.0:25551"
ANKAIOS_SERVER_URL="http://${ANKAIOS_SERVER_SOCKET}"
DEFAULT_ANKAIOS_BIN_PATH="/usr/local/bin"

display_usage() {
    echo -e "Usage: $0 EXAMPLE"
    echo -e "Build and run an Ankaios Rust SDK example."
    echo -e "  EXAMPLE: one of the apps, e.g. hello_ankaios"
    echo -e "Optionally, set environment variable for alternative Ankaios executable path: export ANK_BIN_DIR=/path/to/ankaios/executables, if not set default path: '${DEFAULT_ANKAIOS_BIN_PATH}'"
}

run_ankaios() {
  ANKAIOS_LOG_DIR=${SCRIPT_DIR}/logs
  mkdir -p ${ANKAIOS_LOG_DIR}

  # Start the Ankaios server
  echo "Starting Ankaios server located in '${ANK_BIN_DIR}'."
  RUST_LOG=debug ${ANK_BIN_DIR}/ank-server --insecure --address ${ANKAIOS_SERVER_SOCKET} > ${ANKAIOS_LOG_DIR}/ankaios-server.log 2>&1 &

  sleep 2
  # Start an Ankaios agent
  echo "Starting Ankaios agent agent_Rust_SDK located in '${ANK_BIN_DIR}'."
  RUST_LOG=debug ${ANK_BIN_DIR}/ank-agent --insecure --name agent_Rust_SDK --server-url ${ANKAIOS_SERVER_URL} > ${ANKAIOS_LOG_DIR}/ankaios-agent.log 2>&1 &

  sleep 2
  echo "Applying app manifest"
  ank -k apply manifest.yaml

  # Wait for any process to exit
  wait -n

  # Exit with status of process that exited first
  exit $?
}

if [ -z $1 ]; then
  display_usage
  exit 1
fi

# Check if app exists and copy it to the example directory
if [ ! -f "apps/$1.rs" ]; then
  echo "Rust app '$1.rs' not found!"
  exit 2
fi
cp -f apps/$1.rs app/src/main.rs

if [ -z ${ANK_BIN_DIR} ]; then
  ANK_BIN_DIR=${DEFAULT_ANKAIOS_BIN_PATH}
fi

ANK_BIN_DIR=${ANK_BIN_DIR%/} # remove trailing / if there is one

if [[ ! -f ${ANK_BIN_DIR}/ank-server || ! -f ${ANK_BIN_DIR}/ank-agent ]]; then
  echo "Failed to run example: no Ankaios executables inside '${ANK_BIN_DIR}'."
  display_usage
  exit 3
fi

echo Build Ankaios Rust SDK example ... "${@:2}"
podman build "${@:2}" -t app:0.1 -f app/Dockerfile ${SCRIPT_DIR}/..
echo done.

if pgrep -x "ank-server" >/dev/null
then
  echo -e "\nAbort startup. Ankaios server is already running."
  echo "Shutdown the Ankaios server instance manually or"
  echo -e "if 'run_example.sh' was executed previously,\nexecute 'shutdown_example.sh' afterwards to stop the example."
  exit 4
fi

run_ankaios &
