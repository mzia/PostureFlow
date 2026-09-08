#!/usr/bin/env bash
# ==============================================================================
# Debian Package Builder for pop-profile (Phase 2)
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== Building Debian Package for pop-profile ===${NC}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

VERSION="1.0.0"
ARCH="amd64"
PKG_NAME="pop-profile"
DIST_DIR="$REPO_ROOT/dist"
STAGE_DIR="$REPO_ROOT/target/debian/${PKG_NAME}_${VERSION}_${ARCH}"

# 1. Build Rust release binary if not present
echo "[*] Ensuring Rust release binary is compiled..."
if [ ! -f "$REPO_ROOT/target/release/pop-profile-daemon" ]; then
    if command -v cargo >/dev/null 2>&1; then
        cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
    elif [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
        cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
    else
        echo -e "${RED}[-] cargo not found. Please install Rust toolchain first.${NC}"
        exit 1
    fi
fi

# 2. Prepare staging directory
echo "[*] Staging package filesystem layout..."
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/DEBIAN"
mkdir -p "$STAGE_DIR/usr/bin"
mkdir -p "$STAGE_DIR/lib/systemd/system"
mkdir -p "$STAGE_DIR/usr/share/polkit-1/actions"
mkdir -p "$STAGE_DIR/usr/share/dbus-1/system.d"
mkdir -p "$STAGE_DIR/usr/share/man/man1"
mkdir -p "$STAGE_DIR/usr/share/bash-completion/completions"
mkdir -p "$STAGE_DIR/usr/share/zsh/vendor-completions"
mkdir -p "$STAGE_DIR/etc/apt/apt.conf.d"
mkdir -p "$DIST_DIR"

# 3. Copy Binaries
install -m 755 "$REPO_ROOT/bin/pop-profile" "$STAGE_DIR/usr/bin/pop-profile"
install -m 755 "$REPO_ROOT/target/release/pop-profile-daemon" "$STAGE_DIR/usr/bin/pop-profile-daemon"

# 4. Copy Service, Polkit, D-Bus
install -m 644 "$REPO_ROOT/data/pop-profile-daemon.service" "$STAGE_DIR/lib/systemd/system/pop-profile-daemon.service"
install -m 644 "$REPO_ROOT/data/io.github.mzia.PopProfile.policy" "$STAGE_DIR/usr/share/polkit-1/actions/io.github.mzia.PopProfile.policy"
install -m 644 "$REPO_ROOT/data/io.github.mzia.PopProfile.conf" "$STAGE_DIR/usr/share/dbus-1/system.d/io.github.mzia.PopProfile.conf"

# 5. Copy Man Page (gzipped)
gzip -c -9 "$REPO_ROOT/man/pop-profile.1" > "$STAGE_DIR/usr/share/man/man1/pop-profile.1.gz"
chmod 644 "$STAGE_DIR/usr/share/man/man1/pop-profile.1.gz"

# 6. Copy Completions
install -m 644 "$REPO_ROOT/completions/pop-profile.bash" "$STAGE_DIR/usr/share/bash-completion/completions/pop-profile"
install -m 644 "$REPO_ROOT/completions/pop-profile.zsh" "$STAGE_DIR/usr/share/zsh/vendor-completions/_pop-profile"

# 7. Copy APT Post-Upgrade Hook
cat << 'EOF' > "$STAGE_DIR/etc/apt/apt.conf.d/99-popos-profile-health"
// Automatically maintain Pop!_OS security profiles after package updates
DPkg::Post-Invoke { "if [ -x /usr/bin/pop-profile ]; then /usr/bin/pop-profile >/dev/null 2>&1 || true; fi"; };
EOF
chmod 644 "$STAGE_DIR/etc/apt/apt.conf.d/99-popos-profile-health"

# 8. Create DEBIAN/control
cat << EOF > "$STAGE_DIR/DEBIAN/control"
Package: ${PKG_NAME}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: M. Zia <https://github.com/mzia/pop-profile-manager>
Depends: ufw (>= 0.36), dbus, polkitd | policykit-1
Section: utils
Priority: optional
Homepage: https://github.com/mzia/pop-profile-manager
Description: Context-aware security, developer, and lifestyle profile manager
 pop-profile dynamically bridges the gap between paranoid security,
 frictionless software engineering, and casual entertainment on Pop!_OS
 and Ubuntu laptops. Includes a native Rust D-Bus daemon and Polkit policy.
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
            systemctl enable pop-profile-daemon.service >/dev/null 2>&1 || true
            systemctl restart pop-profile-daemon.service >/dev/null 2>&1 || true
        fi

        # Update man database
        if which mandb >/dev/null 2>&1; then
            mandb -q >/dev/null 2>&1 || true
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
            systemctl stop pop-profile-daemon.service >/dev/null 2>&1 || true
            systemctl disable pop-profile-daemon.service >/dev/null 2>&1 || true
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
        rm -f /etc/popos-security-profile
        rm -f /etc/sysctl.d/99-popos-security.conf
        rm -f /etc/security/limits.d/99-popos-security.conf
        if which sysctl >/dev/null 2>&1; then
            sysctl --system >/dev/null 2>&1 || true
        fi
        ;;
    remove)
        if [ -d /run/systemd/system ]; then
            systemctl --system daemon-reload >/dev/null 2>&1 || true
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
