#!/bin/sh
# Checks that commit subjects follow Conventional Commits as .spec/conventions.md describes them:
# `type(scope): summary`, a known type, an optional lowercase scope, a lowercase summary of at most
# 72 characters in all without a trailing period. Merge commits are skipped.
#
# Usage: scripts/check-commits.sh [RANGE]   (default: origin/main..HEAD)
set -u

range=${1:-origin/main..HEAD}
types='feat|fix|refactor|perf|test|docs|style|build|ci|chore|revert'
subject_format="^($types)(\\([a-z0-9][a-z0-9-]*\\))?!?: [a-z0-9\`\"'(].*[^.]\$"

subjects=$(git log --no-merges --format='%h %s' "$range") || {
    echo "cannot list the commits of $range" >&2
    exit 2
}

status=0
count=0
# The subjects come from a variable so the loop runs in this shell and can set the status.
while IFS= read -r line; do
    [ -n "$line" ] || continue
    count=$((count + 1))
    hash=${line%% *}
    subject=${line#* }
    problem=''
    if ! printf '%s\n' "$subject" | grep -Eq "$subject_format"; then
        problem="not 'type(scope): summary' with type one of ${types}, a lowercase summary and no final period"
    elif [ "${#subject}" -gt 72 ]; then
        problem="longer than 72 characters (${#subject})"
    fi
    if [ -n "$problem" ]; then
        printf '%s %s\n    %s\n' "$hash" "$subject" "$problem" >&2
        status=1
    fi
done <<EOF
$subjects
EOF

if [ "$status" -eq 0 ]; then
    echo "commits: $count ok"
fi
exit "$status"
