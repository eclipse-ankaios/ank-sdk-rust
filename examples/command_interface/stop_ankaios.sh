#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
RUN_DIR="${SCRIPT_DIR}/ankaios_run"

# Cleanup Ankaios ....
echo "Cleaning up Ankaios..."
for name in ank-agent ank-server; do
  if [ -f "${RUN_DIR}/${name}.pid" ]; then
    kill "$(cat "${RUN_DIR}/${name}.pid")" >/dev/null 2>&1
    rm -f "${RUN_DIR}/${name}.pid"
  fi
done
rm -rf "${RUN_DIR}/agent"
rm -f /tmp/ankaios.sock
echo "OK."

# Cleanup podman
echo "Cleaning up podman..."
podman stop -a >/dev/null 2>&1
podman rm -a >/dev/null 2>&1
echo "OK."
