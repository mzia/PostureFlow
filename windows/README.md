# 🪟 PostureFlow for Windows 11

PostureFlow brings dynamic security postures, hardware power optimization, and lifestyle workflow orchestration to Windows 11.

---

## 🏗️ Architecture on Windows

* **Perimeter Defense:** Managed via **Windows Defender Firewall** (`netsh advfirewall firewall`). Rules are prefixed with `PostureFlow-` to guarantee zero interference with existing system policies.
* **Power Orchestration:** Direct integration with Windows power schemes (`powercfg /setactive`) switching dynamically between **High Performance** (Dev), **Balanced** (Home / Work), and **Power Saver** (Travel), with screen idle timeouts managed per posture.
* **Autonomous Auto-Flow:** Network SSID detection via Windows Native Wifi (`netsh wlan show interfaces`) with zero overhead.
* **App-Aware Triggers:** Process sensing (`tasklist` / `Toolhelp32`) with zero-allocation filtering for developer IDEs, video calls, and gaming clients.
* **Security Cockpit GUI:** Native high-DPI desktop cockpit powered by `egui` and `eframe` (`postureflow-gui.exe`).
* **Privilege Separation:** Background service communication via Named Pipe (`\\.\pipe\PostureFlowPipe`).

---

## 🚀 Building on Windows 11

### Prerequisites
* Rust 1.80+ (`x86_64-pc-windows-msvc` or `x86_64-pc-windows-gnu`)
* Windows 11 (build 22000+) or Windows 10 (21H2+)

### Compile
```powershell
# Build all PostureFlow Windows binaries
cargo build --release --bin postureflow-gui --bin postureflow-daemon
```

### Run
```powershell
# Launch the Security Cockpit GUI
.\target\release\postureflow-gui.exe

# Or use the CLI
.\target\release\postureflow-daemon.exe status
.\target\release\postureflow-daemon.exe set work
```

---

## 📦 Packaging & MSI Installer
To build the MSI installer package using WiX Toolset:
```powershell
.\windows\installer\build_msi.ps1
```
