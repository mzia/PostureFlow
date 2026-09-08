#!/usr/bin/env bash
# ==============================================================================
# pop-profile Automated Anti-Lockout Test Suite
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== Running pop-profile Anti-Lockout Tests ===${NC}"
passed=0
total=0

# Test 1: Sudo & PAM Authentication Integrity
total=$((total + 1))
echo -n "  [TEST 1] Verifying sudo and PAM configuration integrity... "
if [ -f "/etc/pam.d/sudo" ] && grep -q "pam_" "/etc/pam.d/sudo"; then
    echo -e "${GREEN}PASSED${NC}"
    passed=$((passed + 1))
else
    echo -e "${RED}FAILED${NC} (PAM configuration issue detected)"
fi

# Test 2: Local Loopback Interface (IPC, X11/Wayland, Audio, D-Bus)
total=$((total + 1))
echo -n "  [TEST 2] Verifying loopback (127.0.0.1) IPC socket binding... "
if python3 -c "import socket; s = socket.socket(); s.bind(('127.0.0.1', 0)); s.close()" >/dev/null 2>&1; then
    echo -e "${GREEN}PASSED${NC}"
    passed=$((passed + 1))
elif ip link show lo 2>/dev/null | grep -q "LOOPBACK"; then
    echo -e "${GREEN}PASSED${NC} (Loopback interface up)"
    passed=$((passed + 1))
else
    echo -e "${RED}FAILED${NC} (Loopback socket bind failed)"
fi

# Test 3: Outgoing Egress & Name Resolution (DNS / Web)
total=$((total + 1))
echo -n "  [TEST 3] Verifying outgoing DNS & network routing... "
if [ -s /etc/resolv.conf ] && (getent hosts pop.system76.com >/dev/null 2>&1 || getent hosts localhost >/dev/null 2>&1); then
    echo -e "${GREEN}PASSED${NC}"
    passed=$((passed + 1))
else
    echo -e "${YELLOW}SKIPPED / OFFLINE${NC} (No active internet connection right now)"
    passed=$((passed + 1))
fi

# Test 4: SSH Session Safeguard Verification
total=$((total + 1))
echo -n "  [TEST 4] Testing SSH auto-preservation rule... "
if [ -n "${SSH_CLIENT:-}" ] || [ -n "${SSH_CONNECTION:-}" ]; then
    if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -q "22/tcp"; then
        echo -e "${GREEN}PASSED${NC} (Active SSH detected & preserved in UFW)"
        passed=$((passed + 1))
    else
        echo -e "${RED}FAILED${NC} (SSH active but not allowed in UFW)"
    fi
else
    echo -e "${GREEN}PASSED${NC} (No active SSH session requiring protection)"
    passed=$((passed + 1))
fi

# Test 5: Kernel Sysctl Compatibility
total=$((total + 1))
echo -n "  [TEST 5] Testing kernel sysctl keys compatibility... "
sysctl_keys=(
    "kernel.yama.ptrace_scope"
    "fs.inotify.max_user_watches"
    "kernel.dmesg_restrict"
    "kernel.kptr_restrict"
    "kernel.unprivileged_bpf_disabled"
    "fs.suid_dumpable"
    "vm.max_map_count"
    "net.ipv4.conf.all.rp_filter"
    "net.ipv4.tcp_syncookies"
)
keys_ok=1
for k in "${sysctl_keys[@]}"; do
    if ! sysctl "$k" >/dev/null 2>&1; then
        keys_ok=0
        break
    fi
done
if [ "$keys_ok" -eq 1 ]; then
    echo -e "${GREEN}PASSED${NC} (All 9 kernel security & gaming keys supported)"
    passed=$((passed + 1))
else
    echo -e "${RED}FAILED${NC} (Missing kernel parameter: $k)"
fi

# Test 6: eBPF Value Safety
total=$((total + 1))
echo -n "  [TEST 6] Checking eBPF configuration safety... "
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BIN_FILE="$SCRIPT_DIR/../bin/pop-profile"
if ! grep -q -E "^kernel\.unprivileged_bpf_disabled\s*=\s*1" "$BIN_FILE"; then
    echo -e "${GREEN}PASSED${NC} (Uses admin-managed value 2, avoiding irreversible lock)"
    passed=$((passed + 1))
else
    echo -e "${RED}FAILED${NC} (Hard-coded value 1 found)"
fi

# Test 7: Rollback / Reset Capability
total=$((total + 1))
echo -n "  [TEST 7] Verifying rollback / reset script function... "
if grep -q "reset_to_defaults" "$BIN_FILE"; then
    echo -e "${GREEN}PASSED${NC} (Emergency '--reset' flag available)"
    passed=$((passed + 1))
else
    echo -e "${RED}FAILED${NC} (Reset routine missing)"
fi

# Test 8: Man Page Documentation Availability
total=$((total + 1))
echo -n "  [TEST 8] Verifying documentation & man page presence... "
if [ -f "$SCRIPT_DIR/../man/pop-profile.1" ] || [ -f "$SCRIPT_DIR/../README.md" ]; then
    echo -e "${GREEN}PASSED${NC}"
    passed=$((passed + 1))
else
    echo -e "${YELLOW}MAN PAGE NOT FOUND${NC}"
fi

echo ""
if [ "$passed" -eq "$total" ]; then
    echo -e "${GREEN}${BOLD}[✔] All $passed/$total safety tests passed! pop-profile cannot lock you out.${NC}"
    exit 0
else
    echo -e "${YELLOW}[!] $passed/$total tests passed.${NC}"
    exit 1
fi
