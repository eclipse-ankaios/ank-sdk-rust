#!/bin/bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
RUN_DIR="${SCRIPT_DIR}/ankaios_run"
AGENT_NAME="agent_A"
DEFAULT_ANKAIOS_BIN_PATH="/usr/local/bin"

display_usage() {
    echo -e "Usage: $0 [tcp|unix]"
    echo -e "Start an Ankaios server and agent for the command interface examples."
    echo -e "  tcp:  plaintext TCP on 127.0.0.1:25551 (default)"
    echo -e "  unix: Unix domain socket at /tmp/ankaios.sock"
    echo -e "Logs and runtime files are stored in '${RUN_DIR}'."
    echo -e "Optionally, set environment variable for alternative Ankaios executable path: export ANK_BIN_DIR=/path/to/ankaios/executables, if not set default path: '${DEFAULT_ANKAIOS_BIN_PATH}'"
}

TRANSPORT=${1:-tcp}
case ${TRANSPORT} in
  tcp)
    SERVER_ADDRESS="127.0.0.1:25551"
    SERVER_URL="http://${SERVER_ADDRESS}"
    TLS_FLAG="--insecure"
    ;;
  unix)
    SERVER_ADDRESS="unix:///tmp/ankaios.sock"
    SERVER_URL="${SERVER_ADDRESS}"
    TLS_FLAG=""
    ;;
  *)
    display_usage
    exit 1
    ;;
esac

ANK_BIN_DIR=${ANK_BIN_DIR:-${DEFAULT_ANKAIOS_BIN_PATH}}
ANK_BIN_DIR=${ANK_BIN_DIR%/} # remove trailing / if there is one

if [[ ! -f ${ANK_BIN_DIR}/ank-server || ! -f ${ANK_BIN_DIR}/ank-agent || ! -f ${ANK_BIN_DIR}/ank ]]; then
  echo "Failed to start Ankaios: no Ankaios executables inside '${ANK_BIN_DIR}'."
  display_usage
  exit 2
fi

if pgrep -x "ank-server" >/dev/null || pgrep -x "ank-agent" >/dev/null
then
  echo -e "\nAbort startup. An Ankaios server or agent is already running."
  echo -e "Shutdown the instance manually or, if 'start_ankaios.sh' was executed previously,\nexecute 'stop_ankaios.sh' first."
  exit 3
fi

# The agent run folder must be owned by the current user and not accessible by others
mkdir -p "${RUN_DIR}/agent"
chmod 700 "${RUN_DIR}/agent"
rm -f /tmp/ankaios.sock

echo "Starting Ankaios server on '${SERVER_ADDRESS}'."
RUST_LOG=debug ${ANK_BIN_DIR}/ank-server ${TLS_FLAG} --address ${SERVER_ADDRESS} > "${RUN_DIR}/ankaios-server.log" 2>&1 &
echo $! > "${RUN_DIR}/ank-server.pid"

sleep 2
echo "Starting Ankaios agent '${AGENT_NAME}'."
RUST_LOG=debug ${ANK_BIN_DIR}/ank-agent ${TLS_FLAG} --name ${AGENT_NAME} --server-url ${SERVER_URL} --run-folder "${RUN_DIR}/agent" > "${RUN_DIR}/ankaios-agent.log" 2>&1 &
echo $! > "${RUN_DIR}/ank-agent.pid"

# Wait until the agent is connected to the server
for _ in $(seq 1 20); do
  if ${ANK_BIN_DIR}/ank ${TLS_FLAG} --server-url ${SERVER_URL} get agents 2>/dev/null | grep -q "${AGENT_NAME}"; then
    echo "Ankaios is ready. Logs are in '${RUN_DIR}'."
    echo "Run an example with: ./target/x86_64-unknown-linux-musl/release/test_app <example> ${SERVER_URL} ${AGENT_NAME}"
    echo "Stop and clean up with: ./stop_ankaios.sh"
    exit 0
  fi
  sleep 1
done

echo "Ankaios did not become ready in time. Check the logs in '${RUN_DIR}'."
exit 4
