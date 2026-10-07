#!/usr/bin/env bash
# Fills a throwaway database with sample areas and tasks by talking to falog-mcp, exactly as an
# assistant would. Useful for screenshots and for trying the app (macOS and Linux).
#
#   ./scripts/seed-demo.sh [DATABASE]          # default: target/demo.db
#   FALOG_DB="$PWD/target/demo.db" cargo run -p falog-desktop
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
database="${1:-$root/target/demo.db}"

cargo build --quiet -p falog-mcp --manifest-path "$root/Cargo.toml"
target_dir="$(cd "$root" && cargo metadata --format-version 1 --no-deps 2> /dev/null |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
mcp="${target_dir:-$root/target}/debug/falog-mcp"

mkdir -p "$(dirname "$database")"
database="$(cd "$(dirname "$database")" && pwd)/$(basename "$database")"
rm -f "$database" "$database-wal" "$database-shm"
export FALOG_DB="$database"

# Today plus `$1` days, as YYYY-MM-DD, with GNU date (Linux) or BSD date (macOS).
day() {
    if date -d now > /dev/null 2>&1; then
        date -d "$1 days" +%F
    else
        case "$1" in
            -*) date -v "${1}d" +%F ;;
            *) date -v "+${1}d" +%F ;;
        esac
    fi
}

calls=(
    '"create_area","arguments":{"name":"Work"}'
    '"create_area","arguments":{"name":"Home"}'
    '"create_area","arguments":{"name":"Studies"}'
    '"create_area","arguments":{"name":"Health"}'
    '"create_task","arguments":{"title":"Fix the sign-in bug on Android","area":"work","priority":"urgent","due_date":"'"$(day -2)"'","requester":"Ana","status":"in_progress","description":"Users on Android 14 get a blank screen after choosing an account. Repro on the staging build."}'
    '"create_task","arguments":{"title":"Review the payments PR","area":"work","priority":"high","due_date":"'"$(day 0)"'","requester":"Carlos","status":"waiting"}'
    '"create_task","arguments":{"title":"Send the monthly report","area":"work","due_date":"'"$(day 1)"'","requester":"Marta"}'
    '"create_task","arguments":{"title":"Pay the electricity bill","area":"home","due_date":"'"$(day 3)"'"}'
    '"create_task","arguments":{"title":"Book the car service","area":"home","priority":"high","due_date":"'"$(day 9)"'"}'
    '"create_task","arguments":{"title":"Read chapter 4 of the Rust book","area":"studies","priority":"low"}'
    '"create_task","arguments":{"title":"Plan the weekend trip","area":"home","status":"in_progress"}'
    '"create_task","arguments":{"title":"Get a quote from the plumber for the kitchen","area":"home","status":"waiting","requester":"Joe"}'
    '"create_task","arguments":{"title":"Submit the course assignment","area":"studies","status":"done"}'
    '"create_task","arguments":{"title":"Renew the gym membership","area":"health","status":"done"}'
    '"create_task","arguments":{"title":"Dentist appointment at 3 pm","area":"health","due_date":"'"$(day 2)"'"}'
    '"create_task","arguments":{"title":"Renew car insurance","area":"home","priority":"high","due_date":"'"$(day 6)"'","description":"Current policy ends next month. Compare at least two quotes."}'
    '"add_note","arguments":{"id":1,"note":"Reproduced on a Pixel 8 emulator; the redirect URI is missing from the console."}'
)

id=0
for call in "${calls[@]}"; do
    id=$((id + 1))
    printf '{"jsonrpc":"2.0","id":%d,"method":"tools/call","params":{"name":%s}}\n' "$id" "$call"
done | "$mcp" | if command -v jq > /dev/null; then jq -r '.result.content[0].text'; else cat; fi

printf '\nDemo database: %s\n' "$database"
