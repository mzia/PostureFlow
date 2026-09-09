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
sudo pop-profile --travel    # Coffee shops, airports & public Wi-Fi (stealth mode, alias: --secure)
sudo pop-profile --reset     # Reset all settings back to Pop!_OS factory defaults
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

## 🦀 Rust D-Bus Daemon & Polkit Security

`pop-profile` includes a native Rust system daemon (`pop-profile-daemon`) built with [`zbus`](https://crates.io/crates/zbus) providing an asynchronous D-Bus service on `io.github.mzia.PopProfile`.

### Three-Tier Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│  FRONTENDS:                                                 │
│  1. COSMIC Panel Applet (`pop-profile-applet`)              │
│     • StatusNotifierItem + DBusMenu in top panel            │
│     • Live symbolic icons, 1-click switcher & launcher      │
│  2. Floating Settings GUI (`pop-profile-gui`)               │
│     • COSMIC-styled floating window with tabbed editor      │
│     • Visual UFW rules, Framework power, TOML import/export │
│  3. CLI (`pop-profile`)                                     │
│     • Instant terminal posture switching & status           │
└──────────────────────────────┬──────────────────────────────┘
                               │ D-Bus Calls (io.github.mzia.PopProfile)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  SECURITY: Polkit Policy (io.github.mzia.PopProfile)        │
│  • 5-Minute Cached Admin Auth (`auth_admin_keep`)           │
│  • Prompts once on first switch, instant subsequent actions │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  PRIVILEGED BACKEND: pop-profile-daemon (Rust + zbus)       │
│  • Emits ProfileChanged signals to update UI components     │
│  • Declarative TOML scanner (/etc/pop-profile/profiles.d)   │
│  • Safety engine (anti-lockout, loopback, SSH preservation) │
│  • Manages sysctl, UFW, Framework battery, and limits       │
└─────────────────────────────────────────────────────────────┘
```

### 5-Minute Cached Admin Authorization (`auth_admin_keep`)

Desktop profile switching and custom profile imports use Polkit's `auth_admin_keep` policy (matching `sudo`). When you switch profiles or import configurations from the desktop, you authenticate once with your password/fingerprint; subsequent changes over the next 5 minutes are applied instantly without interrupting your workflow.

### D-Bus API Specification (`io.github.mzia.PopProfile`)

| Member | Type | Signature | Description |
| :--- | :--- | :--- | :--- |
| `GetActiveProfile` | Method | `() -> (s)` | Returns active profile identifier (`home`, `work`, `dev`, `travel`, or custom). |
| `SetProfile` | Method | `(s) -> ()` | Validates posture, applies kernel/firewall/power rules, and broadcasts signal. |
| `GetStatus` | Method | `() -> (s)` | Returns live kernel, UFW firewall, and Framework power status report. |
| `ResetToDefaults` | Method | `() -> ()` | Restores system settings to factory Pop!_OS defaults. |
| `ListProfiles` | Method | `() -> (as)` | Discovers and returns all built-in and custom profile IDs. |
| `GetProfileDetails` | Method | `(s) -> (s)` | Returns the complete declarative TOML configuration for any profile. |
| `ValidateProfile` | Method | `(s) -> (b, s)` | Validates TOML against anti-lockout rules, returning sanitized TOML or error. |
| `SaveCustomProfile` | Method | `(s, s) -> ()` | Saves a validated profile TOML to `/etc/pop-profile/profiles.d/<id>.pop-profile.toml`. |
| `DeleteCustomProfile`| Method | `(s) -> ()` | Removes a custom profile from `/etc/pop-profile/profiles.d/`. |
| `ProfileChanged` | Signal | `(s)` | Broadcasts when a profile switch occurs. |

---

## 🖥️ Native COSMIC Desktop Panel Applet

`pop-profile-applet` integrates directly into the **Pop!_OS COSMIC desktop panel** (`cosmic-applet-status-area`):

* **Real-time Symbolic Icons:** Automatically synchronizes with your active profile:
  - 🏠 **Home:** `user-home-symbolic`
  - 💼 **Work:** `applications-office-symbolic`
  - 💻 **Dev:** `utilities-terminal-symbolic`
  - ✈️ **Travel:** `security-high-symbolic` (alias: `secure`)
* **One-Click Native Popover Menu:** Powered by the standard `com.canonical.dbusmenu` protocol, rendered natively inside the panel using your active COSMIC theme and accent colors.
* **Instant Profile Cycling:** Left-click the panel icon directly to cycle instantly through profiles (`Home` ➔ `Work` ➔ `Dev` ➔ `Travel`).
* **Direct GUI Launcher:** Click **`⚙ Configure Profiles & Import...`** to open the floating settings GUI.
* **Desktop Notifications:** Dispatches native notifications on profile changes and security posture audits via `org.freedesktop.Notifications`.
* **Resilient Watchdog:** Automatically reconnects and re-registers whenever `cosmic-panel` or the session restarts.
* **Session Autostart:** Ships with a systemd user service (`pop-profile-applet.service`) and standard desktop entry (`io.github.mzia.PopProfile.Applet.desktop`).

### Running the Applet
```bash
# Start in background session
pop-profile-applet

# Query current posture
pop-profile-applet --status

# Quick cycle next profile
pop-profile-applet --cycle

# Reset all settings to Pop!_OS factory defaults via D-Bus daemon
pop-profile-applet --reset
```

---

## 🎨 COSMIC Floating Desktop GUI (`pop-profile-gui`)

`pop-profile-gui` provides a rich, COSMIC-styled desktop window designed to open as a floating tile (`io.github.mzia.PopProfile.Settings`):

* **Profile Sidebar:** Browse built-in profiles and custom configurations with live active badges.
* **Factory Defaults Reset**: One-click **`🔄 Reset to Factory Defaults`** with safety confirmation dialog to immediately restore unmanaged out-of-the-box settings (UFW disabled, balanced power profile, battery 100%, and default 15-minute idle delay).
* **1-Click Import & Export:**
  - **`📥 Import Config`**: Pick any `.pop-profile.toml` file to inspect, validate, and install.
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

Custom profiles are stored as `.pop-profile.toml` files in `/etc/pop-profile/profiles.d/` (system-wide) or `~/.config/pop-profile/profiles.d/` (user-specific).

### Example Configuration: `ai-lab.pop-profile.toml`

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
│   ├── system.rs                    # System controller (sysctl, ufw, limits, power)
│   ├── dbus.rs                      # zbus asynchronous D-Bus service & signals
│   ├── config/                      # Declarative profile engine
│   │   ├── mod.rs                   # Built-in profile definitions & directory scanner
│   │   ├── schema.rs                # Serde TOML schema (firewall, kernel, framework)
│   │   └── validator.rs             # Strict anti-lockout safety validation
│   ├── bin/
│   │   ├── pop-profile-applet.rs    # Native COSMIC Panel Applet entrypoint
│   │   └── pop-profile-gui.rs       # COSMIC-themed floating settings GUI entrypoint
│   └── applet/
│       ├── mod.rs                   # Applet module definitions
│       ├── state.rs                 # Thread-safe profile state manager
│       ├── client.rs                # System daemon proxy & notifications
│       ├── menu.rs                  # com.canonical.dbusmenu provider
│       └── sni.rs                   # org.kde.StatusNotifierItem provider
├── data/
│   ├── io.github.mzia.PopProfile.Applet.desktop   # COSMIC panel applet desktop entry
│   ├── io.github.mzia.PopProfile.Settings.desktop # Floating GUI settings desktop entry
│   ├── pop-profile-applet.service   # Systemd user session autostart service
│   ├── pop-profile-daemon.service   # Systemd privileged system service
│   ├── io.github.mzia.PopProfile.policy # Polkit 5-min cached admin authorization
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
  - [x] 4 lifestyle/context profiles (Home, Work, Dev, Travel)
  - [x] Anti-lockout invariant test suite
  - [x] Rust daemon with `zbus` on `io.github.mzia.PopProfile`
  - [x] Polkit policy with 5-minute cached admin authorization (`auth_admin_keep`)
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
- [x] **Phase 4: Declarative Custom Profile Engine**
  - [x] Declarative `.pop-profile.toml` schema (metadata, firewall, sysctl, framework power)
  - [x] Dynamic multi-directory profile scanner (`/etc/pop-profile/profiles.d/`, `~/.config/pop-profile/profiles.d/`)
  - [x] Anti-lockout validation & sanitization engine (loopback, eBPF, pipes)
  - [x] D-Bus API extension (`ListProfiles`, `GetProfileDetails`, `ValidateProfile`, `SaveCustomProfile`, `DeleteCustomProfile`)
- [x] **Phase 5: Floating Desktop Settings GUI**
  - [x] COSMIC-styled floating window application (`pop-profile-gui`)
  - [x] Sidebar profile manager with active indicators and 1-click TOML import/export
  - [x] Interactive tabbed editor (General, Firewall, Kernel, Framework Power)
  - [x] Live "Test & Verify Safety" pre-flight checks
  - [x] Direct launcher integration in COSMIC top bar panel applet menu
- [ ] **Phase 6: Flatpak Distribution**
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
