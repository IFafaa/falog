# Removes what install.ps1 set up. Your tasks database (%APPDATA%\Falog) is kept unless -RemoveData.
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

if ($RemoveData) {
    Remove-Item (Join-Path $env:APPDATA 'Falog') -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host 'Falog and its data were removed.'
} else {
    Write-Host "Falog was removed. Your tasks are still in $(Join-Path $env:APPDATA 'Falog')."
}
