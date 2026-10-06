# Builds Falog in release mode and installs it for the current user:
#   - binaries in %LOCALAPPDATA%\Falog
#   - launch at sign-in (HKCU Run key) and a Start menu shortcut
#   - the MCP server registered in Claude Code (user scope), if the `claude` CLI is available
#   - the assistant workspace copied to -AssistantDir
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
param(
    [string]$AssistantDir = (Join-Path $HOME 'falog-assistant')
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$dest = Join-Path $env:LOCALAPPDATA 'Falog'
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# Voice input compiles whisper.cpp, which needs CMake (shipped with the VS Build Tools) and
# libclang (LLVM). Find them if they are installed but not on PATH.
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $vs = & $vswhere -latest -products * -property installationPath
        $cmake = Join-Path $vs 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin'
        if (Test-Path $cmake) { $env:Path = "$cmake;$env:Path" }
    }
}
if (-not $env:LIBCLANG_PATH -and (Test-Path "$env:ProgramFiles\LLVM\bin\libclang.dll")) {
    $env:LIBCLANG_PATH = "$env:ProgramFiles\LLVM\bin"
}
$features = @()
if (-not (Get-Command cmake -ErrorAction SilentlyContinue) -or -not ($env:LIBCLANG_PATH -or (Get-Command clang -ErrorAction SilentlyContinue))) {
    Write-Warning 'CMake or LLVM not found: building without voice input (winget install LLVM.LLVM to enable it).'
    $features = @('--no-default-features')
}

Write-Host '==> Building (release)'
cargo build --release --workspace @features --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

Write-Host "==> Installing to $dest"
# Running executables cannot be overwritten.
Get-Process falog, falog-mcp -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item "$root\target\release\falog.exe", "$root\target\release\falog-mcp.exe" $dest -Force

Write-Host '==> Launch at sign-in'
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Falog' -Value "`"$dest\falog.exe`""

Write-Host '==> Start menu shortcut'
$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut(
    (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Falog.lnk'))
$shortcut.TargetPath = "$dest\falog.exe"
$shortcut.WorkingDirectory = $dest
$shortcut.Description = 'Falog task board'
$shortcut.Save()

Write-Host '==> Registering the MCP server with Claude Code'
$claude = Get-Command claude -ErrorAction SilentlyContinue
if ($claude) {
    $ErrorActionPreference = 'Continue'
    & $claude.Source mcp remove --scope user falog *> $null
    & $claude.Source mcp add --scope user falog -- "$dest\falog-mcp.exe"
    $ErrorActionPreference = 'Stop'
} else {
    Write-Warning "The 'claude' CLI was not found. Register the server yourself:"
    Write-Host "  claude mcp add --scope user falog -- `"$dest\falog-mcp.exe`""
}

Write-Host "==> Assistant workspace at $AssistantDir"
New-Item -ItemType Directory -Force (Join-Path $AssistantDir '.claude') | Out-Null
Copy-Item "$root\assistant\CLAUDE.md" $AssistantDir -Force
Copy-Item "$root\assistant\.claude\settings.json" (Join-Path $AssistantDir '.claude') -Force

Write-Host '==> Starting Falog'
Start-Process "$dest\falog.exe"
Write-Host "Done. Open a Claude Code session in $AssistantDir and tell it about your tasks."
