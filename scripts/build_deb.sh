#!/usr/bin/env bash
# ==============================================================================
# Debian Package Builder for PostureFlow
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== Building Debian Package for PostureFlow ===${NC}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

VERSION="1.0.0"
ARCH="amd64"
PKG_NAME="postureflow"
DIST_DIR="$REPO_ROOT/dist"
STAGE_DIR="$REPO_ROOT/target/debian/${PKG_NAME}_${VERSION}_${ARCH}"

# 1. Build Rust release binaries
echo "[*] Compiling latest Rust release binaries..."
if command -v cargo >/dev/null 2>&1; then
    cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
elif [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
    cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
else
    echo -e "${RED}[-] cargo not found. Please install Rust toolchain first.${NC}"
    exit 1
fi

# 2. Prepare staging directory
echo "[*] Staging package filesystem layout..."
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/DEBIAN"
mkdir -p "$STAGE_DIR/usr/bin"
mkdir -p "$STAGE_DIR/lib/systemd/system"
mkdir -p "$STAGE_DIR/usr/lib/systemd/user"
mkdir -p "$STAGE_DIR/usr/share/applications"
mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"
mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/status"
mkdir -p "$STAGE_DIR/usr/share/polkit-1/actions"
mkdir -p "$STAGE_DIR/usr/share/dbus-1/system.d"
mkdir -p "$STAGE_DIR/usr/share/man/man1"
mkdir -p "$STAGE_DIR/usr/share/bash-completion/completions"
mkdir -p "$STAGE_DIR/usr/share/zsh/vendor-completions"
mkdir -p "$STAGE_DIR/etc/apt/apt.conf.d"
mkdir -p "$STAGE_DIR/etc/postureflow/profiles.d"
mkdir -p "$DIST_DIR"

# 3. Copy Binaries & Applet Symlinks
install -m 755 "$REPO_ROOT/bin/postureflow" "$STAGE_DIR/usr/bin/postureflow"

install -m 755 "$REPO_ROOT/target/release/postureflow-daemon" "$STAGE_DIR/usr/bin/postureflow-daemon"

install -m 755 "$REPO_ROOT/target/release/postureflow-applet" "$STAGE_DIR/usr/bin/postureflow-applet"
ln -sf postureflow-applet "$STAGE_DIR/usr/bin/cosmic-applet-postureflow"

install -m 755 "$REPO_ROOT/target/release/postureflow-gui" "$STAGE_DIR/usr/bin/postureflow-gui"

if command -v strip >/dev/null 2>&1; then
    strip --strip-unneeded "$STAGE_DIR/usr/bin/postureflow-daemon"
    strip --strip-unneeded "$STAGE_DIR/usr/bin/postureflow-applet"
    strip --strip-unneeded "$STAGE_DIR/usr/bin/postureflow-gui"
fi

# 4. Copy Service, Polkit, D-Bus, Desktop Entry
install -m 644 "$REPO_ROOT/data/postureflow-daemon.service" "$STAGE_DIR/lib/systemd/system/postureflow-daemon.service"

install -m 644 "$REPO_ROOT/data/postureflow-applet.service" "$STAGE_DIR/usr/lib/systemd/user/postureflow-applet.service"

install -m 644 "$REPO_ROOT/data/io.github.mzia.PostureFlow.desktop" "$STAGE_DIR/usr/share/applications/io.github.mzia.PostureFlow.desktop"
install -m 644 "$REPO_ROOT/data/io.github.mzia.PostureFlow.Applet.desktop" "$STAGE_DIR/usr/share/applications/io.github.mzia.PostureFlow.Applet.desktop"

install -m 644 "$REPO_ROOT/data/icons/io.github.mzia.PostureFlow.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/io.github.mzia.PostureFlow.svg"
install -m 644 "$REPO_ROOT/data/icons/postureflow-home-symbolic.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/status/postureflow-home-symbolic.svg"
install -m 644 "$REPO_ROOT/data/icons/postureflow-work-symbolic.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/status/postureflow-work-symbolic.svg"
install -m 644 "$REPO_ROOT/data/icons/postureflow-dev-symbolic.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/status/postureflow-dev-symbolic.svg"
install -m 644 "$REPO_ROOT/data/icons/postureflow-travel-symbolic.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/status/postureflow-travel-symbolic.svg"
install -m 644 "$REPO_ROOT/data/icons/postureflow-symbolic.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/status/postureflow-symbolic.svg"

install -m 644 "$REPO_ROOT/data/io.github.mzia.PostureFlow.policy" "$STAGE_DIR/usr/share/polkit-1/actions/io.github.mzia.PostureFlow.policy"
install -m 644 "$REPO_ROOT/data/io.github.mzia.PostureFlow.conf" "$STAGE_DIR/usr/share/dbus-1/system.d/io.github.mzia.PostureFlow.conf"

# 5. Copy Man Page (gzipped)
gzip -c -9 "$REPO_ROOT/man/postureflow.1" > "$STAGE_DIR/usr/share/man/man1/postureflow.1.gz"
chmod 644 "$STAGE_DIR/usr/share/man/man1/postureflow.1.gz"

# 6. Copy Completions
install -m 644 "$REPO_ROOT/completions/postureflow.bash" "$STAGE_DIR/usr/share/bash-completion/completions/postureflow"

install -m 644 "$REPO_ROOT/completions/postureflow.zsh" "$STAGE_DIR/usr/share/zsh/vendor-completions/_postureflow"

# 7. Copy APT Post-Upgrade Hook
cat << 'EOF' > "$STAGE_DIR/etc/apt/apt.conf.d/99-postureflow-health"
// Automatically maintain PostureFlow security & power profiles after package updates
DPkg::Post-Invoke { "if [ -x /usr/bin/postureflow ]; then /usr/bin/postureflow >/dev/null 2>&1 || true; fi"; };
EOF
chmod 644 "$STAGE_DIR/etc/apt/apt.conf.d/99-postureflow-health"

# 8. Create DEBIAN/control
cat << EOF > "$STAGE_DIR/DEBIAN/control"
Package: ${PKG_NAME}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: M. Zia <https://github.com/mzia/PostureFlow>
Depends: ufw (>= 0.36), dbus, polkitd | policykit-1
Conflicts: pop-profile (<= ${VERSION})
Replaces: pop-profile (<= ${VERSION})
Section: utils
Priority: optional
Homepage: https://github.com/mzia/PostureFlow
Description: Dynamic security posture, hardware power, and lifestyle workflow orchestrator
 PostureFlow dynamically bridges the gap between paranoid security,
 frictionless software engineering, and casual entertainment on
 Linux and Pop!_OS laptops. Includes a native Rust D-Bus daemon,
 status bar applet, GUI settings, and Polkit policy.
EOF
chmod 644 "$STAGE_DIR/DEBIAN/control"

# 9. Create DEBIAN/postinst
cat << 'EOF' > "$STAGE_DIR/DEBIAN/postinst"
#!/bin/sh
set -e

case "$1" in
    configure)
        # Reload systemd and D-Bus
        if [ -d /run/systemd/system ]; then
            systemctl --system daemon-reload >/dev/null 2>&1 || true
            systemctl reload dbus >/dev/null 2>&1 || true
            systemctl enable postureflow-daemon.service >/dev/null 2>&1 || true
            systemctl restart postureflow-daemon.service >/dev/null 2>&1 || true
        fi

        # Update man, desktop, and icon databases
        if which mandb >/dev/null 2>&1; then
            mandb -q >/dev/null 2>&1 || true
        fi
        if which update-desktop-database >/dev/null 2>&1; then
            update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
        fi
        if which gtk-update-icon-cache >/dev/null 2>&1; then
            gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || true
        fi
        ;;
