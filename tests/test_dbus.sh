#!/usr/bin/env bash
# ==============================================================================
# D-Bus Integration Test for postureflow-daemon (with pop-profile compat)
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== Testing postureflow-daemon D-Bus Interface ===${NC}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BIN_PATH="$SCRIPT_DIR/../target/release/postureflow-daemon"
if [ ! -f "$BIN_PATH" ]; then
    BIN_PATH="$SCRIPT_DIR/../target/release/pop-profile-daemon"
fi

if [ ! -f "$BIN_PATH" ]; then
    echo -e "${RED}[-] Binary not found at $BIN_PATH. Run 'cargo build --release' first.${NC}"
    exit 1
fi

dbus-run-session bash << 'EOF'
set -eo pipefail

BIN="./target/release/postureflow-daemon"
if [ ! -f "$BIN" ]; then
    BIN="./target/release/pop-profile-daemon"
fi

TEST_STATE=$(mktemp)
export POSTUREFLOW_STATE_FILE="$TEST_STATE"
export POP_PROFILE_STATE_FILE="$TEST_STATE"

echo "[*] Launching postureflow-daemon on test session bus..."
$BIN --daemon --session-bus &
DAEMON_PID=$!

cleanup() {
    kill $DAEMON_PID 2>/dev/null || true
    rm -f "$TEST_STATE"
}
trap cleanup EXIT

# Wait for D-Bus registration
sleep 1

echo "[*] Introspecting D-Bus service at io.github.mzia.PostureFlow..."
gdbus introspect --session --dest io.github.mzia.PostureFlow --object-path /io/github/mzia/PostureFlow > /dev/null

echo "[*] Calling GetActiveProfile() via primary io.github.mzia.PostureFlow..."
ACTIVE_PROFILE=$(gdbus call --session --dest io.github.mzia.PostureFlow --object-path /io/github/mzia/PostureFlow --method io.github.mzia.PostureFlow.GetActiveProfile)
echo "    -> Output: $ACTIVE_PROFILE"

echo "[*] Calling ListProfiles() via primary io.github.mzia.PostureFlow..."
PROFILES_LIST=$(gdbus call --session --dest io.github.mzia.PostureFlow --object-path /io/github/mzia/PostureFlow --method io.github.mzia.PostureFlow.ListProfiles)
echo "    -> Output: (Discovered $(echo "$PROFILES_LIST" | grep -o 'home' | wc -l) home profile entries)"
if [[ "$PROFILES_LIST" != *"home"* ]] || [[ "$PROFILES_LIST" != *"dev"* ]]; then
    echo "[-] Error: Expected built-in profiles in ListProfiles"
    exit 1
fi

echo "[*] Calling GetProfileDetails('dev')..."
DEV_TOML=$(gdbus call --session --dest io.github.mzia.PostureFlow --object-path /io/github/mzia/PostureFlow --method io.github.mzia.PostureFlow.GetProfileDetails "dev")
if [[ "$DEV_TOML" != *"Developer"* ]]; then
    echo "[-] Error: Failed to retrieve profile details for 'dev'"
    exit 1
fi
echo "    -> Output: (Retrieved $(echo "$DEV_TOML" | wc -c) bytes of TOML)"

echo "[*] Calling ValidateProfile()..."
VALID_TOML='[profile]\nid = "test-ai"\nname = "AI Test"\n[firewall]\nallow_loopback = false'
VAL_RES=$(gdbus call --session --dest io.github.mzia.PostureFlow --object-path /io/github/mzia/PostureFlow --method io.github.mzia.PostureFlow.ValidateProfile "$VALID_TOML")
if [[ "$VAL_RES" != *"(true,"* ]]; then
    echo "[-] Error: ValidateProfile expected true"
    exit 1
fi
echo "    -> Output: Successfully validated and sanitized custom profile"

echo "[*] Verifying Backwards Compatibility: Calling GetActiveProfile() via legacy io.github.mzia.PopProfile..."
LEGACY_PROFILE=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetActiveProfile)
echo "    -> Output: $LEGACY_PROFILE"
if [[ "$LEGACY_PROFILE" != "$ACTIVE_PROFILE" ]]; then
    echo "[-] Error: Legacy D-Bus service returned different profile than primary service"
    exit 1
fi

echo -e "\033[0;32m[✔] All PostureFlow & legacy PopProfile D-Bus methods responded successfully!\033[0m"
EOF
