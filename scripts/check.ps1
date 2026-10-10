# Runs the CI quality gate (.github/workflows/ci.yml) on this machine before you push: format,
# clippy, tests, docs and the repository policy, plus cargo-deny, typos and the commit messages of
# unpushed commits when those tools are installed. Coverage, the minimum Rust version and the other
# operating systems run only in CI. See the CI section of .spec/conventions.md.
#
#   .\scripts\check.ps1              # with and without voice input
#   .\scripts\check.ps1 -NoVoice     # only without voice (no CMake or LLVM needed)
#   .\scripts\check.ps1 -Gpu         # also lint the gpu feature (needs the Vulkan SDK)
param(
    [switch]$NoVoice,
    [switch]$Gpu
)
$ErrorActionPreference = 'Stop'
Set-Location (Resolve-Path (Join-Path $PSScriptRoot '..'))

$failed = New-Object System.Collections.Generic.List[string]

# Runs one check and remembers it when it fails.
function Step([string]$name, [scriptblock]$command) {
    Write-Host "`n==> $name" -ForegroundColor Cyan
    # Exit codes decide; Windows PowerShell would otherwise stop on the progress cargo writes to
    # stderr whenever the output is redirected.
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    & $command
    if ($LASTEXITCODE -ne 0) { $failed.Add($name) }
}
function Skip([string]$name, [string]$why) {
    Write-Host "`n==> ${name}: skipped, $why" -ForegroundColor DarkGray
}

# The policy and commit checks are POSIX sh. Git for Windows ships bash next to git.exe (the bash on
# PATH may be WSL's, which sees another file system).
function Find-Bash {
    $git = (Get-Command git -ErrorAction SilentlyContinue).Source
    for ($dir = if ($git) { Split-Path $git } else { $null }; $dir; $dir = Split-Path $dir) {
        $bash = Join-Path $dir 'bin\bash.exe'
        if (Test-Path $bash) { return $bash }
    }
    if ($IsLinux -or $IsMacOS) { return (Get-Command sh -ErrorAction SilentlyContinue).Source }
    return $null
}
$bash = Find-Bash

Step 'format' { cargo fmt --all -- --check }

Step 'clippy-no-default' { cargo clippy --workspace --all-targets --locked --no-default-features -- -D warnings }
if (-not $NoVoice) {
    Step 'clippy-default' { cargo clippy --workspace --all-targets --locked -- -D warnings }
}
if ($Gpu) {
    Step 'clippy-gpu' { cargo clippy --workspace --all-targets --locked --features falog-desktop/gpu -- -D warnings }
}

Step 'test-no-default' { cargo test --workspace --locked --no-default-features }
if (-not $NoVoice) {
    Step 'test-default' { cargo test --workspace --locked }
}

$docFeatures = @(if ($NoVoice) { '--no-default-features' })
Step 'docs' {
    $env:RUSTDOCFLAGS = '-D warnings'
    try { cargo doc --workspace --no-deps --document-private-items --locked $docFeatures }
    finally { Remove-Item Env:RUSTDOCFLAGS }
}

if ($bash) {
    Step 'policy' { & $bash scripts/check-policy.sh }
} else {
    $failed.Add('policy')
    Skip 'policy' 'needs Git for Windows (bash)'
}

cargo deny --version *> $null
if ($LASTEXITCODE -eq 0) {
    Step 'dependencies' { cargo deny --locked check }
} else {
    Skip 'dependencies' 'install it with: cargo install --locked cargo-deny'
}
cargo machete --version *> $null
if ($LASTEXITCODE -eq 0) {
    Step 'unused-dependencies' { cargo machete }
} else {
    Skip 'unused-dependencies' 'install it with: cargo install --locked cargo-machete'
}
if (Get-Command npx -ErrorAction SilentlyContinue) {
    Step 'duplication' { npx --yes jscpd@5.4.1 crates tools }
} else {
    Skip 'duplication' 'needs Node.js (npx)'
}
if (Get-Command typos -ErrorAction SilentlyContinue) {
    Step 'typos' { typos }
} else {
    Skip 'typos' 'install it with: cargo install --locked typos-cli'
}
git rev-parse --verify --quiet origin/main *> $null
if ($LASTEXITCODE -eq 0 -and $bash) {
    Step 'commits' { & $bash scripts/check-commits.sh origin/main..HEAD }
} else {
    Skip 'commits' 'no origin/main to compare with'
}

if ($failed.Count -gt 0) {
    Write-Host "`nFailed: $($failed -join ', ')" -ForegroundColor Red
    exit 1
}
Write-Host "`nAll checks passed." -ForegroundColor Green
