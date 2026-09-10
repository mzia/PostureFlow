# PostureFlow

[![CI Safety & D-Bus Tests](https://github.com/mzia/pop-profile-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/mzia/pop-profile-manager/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust: 1.80+](https://img.shields.io/badge/Rust-1.80%2B-red.svg)](https://www.rust-lang.org)
[![OS: Pop!_OS](https://img.shields.io/badge/OS-Pop!__OS%20%7C%20Ubuntu-orange.svg)](https://system76.com/pop)
[![Hardware: Framework Laptop](https://img.shields.io/badge/Hardware-Framework%20Laptop-black.svg)](https://frame.work)

> **Dynamic security posture, hardware power, and lifestyle workflow orchestrator for Linux & Pop!_OS laptops.**

`PostureFlow` dynamically bridges the gap between paranoid security, frictionless software engineering, and casual entertainment. Switch postures with a single command or status bar click without ever risking lockout from your laptop.

---

## ⚡ Quick Start

### Option 1: Install via Pre-Built Debian Package (Recommended)

Download the latest `.deb` from [GitHub Releases](https://github.com/mzia/pop-profile-manager/releases) and install:
```bash
sudo dpkg -i dist/postureflow_1.0.0_amd64.deb
```

### Option 2: Install from Source

```bash
git clone https://github.com/mzia/pop-profile-manager.git
cd pop-profile-manager

# Build Rust binaries and Debian package
make deb
sudo dpkg -i dist/postureflow_1.0.0_amd64.deb

# Or install directly with the installer script
sudo ./install.sh
```

### Option 3: Install via Flatpak Bundle

```bash
# Build and install the sandboxed Flatpak package
make flatpak
flatpak install --user dist/io.github.mzia.PostureFlow.flatpak

# Run Flatpak application
flatpak run io.github.mzia.PostureFlow
```

Now switch postures anytime:
```bash
sudo postureflow --home      # Streaming, Proton gaming, phone sync (GSConnect)
sudo postureflow --work      # Office & corporate VPN (dev ports blocked to LAN)
sudo postureflow --dev       # Coding & debugging (ptrace allowed, 524k file watchers)
sudo postureflow --travel    # Coffee shops, airports & public Wi-Fi (stealth mode, alias: --secure)
sudo postureflow --reset     # Reset all settings back to Pop!_OS factory defaults
postureflow --status         # Inspect live posture without sudo
postureflow --score          # Calculate real-time Security Posture Score (0-100%, Grade)
postureflow --ports          # Inspect live listening sockets & process owners
postureflow --autoflow       # Check autonomous network detection & SSID rules
```

---

## 🎯 Why PostureFlow?

Linux security tools (like UFW, firewalld, or raw sysctl) are static. But laptop users live in dynamic contexts:

* **The Dev Friction Problem:** Strict security (`ptrace_scope = 2`) blocks `gdb`, `lldb`, and VS Code from debugging processes. Default `inotify` limits crash Vite and Webpack. Devs end up turning off security entirely.
* **The Corporate Leak Risk:** Running a local dev server (`0.0.0.0:3000`) or test database on a corporate office Wi-Fi exposes your code and data to everyone on the subnet.
* **The Home Entertainment Barrier:** Overly aggressive firewalls break Steam Remote Play, local game downloads, and phone sync (GSConnect/LocalSend).
* **The Update Drift Problem:** Every `apt upgrade` or kernel bump wipes runtime sysctl tuning and resets UFW configurations.

`PostureFlow` solves all of this with zero dependencies, a native Rust D-Bus daemon, Polkit cached authorization, and built-in anti-lockout guarantees.

---

## 📊 The Four Profiles

| Feature / Setting | 🏠 Home | 💼 Work | 💻 Dev | ✈️ Travel |
| :--- | :--- | :--- | :--- | :--- |
| **Context** | Couch / Streaming / Gaming | Office / Corporate VPN | Coding / Testing / Lab | Coffee shop / Airport (Lockdown) |
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

`PostureFlow` is built around safety invariants ensuring you can **never lock yourself out**:

1. **SSH Auto-Preservation:** If an active SSH session or daemon is detected, port `22/tcp` is automatically whitelisted before the firewall is touched.
2. **Loopback IPC Guarantee:** Explicit `allow on lo` rules guarantee desktop environments (Wayland, X11, COSMIC, GNOME), PipeWire audio, and D-Bus never freeze.
3. **Outbound Internet Egress:** Default-allow outgoing stateful tracking ensures browsing, DNS, and package updates never break.
4. **PAM & Sudo Untouched:** User authentication and login files are never modified.

Run the test suite anytime:
```bash
postureflow --test
```

---

## 🦀 Rust D-Bus Daemon & Polkit Security

`PostureFlow` includes a native Rust system daemon (`postureflow-daemon`) built with [`zbus`](https://crates.io/crates/zbus) providing an asynchronous D-Bus service on `io.github.mzia.PostureFlow`.

### Three-Tier Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│  FRONTENDS:                                                 │
│  1. Top Bar Applet (`postureflow-applet`)                   │
│     • StatusNotifierItem + DBusMenu in top panel            │
│     • Live symbolic icons, 1-click switcher & launcher      │
│  2. Floating Settings GUI (`postureflow-gui`)               │
│     • COSMIC-styled floating window with tabbed editor      │
│     • Visual UFW rules, Framework power, TOML import/export │
│  3. CLI (`postureflow`)                                     │
│     • Instant terminal posture switching & status           │
└──────────────────────────────┬──────────────────────────────┘
                                │ D-Bus Calls (io.github.mzia.PostureFlow)
                                ▼
┌─────────────────────────────────────────────────────────────┐
│  SECURITY: Polkit Policy (io.github.mzia.PostureFlow)       │
│  • 5-Minute Cached Admin Auth (`auth_admin_keep`)           │
│  • Prompts once on first switch, instant subsequent actions │
└──────────────────────────────┬──────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│  PRIVILEGED BACKEND: postureflow-daemon (Rust + zbus)       │
│  • Emits ProfileChanged signals to update UI components     │
│  • Declarative TOML scanner (/etc/postureflow/profiles.d)   │
│  • Safety engine (anti-lockout, loopback, SSH preservation) │
│  • Registers native io.github.mzia.PostureFlow service      │
│  • Manages sysctl, UFW, Framework battery, and limits       │
└─────────────────────────────────────────────────────────────┘
```

### 5-Minute Cached Admin Authorization (`auth_admin_keep`)

Desktop profile switching and custom profile imports use Polkit's `auth_admin_keep` policy (matching `sudo`). When you switch profiles or import configurations from the desktop, you authenticate once with your password/fingerprint; subsequent changes over the next 5 minutes are applied instantly without interrupting your workflow.

### D-Bus API Specification (`io.github.mzia.PostureFlow`)

| Member | Type | Signature | Description |
| :--- | :--- | :--- | :--- |
| `GetActiveProfile` | Method | `() -> (s)` | Returns active profile identifier (`home`, `work`, `dev`, `travel`, or custom). |
| `SetProfile` | Method | `(s) -> ()` | Validates posture, applies kernel/firewall/power rules, and broadcasts signal. |
| `GetStatus` | Method | `() -> (s)` | Returns live kernel, UFW firewall, and Framework power status report. |
| `ResetToDefaults` | Method | `() -> ()` | Restores system settings to factory Pop!_OS defaults. |
| `ListProfiles` | Method | `() -> (as)` | Discovers and returns all built-in and custom profile IDs. |
| `GetProfileDetails` | Method | `(s) -> (s)` | Returns the complete declarative TOML configuration for any profile. |
| `ValidateProfile` | Method | `(s) -> (b, s)` | Validates TOML against anti-lockout rules, returning sanitized TOML or error. |
| `SaveCustomProfile` | Method | `(s, s) -> ()` | Saves a validated profile TOML to `/etc/postureflow/profiles.d/<id>.postureflow.toml`. |
| `DeleteCustomProfile`| Method | `(s) -> ()` | Removes a custom profile from `/etc/postureflow/profiles.d/`. |
| `GetPostureScore` | Method | `() -> (i, s)` | Returns real-time score (0-100) and itemized audit report JSON. |
| `GetListeningPorts` | Method | `() -> (s)` | Returns JSON array of all active listening sockets, PIDs, and process names. |
| `BlockPort` | Method | `(q, s) -> ()` | Immediately denies and blocks incoming traffic on a port via UFW. |
| `GetAutoFlowStatus` | Method | `() -> (b, s, s)` | Returns `(enabled, active_network_ssid, matched_profile)`. |
| `SetAutoFlowEnabled`| Method | `(b) -> ()` | Enables or disables the autonomous network watcher daemon engine. |
| `GetAutoFlowConfig` | Method | `() -> (s)` | Returns the active `autoflow.toml` configuration content. |
| `SaveAutoFlowConfig`| Method | `(s) -> ()` | Updates and saves `/etc/postureflow/autoflow.toml`. |
| `ProfileChanged` | Signal | `(s)` | Broadcasts when a profile switch occurs. |

---

## 🖥️ Desktop Top Bar & COSMIC Panel Applet

`postureflow-applet` integrates directly into the **Pop!_OS COSMIC desktop panel** and standard Freedesktop system trays:

* **Real-time Symbolic Icons:** Automatically synchronizes with your active profile:
  - 🏠 **Home:** `user-home-symbolic`
  - 💼 **Work:** `applications-office-symbolic`
  - 💻 **Dev:** `utilities-terminal-symbolic`
  - ✈️ **Travel:** `security-high-symbolic` (alias: `secure`)
* **One-Click Native Popover Menu:** Powered by the standard `com.canonical.dbusmenu` protocol, rendered natively inside the panel using your active desktop theme and accent colors.
* **Instant Profile Cycling:** Left-click the panel icon directly to cycle instantly through profiles (`Home` ➔ `Work` ➔ `Dev` ➔ `Travel`).
* **Direct GUI Launcher:** Click **`⚙ Configure Profiles & Import...`** to open the floating settings GUI.
* **Desktop Notifications:** Dispatches native notifications on profile changes and security posture audits via `org.freedesktop.Notifications`.
* **Resilient Watchdog:** Automatically reconnects and re-registers whenever the desktop shell or session restarts.
* **Session Autostart:** Ships with a systemd user service (`postureflow-applet.service`) and standard desktop entry (`io.github.mzia.PostureFlow.Applet.desktop`).

### Running the Applet
```bash
# Start in background session
postureflow-applet

# Query current posture
postureflow-applet --status

# Quick cycle next profile
postureflow-applet --cycle

# Reset all settings to factory defaults via D-Bus daemon
postureflow-applet --reset
```

---

## 🎨 Floating Desktop Settings GUI (`postureflow-gui`)

`postureflow-gui` provides a rich, modern desktop window designed to open as a floating tile (`io.github.mzia.PostureFlow`):

* **Profile Sidebar:** Browse built-in profiles and custom configurations with live active badges.
* **Factory Defaults Reset**: One-click **`🔄 Reset to Factory Defaults`** with safety confirmation dialog to immediately restore unmanaged out-of-the-box settings (UFW disabled, balanced power profile, battery 100%, and default 15-minute idle delay).
* **1-Click Import & Export:**
  - **`📥 Import Config`**: Pick any `.postureflow.toml` file to inspect, validate, and install.
  - **`📤 Export Config`**: Export custom or built-in profiles to share with teammates.
* **Tabbed Visual Editor:**
  - **General:** Profile ID, Name, Description, and Category tags.
  - **Firewall:** Configure inbound/outbound default policies, loopback isolation, interface whitelists, and an interactive port table (add/remove TCP/UDP ports with custom descriptions).
  - **Kernel & OS:** Fine-tune sysctl parameters against safe whitelisted keys (`ptrace_scope`, `bpf_jit_harden`, `inotify.max_user_watches`, etc.) and system security limits (`nofile`).
  - **Framework & Power:** Set battery charge limits (e.g. 80% threshold for battery health), energy performance profile, and display sleep timeouts.
* **Interactive Safety Verification:** Test configuration safety in real time before saving or applying with the **`Test & Verify Safety`** button.
* **Instant Activation:** Apply changes system-wide with **`⚡ Activate Profile`**.

---

## 📝 Custom Profiles & Declarative TOML Schema

Custom profiles are stored as `.postureflow.toml` files in `/etc/postureflow/profiles.d/` (system-wide) or `~/.config/postureflow/profiles.d/` (user-specific).

### Example Configuration: `ai-lab.postureflow.toml`

```toml
[profile]
id = "ai-lab"
name = "AI & ML Development Lab"
description = "Optimized for local LLM inference, PyTorch, Ollama, and WebUI"
category = "development"

[firewall]
default_incoming = "deny"
default_outgoing = "allow"
allow_loopback = true
allowed_interfaces = ["lo", "docker0"]

[[firewall.ports]]
port = 11434
proto = "tcp"
comment = "Ollama Local API"

[[firewall.ports]]
port = 8080
proto = "tcp"
comment = "Local WebUI / Open-WebUI"

[kernel]
ptrace_scope = 1
bpf_disabled = 2
inotify_max_user_watches = 1048576

[framework]
battery_charge_limit = 80
platform_energy_profile = "performance"
idle_timeout_seconds = 1800
```

### Strict Anti-Lockout Validation

Every imported or saved profile is checked by the safety validator:
1. **Loopback Protection:** `allow_loopback` is forcibly set to `true` to ensure Wayland, X11, D-Bus, and audio never freeze.
2. **eBPF Safety:** `bpf_disabled = 1` is automatically sanitized to `2` to prevent irreversible runtime locks that would block tools until reboot.
3. **Pipe Injection Prevention:** Sysctl keys like `fs.suid_dumpable` or `kernel.core_pattern` cannot contain pipe characters (`|`) to prevent privilege escalation.
4. **Sysctl Whitelisting:** Only vetted, safe kernel performance and security keys can be altered.

---

## 🔄 Self-Healing Post-Upgrade Hook

`PostureFlow` includes an APT post-invoke hook registered at `/etc/apt/apt.conf.d/99-postureflow-health`. 

Whenever an `apt upgrade`, kernel update, or patch finishes installing, the hook automatically validates and re-applies your profile settings in the background.

---

## 📖 Manual Page & Completions

* **Manual Page:**
  ```bash
  man postureflow
  ```
* **Shell Completions:** Automatically installed for **Bash** (`/etc/bash_completion.d/postureflow`) and **Zsh** (`/usr/share/zsh/vendor-completions/_postureflow`).

---

## 📂 Repository Structure

```text
postureflow/
├── bin/
│   └── postureflow                  # Standalone CLI executable
├── src/
│   ├── lib.rs                       # Shared library modules
│   ├── main.rs                      # Rust D-Bus daemon entrypoint
│   ├── profile.rs                   # Profile enum & icon mappings
│   ├── system.rs                    # System controller (sysctl, ufw, limits, power)
│   ├── dbus.rs                      # Native zbus D-Bus service (PostureFlow)
│   ├── config/                      # Declarative profile engine
│   │   ├── mod.rs                   # Built-in profile definitions & directory scanner
│   │   ├── schema.rs                # Serde TOML schema (firewall, kernel, framework)
│   │   └── validator.rs             # Strict anti-lockout safety validation
│   ├── bin/
│   │   ├── postureflow-applet.rs    # Top Bar / COSMIC Panel Applet entrypoint
│   │   └── postureflow-gui.rs       # Floating settings GUI entrypoint
│   ├── applet/
│   │   ├── mod.rs                   # Applet module definitions
│   │   ├── state.rs                 # Thread-safe profile state manager
│   │   ├── client.rs                # System daemon proxy & notifications
│   │   ├── menu.rs                  # com.canonical.dbusmenu provider
│   │   └── sni.rs                   # org.kde.StatusNotifierItem provider
│   ├── inspector/                   # Security auditing & socket inspector
│   │   ├── mod.rs                   # Inspector module definition
│   │   ├── ports.rs                 # Live socket inspection & process resolution
│   │   └── score.rs                 # 100-point security scoring engine
│   └── autoflow/                    # Autonomous context engine
│       └── mod.rs                   # NetworkManager D-Bus / SSID event watcher
├── data/
│   ├── io.github.mzia.PostureFlow.desktop         # Floating GUI settings desktop entry
│   ├── io.github.mzia.PostureFlow.Applet.desktop  # Top bar panel applet desktop entry
│   ├── io.github.mzia.PostureFlow.metainfo.xml    # AppStream 1.0 metadata
│   ├── icons/
│   │   └── io.github.mzia.PostureFlow.svg         # High-resolution vector icon
│   ├── postureflow-applet.service                 # Systemd user session autostart service
│   ├── postureflow-daemon.service                 # Systemd privileged system service
│   ├── io.github.mzia.PostureFlow.policy          # Polkit 5-min cached admin authorization
│   └── io.github.mzia.PostureFlow.conf            # D-Bus system bus permissions
├── io.github.mzia.PostureFlow.yml                 # Flatpak application manifest
├── man/
│   └── postureflow.1                              # Native Linux manual page
├── completions/
│   ├── postureflow.bash                           # Bash auto-completion
│   └── postureflow.zsh                            # Zsh auto-completion
├── scripts/
│   ├── build_deb.sh                               # Automated Debian .deb package builder
│   ├── build_flatpak.sh                           # Flatpak build and bundle script
│   └── generate_cargo_sources.py                  # Offline cargo vendor generator
├── tests/
│   ├── test_safety.sh                             # 8-point automated anti-lockout test suite
│   ├── test_dbus.sh                               # D-Bus integration test suite
│   └── test_applet.sh                             # Applet StatusNotifierItem/DBusMenu integration test
├── .github/
│   └── workflows/
│       ├── ci.yml                                 # CI testing workflow (Rust + Safety + D-Bus)
│       ├── release.yml                            # Automated .deb build & GitHub Releases
│       └── flatpak.yml                            # Automated Flatpak bundle validation
├── install.sh                                     # One-command system installer
├── uninstall.sh                                   # Clean uninstaller (restores Pop!_OS defaults)
├── Makefile                                       # 'make build', 'make install', 'make deb', 'make flatpak'
├── LICENSE                                        # MIT License (© M. Zia)
└── README.md                                      # Project documentation
```

---

## 🗺️ Project Roadmap

- [x] **Phase 1: CLI & Rust D-Bus Daemon**
  - [x] 4 lifestyle/context profiles (Home, Work, Dev, Travel)
  - [x] Anti-lockout invariant test suite
  - [x] Rust daemon with `zbus` on `io.github.mzia.PostureFlow`
  - [x] Polkit policy with 5-minute cached admin authorization (`auth_admin_keep`)
  - [x] APT post-upgrade self-healing hook
- [x] **Phase 2: Packaging & Distribution**
  - [x] Debian `.deb` package generation (`scripts/build_deb.sh` / `make deb`)
  - [x] Standard systemd, Polkit, D-Bus, completions, and man page packaging
  - [x] Automated GitHub Actions release workflow (`.github/workflows/release.yml`)
- [x] **Phase 3: Top Bar & COSMIC Panel Applet**
  - [x] Rust panel applet (`postureflow-applet`) with live status icon
  - [x] Standard `StatusNotifierItem` + `com.canonical.dbusmenu` architecture
  - [x] Dynamic symbolic icons matching Pop!_OS / COSMIC desktop theme
  - [x] Popover dropdown menu with one-click profile switching and power status
  - [x] Watchdog auto-reconnect on panel / session restarts
  - [x] Desktop entry and systemd user service
- [x] **Phase 4: Declarative Custom Profile Engine**
  - [x] Declarative `.postureflow.toml` schema (metadata, firewall, sysctl, framework power)
  - [x] Dynamic multi-directory profile scanner (`/etc/postureflow/profiles.d/`, `~/.config/postureflow/profiles.d/`)
  - [x] Anti-lockout validation & sanitization engine (loopback, eBPF, pipes)
  - [x] D-Bus API extension (`ListProfiles`, `GetProfileDetails`, `ValidateProfile`, `SaveCustomProfile`, `DeleteCustomProfile`)
- [x] **Phase 5: Floating Desktop Settings GUI**
  - [x] Modern styled floating window application (`postureflow-gui`)
  - [x] Sidebar profile manager with active indicators and 1-click TOML import/export
  - [x] Interactive tabbed editor (General, Firewall, Kernel, Framework Power)
  - [x] Live "Test & Verify Safety" pre-flight checks
  - [x] Direct launcher integration in top bar panel applet menu
- [x] **Phase 6: Flatpak Distribution**
  - [x] Flathub-compliant Flatpak manifest (`io.github.mzia.PostureFlow.yml`)
  - [x] Sandboxed desktop portal integration (Wayland, X11 fallback, DRI, file chooser)
  - [x] Host system D-Bus portal access (`io.github.mzia.PostureFlow`) for privileged operations
  - [x] AppStream 1.0 metainfo specification (`data/io.github.mzia.PostureFlow.metainfo.xml`)
  - [x] Scalable vector application iconography (`data/icons/io.github.mzia.PostureFlow.svg`)
  - [x] Zero-dependency offline cargo sources generator (`scripts/generate_cargo_sources.py`)
  - [x] Automated builder & packager (`scripts/build_flatpak.sh` / `make flatpak`)
  - [x] Automated GitHub Actions Flatpak CI workflow (`.github/workflows/flatpak.yml`)
- [x] **Phase 7: Autonomous Context & Security Cockpit**
  - [x] Reactive Auto-Flow autonomous network watcher daemon
  - [x] Live socket inspector with process identification and exposure classification
  - [x] 100-point security posture scoring engine (Cockpit tab in GUI)
  - [x] Full codebase refactoring to pure PostureFlow architecture

---

## 🗑️ Uninstallation

To cleanly remove `PostureFlow` and restore system settings to factory defaults:
```bash
sudo ./uninstall.sh
```

---

## 📄 License

MIT © M. Zia
