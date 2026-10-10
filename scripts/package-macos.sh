#!/bin/sh
# Packages release binaries into a disk image for download: Falog.app (scripts/macos-app.sh) next to a
# shortcut to /Applications, so installing is dragging one onto the other. The release workflow
# (.github/workflows/release.yml) runs it; the app is signed ad hoc, not notarized.
#
#   scripts/package-macos.sh BIN_DIR OUT.dmg
set -eu

if [ $# -ne 2 ]; then
    echo "usage: $0 BIN_DIR OUT.dmg" >&2
    exit 2
fi
bin_dir=$1
out=$2
root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)"

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
sh "$root/scripts/macos-app.sh" "$bin_dir" "$staging/Falog.app" "$version"
codesign --verify --deep --strict "$staging/Falog.app"
ln -s /Applications "$staging/Applications"

rm -f "$out"
# hdiutil now and then fails with "Resource busy" on build machines; a retry gets through.
attempt=1
until hdiutil create -volname Falog -srcfolder "$staging" -fs HFS+ -format UDZO -ov "$out"; do
    if [ "$attempt" -ge 5 ]; then
        echo "hdiutil create failed $attempt times" >&2
        exit 1
    fi
    attempt=$((attempt + 1))
    sleep 5
done
hdiutil verify "$out"
echo "Done: $out (Falog $version)"
