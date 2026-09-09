.PHONY: all build install install-dev uninstall reset test deb help

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
MANDIR ?= $(PREFIX)/share/man/man1

all: build

help:
	@echo "pop-profile Makefile"
	@echo "Targets:"
	@echo "  make build        - Compile Rust daemon and COSMIC applet in release mode"
	@echo "  make install      - Install pop-profile CLI, daemon, COSMIC applet, and services (requires sudo)"
	@echo "  make install-dev  - Developer install with live symlinks, daemon service, and Dev profile (requires sudo)"
	@echo "  make reset        - Reset system to Pop!_OS factory defaults (requires sudo)"
	@echo "  make uninstall    - Remove installed files and reset defaults (requires sudo)"
	@echo "  make test         - Run safety, D-Bus daemon, and COSMIC applet test suites"
	@echo "  make deb          - Build Debian (.deb) package in dist/"

build:
	@cargo build --release

install:
	@sudo ./install.sh

install-dev:
	@sudo ./install.sh --dev

reset:
	@sudo ./bin/pop-profile --reset

uninstall:
	@sudo ./uninstall.sh

test:
	@bash ./tests/test_safety.sh
	@bash ./tests/test_dbus.sh
	@bash ./tests/test_applet.sh

deb:
	@./scripts/build_deb.sh

