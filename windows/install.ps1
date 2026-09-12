# PostureFlow for Windows 11 - Automated Installation Script
# Requires Administrator Privileges

$ErrorActionPreference = "Stop"

function Assert-Administrator {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Error "Elevated Administrator permissions are required to install PostureFlow. Please re-run PowerShell as Administrator."
    }
}

Assert-Administrator

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Installing PostureFlow for Windows 11 " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

$InstallDir = "$env:ProgramFiles\PostureFlow"
$DataDir = "$env:ProgramData\PostureFlow"
$SourceDir = $PSScriptRoot

# 1. Create target directories
Write-Host "[1/6] Creating program and data directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
New-Item -ItemType Directory -Path $DataDir -Force | Out-Null

# 2. Copy binaries
Write-Host "[2/6] Deploying PostureFlow binaries..." -ForegroundColor Yellow
$Binaries = @("postureflow-gui.exe", "postureflow-daemon.exe")
foreach ($bin in $Binaries) {
    $sourcePath = Join-Path $SourceDir $bin
    if (-not (Test-Path $sourcePath)) {
        $sourcePath = Join-Path $SourceDir "..\target\release\$bin"
    }
    if (Test-Path $sourcePath) {
        Copy-Item -Path $sourcePath -Destination $InstallDir -Force
        Write-Host "  -> Installed $bin to $InstallDir" -ForegroundColor Green
    } else {
        Write-Warning "Binary $bin not found in $SourceDir. Please compile with 'cargo build --release' first."
    }
}

# 3. Add to System PATH
Write-Host "[3/6] Adding $InstallDir to System PATH..." -ForegroundColor Yellow
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$InstallDir", "Machine")
    Write-Host "  -> Added to Machine PATH successfully." -ForegroundColor Green
} else {
    Write-Host "  -> Already present in Machine PATH." -ForegroundColor Gray
}

# 4. Create Start Menu Shortcuts
Write-Host "[4/6] Generating Start Menu shortcuts..." -ForegroundColor Yellow
$ProgramsFolder = [Environment]::GetFolderPath('CommonPrograms')
$ShortcutDir = Join-Path $ProgramsFolder "PostureFlow"
New-Item -ItemType Directory -Path $ShortcutDir -Force | Out-Null

$WScriptShell = New-Object -ComObject WScript.Shell
$Shortcut = $WScriptShell.CreateShortcut((Join-Path $ShortcutDir "PostureFlow Cockpit.lnk"))
$Shortcut.TargetPath = Join-Path $InstallDir "postureflow-gui.exe"
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Description = "PostureFlow Dynamic Security & Lifestyle Posture Manager"
$Shortcut.Save()
Write-Host "  -> Created shortcut in Start Menu: $ShortcutDir" -ForegroundColor Green

# 5. Configure Windows Defender Firewall baseline
Write-Host "[5/6] Ensuring Windows Defender Firewall baseline..." -ForegroundColor Yellow
netsh advfirewall firewall add rule name="PostureFlow-AntiLockout-RDP" dir=in action=allow protocol=TCP localport=3389 enable=yes | Out-Null
netsh advfirewall firewall add rule name="PostureFlow-Loopback-In" dir=in action=allow remoteip=127.0.0.1 enable=yes | Out-Null
Write-Host "  -> Loopback and RDP anti-lockout rules configured." -ForegroundColor Green

# 6. Initialize Default State
Write-Host "[6/6] Initializing default state..." -ForegroundColor Yellow
Set-Content -Path (Join-Path $DataDir "state") -Value "home"
Write-Host "  -> Initial posture initialized to 'HOME'." -ForegroundColor Green

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " PostureFlow v1.0.0 installed successfully on Windows 11! " -ForegroundColor Green
Write-Host " Launch with: postureflow-gui.exe or run 'postureflow status'" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
