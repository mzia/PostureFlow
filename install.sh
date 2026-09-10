#!/usr/bin/env bash
# ==============================================================================
# PostureFlow Installer (CLI + Rust D-Bus Daemon + Applet + GUI + Polkit)
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
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
    echo -e "\n${BOLD}${CYAN}=== Installing PostureFlow in DEVELOPER MODE ===${NC}"
    echo -e "${YELLOW}[*] Live workspace symlinks enabled. Recompiling with 'cargo build --release' will immediately reflect system-wide.${NC}"
else
    echo -e "\n${BOLD}${CYAN}=== Installing PostureFlow & Rust D-Bus Daemon ===${NC}"
fi

# 1. Install CLI & Rust Binaries
if [ "$DEV_MODE" -eq 1 ]; then
    echo "[*] Linking workspace CLI and binaries to /usr/local/bin and /usr/bin..."
    ln -sf "$SCRIPT_DIR/bin/postureflow" /usr/local/bin/postureflow
    ln -sf /usr/local/bin/postureflow /usr/bin/postureflow

    if [ -f "$SCRIPT_DIR/target/release/postureflow-daemon" ]; then
        ln -sf "$SCRIPT_DIR/target/release/postureflow-daemon" /usr/local/bin/postureflow-daemon
        ln -sf /usr/local/bin/postureflow-daemon /usr/bin/postureflow-daemon
    fi
    if [ -f "$SCRIPT_DIR/target/release/postureflow-applet" ]; then
        ln -sf "$SCRIPT_DIR/target/release/postureflow-applet" /usr/local/bin/postureflow-applet
        ln -sf /usr/local/bin/postureflow-applet /usr/bin/postureflow-applet
        ln -sf postureflow-applet /usr/bin/cosmic-applet-postureflow
        ln -sf postureflow-applet /usr/local/bin/cosmic-applet-postureflow
    fi
    if [ -f "$SCRIPT_DIR/target/release/postureflow-gui" ]; then
        ln -sf "$SCRIPT_DIR/target/release/postureflow-gui" /usr/local/bin/postureflow-gui
        ln -sf /usr/local/bin/postureflow-gui /usr/bin/postureflow-gui
    fi
else
    echo "[*] Installing CLI to /usr/local/bin/postureflow and /usr/bin/postureflow..."
    install -m 755 "$SCRIPT_DIR/bin/postureflow" /usr/local/bin/postureflow
    ln -sf /usr/local/bin/postureflow /usr/bin/postureflow

    if [ -f "$SCRIPT_DIR/target/release/postureflow-daemon" ]; then
        echo "[*] Installing Rust daemon..."
        install -m 755 "$SCRIPT_DIR/target/release/postureflow-daemon" /usr/local/bin/postureflow-daemon
        ln -sf /usr/local/bin/postureflow-daemon /usr/bin/postureflow-daemon
    fi
    if [ -f "$SCRIPT_DIR/target/release/postureflow-applet" ]; then
        echo "[*] Installing Status Bar / COSMIC Applet..."
        install -m 755 "$SCRIPT_DIR/target/release/postureflow-applet" /usr/local/bin/postureflow-applet
        ln -sf /usr/local/bin/postureflow-applet /usr/bin/postureflow-applet
        ln -sf postureflow-applet /usr/bin/cosmic-applet-postureflow
        ln -sf postureflow-applet /usr/local/bin/cosmic-applet-postureflow
    fi
    if [ -f "$SCRIPT_DIR/target/release/postureflow-gui" ]; then
        echo "[*] Installing PostureFlow GUI..."
        install -m 755 "$SCRIPT_DIR/target/release/postureflow-gui" /usr/local/bin/postureflow-gui
        ln -sf /usr/local/bin/postureflow-gui /usr/bin/postureflow-gui
    fi
fi

# 2. Ensure custom profiles directory exists
mkdir -p /etc/postureflow/profiles.d

# 3. Install Desktop Entries & Icons
install -d /usr/share/applications
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.desktop" ]; then
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.desktop" /usr/share/applications/io.github.mzia.PostureFlow.desktop
fi
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.Applet.desktop" ]; then
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.Applet.desktop" /usr/share/applications/io.github.mzia.PostureFlow.Applet.desktop
fi

if [ -f "$SCRIPT_DIR/data/icons/io.github.mzia.PostureFlow.svg" ]; then
    install -d /usr/share/icons/hicolor/scalable/apps
    install -m 644 "$SCRIPT_DIR/data/icons/io.github.mzia.PostureFlow.svg" /usr/share/icons/hicolor/scalable/apps/io.github.mzia.PostureFlow.svg
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi

# 4. Install User Applet Service
if [ -f "$SCRIPT_DIR/data/postureflow-applet.service" ]; then
    echo "[*] Installing systemd user applet service..."
    install -d /usr/lib/systemd/user
    install -m 644 "$SCRIPT_DIR/data/postureflow-applet.service" /usr/lib/systemd/user/postureflow-applet.service
fi

# 5. Install Polkit Policy (Passwordless desktop profile switching)
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.policy" ]; then
    echo "[*] Installing Polkit policy to /usr/share/polkit-1/actions/..."
    install -d /usr/share/polkit-1/actions
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.policy" /usr/share/polkit-1/actions/io.github.mzia.PostureFlow.policy
fi

