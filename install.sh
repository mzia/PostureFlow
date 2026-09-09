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

DEV_MODE=0
if [[ "$1" == "--dev" ]] || [[ "$1" == "-d" ]]; then
    DEV_MODE=1
    echo -e "\n${BOLD}${CYAN}=== Installing pop-profile in DEVELOPER MODE ===${NC}"
    echo -e "${YELLOW}[*] Live workspace symlinks enabled. Recompiling with 'cargo build --release' will immediately reflect system-wide.${NC}"
else
    echo -e "\n${BOLD}${CYAN}=== Installing pop-profile & Rust D-Bus Daemon ===${NC}"
fi

# 1. Install CLI & Rust Binaries
if [ "$DEV_MODE" -eq 1 ]; then
    echo "[*] Linking workspace CLI and binaries to /usr/local/bin and /usr/bin..."
    ln -sf "$SCRIPT_DIR/bin/pop-profile" /usr/local/bin/pop-profile
    ln -sf "$SCRIPT_DIR/bin/pop-profile" /usr/bin/pop-profile
    if [ -f "$SCRIPT_DIR/target/release/pop-profile-daemon" ]; then
        ln -sf "$SCRIPT_DIR/target/release/pop-profile-daemon" /usr/local/bin/pop-profile-daemon
        ln -sf "$SCRIPT_DIR/target/release/pop-profile-daemon" /usr/bin/pop-profile-daemon
    fi
    if [ -f "$SCRIPT_DIR/target/release/pop-profile-applet" ]; then
        ln -sf "$SCRIPT_DIR/target/release/pop-profile-applet" /usr/local/bin/pop-profile-applet
        ln -sf "$SCRIPT_DIR/target/release/pop-profile-applet" /usr/bin/pop-profile-applet
        ln -sf /usr/bin/pop-profile-applet /usr/bin/cosmic-applet-popprofile
        ln -sf /usr/local/bin/pop-profile-applet /usr/local/bin/cosmic-applet-popprofile
    fi
    if [ -f "$SCRIPT_DIR/target/release/pop-profile-gui" ]; then
        ln -sf "$SCRIPT_DIR/target/release/pop-profile-gui" /usr/local/bin/pop-profile-gui
        ln -sf "$SCRIPT_DIR/target/release/pop-profile-gui" /usr/bin/pop-profile-gui
    fi
else
    echo "[*] Installing CLI to /usr/local/bin/pop-profile and /usr/bin/pop-profile..."
    install -m 755 "$SCRIPT_DIR/bin/pop-profile" /usr/local/bin/pop-profile
    ln -sf /usr/local/bin/pop-profile /usr/bin/pop-profile
    if [ -f "$SCRIPT_DIR/target/release/pop-profile-daemon" ]; then
        echo "[*] Installing Rust daemon..."
        install -m 755 "$SCRIPT_DIR/target/release/pop-profile-daemon" /usr/local/bin/pop-profile-daemon
        ln -sf /usr/local/bin/pop-profile-daemon /usr/bin/pop-profile-daemon
    fi
    if [ -f "$SCRIPT_DIR/target/release/pop-profile-applet" ]; then
        echo "[*] Installing COSMIC Applet..."
        install -m 755 "$SCRIPT_DIR/target/release/pop-profile-applet" /usr/local/bin/pop-profile-applet
        ln -sf /usr/local/bin/pop-profile-applet /usr/bin/pop-profile-applet
        ln -sf /usr/bin/pop-profile-applet /usr/bin/cosmic-applet-popprofile
        ln -sf /usr/local/bin/pop-profile-applet /usr/local/bin/cosmic-applet-popprofile
    fi
    if [ -f "$SCRIPT_DIR/target/release/pop-profile-gui" ]; then
        echo "[*] Installing Pop! Profile GUI..."
        install -m 755 "$SCRIPT_DIR/target/release/pop-profile-gui" /usr/local/bin/pop-profile-gui
        ln -sf /usr/local/bin/pop-profile-gui /usr/bin/pop-profile-gui
    fi
fi

# 2. Ensure custom profiles directory exists
mkdir -p /etc/pop-profile/profiles.d