esac
exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/postinst"

# 10. Create DEBIAN/prerm
cat << 'EOF' > "$STAGE_DIR/DEBIAN/prerm"
#!/bin/sh
set -e

case "$1" in
    remove|deconfigure)
        if [ -d /run/systemd/system ]; then
            systemctl stop postureflow-daemon.service >/dev/null 2>&1 || true
            systemctl disable postureflow-daemon.service >/dev/null 2>&1 || true
        fi
        ;;
esac
exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/prerm"

# 11. Create DEBIAN/postrm
cat << 'EOF' > "$STAGE_DIR/DEBIAN/postrm"
#!/bin/sh
set -e

case "$1" in
    purge)
        # Revert system settings to factory defaults
        if which ufw >/dev/null 2>&1; then
            ufw --force disable >/dev/null 2>&1 || true
            ufw --force reset >/dev/null 2>&1 || true
        fi
        rm -f /etc/postureflow-state
        rm -f /etc/popos-security-profile
        rm -f /etc/sysctl.d/99-postureflow.conf
        rm -f /etc/sysctl.d/99-popos-security.conf
        rm -f /etc/security/limits.d/99-postureflow.conf
        rm -f /etc/security/limits.d/99-popos-security.conf
        if which sysctl >/dev/null 2>&1; then
            sysctl --system >/dev/null 2>&1 || true
        fi
        if which update-desktop-database >/dev/null 2>&1; then
            update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
        fi
        if which gtk-update-icon-cache >/dev/null 2>&1; then
            gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || true
        fi
        ;;
    remove)
        if [ -d /run/systemd/system ]; then
            systemctl --system daemon-reload >/dev/null 2>&1 || true
        fi
        if which update-desktop-database >/dev/null 2>&1; then
            update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
        fi
        if which gtk-update-icon-cache >/dev/null 2>&1; then
            gtk-update-icon-cache -q /usr/share/icons/hicolor >/dev/null 2>&1 || true
        fi
        ;;
esac
exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/postrm"

# 12. Build the .deb package
DEB_FILE="$DIST_DIR/${PKG_NAME}_${VERSION}_${ARCH}.deb"
echo "[*] Packing .deb archive..."
dpkg-deb --build --root-owner-group "$STAGE_DIR" "$DEB_FILE"

echo -e "${GREEN}${BOLD}[✔] Package successfully created at:${NC} $DEB_FILE"
echo ""
echo "Verify package with:"
echo "  dpkg-deb -I $DEB_FILE"
echo "  dpkg-deb -c $DEB_FILE"
echo "Install with:"
echo "  sudo dpkg -i $DEB_FILE"