# 6. Install D-Bus System Bus Configuration
if [ -f "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.conf" ]; then
    echo "[*] Installing D-Bus system configuration..."
    install -d /usr/share/dbus-1/system.d
    install -m 644 "$SCRIPT_DIR/data/io.github.mzia.PostureFlow.conf" /usr/share/dbus-1/system.d/io.github.mzia.PostureFlow.conf
    # Reload D-Bus configuration
    if command -v systemctl >/dev/null 2>&1; then
        systemctl reload dbus 2>/dev/null || true
    fi
fi

# 7. Install Systemd Service Unit
if [ -f "$SCRIPT_DIR/data/postureflow-daemon.service" ]; then
    echo "[*] Installing systemd daemon service..."
    install -m 644 "$SCRIPT_DIR/data/postureflow-daemon.service" /etc/systemd/system/postureflow-daemon.service
    if command -v systemctl >/dev/null 2>&1; then
        echo "[*] Enabling and restarting postureflow-daemon service..."
        systemctl daemon-reload 2>/dev/null || true
        systemctl enable postureflow-daemon.service 2>/dev/null || true
        systemctl restart postureflow-daemon.service 2>/dev/null || true
    fi
fi

# 8. Install Man page
echo "[*] Installing manual page to /usr/local/share/man/man1/postureflow.1..."
install -d /usr/local/share/man/man1
if [ -f "$SCRIPT_DIR/man/postureflow.1" ]; then
    install -m 644 "$SCRIPT_DIR/man/postureflow.1" /usr/local/share/man/man1/postureflow.1
fi
if command -v mandb >/dev/null 2>&1; then
    mandb -q >/dev/null 2>&1 || true
fi

# 9. Install Completions
if [ -d /etc/bash_completion.d ]; then
    echo "[*] Installing bash completion..."
    install -m 644 "$SCRIPT_DIR/completions/postureflow.bash" /etc/bash_completion.d/postureflow
fi
if [ -d /usr/share/zsh/vendor-completions ]; then
    echo "[*] Installing zsh completion..."
    install -m 644 "$SCRIPT_DIR/completions/postureflow.zsh" /usr/share/zsh/vendor-completions/_postureflow
fi

# 10. Install APT post-upgrade hook for persistence
echo "[*] Registering APT post-upgrade maintenance hook..."
mkdir -p /etc/apt/apt.conf.d/
cat << 'EOF' > /etc/apt/apt.conf.d/99-postureflow-health
// Automatically maintain PostureFlow security & power profiles after package updates
DPkg::Post-Invoke { "if [ -x /usr/bin/postureflow ]; then /usr/bin/postureflow >/dev/null 2>&1 || true; elif [ -x /usr/local/bin/postureflow ]; then /usr/local/bin/postureflow >/dev/null 2>&1 || true; fi"; };
EOF
chmod 644 /etc/apt/apt.conf.d/99-postureflow-health
# Clean up old hook if present
rm -f /etc/apt/apt.conf.d/99-popos-profile-health

# 11. Configure User Applet Service for active desktop user
if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    USER_UID=$(id -u "$SUDO_USER" 2>/dev/null || echo "1000")
    if [ -d "/run/user/$USER_UID" ]; then
        echo "[*] Activating PostureFlow Applet user service for '$SUDO_USER'..."
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user daemon-reload >/dev/null 2>&1 || true
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user enable --now postureflow-applet.service >/dev/null 2>&1 || true
    fi
fi

# 12. Developer Mode profile activation
if [ "$DEV_MODE" -eq 1 ]; then
    echo "[*] Developer Mode: Activating Developer / Coding profile..."
    if [ -x /usr/bin/postureflow ]; then
        /usr/bin/postureflow --dev || true
    elif [ -x /usr/local/bin/postureflow ]; then
        /usr/local/bin/postureflow --dev || true
    fi
fi

# 13. Run safety verification
echo "[*] Running verification tests..."
bash "$SCRIPT_DIR/tests/test_safety.sh"

echo ""
echo -e "${GREEN}${BOLD}[✔] PostureFlow & Rust D-Bus components installed successfully!${NC}"
if [ "$DEV_MODE" -eq 1 ]; then
    echo -e "${CYAN}Developer mode is ACTIVE:${NC}"
    echo "  • Live workspace binaries linked"
    echo "  • Inotify watches maximized (524,288)"
    echo "  • Core dumps enabled for debugging"
    echo "  • Container & dev ports opened"
    echo "  • Status bar & COSMIC panel applet running"
fi
echo "Usage:"
echo "  postureflow --status         # Check active posture"
echo "  postureflow --dev            # Switch to Dev profile"
echo "  postureflow --work           # Switch to Work profile"
echo "  postureflow --home           # Switch to Home profile"
echo "  postureflow --travel         # Switch to Travel profile (alias: --secure)"
echo "  postureflow --reset          # Reset all settings to factory defaults"
echo ""
echo "Service Status:"
echo "  systemctl status postureflow-daemon.service"
echo "  systemctl --user status postureflow-applet.service"
