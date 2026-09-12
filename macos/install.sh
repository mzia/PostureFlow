#!/bin/bash
# PostureFlow for macOS - Automated Installation Script
# Supports macOS 15 Sequoia and macOS 14 Sonoma (Apple Silicon & Intel)
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}==========================================${NC}"
echo -e "${CYAN}   Installing PostureFlow for macOS       ${NC}"
echo -e "${CYAN}==========================================${NC}"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./install.sh)${NC}"
    exit 1
fi

REAL_USER="${SUDO_USER:-$USER}"
USER_HOME=$(eval echo "~$REAL_USER")
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 1. Build Swift Package if not pre-built
if [ ! -f "$SCRIPT_DIR/.build/release/postureflow" ] && [ ! -f "$SCRIPT_DIR/dist/postureflow" ]; then
    echo -e "${YELLOW}[1/5] Building Swift release binaries...${NC}"
    sudo -u "$REAL_USER" swift build --package-path "$SCRIPT_DIR" -c release
fi

BIN_DIR="$SCRIPT_DIR/.build/release"
if [ -f "$SCRIPT_DIR/dist/postureflow" ]; then
    BIN_DIR="$SCRIPT_DIR/dist"
fi

# 2. Deploy CLI tool to /usr/local/bin
echo -e "${YELLOW}[2/5] Installing CLI tool to /usr/local/bin...${NC}"
mkdir -p /usr/local/bin
if [ -f "$BIN_DIR/postureflow" ]; then
    cp -f "$BIN_DIR/postureflow" /usr/local/bin/postureflow
    chmod 755 /usr/local/bin/postureflow
    echo -e "${GREEN}  -> Installed /usr/local/bin/postureflow${NC}"
fi

# 3. Deploy Privileged Helper Daemon
echo -e "${YELLOW}[3/5] Configuring Privileged Helper Tool & launchd...${NC}"
mkdir -p /Library/PrivilegedHelperTools
mkdir -p /Library/LaunchDaemons
mkdir -p "/Library/Application Support/PostureFlow"

if [ -f "$BIN_DIR/PostureFlowHelper" ]; then
    cp -f "$BIN_DIR/PostureFlowHelper" /Library/PrivilegedHelperTools/io.github.mzia.postureflow.helper
    chmod 755 /Library/PrivilegedHelperTools/io.github.mzia.postureflow.helper
    chown root:wheel /Library/PrivilegedHelperTools/io.github.mzia.postureflow.helper
fi

# Write launchd daemon plist
cat << 'EOF' > /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.github.mzia.postureflow.helper</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/PrivilegedHelperTools/io.github.mzia.postureflow.helper</string>
    </array>
    <key>MachServices</key>
    <dict>
        <key>io.github.mzia.postureflow.helper</key>
        <true/>
    </dict>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF
chmod 644 /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist
chown root:wheel /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist

# 4. Bootstrap daemon
echo -e "${YELLOW}[4/5] Bootstrapping background service...${NC}"
launchctl bootout system /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist 2>/dev/null || true
launchctl bootstrap system /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist 2>/dev/null || true

# 5. Initialize State & User Configuration
echo -e "${YELLOW}[5/5] Initializing state & user preferences...${NC}"
echo "home" > "/Library/Application Support/PostureFlow/state"
chmod 644 "/Library/Application Support/PostureFlow/state"

USER_CONFIG_DIR="$USER_HOME/.config/postureflow"
mkdir -p "$USER_CONFIG_DIR"
chown -R "$REAL_USER" "$USER_CONFIG_DIR"

echo -e "${GREEN}==========================================================${NC}"
echo -e "${GREEN} PostureFlow for macOS installed successfully!            ${NC}"
echo -e "${GREEN} Verify via: postureflow status                          ${NC}"
echo -e "${GREEN}==========================================================${NC}"
