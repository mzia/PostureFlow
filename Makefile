.PHONY: all build install install-dev uninstall reset test scan snyk deb flatpak flatpak-sources help

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
MANDIR ?= $(PREFIX)/share/man/man1

all: build

help:
	@echo "PostureFlow Makefile"
	@echo "Targets:"
	@echo "  make build           - Compile Rust daemon, GUI, and applet in release mode"
	@echo "  make install         - Install PostureFlow CLI, daemon, GUI, applet, and services (requires sudo)"
	@echo "  make install-dev     - Developer install with live symlinks, daemon service, and Dev profile (requires sudo)"
	@echo "  make reset           - Reset system to Pop!_OS factory defaults (requires sudo)"
	@echo "  make uninstall       - Remove installed files and reset defaults (requires sudo)"
	@echo "  make test            - Run safety, D-Bus daemon, and COSMIC applet test suites"
	@echo "  make scan            - Scan dependencies & code for vulnerabilities (cargo audit + Snyk)"
	@echo "  make snyk            - Run Snyk security SAST scan on source code"
	@echo "  make deb             - Build Debian (.deb) package in dist/"
	@echo "  make flatpak         - Build and export Flatpak bundle (.flatpak) in dist/"
	@echo "  make flatpak-sources - Generate cargo-sources.json for Flatpak offline build"

build:
	@cargo build --release

install:
	@sudo ./install.sh

install-dev:
	@sudo ./install.sh --dev

reset:
	@sudo ./bin/postureflow --reset

uninstall:
	@sudo ./uninstall.sh

test:
	@bash ./tests/test_safety.sh
	@bash ./tests/test_dbus.sh
	@bash ./tests/test_applet.sh

scan:
	@echo "=== [1/2] RustSec Security Advisory Audit (Cargo.lock) ==="
	@if command -v cargo-audit >/dev/null 2>&1 || cargo audit --version >/dev/null 2>&1; then \
		cargo audit; \
	else \
		echo "cargo-audit not installed. Run: cargo install cargo-audit"; \
	fi
	@echo ""
	@echo "=== [2/2] Snyk Security Vulnerability Scan ==="
	@if command -v snyk >/dev/null 2>&1; then \
		if snyk whoami >/dev/null 2>&1 || [ -n "$$SNYK_TOKEN" ]; then \
			echo "Running Snyk Code static analysis (SAST)..."; \
			snyk code test || echo "Note: Enable Snyk Code in Snyk organization settings (https://app.snyk.io/)"; \
		else \
			echo "Snyk CLI installed (~/.local/bin/snyk). Run 'snyk auth' or set SNYK_TOKEN to enable."; \
		fi \
	else \
		echo "Snyk CLI not found at ~/.local/bin/snyk."; \
	fi

snyk:
	@if command -v snyk >/dev/null 2>&1; then \
		snyk code test; \
	else \
		echo "Snyk CLI not found at ~/.local/bin/snyk."; \
		exit 1; \
	fi

deb:
	@./scripts/build_deb.sh

flatpak-sources:
	@python3 ./scripts/generate_cargo_sources.py Cargo.lock -o cargo-sources.json

flatpak: flatpak-sources
	@./scripts/build_flatpak.sh
