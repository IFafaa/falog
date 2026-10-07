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

# Voice input compiles whisper.cpp, which needs CMake and Ninja (both ship with the VS Build Tools),
# libclang (LLVM) and, for GPU acceleration, the Vulkan SDK. Load the Visual Studio developer
# environment so CMake can use Ninja: the default MSBuild generator hits Windows' 260-character
# path limit on the Vulkan shader build.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vswhere) {
    $env:Path = "$(Split-Path $vswhere);$env:Path"  # VsDevCmd calls vswhere itself
    $vs = & $vswhere -latest -products * -property installationPath
    $devShell = Join-Path $vs 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll'
    if (Test-Path $devShell) {
        Import-Module $devShell
        Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
        if (Get-Command ninja -ErrorAction SilentlyContinue) { $env:CMAKE_GENERATOR = 'Ninja' }
    }
}
if (-not $env:LIBCLANG_PATH -and (Test-Path "$env:ProgramFiles\LLVM\bin\libclang.dll")) {
    $env:LIBCLANG_PATH = "$env:ProgramFiles\LLVM\bin"
}
if (-not $env:VULKAN_SDK) {
    $env:VULKAN_SDK = [Environment]::GetEnvironmentVariable('VULKAN_SDK', 'Machine')
}

$features = @()
if (-not (Get-Command cmake -ErrorAction SilentlyContinue) -or -not ($env:LIBCLANG_PATH -or (Get-Command clang -ErrorAction SilentlyContinue))) {
    Write-Warning 'CMake or LLVM not found: building without voice input (winget install LLVM.LLVM to enable it).'
    $features = @('--no-default-features')
} elseif ($env:VULKAN_SDK -and $env:CMAKE_GENERATOR -eq 'Ninja') {
    Write-Host '==> Vulkan SDK found: speech recognition can run on the GPU'
    $features = @('--features', 'falog-desktop/gpu')
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
Copy-Item "$root\crates\falog-desktop\assets\icon\falog.ico" $dest -Force

Write-Host '==> Launch at sign-in'
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Falog' -Value "`"$dest\falog.exe`""

Write-Host '==> Start menu shortcut'
$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut(
    (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Falog.lnk'))
$shortcut.TargetPath = "$dest\falog.exe"
$shortcut.WorkingDirectory = $dest
$shortcut.Description = 'Falog task board'
# falog.exe carries the icon too; the separate file keeps the shortcut right if embedding it failed.
$shortcut.IconLocation = "$dest\falog.ico,0"
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
