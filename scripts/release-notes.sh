#!/bin/sh
# Writes the release notes of a tag as Markdown: the Conventional Commits since the previous release,
# grouped into Features (feat), Fixes (fix), Performance (perf) and Other. chore, ci, docs, style and
# test commits are left out unless nothing else changed. The release workflow
# (.github/workflows/release.yml) puts the output in the release body; the landing page shows it as the
# changelog. See .spec/releases.md.
#
#   scripts/release-notes.sh TAG
#
# A stable tag (v0.3.0) is compared with the previous stable tag, so its notes cover the previews in
# between; a preview tag (v0.3.0-beta.2) with the previous tag of either kind. Needs the full history
# and tags (actions/checkout with fetch-depth: 0).
set -eu

if [ $# -ne 1 ]; then
    echo "usage: $0 TAG" >&2
    exit 2
fi
tag=$1
repo=${GITHUB_REPOSITORY:-IFafaa/falog}

case "$tag" in
    *-*) prev=$(git describe --tags --abbrev=0 --match 'v[0-9]*' "$tag^" 2> /dev/null || true) ;;
    *) prev=$(git describe --tags --abbrev=0 --match 'v[0-9]*' --exclude 'v*-*' "$tag^" 2> /dev/null || true) ;;
esac

nl='
'
features=""
fixes=""
performance=""
other=""
minor=""
log=$(git log --no-merges --reverse --format='%h %s' "${prev:+$prev..}$tag")
while IFS= read -r line; do
    [ -n "$line" ] || continue
    sha=${line%% *}
    subject=${line#* }
    # type(scope)!: summary
    head=${subject%%: *}
    summary=${subject#*: }
    breaking=""
    case "$head" in
        *!) breaking=1 head=${head%!} ;;
    esac
    case "$head" in
        *\(*\)) type=${head%%\(*} scope=${head#*\(} scope=${scope%\)} ;;
        *) type=$head scope="" ;;
    esac
    case "$type" in
        "" | *[!a-z]*) type="" scope="" breaking="" summary=$subject ;;
    esac
    if [ "$head" = "$subject" ]; then
        type="" scope="" breaking=""
    fi
    first=$(printf '%.1s' "$summary" | tr '[:lower:]' '[:upper:]')
    entry="- ${breaking:+**Breaking:** }${scope:+**$scope:** }$first${summary#?} ($sha)$nl"
    case "$type" in
        feat) features="$features$entry" ;;
        fix) fixes="$fixes$entry" ;;
        perf) performance="$performance$entry" ;;
        chore | ci | docs | style | test) minor="$minor$entry" ;;
        *) other="$other$entry" ;;
    esac
done << EOF
$log
EOF

if [ -z "$features$fixes$performance$other" ]; then
    other=$minor
fi

section() {
    [ -n "$2" ] || return 0
    printf '## %s\n\n%s\n' "$1" "$2"
}
[ -n "$prev" ] || printf 'The first release of Falog.\n\n'
section Features "$features"
section Fixes "$fixes"
section Performance "$performance"
section Other "$other"
if [ -n "$prev" ]; then
    printf '**Full changelog:** https://github.com/%s/compare/%s...%s\n' "$repo" "$prev" "$tag"
else
    printf '**Full changelog:** https://github.com/%s/commits/%s\n' "$repo" "$tag"
fi
