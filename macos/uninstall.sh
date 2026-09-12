#!/bin/bash
# PostureFlow for macOS - Automated Uninstallation Script
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}==========================================${NC}"
echo -e "${CYAN}   Uninstalling PostureFlow from macOS    ${NC}"
echo -e "${CYAN}==========================================${NC}"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./uninstall.sh)${NC}"
    exit 1
fi

# 1. Terminate and unregister background service
echo -e "${YELLOW}[1/4] Stopping background service...${NC}"
launchctl bootout system /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist 2>/dev/null || true
rm -f /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist
rm -f /Library/PrivilegedHelperTools/io.github.mzia.postureflow.helper

# 2. Flush PostureFlow packet filter anchors
echo -e "${YELLOW}[2/4] Resetting pfctl firewall rules...${NC}"
pfctl -a "postureflow/*" -F all 2>/dev/null || true

# 3. Remove CLI binary
echo -e "${YELLOW}[3/4] Removing CLI binary...${NC}"
rm -f /usr/local/bin/postureflow

# 4. Clean Application Support
echo -e "${YELLOW}[4/4] Removing system state...${NC}"
rm -rf "/Library/Application Support/PostureFlow"

echo -e "${GREEN}==========================================================${NC}"
echo -e "${GREEN} PostureFlow has been completely removed from macOS.      ${NC}"
echo -e "${GREEN}==========================================================${NC}"
