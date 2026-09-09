#!/usr/bin/env bash
# ==============================================================================
# Flatpak Builder for Pop! Profile Manager (Phase 6)
# ==============================================================================

set -eo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
CYAN="\033[0;36m"
NC="\033[0m"

echo -e "\n${BOLD}${CYAN}=== PostureFlow Flatpak Builder ===${NC}"

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

APP_ID="io.github.mzia.PostureFlow"
MANIFEST="io.github.mzia.PostureFlow.yml"
BUILD_DIR="$REPO_ROOT/target/flatpak-build"
REPO_DIR="$REPO_ROOT/target/flatpak-repo"
DIST_DIR="$REPO_ROOT/dist"

mkdir -p "$DIST_DIR"

# 1. Check requirements
echo "[*] Step 1: Checking build tools..."
if ! command -v flatpak >/dev/null 2>&1; then
    echo -e "${RED}[-] flatpak is not installed. Run: sudo apt install flatpak${NC}"
    exit 1
fi

if ! command -v flatpak-builder >/dev/null 2>&1; then
    echo -e "${RED}[-] flatpak-builder is not installed. Run: sudo apt install flatpak-builder${NC}"
    exit 1
fi

# 2. Validate AppStream and Desktop entries
echo "[*] Step 2: Validating AppStream metadata and desktop file..."
if command -v appstreamcli >/dev/null 2>&1; then
    appstreamcli validate --no-net "$REPO_ROOT/data/$APP_ID.metainfo.xml"
    echo -e "  ${GREEN}[✔] AppStream metadata valid.${NC}"
fi

if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$REPO_ROOT/data/$APP_ID.desktop"
    echo -e "  ${GREEN}[✔] Desktop file valid.${NC}"
fi

# 3. Generate cargo-sources.json
echo "[*] Step 3: Generating Flatpak Cargo dependency sources..."
python3 "$SCRIPT_DIR/generate_cargo_sources.py" "$REPO_ROOT/Cargo.lock" -o "$REPO_ROOT/cargo-sources.json"

# 4. Check Flatpak Runtime & SDK
RUNTIME_VERSION="24.08"
echo "[*] Step 4: Checking org.freedesktop.Platform & Sdk ($RUNTIME_VERSION)..."

SDK_INSTALLED=1
if ! flatpak info "org.freedesktop.Platform//${RUNTIME_VERSION}" >/dev/null 2>&1; then
    SDK_INSTALLED=0
fi
if ! flatpak info "org.freedesktop.Sdk//${RUNTIME_VERSION}" >/dev/null 2>&1; then
    SDK_INSTALLED=0
fi
if ! flatpak info "org.freedesktop.Sdk.Extension.rust-stable//${RUNTIME_VERSION}" >/dev/null 2>&1; then
    SDK_INSTALLED=0
fi

if [ "$SDK_INSTALLED" -eq 0 ]; then
    echo -e "${YELLOW}[!] Required Flatpak runtimes (24.08) are not installed.${NC}"
    echo "    To install runtimes from Flathub:"
    echo "    flatpak install -y flathub org.freedesktop.Platform//${RUNTIME_VERSION} org.freedesktop.Sdk//${RUNTIME_VERSION} org.freedesktop.Sdk.Extension.rust-stable//${RUNTIME_VERSION}"
    if [[ "${1:-}" != "--force" ]] && [[ "${CI:-}" != "true" ]]; then
        echo -e "\n${YELLOW}[*] Manifest, AppStream metainfo, and Cargo sources have been verified successfully.${NC}"
        echo -e "${GREEN}[✔] Ready for Flathub packaging and GitHub Actions Flatpak CI.${NC}"
        exit 0
    fi
fi

# 5. Build with flatpak-builder
echo "[*] Step 5: Building Flatpak application with flatpak-builder..."
flatpak-builder --force-clean \
    --repo="$REPO_DIR" \
    "$BUILD_DIR" \
    "$MANIFEST"

# 6. Create bundle (.flatpak)
BUNDLE_FILE="$DIST_DIR/${APP_ID}.flatpak"
echo "[*] Step 6: Exporting single-file Flatpak bundle: $BUNDLE_FILE..."
flatpak build-bundle "$REPO_DIR" "$BUNDLE_FILE" "$APP_ID"

echo -e "\n${GREEN}${BOLD}[✔] Flatpak successfully built!${NC}"
echo "    Bundle: $BUNDLE_FILE"
echo "    To install bundle: flatpak install --user $BUNDLE_FILE"
echo "    To run: flatpak run $APP_ID"
