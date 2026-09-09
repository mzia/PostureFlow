#!/usr/bin/env bash
# ==============================================================================
# Integration Test for pop-profile-applet & COSMIC StatusNotifierItem/DBusMenu
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
YELLOW="\033[1;33m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== Testing pop-profile-applet Integration ===${NC}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DAEMON_BIN="$SCRIPT_DIR/../target/release/pop-profile-daemon"
APPLET_BIN="$SCRIPT_DIR/../target/release/pop-profile-applet"

if [ ! -f "$DAEMON_BIN" ] || [ ! -f "$APPLET_BIN" ]; then
    echo -e "${RED}[-] Binaries not found. Run 'cargo build --release' first.${NC}"
    exit 1
fi

dbus-run-session bash << 'EOF'
set -eo pipefail

DAEMON="./target/release/pop-profile-daemon"
APPLET="./target/release/pop-profile-applet"

TEST_STATE=$(mktemp)
export POP_PROFILE_STATE_FILE="$TEST_STATE"

echo "[*] Step 1: Starting pop-profile-daemon on test bus..."
$DAEMON --daemon --session-bus &
DAEMON_PID=$!

sleep 1

echo "[*] Step 2: Starting pop-profile-applet on test bus..."
$APPLET --session-bus &
APPLET_PID=$!

cleanup() {
    echo "[*] Cleaning up background test processes..."
    kill $APPLET_PID 2>/dev/null || true
    kill $DAEMON_PID 2>/dev/null || true
    rm -f "$TEST_STATE"
}
trap cleanup EXIT

sleep 2

# Discover the unique bus name for the applet
APPLET_DEST=$(busctl --user list | grep pop-profile-app | awk '{print $1}' || true)
if [ -z "$APPLET_DEST" ]; then
    APPLET_DEST=":1.2"
fi
echo "[*] Discovered Applet Bus Destination: $APPLET_DEST"

echo "[*] Step 3: Verifying StatusNotifierItem properties on D-Bus..."
SNI_ID=$(gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem Id)
echo "    -> SNI Id: $SNI_ID"

MENU_PATH=$(gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem Menu)
echo "    -> Menu Path: $MENU_PATH"

INITIAL_ICON=$(gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem IconName)
echo "    -> Initial Icon: $INITIAL_ICON"

echo "[*] Step 4: Calling GetLayout on /com/canonical/dbusmenu..."
LAYOUT=$(gdbus call --session --dest "$APPLET_DEST" --object-path /com/canonical/dbusmenu --method com.canonical.dbusmenu.GetLayout 0 10 "[]")
echo "    -> Layout successfully retrieved ($(echo "$LAYOUT" | wc -c) bytes)"
if [[ "$LAYOUT" != *"Configure Profiles"* ]]; then
    echo "[-] Error: Expected 'Configure Profiles' in menu layout"
    exit 1
fi

echo "[*] Step 5: Simulating user click on Work Profile (Item ID 11)..."
gdbus call --session --dest "$APPLET_DEST" --object-path /com/canonical/dbusmenu --method com.canonical.dbusmenu.Event 11 "clicked" "<0>" 0

sleep 1

ACTIVE_PROFILE=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetActiveProfile)
echo "    -> Active Profile from Daemon: $ACTIVE_PROFILE"
if [[ "$ACTIVE_PROFILE" != *"work"* ]]; then
    echo "[-] Error: Expected active profile 'work', got: $ACTIVE_PROFILE"
    exit 1
fi

UPDATED_ICON=$(gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem IconName)
echo "    -> Updated Icon: $UPDATED_ICON"
if [[ "$UPDATED_ICON" != *"applications-office-symbolic"* ]]; then
    echo "[-] Error: Expected work icon, got: $UPDATED_ICON"
    exit 1
fi

echo "[*] Step 6: Simulating left click (Activate) to cycle to Dev profile..."
gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.kde.StatusNotifierItem.Activate 0 0

sleep 1

CYCLED_PROFILE=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetActiveProfile)
echo "    -> Cycled Profile from Daemon: $CYCLED_PROFILE"
if [[ "$CYCLED_PROFILE" != *"dev"* ]]; then
    echo "[-] Error: Expected cycled profile 'dev', got: $CYCLED_PROFILE"
    exit 1
fi

CYCLED_ICON=$(gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem IconName)
echo "    -> Cycled Icon: $CYCLED_ICON"
if [[ "$CYCLED_ICON" != *"utilities-terminal-symbolic"* ]]; then
    echo "[-] Error: Expected dev icon, got: $CYCLED_ICON"
    exit 1
fi

echo "[*] Step 7: Simulating left click (Activate) to cycle to Travel profile..."
gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.kde.StatusNotifierItem.Activate 0 0

sleep 1

TRAVEL_PROFILE=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetActiveProfile)
echo "    -> Cycled Profile from Daemon: $TRAVEL_PROFILE"
if [[ "$TRAVEL_PROFILE" != *"travel"* ]]; then
    echo "[-] Error: Expected cycled profile 'travel', got: $TRAVEL_PROFILE"
    exit 1
fi

TRAVEL_ICON=$(gdbus call --session --dest "$APPLET_DEST" --object-path /StatusNotifierItem --method org.freedesktop.DBus.Properties.Get org.kde.StatusNotifierItem IconName)
echo "    -> Cycled Icon: $TRAVEL_ICON"
if [[ "$TRAVEL_ICON" != *"security-high-symbolic"* ]]; then
    echo "[-] Error: Expected travel icon, got: $TRAVEL_ICON"
    exit 1
fi

echo "[*] Step 8: Simulating user click on Reset to Factory Defaults (Item ID 22)..."
gdbus call --session --dest "$APPLET_DEST" --object-path /com/canonical/dbusmenu --method com.canonical.dbusmenu.Event 22 "clicked" "<0>" 0

sleep 1

RESET_PROFILE=$(gdbus call --session --dest io.github.mzia.PopProfile --object-path /io/github/mzia/PopProfile --method io.github.mzia.PopProfile.GetActiveProfile)
echo "    -> Profile after Reset: $RESET_PROFILE"
if [[ "$RESET_PROFILE" != *"default"* ]] && [[ "$RESET_PROFILE" != *"home"* ]]; then
    echo "[-] Error: Expected active profile 'default' or 'home' after reset, got: $RESET_PROFILE"
    exit 1
fi

echo -e "\n\033[0;32m[✔] All applet StatusNotifierItem & DBusMenu tests passed successfully!\033[0m"
EOF
