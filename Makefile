.PHONY: install uninstall test help

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
MANDIR ?= $(PREFIX)/share/man/man1

help:
	@echo "pop-profile Makefile"
	@echo "Targets:"
	@echo "  make install    - Install pop-profile binary, man page, and completions (requires sudo)"
	@echo "  make uninstall  - Remove installed files and reset defaults (requires sudo)"
	@echo "  make test       - Run anti-lockout test suite"

install:
	@sudo ./install.sh

uninstall:
	@sudo ./uninstall.sh

test:
	@bash ./tests/test_safety.sh
	@bash ./tests/test_dbus.sh

deb:
	@./scripts/build_deb.sh

