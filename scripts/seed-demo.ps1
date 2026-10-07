# Fills a throwaway database with sample areas and tasks by talking to falog-mcp,
# exactly as an assistant would. Useful for screenshots and for trying the app.
#
#   .\scripts\seed-demo.ps1                     # creates .\target\demo.db
#   $env:FALOG_DB = "$PWD\target\demo.db"; cargo run -p falog-desktop
param(
    [string]$Database = (Join-Path $PSScriptRoot '..\target\demo.db')
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')

cargo build --quiet -p falog-mcp --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$OutputEncoding = [Text.UTF8Encoding]::new($false)
$Database = [IO.Path]::GetFullPath($Database)
Remove-Item "$Database*" -ErrorAction SilentlyContinue
$env:FALOG_DB = $Database

function Day([int]$offset) { (Get-Date).AddDays($offset).ToString('yyyy-MM-dd') }

$calls = @(
    @{ name = 'create_area'; arguments = @{ name = 'Acme Health' } },
    @{ name = 'create_area'; arguments = @{ name = 'Globex' } },
    @{ name = 'create_area'; arguments = @{ name = 'Initech' } },
    @{ name = 'create_area'; arguments = @{ name = 'Personal' } },
    @{ name = 'create_task'; arguments = @{ title = 'Fix Google sign-in on Android'; area = 'acme'; priority = 'urgent'; due_date = (Day -2); requester = 'Ana'; status = 'in_progress'; description = 'Users on Android 14 get a blank screen after choosing an account. Repro on the staging build.' } },
    @{ name = 'create_task'; arguments = @{ title = 'Review the payments module PR'; area = 'globex'; priority = 'high'; due_date = (Day 0); requester = 'Carlos'; status = 'waiting' } },
    @{ name = 'create_task'; arguments = @{ title = 'Export enrollment report as CSV'; area = 'initech'; due_date = (Day 1); requester = 'Marta' } },
    @{ name = 'create_task'; arguments = @{ title = 'Upgrade the API to Node 22'; area = 'globex'; due_date = (Day 3) } },
    @{ name = 'create_task'; arguments = @{ title = 'Add rate limiting to the public endpoints'; area = 'acme'; priority = 'high'; due_date = (Day 9) } },
    @{ name = 'create_task'; arguments = @{ title = 'Write onboarding docs for the mobile app'; area = 'acme'; priority = 'low' } },
    @{ name = 'create_task'; arguments = @{ title = 'Investigate slow dashboard queries'; area = 'initech'; status = 'in_progress'; requester = 'Paulo' } },
    @{ name = 'create_task'; arguments = @{ title = 'Wait for design sign-off on the new navbar'; area = 'initech'; status = 'waiting'; requester = 'Lia' } },
    @{ name = 'create_task'; arguments = @{ title = 'Ship hotfix for appointment reminders'; area = 'acme'; status = 'done' } },
    @{ name = 'create_task'; arguments = @{ title = 'Rotate staging database credentials'; area = 'globex'; status = 'done' } },
    @{ name = 'create_task'; arguments = @{ title = 'Dentist appointment at 3 pm'; area = 'personal'; due_date = (Day 2) } },
    @{ name = 'create_task'; arguments = @{ title = 'Renew car insurance'; area = 'personal'; priority = 'high'; due_date = (Day 6); description = 'Current policy ends next month. Compare at least two quotes.' } },
    @{ name = 'add_note'; arguments = @{ id = 1; note = 'Reproduced on a Pixel 8 emulator; the redirect URI is missing from the console.' } }
)

$id = 0
$requests = foreach ($call in $calls) {
    $id++
    @{ jsonrpc = '2.0'; id = $id; method = 'tools/call'; params = $call } | ConvertTo-Json -Depth 5 -Compress
}
$requests | & "$root\target\debug\falog-mcp.exe" | ForEach-Object {
    ($_ | ConvertFrom-Json).result.content[0].text
}
Write-Host "`nDemo database: $Database"
