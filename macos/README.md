# 🍎 PostureFlow for macOS

> **Dynamic security posture, Apple Silicon power management, and lifestyle workflow orchestrator for macOS.**

PostureFlow for macOS brings the contextual security and power-scaling features of PostureFlow Linux to macOS (Ventura 13, Sonoma 14, and Sequoia 15+) and Apple Silicon MacBooks (M1/M2/M3/M4).

---

## 🌟 Key Features on macOS

* **SwiftUI `MenuBarExtra`:** Lives natively in the top menu bar status area with dynamic SF Symbols and profile accent colors.
* **Apple Silicon Power Optimization:** Programmatic control over macOS Low Power Mode (`pmset lowpowermode`) and display sleep timers (`pmset displaysleep`), ensuring maximum battery life during Travel and zero stutter during Work video calls.
* **Packet Filter (`pfctl`) Anchoring:** Isolated perimeter firewall rules placed at `/etc/pf.anchors/com.postureflow`, leaving the system `/etc/pf.conf` intact.
* **Zero-Overhead Auto-Flow:** Network interface transitions and SSID detection using `CoreWLAN` and `NWPathMonitor` with zero polling.
* **App-Aware Dynamic Triggers:** Instant posture elevation when applications like Zoom, Microsoft Teams, Slack, or Xcode launch via `NSWorkspace.notificationCenter`.
* **Privileged Helper Tool (`SMAppService`):** Separation of privileges where the GUI runs completely unprivileged and system-level operations run via a root daemon (`com.postureflow.helper`) communicating across typed Mach XPC.

---

## 📁 Codebase Layout

```text
macos/
├── Package.swift                             # Swift Package Manager manifest
├── README.md                                 # macOS documentation & developer guide
├── Resources/
│   ├── com.postureflow.helper.plist          # Launchd privileged helper service definition
│   └── PostureFlow.entitlements              # App security & network entitlements
├── Sources/
│   ├── PostureFlowShared/                    # Shared library across app, daemon, and CLI
│   │   ├── Models/
│   │   │   ├── PostureMode.swift             # Home, Work, Dev, Travel posture definitions
│   │   │   ├── PostureScore.swift            # 100-point security posture scoring engine
│   │   │   └── PostureConfig.swift           # Configuration schema and persistence (~/.config)
│   │   └── IPC/
│   │       └── PostureFlowHelperProtocol.swift # Mach XPC protocol definition
│   ├── PostureFlowApp/                       # MenuBarExtra SwiftUI application
│   │   ├── App/
│   │   │   ├── PostureFlowApp.swift          # Main entrypoint with MenuBarExtra scene
│   │   │   ├── AppDelegate.swift             # Accessory activation & window management
│   │   │   └── StateStore.swift              # Reactive StateStore single source of truth
│   │   ├── Views/
│   │   │   ├── StatusCardView.swift          # Top popover card with live score
│   │   │   ├── MenuContentView.swift         # Profile switcher and quick toggles
│   │   │   └── SettingsView.swift            # Multi-tab Security Cockpit window
│   │   ├── Engines/
│   │   │   ├── AutoFlowMonitor.swift         # CoreWLAN + NWPathMonitor event monitor
│   │   │   ├── AppTriggerEngine.swift        # NSWorkspace event engine
│   │   │   └── CircadianScheduler.swift      # Local time and schedule manager
│   │   └── IPC/
│   │       └── XPCClient.swift               # Typed async/await XPC client
│   ├── PostureFlowHelper/                    # Privileged Root Helper Daemon
│   │   ├── main.swift                        # Mach service listener entrypoint
│   │   ├── HelperService.swift               # Implementation of PostureFlowHelperProtocol
│   │   ├── PacketFilterManager.swift         # pfctl anchor generator & controller
│   │   └── PowerManager.swift                # pmset & battery telemetry manager
│   └── postureflow-cli/                      # Terminal CLI tool for macOS
│       └── main.swift                        # Command-line flags and argument parser
└── Tests/
    ├── PostureFlowSharedTests/
    │   └── PostureModelTests.swift           # Tests for posture metadata & scoring
    └── PostureFlowHelperTests/
        └── PacketFilterManagerTests.swift    # Tests for pfctl anchor generation
```

---

## 🛠️ Building & Developing

### Requirements
* macOS 13.0 (Ventura) or later
* Xcode 15.0+ or Swift 5.9+ toolchain

### Build via Swift Package Manager
```bash
cd macos

# Build all targets (App, Helper, CLI, Shared)
swift build

# Run unit tests
swift test
```

### Running the CLI
```bash
swift run postureflow --status
swift run postureflow --score
swift run postureflow --travel
```

### Running the MenuBarExtra App
```bash
swift run PostureFlowApp
```

### Registering the Privileged Helper Tool
In macOS Ventura and later, the helper daemon is managed via `SMAppService`:
```swift
let service = SMAppService.daemon(plistName: "com.postureflow.helper.plist")
try service.register()
```
Or manually for local testing:
```bash
sudo cp Resources/com.postureflow.helper.plist /Library/LaunchDaemons/
sudo cp .build/debug/PostureFlowHelper /Library/PrivilegedHelperTools/com.postureflow.helper
sudo launchctl load -w /Library/LaunchDaemons/com.postureflow.helper.plist
```

---

## 🔒 Security & Packet Filter Architecture

PostureFlow for macOS does **not** modify `/etc/pf.conf` directly. Instead, it maintains rules in `/etc/pf.anchors/com.postureflow` and references the anchor:

```pf
anchor "com.postureflow"
load anchor "com.postureflow" from "/etc/pf.anchors/com.postureflow"
```

| Posture | Inbound Policy | Packet Filter Rules | Power Profile |
| :--- | :--- | :--- | :--- |
| 🏠 **Home** | LAN + AirDrop | Passes mDNS (`5353/udp`), LAN subnets, Steam; blocks WAN | Standard |
| 💼 **Work** | VPN + IPP | Blocks LAN dev ports; passes `utun*` (VPN) and CUPS (`631`) | High Performance |
| 💻 **Dev** | Developer Ports | Passes local dev ports (`3000`, `5173`, `8080`, `8000`) | Standard |
| ✈️ **Travel** | Lockdown | Drops ICMP ping (Stealth); blocks all inbound | Low Power Mode |
