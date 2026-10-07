# Removes what install.ps1 set up. Your tasks (~\.falog) are kept unless -RemoveData.
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\uninstall.ps1 [-RemoveData]
param(
    [switch]$RemoveData
)
$ErrorActionPreference = 'Stop'

Get-Process falog, falog-mcp -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Falog' -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Falog.lnk') -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:LOCALAPPDATA 'Falog') -Recurse -Force -ErrorAction SilentlyContinue

$claude = Get-Command claude -ErrorAction SilentlyContinue
if ($claude) {
    $ErrorActionPreference = 'Continue'
    & $claude.Source mcp remove --scope user falog
    $ErrorActionPreference = 'Stop'
}

# Data lives in ~\.falog; older versions kept it in %APPDATA%\Falog (and eframe in %APPDATA%\falog\data).
$data = Join-Path $HOME '.falog'
if ($RemoveData) {
    Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $env:APPDATA 'Falog') -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host 'Falog and its data were removed.'
} else {
    Write-Host "Falog was removed. Your tasks are still in $data."
}
