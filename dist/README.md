# 📦 PostureFlow v1.0.0 Release Packages

This directory contains pre-built installation packages and scripts for PostureFlow v1.0.0 across all supported operating systems.

---

## 🐧 Linux: Ubuntu 24.04 LTS & Pop!_OS 24.04 LTS (COSMIC)

* **Package:** `postureflow_1.0.0_amd64.deb` (7.8 MB)
* **Contents:**
  * `postureflow-daemon` (Background system daemon)
  * `postureflow-applet` (System status badge / notification tray)
  * `postureflow-gui` (Security Cockpit UI in `egui` / `eframe`)
  * `systemd` unit files, D-Bus policies, Polkit rules, Hicolor icons, and manpages.

### Installation
```bash
sudo apt update
sudo apt install -y ./dist/postureflow_1.0.0_amd64.deb
```

### Uninstallation
```bash
sudo apt remove --purge -y postureflow
```

---

## 🪟 Windows: Windows 11 (24H2 / 23H2 x86_64)

* **Package:** `postureflow_1.0.0_windows_x64.zip` (8.0 MB)
* **Contents:**
  * `postureflow-daemon.exe` (Background posture and network monitor)
  * `postureflow-gui.exe` (Security Cockpit UI for Windows)
  * `install.ps1` (Automated elevated PowerShell installer)
  * `uninstall.ps1` (Automated elevated PowerShell uninstaller)
  * `PostureFlow.manifest` (PerMonitorV2 High-DPI application manifest)
  * Documentation & system service definitions

### Installation (Run PowerShell as Administrator)
```powershell
Expand-Archive -Path dist\postureflow_1.0.0_windows_x64.zip -DestinationPath C:\Temp\PostureFlow
cd C:\Temp\PostureFlow\windows-x64
.\install.ps1
```

### Uninstallation (Run PowerShell as Administrator)
```powershell
cd C:\Temp\PostureFlow\windows-x64
.\uninstall.ps1
```

---

## 🍏 macOS: macOS 15 Sequoia & macOS 14 Sonoma (Apple Silicon & Intel)

* **Package:** `postureflow_1.0.0_macos_universal.tar.gz` (240 KB)
* **Contents:**
  * `postureflow` (Terminal CLI binary)
  * `PostureFlowApp` (Native SwiftUI MenuBarExtra App binary)
  * `PostureFlowHelper` (Root Privileged Helper Daemon binary)
  * `install.sh` (Automated Privileged Helper & launchd deployment script)
  * `uninstall.sh` (Daemon removal & packet filter anchor flush script)

### Installation
```bash
tar -xzvf dist/postureflow_1.0.0_macos_universal.tar.gz
cd macos
sudo ./install.sh
```

### Uninstallation
```bash
cd macos
sudo ./uninstall.sh
```
