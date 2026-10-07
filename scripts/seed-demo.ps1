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
# Sample Google Calendar data next to the database, as if two accounts were connected. With no
# OAuth client configured Falog never goes online for it, so the calendar shows exactly this.
$calendarDir = Join-Path (Split-Path $Database) 'calendar'
New-Item -ItemType Directory -Force $calendarDir | Out-Null
$monday = (Get-Date).Date.AddDays(-(((Get-Date).DayOfWeek.value__ + 6) % 7))
function At([int]$day, [string]$time) { $monday.AddDays($day).ToString('yyyy-MM-dd') + "T${time}:00" }
function Date([int]$day) { $monday.AddDays($day).ToString('yyyy-MM-dd') }
$calendars = @{
    work = @{ id = 'me@acme.example'; name = 'Acme'; color = '#9fe1e7'; primary = $true; visible = $true }
    team = @{ id = 'team@acme.example'; name = 'Mobile team'; color = '#f691b2'; primary = $false; visible = $true }
    home = @{ id = 'me@gmail.example'; name = 'Personal'; color = '#7bd148'; primary = $true; visible = $true }
}
$accounts = @(
    @{ email = 'me@acme.example'; refresh_token = 'demo'; calendars = @($calendars.work, $calendars.team) },
    @{ email = 'me@gmail.example'; refresh_token = 'demo'; calendars = @($calendars.home) }
)
$n = 0
function Meeting($calendar, [string]$title, $start, $end, [string]$link = $null, [string]$location = '') {
    $script:n++
    $account = if ($calendar.id -like '*gmail*') { 'me@gmail.example' } else { 'me@acme.example' }
    $kind = if ($start -match 'T') { 'At' } else { 'Date' }
    @{
        account = $account; calendar_id = $calendar.id; id = "demo$script:n"; ical_uid = "demo$script:n@example"
        title = $title; start = @{ $kind = $start }; end = @{ $kind = $end }
        location = $location; description = ''; join_link = $link; html_link = $null
    }
}
$meet = 'https://meet.google.com/abc-defg-hij'
$events = @(
    (Meeting $calendars.work 'Daily standup' (At 0 '09:30') (At 0 '09:45') $meet),
    (Meeting $calendars.work 'Daily standup' (At 1 '09:30') (At 1 '09:45') $meet),
    (Meeting $calendars.work 'Daily standup' (At 2 '09:30') (At 2 '09:45') $meet),
    (Meeting $calendars.work 'Daily standup' (At 3 '09:30') (At 3 '09:45') $meet),
    (Meeting $calendars.work 'Daily standup' (At 4 '09:30') (At 4 '09:45') $meet),
    (Meeting $calendars.team 'Sprint planning' (At 0 '10:00') (At 0 '11:30') $meet),
    (Meeting $calendars.work 'Payments review with Carlos' (At 1 '14:00') (At 1 '15:00') $meet),
    (Meeting $calendars.team 'Design sync: new navbar' (At 1 '14:30') (At 1 '15:30') $meet),
    (Meeting $calendars.work '1:1 with Ana' (At 2 '11:00') (At 2 '11:30') $meet),
    (Meeting $calendars.home 'Dentist' (At 3 '15:00') (At 3 '16:00') $null 'Rua Augusta, 1200'),
    (Meeting $calendars.team 'Release 4.2 go/no-go' (At 3 '17:00') (At 3 '17:30') $meet),
    (Meeting $calendars.work 'Architecture guild' (At 4 '16:00') (At 4 '17:30') $meet),
    (Meeting $calendars.home 'Gym' (At 4 '19:00') (At 4 '20:00')),
    (Meeting $calendars.team 'Mobile offsite' (Date 2) (Date 4)),
    (Meeting $calendars.home 'Mom''s birthday' (Date 5) (Date 6))
)
$cache = @{ from = (Date -35); to = (Date 42); events = $events }
$config = @{ client = @{ id = ''; secret = '' }; accounts = $accounts }
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $calendarDir 'google.json'), ($config | ConvertTo-Json -Depth 6), $utf8)
[IO.File]::WriteAllText((Join-Path $calendarDir 'events.json'), ($cache | ConvertTo-Json -Depth 6), $utf8)

Write-Host "`nDemo database: $Database"
