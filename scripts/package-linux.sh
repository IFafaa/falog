#!/bin/sh
# Packages release binaries into the Linux download, falog-linux-x86_64.tar.gz, which
# scripts/install-release.sh installs. The release workflow (.github/workflows/release.yml) runs it.
#
#   scripts/package-linux.sh BIN_DIR OUT.tar.gz
#
# The archive holds one folder, falog-linux-x86_64/, with:
#   falog, falog-mcp          the app and the MCP server
#   falog.desktop             app menu entry (Exec=falog; the installer writes the full path)
#   icons/hicolor/...         the icon, laid out as in ~/.local/share/icons/hicolor
#   assistant/                the assistant workspace (CLAUDE.md, .claude/settings.json)
#   LICENSE, THIRD-PARTY-NOTICES.md
set -eu

if [ $# -ne 2 ]; then
    echo "usage: $0 BIN_DIR OUT.tar.gz" >&2
    exit 2
fi
bin_dir=$1
out=$2
root="$(cd "$(dirname "$0")/.." && pwd)"
icons="$root/crates/falog-desktop/assets/icon"
name=falog-linux-x86_64

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
dir="$staging/$name"
mkdir -p "$dir/icons/hicolor/scalable/apps" "$dir/assistant/.claude"

cp "$bin_dir/falog" "$bin_dir/falog-mcp" "$dir/"
chmod 755 "$dir/falog" "$dir/falog-mcp"

cp "$icons/falog.svg" "$dir/icons/hicolor/scalable/apps/falog.svg"
for png in "$icons"/png/falog-*.png; do
    size="${png##*/falog-}"
    size="${size%.png}"
    mkdir -p "$dir/icons/hicolor/${size}x${size}/apps"
    cp "$png" "$dir/icons/hicolor/${size}x${size}/apps/falog.png"
done

cat > "$dir/falog.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Falog
GenericName=Task board
Comment=Task board you talk to
Exec=falog
Icon=falog
Terminal=false
Categories=Office;ProjectManagement;
StartupWMClass=falog
EOF

cp "$root/assistant/CLAUDE.md" "$dir/assistant/"
cp "$root/assistant/.claude/settings.json" "$dir/assistant/.claude/"
cp "$root/LICENSE" "$root/crates/falog-desktop/assets/THIRD-PARTY-NOTICES.md" "$dir/"

out_dir="$(cd "$(dirname "$out")" && pwd)"
tar -czf "$out_dir/$(basename "$out")" -C "$staging" "$name"
echo "Done: $out"
