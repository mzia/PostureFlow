#!/usr/bin/env bash
# ==============================================================================
# pop-profile Installer (CLI + Rust D-Bus Daemon + Polkit)
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[-] Please run with sudo: sudo ./install.sh${NC}"
    exit 1
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

echo -e "\n${BOLD}${CYAN}=== Installing pop-profile & Rust D-Bus Daemon ===${NC}"

# 1. Install CLI binary
echo "[*] Installing CLI to /usr/local/bin/pop-profile..."
install -m 755 "$SCRIPT_DIR/bin/pop-profile" /usr/local/bin/pop-profile

# 2. Install Rust Daemon & Applet binaries if built
if [ -f "$SCRIPT_DIR/target/release/pop-profile-daemon" ]; then
    echo "[*] Installing Rust daemon to /usr/local/bin/pop-profile-daemon..."
    install -m 755 "$SCRIPT_DIR/target/release/pop-profile-daemon" /usr/local/bin/pop-profile-daemon
fi
if [ -f "$SCRIPT_DIR/target/release/pop-profile-applet" ]; then
    echo "[*] Installing COSMIC Applet to /usr/local/bin/pop-profile-applet..."
    install -m 755 "$SCRIPT_DIR/target/release/pop-profile-applet" /usr/local/bin/pop-profile-applet
    ln -sf /usr/local/bin/pop-profile-applet /usr/local/bin/cosmic-applet-popprofile
fi

# 3. Install COSMIC Desktop Entry & User Service
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PopProfile.Applet.desktop" ]; then
    echo "[*] Installing COSMIC Applet desktop entry..."
    install -d /usr/share/applications
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PopProfile.Applet.desktop" /usr/share/applications/io.github.mzia.PopProfile.Applet.desktop
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
    fi
fi
if [ -f "$SCRIPT_DIR/data/pop-profile-applet.service" ]; then
    echo "[*] Installing systemd user applet service..."
    install -d /usr/lib/systemd/user
    install -m 644 "$SCRIPT_DIR/data/pop-profile-applet.service" /usr/lib/systemd/user/pop-profile-applet.service
fi

# 4. Install Polkit Policy (Passwordless desktop profile switching)
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PopProfile.policy" ]; then
    echo "[*] Installing Polkit policy to /usr/share/polkit-1/actions/..."
    install -d /usr/share/polkit-1/actions
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PopProfile.policy" /usr/share/polkit-1/actions/io.github.mzia.PopProfile.policy
fi

# 5. Install D-Bus System Bus Configuration
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PopProfile.conf" ]; then
    echo "[*] Installing D-Bus system configuration..."
    install -d /usr/share/dbus-1/system.d
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PopProfile.conf" /usr/share/dbus-1/system.d/io.github.mzia.PopProfile.conf
    # Reload D-Bus configuration
    if command -v systemctl >/dev/null 2>&1; then
        systemctl reload dbus 2>/dev/null || true
    fi
fi

# 6. Install Systemd Service Unit
if [ -f "$SCRIPT_DIR/data/pop-profile-daemon.service" ]; then
    echo "[*] Installing systemd daemon service..."
    install -m 644 "$SCRIPT_DIR/data/pop-profile-daemon.service" /etc/systemd/system/pop-profile-daemon.service
    if command -v systemctl >/dev/null 2>&1; then
        systemctl daemon-reload 2>/dev/null || true
    fi
fi

# 6. Install Man page
echo "[*] Installing manual page to /usr/local/share/man/man1/pop-profile.1..."
install -d /usr/local/share/man/man1
install -m 644 "$SCRIPT_DIR/man/pop-profile.1" /usr/local/share/man/man1/pop-profile.1
if command -v mandb >/dev/null 2>&1; then
    mandb -q >/dev/null 2>&1 || true
fi

# 7. Install Completions
if [ -d /etc/bash_completion.d ]; then
    echo "[*] Installing bash completion..."
    install -m 644 "$SCRIPT_DIR/completions/pop-profile.bash" /etc/bash_completion.d/pop-profile
fi
if [ -d /usr/share/zsh/vendor-completions ]; then
    echo "[*] Installing zsh completion..."
    install -m 644 "$SCRIPT_DIR/completions/pop-profile.zsh" /usr/share/zsh/vendor-completions/_pop-profile
fi

# 8. Install APT post-upgrade hook for persistence
echo "[*] Registering APT post-upgrade maintenance hook..."
mkdir -p /etc/apt/apt.conf.d/
cat << 'EOF' > /etc/apt/apt.conf.d/99-popos-profile-health
// Automatically maintain Pop!_OS security profiles after package updates
DPkg::Post-Invoke { "if [ -x /usr/local/bin/pop-profile ]; then /usr/local/bin/pop-profile >/dev/null 2>&1 || true; fi"; };
EOF
chmod 644 /etc/apt/apt.conf.d/99-popos-profile-health

# 9. Run safety verification
echo "[*] Running verification tests..."
bash "$SCRIPT_DIR/tests/test_safety.sh"

echo ""
echo -e "${GREEN}${BOLD}[✔] pop-profile & Rust D-Bus components installed successfully!${NC}"
echo "Usage:"
echo "  sudo pop-profile --home      # Streaming & Gaming"
echo "  sudo pop-profile --work      # Office & Corporate VPN"
echo "  sudo pop-profile --dev       # Coding & Debugging"
echo "  sudo pop-profile --secure    # Travel & Lockdown"
echo "  pop-profile --status         # Check active posture"
echo ""
echo "D-Bus Daemon Service:"
echo "  sudo systemctl start pop-profile-daemon    # Start background D-Bus service"
echo "  sudo systemctl enable pop-profile-daemon   # Enable on boot for COSMIC Applets"
