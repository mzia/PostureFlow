.PHONY: all build install install-dev uninstall reset test deb flatpak flatpak-sources help

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

deb:
	@./scripts/build_deb.sh

flatpak-sources:
	@python3 ./scripts/generate_cargo_sources.py Cargo.lock -o cargo-sources.json

flatpak: flatpak-sources
	@./scripts/build_flatpak.sh
