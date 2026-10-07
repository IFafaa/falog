#!/usr/bin/env bash
# Builds Falog in release mode and installs it for the current user, on macOS and Linux:
#   - macOS: ~/Applications/Falog.app (falog and falog-mcp in Contents/MacOS), so it shows in
#     Launchpad, Spotlight and the Dock; launch at sign-in through a LaunchAgent
#   - Linux: falog and falog-mcp in ~/.local/bin, a desktop entry and icon, launch at sign-in through
#     ~/.config/autostart
#   - the MCP server registered in Claude Code (user scope), if the `claude` CLI is available
#   - the assistant workspace copied to --assistant-dir (default ~/falog-assistant)
#   - Falog started
#
#   ./scripts/install.sh [--assistant-dir DIR] [--no-gpu] [--no-voice] [--no-start]
#
# Written for the bash 3.2 that macOS ships.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
assistant_dir="$HOME/falog-assistant"
gpu=1
voice=1
start=1
while [ $# -gt 0 ]; do
    case "$1" in
        --assistant-dir) assistant_dir="$2"; shift 2 ;;
        --no-gpu) gpu=0; shift ;;
        --no-voice) voice=0; shift ;;
        --no-start) start=0; shift ;;
        -h | --help) sed -n '2,13p' "$0"; exit 0 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
done

case "$(uname -s)" in
    Darwin) os=macos ;;
    Linux) os=linux ;;
    *) echo "This script installs Falog on macOS and Linux; on Windows use scripts/install.ps1." >&2; exit 1 ;;
esac

step() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! command -v cargo > /dev/null; then
    echo "cargo was not found. Install Rust from https://rustup.rs and run this again." >&2
    exit 1
fi

