#!/usr/bin/env bash
# ==============================================================================
# D-Bus Integration Test for pop-profile-daemon
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== Testing pop-profile-daemon D-Bus Interface ===${NC}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BIN_PATH="$SCRIPT_DIR/../target/release/pop-profile-daemon"

if [ ! -f "$BIN_PATH" ]; then
    echo -e "${RED}[-] Binary not found at $BIN_PATH. Run 'cargo build --release' first.${NC}"
    exit 1
fi

dbus-run-session bash << 'EOF'
set -eo pipefail

BIN="./target/release/pop-profile-daemon"

TEST_STATE=$(mktemp)
export POP_PROFILE_STATE_FILE="$TEST_STATE"

echo "[*] Launching pop-profile-daemon on test session bus..."
$BIN --daemon --session-bus &
DAEMON_PID=$!

cleanup() {
    kill $DAEMON_PID 2>/dev/null || true
    rm -f "$TEST_STATE"
}
trap cleanup EXIT

# Wait for D-Bus registration
sleep 1

echo "[*] Introspecting D-Bus service at io.github.mzia.PopProfile..."
gdbus introspect --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile > /dev/null

echo "[*] Calling GetActiveProfile()..."
ACTIVE_PROFILE=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetActiveProfile)
echo "    -> Output: $ACTIVE_PROFILE"

echo "[*] Calling GetStatus()..."
STATUS_OUT=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetStatus)
echo "    -> Output: (Received $(echo "$STATUS_OUT" | wc -c) bytes of status report)"

echo -e "\033[0;32m[✔] All D-Bus methods responded successfully!\033[0m"
EOF