# 3. Install COSMIC Desktop Entries & User Service
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PopProfile.Applet.desktop" ]; then
    echo "[*] Installing COSMIC Applet desktop entry..."
    install -d /usr/share/applications
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PopProfile.Applet.desktop" /usr/share/applications/io.github.mzia.PopProfile.Applet.desktop
fi
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PopProfile.Settings.desktop" ]; then
    echo "[*] Installing Pop! Profile GUI Settings desktop entry..."
    install -d /usr/share/applications
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PopProfile.Settings.desktop" /usr/share/applications/io.github.mzia.PopProfile.Settings.desktop
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
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
        echo "[*] Enabling and restarting pop-profile-daemon service..."
        systemctl daemon-reload 2>/dev/null || true
        systemctl enable pop-profile-daemon.service 2>/dev/null || true
        systemctl restart pop-profile-daemon.service 2>/dev/null || true
    fi
fi

# 7. Install Man page
echo "[*] Installing manual page to /usr/local/share/man/man1/pop-profile.1..."
install -d /usr/local/share/man/man1
install -m 644 "$SCRIPT_DIR/man/pop-profile.1" /usr/local/share/man/man1/pop-profile.1
if command -v mandb >/dev/null 2>&1; then
    mandb -q >/dev/null 2>&1 || true
fi

# 8. Install Completions
if [ -d /etc/bash_completion.d ]; then
    echo "[*] Installing bash completion..."
    install -m 644 "$SCRIPT_DIR/completions/pop-profile.bash" /etc/bash_completion.d/pop-profile
fi
if [ -d /usr/share/zsh/vendor-completions ]; then
    echo "[*] Installing zsh completion..."
    install -m 644 "$SCRIPT_DIR/completions/pop-profile.zsh" /usr/share/zsh/vendor-completions/_pop-profile
fi

# 9. Install APT post-upgrade hook for persistence
echo "[*] Registering APT post-upgrade maintenance hook..."
mkdir -p /etc/apt/apt.conf.d/
cat << 'EOF' > /etc/apt/apt.conf.d/99-popos-profile-health
// Automatically maintain Pop!_OS security profiles after package updates
DPkg::Post-Invoke { "if [ -x /usr/bin/pop-profile ]; then /usr/bin/pop-profile >/dev/null 2>&1 || true; elif [ -x /usr/local/bin/pop-profile ]; then /usr/local/bin/pop-profile >/dev/null 2>&1 || true; fi"; };
EOF
chmod 644 /etc/apt/apt.conf.d/99-popos-profile-health

# 10. Configure User Applet Service for active desktop user
if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    USER_UID=$(id -u "$SUDO_USER" 2>/dev/null || echo "1000")
    if [ -d "/run/user/$USER_UID" ]; then
        echo "[*] Activating COSMIC Applet user service for '$SUDO_USER'..."
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user daemon-reload >/dev/null 2>&1 || true
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user enable --now pop-profile-applet.service >/dev/null 2>&1 || true
    fi
fi

# 11. Developer Mode profile activation
if [ "$DEV_MODE" -eq 1 ]; then
    echo "[*] Developer Mode: Activating Developer / Coding profile..."
    if [ -x /usr/bin/pop-profile ]; then
        /usr/bin/pop-profile --dev || true
    elif [ -x /usr/local/bin/pop-profile ]; then
        /usr/local/bin/pop-profile --dev || true
    fi
fi

# 12. Run safety verification
echo "[*] Running verification tests..."
bash "$SCRIPT_DIR/tests/test_safety.sh"

echo ""
echo -e "${GREEN}${BOLD}[✔] pop-profile & Rust D-Bus components installed successfully!${NC}"
if [ "$DEV_MODE" -eq 1 ]; then
    echo -e "${CYAN}Developer mode is ACTIVE:${NC}"
    echo "  • Live workspace binaries linked"
    echo "  • Inotify watches maximized (524,288)"
    echo "  • Core dumps enabled for debugging"
    echo "  • Container & dev ports opened"
    echo "  • COSMIC panel applet running"
fi
echo "Usage:"
echo "  pop-profile --status         # Check active posture"
echo "  pop-profile --dev            # Switch to Dev profile"
echo "  pop-profile --work           # Switch to Work profile"
echo "  pop-profile --home           # Switch to Home profile"
echo "  pop-profile --travel         # Switch to Travel profile (alias: --secure)"
echo "  pop-profile --reset          # Reset all settings to factory defaults"
echo ""
echo "Service Status:"
echo "  systemctl status pop-profile-daemon.service"
echo "  systemctl --user status pop-profile-applet.service"
