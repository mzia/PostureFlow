# pop-profile-manager

[![CI Safety & D-Bus Tests](https://github.com/mzia/pop-profile-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/mzia/pop-profile-manager/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust: 1.80+](https://img.shields.io/badge/Rust-1.80%2B-red.svg)](https://www.rust-lang.org)
[![OS: Pop!_OS](https://img.shields.io/badge/OS-Pop!__OS%20%7C%20Ubuntu-orange.svg)](https://system76.com/pop)
[![Hardware: Framework Laptop](https://img.shields.io/badge/Hardware-Framework%20Laptop-black.svg)](https://frame.work)

> **Context-aware security, developer, and lifestyle profile manager for Pop!_OS and Ubuntu laptops.**

`pop-profile` dynamically bridges the gap between paranoid security, frictionless software engineering, and casual entertainment. Switch postures with a single command without ever being locked out of your laptop.

---

## ⚡ Quick Start

### Option 1: Install via Pre-Built Debian Package (Recommended)

Download the latest `.deb` from [GitHub Releases](https://github.com/mzia/pop-profile-manager/releases) and install:
```bash
sudo dpkg -i pop-profile_1.0.0_amd64.deb
```

### Option 2: Install from Source

```bash
git clone https://github.com/mzia/pop-profile-manager.git
cd pop-profile-manager

# Build Rust daemon and Debian package
make deb
sudo dpkg -i dist/pop-profile_1.0.0_amd64.deb

# Or install directly with the installer script
sudo ./install.sh
```

Now switch postures anytime:
```bash
sudo pop-profile --home      # Streaming, Proton gaming, phone sync (GSConnect)
sudo pop-profile --work      # Office & corporate VPN (dev ports blocked to LAN)
sudo pop-profile --dev       # Coding & debugging (ptrace allowed, 524k file watchers)
sudo pop-profile --secure    # Coffee shops, airports & public Wi-Fi (stealth mode)
pop-profile --status         # Inspect live posture without sudo
```

---

## 🎯 Why pop-profile?

Linux security tools (like UFW, firewalld, or raw sysctl) are static. But laptop users live in dynamic contexts:

* **The Dev Friction Problem:** Strict security (`ptrace_scope = 2`) blocks `gdb`, `lldb`, and VS Code from debugging processes. Default `inotify` limits crash Vite and Webpack. Devs end up turning off security entirely.
* **The Corporate Leak Risk:** Running a local dev server (`0.0.0.0:3000`) or test database on a corporate office Wi-Fi exposes your code and data to everyone on the subnet.
* **The Home Entertainment Barrier:** Overly aggressive firewalls break Steam Remote Play, local game downloads, and phone sync (GSConnect/LocalSend).
* **The Update Drift Problem:** Every `apt upgrade` or kernel bump wipes runtime sysctl tuning and resets UFW configurations.

`pop-profile` solves all of this with zero dependencies, Rust D-Bus integration, and built-in anti-lockout guarantees.

---

## 📊 The Four Profiles

| Feature / Setting | 🏠 Home | 💼 Work | 💻 Dev | 🛡️ Secure |
| :--- | :--- | :--- | :--- | :--- |
| **Context** | Couch / Streaming / Gaming | Office / Corporate VPN | Coding / Testing / Lab | Coffee shop / Airport |
| **Inbound Firewall** | LAN trusted, WAN blocked | WAN blocked, VPN allowed | Dev ports open | **100% Blocked (Stealth)** |
| **Dev Ports (3000, 8000...)** | ❌ Blocked | ❌ **Strictly blocked to LAN** | ✅ **Allowed** | ❌ Blocked |
| **Corporate VPNs** | Allowed | ✅ **Unblocked (`tun+`, `wg+`)** | Allowed | Outbound only |
| **Steam Remote Play** | ✅ **Allowed (`27031-27040`)**| ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **Phone Sync (GSConnect)** | ✅ **Allowed (`1714-1764`)** | ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **LocalSend Sharing** | ✅ **Allowed (`53317`)** | ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **Proton Gaming (`vm.max_map`)**| **`1,048,576` (No crashes)**| Default | Default | Default |
| **Debugger Hooking (`ptrace`)**| `1` (Overlays hook) | `1` (Standard) | `1` (Full debugger attach) | `2` **(Anti-memory scraping)** |
| **File Watchers (`inotify`)** | 524,288 | 524,288 | **524,288 (Hot-reload ready)**| Default |
| **Crash Dumps** | Enabled | Disabled | **Enabled for `gdb`** | **Disabled (No key leaks)** |
| **Screen Lock Timeout** | **30 minutes** | **5 minutes (Compliance)** | **15 minutes** | **5 minutes** |

---

## 🔒 Anti-Lockout Invariants

`pop-profile` is built around safety invariants ensuring you can **never lock yourself out**:

1. **SSH Auto-Preservation:** If an active SSH session or daemon is detected, port `22/tcp` is automatically whitelisted before the firewall is touched.
2. **Loopback IPC Guarantee:** Explicit `allow on lo` rules guarantee desktop environments (Wayland, X11, COSMIC, GNOME), PipeWire audio, and D-Bus never freeze.
3. **Outbound Internet Egress:** Default-allow outgoing stateful tracking ensures browsing, DNS, and package updates never break.
4. **PAM & Sudo Untouched:** User authentication and login files are never modified.

Run the test suite anytime:
```bash
pop-profile --test
```

---

## 🦀 Rust D-Bus Daemon & Polkit (Phase 1)

`pop-profile` includes a native Rust system daemon (`pop-profile-daemon`) built with [`zbus`](https://crates.io/crates/zbus) providing an asynchronous D-Bus service on `io.github.mzia.PopProfile`.

### Two-Tier Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  FRONTEND: COSMIC Panel Applet / Flatpak GUI (libcosmic)    │
│  • Lives in the panel (symbolic profile icon)               │
│  • Popover UI: Radio buttons for Home, Work, Dev, Secure    │
└──────────────────────────────┬──────────────────────────────┘
                               │ D-Bus Method Call (SetProfile)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  SECURITY: Polkit Policy (io.github.mzia.PopProfile)        │
│  • Passwordless switching for active desktop sessions       │
│  • Located at /usr/share/polkit-1/actions/                  │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  BACKEND: pop-profile-daemon (Rust + zbus)                  │
│  • Emits ProfileChanged signals to update panel applets     │
│  • Manages sysctl, UFW, limits, and idle timeouts           │
└─────────────────────────────────────────────────────────────┘
```

### Building & Running the Daemon

```bash
# Build the optimized release binary
cargo build --release

# Run unit and D-Bus integration tests
cargo test
bash tests/test_dbus.sh

# Enable and start the background system daemon
sudo systemctl enable --now pop-profile-daemon
```

### D-Bus API Specification (`io.github.mzia.PopProfile`)

| Member | Type | Signature | Description |
| :--- | :--- | :--- | :--- |
| `GetActiveProfile` | Method | `() -> (s)` | Returns active profile name (`home`, `work`, `dev`, `secure`). |
| `SetProfile` | Method | `(s) -> ()` | Applies system posture and broadcasts `ProfileChanged`. |
| `GetStatus` | Method | `() -> (s)` | Returns live kernel and firewall status report. |
| `ResetToDefaults`| Method | `() -> ()` | Restores system to factory Pop!_OS defaults. |
| `ProfileChanged` | Signal | `(s)` | Broadcasts when a profile change occurs. |

---

---

## 🖥️ Native COSMIC Desktop Panel Applet

`pop-profile-applet` integrates directly into the **Pop!_OS COSMIC desktop panel** (`cosmic-applet-status-area`):

* **Real-time Symbolic Icons:** Automatically synchronizes with your active profile:
  - 🏠 **Home:** `user-home-symbolic`
  - 💼 **Work:** `applications-office-symbolic`
  - 💻 **Dev:** `utilities-terminal-symbolic`
  - 🔒 **Secure:** `security-high-symbolic`
* **One-Click Native Popover Menu:** Powered by the standard `com.canonical.dbusmenu` protocol, rendered natively inside the panel using your active COSMIC theme and accent colors.
* **Instant Profile Cycling:** Left-click the panel icon directly to cycle instantly through profiles (`Home` ➔ `Work` ➔ `Dev` ➔ `Secure`).
* **Desktop Notifications:** Dispatches native notifications on profile changes and security posture audits via `org.freedesktop.Notifications`.
* **Resilient Watchdog:** Automatically reconnects and re-registers whenever `cosmic-panel` or the session restarts.
* **Session Autostart:** Ships with a systemd user service (`pop-profile-applet.service`) and standard desktop entry (`io.github.mzia.PopProfile.Applet.desktop`).

### Running the Applet
```bash
# Start in session
pop-profile-applet

# Query current posture
pop-profile-applet --status

# Quick cycle next profile
pop-profile-applet --cycle
```

---

## 🔄 Self-Healing Post-Upgrade Hook

`pop-profile` includes an APT post-invoke hook registered at `/etc/apt/apt.conf.d/99-popos-profile-health`. 

Whenever an `apt upgrade`, kernel update, or patch finishes installing, the hook automatically validates and re-applies your profile settings in the background.

---

## 📖 Manual Page & Completions

* **Manual Page:**
  ```bash
  man pop-profile
  ```
* **Shell Completions:** Automatically installed for **Bash** (`/etc/bash_completion.d/pop-profile`) and **Zsh** (`/usr/share/zsh/vendor-completions/_pop-profile`).

---

## 📂 Repository Structure

```text
pop-profile-manager/
├── bin/
│   └── pop-profile                  # Standalone CLI executable
├── src/
│   ├── lib.rs                       # Shared library modules
│   ├── main.rs                      # Rust D-Bus daemon entrypoint
│   ├── profile.rs                   # Profile enum & icon mappings
│   ├── system.rs                    # System controller (sysctl, ufw, limits)
│   ├── dbus.rs                      # zbus asynchronous D-Bus service & signals
│   ├── bin/
│   │   └── pop-profile-applet.rs    # Native COSMIC Panel Applet entrypoint
│   └── applet/
│       ├── mod.rs                   # Applet module definitions
│       ├── state.rs                 # Thread-safe profile state manager
│       ├── client.rs                # System daemon proxy & notifications
│       ├── menu.rs                  # com.canonical.dbusmenu provider
│       └── sni.rs                   # org.kde.StatusNotifierItem provider
├── data/
│   ├── io.github.mzia.PopProfile.Applet.desktop # COSMIC panel applet desktop entry
│   ├── pop-profile-applet.service   # Systemd user session autostart service
│   ├── pop-profile-daemon.service   # Systemd privileged system service
│   ├── io.github.mzia.PopProfile.policy # Polkit passwordless authorization
│   └── io.github.mzia.PopProfile.conf   # D-Bus system bus permissions
├── man/
│   └── pop-profile.1                # Native Linux manual page
├── completions/
│   ├── pop-profile.bash             # Bash auto-completion
│   └── pop-profile.zsh              # Zsh auto-completion
├── scripts/
│   └── build_deb.sh                 # Automated Debian .deb package builder
├── tests/
│   ├── test_safety.sh               # 8-point automated anti-lockout test suite
│   ├── test_dbus.sh                 # D-Bus integration test suite
│   └── test_applet.sh               # Applet StatusNotifierItem/DBusMenu integration test
├── .github/
│   └── workflows/
│       ├── ci.yml                   # CI testing workflow (Rust + Safety + D-Bus)
│       └── release.yml              # Automated .deb build & GitHub Releases
├── install.sh                       # One-command system installer
├── uninstall.sh                     # Clean uninstaller (restores Pop!_OS defaults)
├── Makefile                         # 'make build', 'make install', 'make deb', 'make test'
├── LICENSE                          # MIT License (© M. Zia)
└── README.md                        # Project documentation
```

---

## 🗺️ Project Roadmap

- [x] **Phase 1: CLI & Rust D-Bus Daemon**
  - [x] 4 lifestyle/context profiles (Home, Work, Dev, Secure)
  - [x] Anti-lockout invariant test suite
  - [x] Rust daemon with `zbus` on `io.github.mzia.PopProfile`
  - [x] Polkit policy for passwordless desktop switching
  - [x] APT post-upgrade self-healing hook
- [x] **Phase 2: Packaging & Distribution**
  - [x] Debian `.deb` package generation (`scripts/build_deb.sh` / `make deb`)
  - [x] Standard systemd, Polkit, D-Bus, completions, and man page packaging
  - [x] Automated GitHub Actions release workflow (`.github/workflows/release.yml`)
- [x] **Phase 3: Native COSMIC Panel Applet**
  - [x] Rust COSMIC panel applet (`pop-profile-applet`) with live status icon
  - [x] Standard `StatusNotifierItem` + `com.canonical.dbusmenu` architecture
  - [x] Dynamic symbolic icons matching Pop!_OS / COSMIC desktop theme
  - [x] Popover dropdown menu with one-click profile switching and power status
  - [x] Watchdog auto-reconnect on COSMIC panel / session restarts
  - [x] Desktop entry (`X-CosmicApplet=true`) and systemd user service
- [ ] **Phase 4: Flatpak Distribution**
  - [ ] Flatpak manifest with host D-Bus portal access

---

## 🗑️ Uninstallation

To cleanly remove `pop-profile` and restore Pop!_OS to factory defaults:
```bash
sudo ./uninstall.sh
```

---

## 📄 License

MIT © M. Zia
