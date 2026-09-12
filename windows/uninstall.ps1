# PostureFlow for Windows 11 - Automated Uninstallation Script
# Requires Administrator Privileges

$ErrorActionPreference = "Continue"

function Assert-Administrator {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Error "Elevated Administrator permissions are required to uninstall PostureFlow. Please re-run PowerShell as Administrator."
    }
}

Assert-Administrator

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host " Uninstalling PostureFlow from Windows 11 " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

$InstallDir = "$env:ProgramFiles\PostureFlow"
$DataDir = "$env:ProgramData\PostureFlow"

# 1. Terminate running instances
Write-Host "[1/5] Stopping running PostureFlow processes..." -ForegroundColor Yellow
Stop-Process -Name "postureflow-gui" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "postureflow-daemon" -Force -ErrorAction SilentlyContinue

# 2. Remove Windows Defender Firewall rules
Write-Host "[2/5] Purging PostureFlow firewall rules..." -ForegroundColor Yellow
$rules = @(
    "PostureFlow-AntiLockout-RDP",
    "PostureFlow-AntiLockout-SSH",
    "PostureFlow-Loopback-In",
    "PostureFlow-Loopback-Out",
    "PostureFlow-Block-ICMP",
    "PostureFlow-Port-3000",
    "PostureFlow-Port-5000",
    "PostureFlow-Port-5173",
    "PostureFlow-Port-8000",
    "PostureFlow-Port-8080",
    "PostureFlow-Port-8888",
    "PostureFlow-Port-9000"
)
foreach ($r in $rules) {
    netsh advfirewall firewall delete rule name=$r | Out-Null
}
Write-Host "  -> Removed all PostureFlow firewall rules." -ForegroundColor Green

# 3. Remove Start Menu shortcuts
Write-Host "[3/5] Removing Start Menu shortcuts..." -ForegroundColor Yellow
$ProgramsFolder = [Environment]::GetFolderPath('CommonPrograms')
$ShortcutDir = Join-Path $ProgramsFolder "PostureFlow"
if (Test-Path $ShortcutDir) {
    Remove-Item -Path $ShortcutDir -Recurse -Force
    Write-Host "  -> Removed shortcut directory $ShortcutDir" -ForegroundColor Green
}

# 4. Remove from System PATH
Write-Host "[4/5] Cleaning System PATH variable..." -ForegroundColor Yellow
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($CurrentPath -like "*$InstallDir*") {
    $NewPath = ($CurrentPath -split ';' | Where-Object { $_ -ne $InstallDir -and $_ -ne "" }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "Machine")
    Write-Host "  -> Removed $InstallDir from Machine PATH." -ForegroundColor Green
}

# 5. Remove Program Files and optionally Data
Write-Host "[5/5] Removing installation files..." -ForegroundColor Yellow
if (Test-Path $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force
    Write-Host "  -> Removed $InstallDir" -ForegroundColor Green
}

if (Test-Path $DataDir) {
    Remove-Item -Path $DataDir -Recurse -Force
    Write-Host "  -> Removed $DataDir" -ForegroundColor Green
}

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " PostureFlow has been completely removed from Windows 11. " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
