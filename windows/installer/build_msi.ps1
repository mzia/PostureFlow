# PowerShell script to package PostureFlow into an MSI installer using WiX Toolset
param (
    [string]$Configuration = "release"
)

$ErrorActionPreference = "Stop"

Write-Host "==> Building PostureFlow binaries ($Configuration)..." -ForegroundColor Cyan
cargo build --$Configuration --bin postureflow-gui --bin postureflow-daemon

if (-not (Get-Command candle.exe -ErrorAction SilentlyContinue)) {
    Write-Warning "WiX Toolset (candle.exe / light.exe) not found in PATH."
    Write-Host "Install WiX via: winget install WiX.Toolset" -ForegroundColor Yellow
    exit 0
}

Write-Host "==> Compiling WiX installer..." -ForegroundColor Cyan
candle.exe windows\installer\PostureFlow.wxs -out target\PostureFlow.wixobj
light.exe target\PostureFlow.wixobj -out target\PostureFlow-1.0.0-x64.msi -ext WixUIExtension

Write-Host "==> MSI successfully generated at target\PostureFlow-1.0.0-x64.msi" -ForegroundColor Green
