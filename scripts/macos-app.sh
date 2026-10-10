#!/bin/sh
# Writes the Falog.app bundle on macOS: falog and falog-mcp in Contents/MacOS, the icon, an Info.plist,
# and an ad hoc signature. Used by scripts/install.sh and scripts/package-macos.sh.
#
#   scripts/macos-app.sh BIN_DIR APP VERSION
#
# BIN_DIR holds the release binaries (target/release); APP is the bundle to write, like
# ~/Applications/Falog.app. Files are replaced through a rename, which is safe while Falog runs.
set -eu

if [ $# -ne 3 ]; then
    echo "usage: $0 BIN_DIR APP VERSION" >&2
    exit 2
fi
bin_dir=$1
app=$2
version=$3
icons="$(cd "$(dirname "$0")/.." && pwd)/crates/falog-desktop/assets/icon"

install_file() {
    mkdir -p "$(dirname "$2")"
    cp "$1" "$2.new"
    chmod "$3" "$2.new"
    mv -f "$2.new" "$2"
}

mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
install_file "$bin_dir/falog" "$app/Contents/MacOS/falog" 755
install_file "$bin_dir/falog-mcp" "$app/Contents/MacOS/falog-mcp" 755

icon_key=""
if [ -f "$icons/falog.icns" ]; then
    install_file "$icons/falog.icns" "$app/Contents/Resources/falog.icns" 644
    icon_key="    <key>CFBundleIconFile</key>
    <string>falog</string>"
fi

# NSMicrophoneUsageDescription is required: without it macOS ends the app when it opens the
# microphone.
cat > "$app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Falog</string>
    <key>CFBundleDisplayName</key>
    <string>Falog</string>
    <key>CFBundleIdentifier</key>
    <string>app.falog.Falog</string>
    <key>CFBundleExecutable</key>
    <string>falog</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
$icon_key
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.productivity</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>Falog listens while you dictate to the assistant. Speech is transcribed on this Mac.</string>
</dict>
</plist>
EOF

# Ad hoc signature for the whole bundle: Apple silicon only runs signed code, and stripping or adding
# Info.plist after linking invalidates the linker's signature.
codesign --force --deep --sign - "$app"
