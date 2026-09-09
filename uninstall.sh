#!/usr/bin/env bash
# ==============================================================================
# pop-profile Uninstaller
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

echo -e "\n${BOLD}${CYAN}=== Uninstalling pop-profile ===${NC}"

# Stop systemd service if running
if command -v systemctl >/dev/null 2>&1; then
    systemctl stop pop-profile-daemon 2>/dev/null || true
    systemctl disable pop-profile-daemon 2>/dev/null || true
fi

if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    USER_UID=$(id -u "$SUDO_USER" 2>/dev/null || echo "1000")
    if [ -d "/run/user/$USER_UID" ]; then
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user stop pop-profile-applet.service 2>/dev/null || true
        sudo -u "$SUDO_USER" XDG_RUNTIME_DIR="/run/user/$USER_UID" systemctl --user disable pop-profile-applet.service 2>/dev/null || true
    fi
fi

# Revert system settings to Pop!_OS factory defaults
if [ -x /usr/bin/pop-profile ]; then
    echo "[*] Reverting firewall and sysctl settings to Pop!_OS defaults..."
    /usr/bin/pop-profile --reset || true
elif [ -x /usr/local/bin/pop-profile ]; then
    echo "[*] Reverting firewall and sysctl settings to Pop!_OS defaults..."
    /usr/local/bin/pop-profile --reset || true
fi

# Remove installed files
echo "[*] Removing installed files..."
rm -f /usr/local/bin/pop-profile /usr/bin/pop-profile
rm -f /usr/local/bin/pop-profile-daemon /usr/bin/pop-profile-daemon
rm -f /usr/local/bin/pop-profile-applet /usr/bin/pop-profile-applet
rm -f /usr/local/bin/cosmic-applet-popprofile /usr/bin/cosmic-applet-popprofile
rm -f /usr/share/applications/io.github.mzia.PopProfile.Applet.desktop
rm -f /usr/lib/systemd/user/pop-profile-applet.service
rm -f /usr/share/polkit-1/actions/io.github.mzia.PopProfile.policy
rm -f /usr/share/dbus-1/system.d/io.github.mzia.PopProfile.conf
rm -f /etc/systemd/system/pop-profile-daemon.service
rm -f /usr/local/share/man/man1/pop-profile.1
rm -f /etc/bash_completion.d/pop-profile
rm -f /usr/share/zsh/vendor-completions/_pop-profile
rm -f /etc/apt/apt.conf.d/99-popos-profile-health
rm -f /etc/popos-security-profile
rm -f /etc/sysctl.d/99-popos-security.conf
rm -f /etc/security/limits.d/99-popos-security.conf

if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload 2>/dev/null || true
    systemctl reload dbus 2>/dev/null || true
fi

if command -v mandb >/dev/null 2>&1; then
    mandb -q >/dev/null 2>&1 || true
fi

echo -e "${GREEN}${BOLD}[✔] pop-profile & daemon uninstalled and system restored to factory defaults.${NC}"
