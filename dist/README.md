# 📦 PostureFlow v1.0.1 Release Packages

This directory contains pre-built installation packages and scripts for PostureFlow v1.0.1 across all supported operating systems.

---

## 🐧 Linux: Ubuntu 24.04 LTS & Pop!_OS 24.04 LTS (COSMIC)

* **Package:** `postureflow_1.0.1_amd64.deb` (7.8 MB)
* **Contents:**
  * `postureflow-daemon` (Background system daemon)
  * `postureflow-applet` (System status badge / notification tray)
  * `postureflow-gui` (Security Cockpit UI in `egui` / `eframe`)
  * `systemd` unit files, D-Bus policies, Polkit rules, Hicolor icons, and manpages.

### Installation
```bash
sudo apt update
sudo apt install -y ./dist/postureflow_1.0.1_amd64.deb
```

### Uninstallation
```bash
sudo apt remove --purge -y postureflow
```

---

## 🪟 Windows: Windows 11 (24H2 / 23H2 x86_64)

* **Packages:**
  * `PostureFlow-1.0.1-x64.msi` (Standard Windows Installer Package)
  * `postureflow_1.0.1_windows_x64.zip` (Portable archive with automated deployment scripts)
* **Contents:**
  * `postureflow-daemon.exe` (Background posture and network monitor)
  * `postureflow-gui.exe` (Security Cockpit UI for Windows)
  * `install.ps1` (Automated elevated PowerShell installer)
  * `uninstall.ps1` (Automated elevated PowerShell uninstaller)
  * `PostureFlow.manifest` (PerMonitorV2 High-DPI application manifest)
  * Documentation & system service definitions

### Option A: Standard MSI Installation (Recommended)
1. Double-click `PostureFlow-1.0.1-x64.msi` to launch the Windows Installer wizard.
2. Follow on-screen prompts; PostureFlow Cockpit shortcut will be added to the Start Menu.

### Option B: Automated Script Installation (Run PowerShell as Administrator)
```powershell
Expand-Archive -Path dist\postureflow_1.0.1_windows_x64.zip -DestinationPath C:\Temp\PostureFlow
cd C:\Temp\PostureFlow\windows-x64
.\install.ps1
```

### Uninstallation
* **Via Settings / Control Panel:** Navigate to **Installed Apps**, select **PostureFlow**, and click **Uninstall**.
* **Via PowerShell (Administrator):**
  ```powershell
  cd C:\Temp\PostureFlow\windows-x64
  .\uninstall.ps1
  ```

---

## 🍏 macOS: macOS 15 Sequoia & macOS 14 Sonoma (Apple Silicon & Intel)

* **Packages:**
  * `PostureFlow_1.0.1.dmg` (Standard macOS Drag-and-Drop Disk Image)
  * `postureflow_1.0.1_macos_universal.tar.gz` (Universal archive with automated scripts)
* **Contents:**
  * `PostureFlow.app` (Native SwiftUI MenuBarExtra App bundle)
  * `postureflow` (Terminal CLI binary)
  * `PostureFlowHelper` (Root Privileged Helper Daemon binary)
  * `install.sh` (Automated Privileged Helper & launchd deployment script)
  * `uninstall.sh` (Daemon removal & packet filter anchor flush script)

### Option A: Drag-and-Drop DMG Installation (Recommended)
1. Double-click `PostureFlow_1.0.1.dmg` to mount the image.
2. Drag **PostureFlow.app** into the **Applications** folder.
3. Launch **PostureFlow** from Spotlight or `/Applications`.
4. The PostureFlow shield icon appears in your menu bar.
5. Click the shield icon -> **Settings (gear)** -> **Security & Power** tab.
6. Click **Register Helper Tool**; macOS will prompt for your Touch ID or password to grant background daemon privileges (via `SMAppService`).

### Option B: Automated Script Installation
```bash
tar -xzvf dist/postureflow_1.0.1_macos_universal.tar.gz
cd macos
sudo ./install.sh
```

### Uninstallation
* **Via App / Finder:** Drag `/Applications/PostureFlow.app` to Trash and run:
  ```bash
  sudo launchctl bootout system/com.postureflow.helper 2>/dev/null || true
  sudo rm -f /Library/LaunchDaemons/com.postureflow.helper.plist /Library/PrivilegedHelperTools/com.postureflow.helper
  ```
* **Via Script:**
  ```bash
  cd macos
  sudo ./uninstall.sh
  ```
