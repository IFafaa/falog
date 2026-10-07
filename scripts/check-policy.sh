#!/bin/sh
# Repository policy, run by CI and by scripts/check.sh: fails on things that must never be committed.
# See the CI section of .spec/conventions.md for what each rule is for and how to fix a failure.
#
# Searches the files git tracks plus, when run locally, new files not ignored by .gitignore.
# Usage: scripts/check-policy.sh
set -u

cd "$(git rev-parse --show-toplevel)" || exit 2

status=0
# This file lists every pattern, so it would match itself.
self=':(exclude)scripts/check-policy.sh'

# check DESCRIPTION PATTERN [GIT GREP FLAGS] [EXCLUDED PATHSPEC]: reports the lines matching the
# extended regular expression PATTERN and fails the run.
check() {
    description=$1
    pattern=$2
    flags=${3:-}
    exclude=${4:-$self}
    # shellcheck disable=SC2086 # FLAGS is a list of options.
    hits=$(git grep --untracked -n -I -E $flags -e "$pattern" -- . "$self" "$exclude")
    code=$?
    if [ "$code" -eq 0 ]; then
        printf '%s:\n%s\n\n' "$description" "$hits" >&2
        status=1
    elif [ "$code" -ne 1 ]; then
        printf 'git grep failed while checking: %s\n' "$description" >&2
        status=2
    fi
}

# Secrets. Credentials belong in the user's data folder (calendar/google.json), never in the repo.
check 'Google OAuth client secret' 'GOCSPX-[A-Za-z0-9_-]{20,}'
check 'Google OAuth token' 'ya29\.[A-Za-z0-9_-]{20,}|1//0[A-Za-z0-9_-]{30,}'
check 'Google API key' 'AIza[A-Za-z0-9_-]{35}'
check 'Google Calendar secret iCal address' 'calendar/ical/[^/[:space:]]+/private-[0-9a-f]{32}'
check 'private key' '-----BEGIN ([A-Z0-9]+ )*PRIVATE KEY( BLOCK)?-----'
check 'GitHub token' '(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,}'
check 'Anthropic API key' 'sk-ant-[A-Za-z0-9_-]{20,}'
check 'AWS access key' '(AKIA|ASIA)[0-9A-Z]{16}'
check 'Slack token' 'xox[abposr]-[0-9A-Za-z-]{10,}'

# Files that hold user data or credentials.
files=$(git ls-files --cached --others --exclude-standard |
    grep -E '(^|/)(google\.json|\.env(\..+)?|[^/]+\.(db|db-wal|db-shm|sqlite|pem|key|p12|pfx))$')
if [ -n "$files" ]; then
    printf 'files that hold user data or credentials:\n%s\n\n' "$files" >&2
    status=1
fi

# Other apps named as the model for a design (conventions.md, "Naming other products"). Credits
# required by a license stay in the notices next to the assets.
credits=':(exclude)crates/falog-desktop/assets/THIRD-PARTY-NOTICES.md'
check 'another app named as a design model' \
    'zed|vs ?code|visual studio code|sublime text|notion|todoist|trello|obsidian|raycast|ticktick|things 3' \
    "-i -w" "$credits"
check 'a design described as copied from somewhere else' \
    '(modeled|modelled|styled|patterned) (on|after)|[a-z]-inspired|inspired by|in the style of' \
    -i "$credits"

# Leftovers of a merge or rebase.
check 'merge conflict marker' '^(<{7}|>{7}|[|]{7})( |$)|^={7}$'

if [ "$status" -eq 0 ]; then
    echo 'policy: ok'
fi
exit "$status"
