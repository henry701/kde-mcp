#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
test_root=$(mktemp -d "${TMPDIR:-/tmp}/kde-mcp-tests.XXXXXXXX")
trap 'rm -rf -- "$test_root"' EXIT
mkdir -p "$test_root"/{config,runtime,cache,data,state}

# Approval tests exercise real notification code; never use the desktop bus.
env -u DBUS_SESSION_BUS_ADDRESS -u DBUS_STARTER_ADDRESS \
    -u DBUS_STARTER_BUS_TYPE -u DISPLAY -u WAYLAND_DISPLAY -u AT_SPI_BUS_ADDRESS \
    XDG_CONFIG_HOME="$test_root/config" \
    XDG_RUNTIME_DIR="$test_root/runtime" \
    XDG_CACHE_HOME="$test_root/cache" \
    XDG_DATA_HOME="$test_root/data" \
    XDG_STATE_HOME="$test_root/state" \
    dbus-run-session --config-file scripts/test-bus.conf -- cargo test --workspace "$@"
