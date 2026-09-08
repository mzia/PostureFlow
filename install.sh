#!/usr/bin/env bash
# ==============================================================================
# pop-profile Installer
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

echo -e "\n${BOLD}${CYAN}=== Installing pop-profile ===${NC}"

# 1. Install binary
echo "[*] Installing binary to /usr/local/bin/pop-profile..."
install -m 755 "$SCRIPT_DIR/bin/pop-profile" /usr/local/bin/pop-profile

# 2. Install man page
echo "[*] Installing manual page to /usr/local/share/man/man1/pop-profile.1..."
install -d /usr/local/share/man/man1
install -m 644 "$SCRIPT_DIR/man/pop-profile.1" /usr/local/share/man/man1/pop-profile.1
if command -v mandb >/dev/null 2>&1; then
    mandb -q >/dev/null 2>&1 || true
fi

# 3. Install completions
if [ -d /etc/bash_completion.d ]; then
    echo "[*] Installing bash completion..."
    install -m 644 "$SCRIPT_DIR/completions/pop-profile.bash" /etc/bash_completion.d/pop-profile
fi
if [ -d /usr/share/zsh/vendor-completions ]; then
    echo "[*] Installing zsh completion..."
    install -m 644 "$SCRIPT_DIR/completions/pop-profile.zsh" /usr/share/zsh/vendor-completions/_pop-profile
fi

# 4. Install APT post-upgrade hook for persistence
echo "[*] Registering APT post-upgrade maintenance hook..."
mkdir -p /etc/apt/apt.conf.d/
cat << 'EOF' > /etc/apt/apt.conf.d/99-popos-profile-health
// Automatically maintain Pop!_OS security profiles after package updates
DPkg::Post-Invoke { "if [ -x /usr/local/bin/pop-profile ]; then /usr/local/bin/pop-profile >/dev/null 2>&1 || true; fi"; };
EOF
chmod 644 /etc/apt/apt.conf.d/99-popos-profile-health

# 5. Run safety verification
echo "[*] Running verification tests..."
bash "$SCRIPT_DIR/tests/test_safety.sh"

echo ""
echo -e "${GREEN}${BOLD}[✔] pop-profile installed successfully!${NC}"
echo "Usage:"
echo "  sudo pop-profile --home      # Streaming & Gaming"
echo "  sudo pop-profile --work      # Office & Corporate VPN"
echo "  sudo pop-profile --dev       # Coding & Debugging"
echo "  sudo pop-profile --secure    # Travel & Lockdown"
echo "  pop-profile --status         # Check active posture"
echo "  man pop-profile              # Read manual page"
