#!/usr/bin/env bash
# ==============================================================================
# PostureFlow Uninstaller
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}[-] Please run with sudo: sudo ./uninstall.sh${NC}"
    exit 1
fi

echo -e "\n${BOLD}${CYAN}=== Uninstalling PostureFlow ===${NC}"

# Stop systemd services if running
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop postureflow-daemon 2>/dev/null || true
    systemctl disable postureflow-daemon 2>/dev/null || true
    systemctl stop pop-profile-daemon 2>/dev/null || true
    systemctl disable pop-profile-daemon 2>/dev/null || true
fi

if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    USER_UID=$(id -u "$SUDO_USER" 2>/dev/null || echo "1000")
    if [ -d "/run/user/$USER_UID" ]; then
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user stop postureflow-applet.service 2>/dev/null || true
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user disable postureflow-applet.service 2>/dev/null || true
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user stop pop-profile-applet.service 2>/dev/null || true
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user disable pop-profile-applet.service 2>/dev/null || true
    fi
fi

# Revert system settings to factory defaults
if [ -x /usr/bin/postureflow ]; then
    echo "[*] Reverting firewall and sysctl settings to Pop!_OS defaults..."
    /usr/bin/postureflow --reset || true
elif [ -x /usr/local/bin/postureflow ]; then
    echo "[*] Reverting firewall and sysctl settings to Pop!_OS defaults..."
    /usr/local/bin/postureflow --reset || true
elif [ -x /usr/bin/pop-profile ]; then
    echo "[*] Reverting firewall and sysctl settings to Pop!_OS defaults..."
    /usr/bin/pop-profile --reset || true
fi

# Remove installed files
echo "[*] Removing installed binaries and symlinks..."
rm -f /usr/local/bin/postureflow /usr/bin/postureflow
rm -f /usr/local/bin/pop-profile /usr/bin/pop-profile
rm -f /usr/local/bin/postureflow-daemon /usr/bin/postureflow-daemon
rm -f /usr/local/bin/pop-profile-daemon /usr/bin/pop-profile-daemon
rm -f /usr/local/bin/postureflow-applet /usr/bin/postureflow-applet
rm -f /usr/local/bin/pop-profile-applet /usr/bin/pop-profile-applet
rm -f /usr/local/bin/postureflow-gui /usr/bin/postureflow-gui
rm -f /usr/local/bin/pop-profile-gui /usr/bin/pop-profile-gui
rm -f /usr/local/bin/cosmic-applet-postureflow /usr/bin/cosmic-applet-postureflow
rm -f /usr/local/bin/cosmic-applet-popprofile /usr/bin/cosmic-applet-popprofile

echo "[*] Removing desktop entries and icons..."
rm -f /usr/share/applications/io.github.mzia.PostureFlow.desktop
rm -f /usr/share/applications/io.github.mzia.PostureFlow.Applet.desktop
rm -f /usr/share/applications/io.github.mzia.PopProfile.Applet.desktop
rm -f /usr/share/applications/io.github.mzia.PopProfile.Settings.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/io.github.mzia.PostureFlow.svg
rm -f /usr/share/icons/hicolor/scalable/apps/io.github.mzia.PopProfile.svg

echo "[*] Removing systemd unit files..."
rm -f /usr/lib/systemd/user/postureflow-applet.service
rm -f /usr/lib/systemd/user/pop-profile-applet.service
rm -f /etc/systemd/system/postureflow-daemon.service
rm -f /etc/systemd/system/pop-profile-daemon.service

echo "[*] Removing Polkit and D-Bus configurations..."
rm -f /usr/share/polkit-1/actions/io.github.mzia.PostureFlow.policy
rm -f /usr/share/polkit-1/actions/io.github.mzia.PopProfile.policy
rm -f /usr/share/dbus-1/system.d/io.github.mzia.PostureFlow.conf
rm -f /usr/share/dbus-1/system.d/io.github.mzia.PopProfile.conf

echo "[*] Removing man pages and shell completions..."
rm -f /usr/local/share/man/man1/postureflow.1 /usr/local/share/man/man1/pop-profile.1
rm -f /etc/bash_completion.d/postureflow /etc/bash_completion.d/pop-profile
rm -f /usr/share/zsh/vendor-completions/_postureflow /usr/share/zsh/vendor-completions/_pop-profile

echo "[*] Removing APT hook and profile state files..."
rm -f /etc/apt/apt.conf.d/99-postureflow-health
rm -f /etc/apt/apt.conf.d/99-popos-profile-health
rm -f /etc/postureflow-state
rm -f /etc/popos-security-profile
rm -f /etc/sysctl.d/99-postureflow.conf
rm -f /etc/sysctl.d/99-popos-security.conf
rm -f /etc/security/limits.d/99-postureflow.conf
rm -f /etc/security/limits.d/99-popos-security.conf

if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload 2>/dev/null || true
    systemctl reload dbus 2>/dev/null || true
fi

if command -v mandb >/dev/null 2>&1; then
    mandb -q >/dev/null 2>&1 || true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi

echo -e "${GREEN}${BOLD}[✔] PostureFlow uninstalled and system restored to factory defaults.${NC}"
