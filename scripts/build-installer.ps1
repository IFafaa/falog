# Builds the Windows installer, target\installer\Falog-Setup-<version>.exe, with Inno Setup 6.
#
#   powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1
#
# The installer carries a CPU build (speech recognition included, no GPU): the GPU build needs the
# Vulkan runtime, which not every PC has. Requirements: the MSVC build tools (with CMake), LLVM for
# voice (`winget install LLVM.LLVM`) and Inno Setup (`winget install JRSoftware.InnoSetup`).
param(
    # Build without speech recognition (no CMake/LLVM needed).
    [switch]$NoVoice
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) { throw 'Visual Studio Build Tools not found (vswhere.exe is missing).' }
$vs = & $vswhere -latest -products * -property installationPath
Import-Module (Join-Path $vs 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
if (Get-Command ninja -ErrorAction SilentlyContinue) { $env:CMAKE_GENERATOR = 'Ninja' }
if (-not $env:LIBCLANG_PATH -and (Test-Path "$env:ProgramFiles\LLVM\bin\libclang.dll")) {
    $env:LIBCLANG_PATH = "$env:ProgramFiles\LLVM\bin"
}

$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 not found: winget install JRSoftware.InnoSetup' }

# The newest Visual C++ runtime that ships with the build tools.
$crt = Get-ChildItem (Join-Path $vs 'VC\Redist\MSVC\*\x64\Microsoft.VC14*.CRT') -Directory |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $crt) { throw 'Visual C++ runtime redistributable not found under the build tools.' }

$version = (Select-String -Path "$root\Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$targetDir = Join-Path $root 'target\dist'
$features = if ($NoVoice) { @('--no-default-features') } else { @() }

Write-Host "==> Building Falog $version (release)"
cargo build --release --workspace @features --target-dir $targetDir --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

Write-Host '==> Packaging'
& $iscc /Q "/DAppVersion=$version" "/DBinDir=$targetDir\release" "/DCrtDir=$($crt.FullName)" "$root\installer\falog.iss"
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup failed' }

$setup = Join-Path $root "target\installer\Falog-Setup-$version.exe"
Write-Host "Done: $setup ($([math]::Round((Get-Item $setup).Length / 1MB, 1)) MB)"
