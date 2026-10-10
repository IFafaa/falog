#!/bin/sh
# Runs `cargo build` with the given arguments for the release workflow. On failure it repeats the
# error lines as GitHub annotations: job logs need a GitHub login, annotations are public, so whoever
# fixes the build can read why it failed.
set -u
log=$(mktemp)
cargo build "$@" >"$log" 2>&1
status=$?
cat "$log"
if [ "$status" -ne 0 ]; then
    grep -E 'error(\[E[0-9]+\])?:|CMake Error|fatal error|panicked at' "$log" | tail -n 10 |
        while IFS= read -r line; do
            printf '::error::%s\n' "$line"
        done
fi
rm -f "$log"
exit "$status"
