#!/bin/bash
# PostureFlow for macOS - Automated Installation Script
# Supports macOS 27, macOS 15 Sequoia, and macOS 14 Sonoma (Apple Silicon & Intel)
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}==========================================${NC}"
echo -e "${CYAN}   Installing PostureFlow for macOS       ${NC}"
echo -e "${CYAN}==========================================${NC}"

IS_ROOT=0
if [ "$EUID" -eq 0 ]; then
    IS_ROOT=1
fi

REAL_USER="${SUDO_USER:-$USER}"
USER_HOME=$(eval echo "~$REAL_USER")
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Auto-detect Xcode toolchain if needed
if [ -z "${DEVELOPER_DIR:-}" ]; then
    if [ -d "/Applications/Xcode-beta.app/Contents/Developer" ]; then
        export DEVELOPER_DIR="/Applications/Xcode-beta.app/Contents/Developer"
    elif [ -d "/Applications/Xcode.app/Contents/Developer" ]; then
        export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
    fi
fi

# Locate release binaries
find_bin_dir() {
    for candidate in \
        "$SCRIPT_DIR/.build/out/Products/Release" \
        "$SCRIPT_DIR/.build/release" \
        "$SCRIPT_DIR/.build/arm64-apple-macosx/release" \
        "$SCRIPT_DIR/dist"; do
        if [ -f "$candidate/postureflow" ]; then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

BIN_DIR="$(find_bin_dir 2>/dev/null || echo "")"

# 1. Build Swift Package if not pre-built
if [ -z "$BIN_DIR" ]; then
    echo -e "${YELLOW}[1/4] Building Swift release binaries...${NC}"
    if [ "$IS_ROOT" -eq 1 ]; then
        sudo -u "$REAL_USER" env DEVELOPER_DIR="${DEVELOPER_DIR:-}" swift build --package-path "$SCRIPT_DIR" -c release
    else
        env DEVELOPER_DIR="${DEVELOPER_DIR:-}" swift build --package-path "$SCRIPT_DIR" -c release
    fi
    BIN_DIR="$(find_bin_dir 2>/dev/null || echo "")"
fi

if [ -z "$BIN_DIR" ]; then
    echo -e "${RED}Error: Could not locate compiled release binaries.${NC}"
    exit 1
fi
echo -e "${GREEN}  -> Using binaries from: $BIN_DIR${NC}"

# 2. Deploy CLI tool
echo -e "${YELLOW}[2/4] Installing CLI tool...${NC}"
mkdir -p "$USER_HOME/.local/bin"
if [ -f "$BIN_DIR/postureflow" ]; then
    cp -f "$BIN_DIR/postureflow" "$USER_HOME/.local/bin/postureflow"
    chmod 755 "$USER_HOME/.local/bin/postureflow"
    echo -e "${GREEN}  -> Installed $USER_HOME/.local/bin/postureflow${NC}"

    if [ "$IS_ROOT" -eq 1 ] || [ -w "/usr/local/bin" ]; then
        mkdir -p /usr/local/bin
        cp -f "$BIN_DIR/postureflow" /usr/local/bin/postureflow
        chmod 755 /usr/local/bin/postureflow
        echo -e "${GREEN}  -> Installed /usr/local/bin/postureflow${NC}"
    fi
fi

# 3. Deploy PostureFlow.app
echo -e "${YELLOW}[3/4] Installing PostureFlow.app...${NC}"
TARGET_APP_DIR="$USER_HOME/Applications"
if [ "$IS_ROOT" -eq 1 ]; then
    TARGET_APP_DIR="/Applications"
fi
mkdir -p "$TARGET_APP_DIR/PostureFlow.app/Contents/MacOS"

if [ -f "$BIN_DIR/PostureFlowApp" ]; then
    cp -f "$BIN_DIR/PostureFlowApp" "$TARGET_APP_DIR/PostureFlow.app/Contents/MacOS/PostureFlowApp"
    chmod 755 "$TARGET_APP_DIR/PostureFlow.app/Contents/MacOS/PostureFlowApp"
    if [ -f "$SCRIPT_DIR/Resources/Info.plist" ]; then
        cp -f "$SCRIPT_DIR/Resources/Info.plist" "$TARGET_APP_DIR/PostureFlow.app/Contents/Info.plist"
        chmod 644 "$TARGET_APP_DIR/PostureFlow.app/Contents/Info.plist"
    fi
    mkdir -p "$TARGET_APP_DIR/PostureFlow.app/Contents/Resources"
    for icon in AppIcon.icns AppIcon.png; do
        if [ -f "$SCRIPT_DIR/Resources/$icon" ]; then
            cp -f "$SCRIPT_DIR/Resources/$icon" "$TARGET_APP_DIR/PostureFlow.app/Contents/Resources/$icon"
        fi
    done
    echo -e "${GREEN}  -> Installed $TARGET_APP_DIR/PostureFlow.app (with Security Shield Icon)${NC}"
fi

# Also keep user app folder updated if installing as root
if [ "$IS_ROOT" -eq 1 ] && [ -d "$USER_HOME/Applications" ]; then
    mkdir -p "$USER_HOME/Applications/PostureFlow.app/Contents/MacOS"
    cp -f "$BIN_DIR/PostureFlowApp" "$USER_HOME/Applications/PostureFlow.app/Contents/MacOS/PostureFlowApp"
    chmod 755 "$USER_HOME/Applications/PostureFlow.app/Contents/MacOS/PostureFlowApp"
    if [ -f "$SCRIPT_DIR/Resources/Info.plist" ]; then
        cp -f "$SCRIPT_DIR/Resources/Info.plist" "$USER_HOME/Applications/PostureFlow.app/Contents/Info.plist"
        chmod 644 "$USER_HOME/Applications/PostureFlow.app/Contents/Info.plist"
    fi
    mkdir -p "$USER_HOME/Applications/PostureFlow.app/Contents/Resources"
    for icon in AppIcon.icns AppIcon.png; do
        if [ -f "$SCRIPT_DIR/Resources/$icon" ]; then
            cp -f "$SCRIPT_DIR/Resources/$icon" "$USER_HOME/Applications/PostureFlow.app/Contents/Resources/$icon"
        fi
    done
fi

# 4. Privileged Helper Daemon & System Services
if [ "$IS_ROOT" -eq 1 ]; then
    echo -e "${YELLOW}[4/4] Configuring Privileged Helper Tool & launchd...${NC}"
    mkdir -p /Library/PrivilegedHelperTools
    mkdir -p /Library/LaunchDaemons
    mkdir -p "/Library/Application Support/PostureFlow"
    mkdir -p /etc/pf.anchors

    if [ -f "$BIN_DIR/PostureFlowHelper" ]; then
        cp -f "$BIN_DIR/PostureFlowHelper" /Library/PrivilegedHelperTools/com.postureflow.helper
        chmod 755 /Library/PrivilegedHelperTools/com.postureflow.helper
        chown root:wheel /Library/PrivilegedHelperTools/com.postureflow.helper
    fi

    # Write launchd daemon plist
    if [ -f "$SCRIPT_DIR/Resources/com.postureflow.helper.plist" ]; then
        cp -f "$SCRIPT_DIR/Resources/com.postureflow.helper.plist" /Library/LaunchDaemons/com.postureflow.helper.plist
    else
        cat << 'EOF' > /Library/LaunchDaemons/com.postureflow.helper.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.postureflow.helper</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Library/PrivilegedHelperTools/com.postureflow.helper</string>
    </array>
    <key>MachServices</key>
    <dict>
        <key>com.postureflow.helper</key>
        <true/>
    </dict>
    <key>KeepAlive</key>
    <true/>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF
    fi
    chmod 644 /Library/LaunchDaemons/com.postureflow.helper.plist
    chown root:wheel /Library/LaunchDaemons/com.postureflow.helper.plist

    # Bootstrap daemon
    echo -e "${YELLOW}  -> Bootstrapping background service...${NC}"
    launchctl bootout system /Library/LaunchDaemons/com.postureflow.helper.plist 2>/dev/null || true
    launchctl bootout system /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist 2>/dev/null || true
    rm -f /Library/LaunchDaemons/io.github.mzia.postureflow.helper.plist /Library/PrivilegedHelperTools/io.github.mzia.postureflow.helper 2>/dev/null || true

    launchctl bootstrap system /Library/LaunchDaemons/com.postureflow.helper.plist 2>/dev/null || true

    echo "home" > "/Library/Application Support/PostureFlow/state"
    chmod 644 "/Library/Application Support/PostureFlow/state"
else
    echo -e "${YELLOW}[4/4] Skipping system launchd registration (run with sudo for full system daemon)${NC}"
fi

# 5. Initialize User Configuration
USER_CONFIG_DIR="$USER_HOME/.config/postureflow"
mkdir -p "$USER_CONFIG_DIR"
if [ "$IS_ROOT" -eq 1 ]; then
    chown -R "$REAL_USER" "$USER_CONFIG_DIR"
fi

echo -e "\n${GREEN}==========================================================${NC}"
echo -e "${GREEN} PostureFlow for macOS installed successfully!            ${NC}"
echo -e "${CYAN} • CLI Binary : $USER_HOME/.local/bin/postureflow         ${NC}"
echo -e "${CYAN} • MenuBar App: $TARGET_APP_DIR/PostureFlow.app           ${NC}"
echo -e "${CYAN} • Config Dir : $USER_CONFIG_DIR                          ${NC}"
if [ "$IS_ROOT" -eq 1 ]; then
    echo -e "${CYAN} • Helper Tool: /Library/PrivilegedHelperTools/com.postureflow.helper${NC}"
else
    echo -e "${YELLOW} • Note       : Run 'sudo ./install.sh' or click 'Register Helper Tool'${NC}"
    echo -e "${YELLOW}                in PostureFlow Settings to enable system pfctl firewall.${NC}"
fi
echo -e "${GREEN} Verify via: postureflow --status                         ${NC}"
echo -e "${GREEN}==========================================================${NC}"