# Voice input compiles whisper.cpp, which needs CMake and libclang (for bindgen).
have_libclang() {
    [ -n "${LIBCLANG_PATH:-}" ] && return 0
    if [ "$os" = macos ]; then
        xcode-select -p > /dev/null 2>&1
    else
        command -v llvm-config > /dev/null 2>&1 && return 0
        ls /usr/lib/llvm-*/lib/libclang*.so* /usr/lib64/libclang*.so* /usr/lib/libclang*.so* \
            /usr/lib/*-linux-gnu/libclang*.so* > /dev/null 2>&1
    fi
}

features=()
if [ "$voice" = 0 ]; then
    features=(--no-default-features)
elif ! command -v cmake > /dev/null || ! have_libclang; then
    if [ "$os" = macos ]; then
        warn "CMake or the Xcode command line tools were not found: building without voice input (brew install cmake; xcode-select --install)."
    else
        warn "CMake or libclang were not found: building without voice input (e.g. sudo apt install cmake libclang-dev)."
    fi
    features=(--no-default-features)
elif [ "$gpu" = 1 ]; then
    if [ "$os" = macos ]; then
        step "Speech recognition will run on the GPU (Metal)"
        features=(--features falog-desktop/gpu)
    elif command -v glslc > /dev/null && pkg-config --exists vulkan 2> /dev/null; then
        step "Vulkan headers and glslc found: speech recognition can run on the GPU"
        features=(--features falog-desktop/gpu)
    fi
fi

step "Building (release)"
cargo build --release --workspace ${features[@]+"${features[@]}"} --manifest-path "$root/Cargo.toml"
target_dir="$(cd "$root" && cargo metadata --format-version 1 --no-deps 2> /dev/null |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')"
release="${target_dir:-$root/target}/release"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)"

# Replaces `dest` through a rename, which is safe while the old binary is running.
install_file() {
    local src="$1" dest="$2" mode="$3"
    mkdir -p "$(dirname "$dest")"
    cp "$src" "$dest.new"
    chmod "$mode" "$dest.new"
    mv -f "$dest.new" "$dest"
}

# The same artwork the window uses, when the repository has it as a file.
icon_svg="$root/crates/falog-desktop/assets/icon.svg"
icon_png="$root/crates/falog-desktop/assets/icon-1024.png"

if [ "$os" = macos ]; then
    app="$HOME/Applications/Falog.app"
    exe="$app/Contents/MacOS/falog"
    mcp="$app/Contents/MacOS/falog-mcp"

    step "Installing to $app"
    pkill -x falog 2> /dev/null || true
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    install_file "$release/falog" "$exe" 755
    install_file "$release/falog-mcp" "$mcp" 755

    icon_key=""
    if [ -f "$icon_png" ] && command -v iconutil > /dev/null; then
        iconset="$(mktemp -d)/Falog.iconset"
        mkdir -p "$iconset"
        for size in 16 32 128 256 512; do
            sips -z "$size" "$size" "$icon_png" --out "$iconset/icon_${size}x${size}.png" > /dev/null
            double=$((size * 2))
            sips -z "$double" "$double" "$icon_png" --out "$iconset/icon_${size}x${size}@2x.png" > /dev/null
        done
        iconutil -c icns "$iconset" -o "$app/Contents/Resources/Falog.icns"
        icon_key="    <key>CFBundleIconFile</key>
    <string>Falog</string>"
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
    # Ad hoc signature for the whole bundle: Apple silicon only runs signed code, and stripping or
    # adding Info.plist after linking invalidates the linker's signature.
    codesign --force --deep --sign - "$app" > /dev/null 2>&1 || warn "codesign failed; macOS may refuse to open Falog."
    # Make Launchpad and Spotlight notice the new bundle right away.
    lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
    [ -x "$lsregister" ] && "$lsregister" -f "$app" || true

    step "Launch at sign-in"
    agent="$HOME/Library/LaunchAgents/app.falog.Falog.plist"
    mkdir -p "$(dirname "$agent")"
    cat > "$agent" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>app.falog.Falog</string>
    <key>ProgramArguments</key>
    <array>
        <string>$exe</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
EOF
else
    bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
    data="${XDG_DATA_HOME:-$HOME/.local/share}"
    config="${XDG_CONFIG_HOME:-$HOME/.config}"
    exe="$bin/falog"
    mcp="$bin/falog-mcp"

    step "Installing to $bin"
    pkill -x falog 2> /dev/null || true
    install_file "$release/falog" "$exe" 755
    install_file "$release/falog-mcp" "$mcp" 755

    if [ -f "$icon_svg" ]; then
        install_file "$icon_svg" "$data/icons/hicolor/scalable/apps/falog.svg" 644
    elif [ -f "$icon_png" ]; then
        install_file "$icon_png" "$data/icons/hicolor/512x512/apps/falog.png" 644
    fi
    command -v gtk-update-icon-cache > /dev/null &&
        gtk-update-icon-cache -q -t "$data/icons/hicolor" 2> /dev/null || true

    # Exec quoting as in the desktop entry spec (the path rarely needs it, but $HOME may have spaces).
    exec_line="\"$(printf '%s' "$exe" | sed 's/\\/\\\\\\\\/g; s/["`$]/\\\\&/g; s/%/%%/g')\""

    step "Desktop entry"
    mkdir -p "$data/applications"
    cat > "$data/applications/falog.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Falog
GenericName=Task board
Comment=Task board you talk to
Exec=$exec_line
Icon=falog
Terminal=false
Categories=Office;ProjectManagement;
StartupWMClass=falog
EOF
    command -v update-desktop-database > /dev/null &&
        update-desktop-database -q "$data/applications" 2> /dev/null || true

    step "Launch at sign-in"
    mkdir -p "$config/autostart"
    cat > "$config/autostart/falog.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Falog
Comment=Task board you talk to
Exec=$exec_line
Icon=falog
Terminal=false
X-GNOME-Autostart-enabled=true
EOF

    case ":$PATH:" in
        *":$bin:"*) ;;
        *) warn "$bin is not on your PATH; the app menu entry works, but add it to run falog from a terminal." ;;
    esac
fi

step "Registering the MCP server with Claude Code"
claude="$(command -v claude || true)"
[ -z "$claude" ] && [ -x "$HOME/.local/bin/claude" ] && claude="$HOME/.local/bin/claude"
if [ -n "$claude" ]; then
    "$claude" mcp remove --scope user falog > /dev/null 2>&1 || true
    "$claude" mcp add --scope user falog -- "$mcp" || warn "claude mcp add failed."
else
    warn "The 'claude' CLI was not found. Register the server yourself:"
    printf '  claude mcp add --scope user falog -- "%s"\n' "$mcp"
fi

step "Assistant workspace at $assistant_dir"
mkdir -p "$assistant_dir/.claude"
cp "$root/assistant/CLAUDE.md" "$assistant_dir/"
cp "$root/assistant/.claude/settings.json" "$assistant_dir/.claude/"

if [ "$start" = 1 ]; then
    step "Starting Falog"
    if [ "$os" = macos ]; then
        open "$app"
    else
        nohup "$exe" > /dev/null 2>&1 &
        disown || true
    fi
fi
echo "Done. Open the assistant in Falog, or a Claude Code session in $assistant_dir, and tell it about your tasks."
