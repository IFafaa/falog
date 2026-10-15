#!/bin/sh
# Smoke test of the install scripts, run by CI on Ubuntu and macOS:
#   - scripts/install.sh with a stand-in build, then scripts/uninstall.sh without and with
#     --remove-data (on Linux once with XDG folders set and once with the default folders)
#   - on Linux, scripts/install-release.sh from an archive made by scripts/package-linux.sh: the
#     latest, --version and --preview downloads, a checksum that does not match, --uninstall and
#     --remove-data
# Everything happens in temporary home folders. cargo, claude, curl, pkill and codesign are
# stand-ins first on PATH, so nothing is built, downloaded, registered or stopped for real (on
# macOS, install.sh still registers the temporary Falog.app with Launch Services).
#
#   sh scripts/test-install.sh
#
# On other systems, such as Git Bash on Windows, `uname` answers Linux and the Linux layout is
# checked.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
icons="$root/crates/falog-desktop/assets/icon"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n 1)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# The scripts read these; the tests set them on purpose or not at all.
unset XDG_BIN_HOME XDG_DATA_HOME XDG_CONFIG_HOME

failures=0
# expect DESCRIPTION COMMAND...: runs one check and reports it.
expect() {
    description=$1
    shift
    if "$@"; then
        printf 'ok    %s\n' "$description"
    else
        printf 'FAIL  %s\n' "$description" >&2
        failures=$((failures + 1))
    fi
}
missing() { [ ! -e "$1" ]; }
same() { cmp -s "$1" "$2"; }
has() { grep -qF -- "$2" "$1"; }

case "$(uname -s)" in
    Darwin) os=macos ;;
    Linux) os=linux ;;
    *) os=other ;;
esac

# ---- stand-ins ------------------------------------------------------------------------------------

stubs="$work/stubs"
build="$work/target/release"
release="$work/release"
log="$work/calls.log"
mkdir -p "$stubs" "$build" "$release"
for name in falog falog-mcp; do
    printf '#!/bin/sh\necho "stand-in %s"\n' "$name" > "$build/$name"
    chmod 755 "$build/$name"
done
# Each stand-in writes how it was called to the log.
for name in claude pkill codesign; do
    cat > "$stubs/$name" << EOF
#!/bin/sh
echo "$name \$*" >> "$log"
EOF
done
cat > "$stubs/cargo" << EOF
#!/bin/sh
echo "cargo \$*" >> "$log"
if [ "\$1" = metadata ]; then
    printf '{"target_directory":"%s"}\\n' "$work/target"
fi
EOF
# Serves the files in $release by the last part of the address; the API answers releases.json.
cat > "$stubs/curl" << EOF
#!/bin/sh
out=
url=
while [ \$# -gt 0 ]; do
    case "\$1" in
        -o) out=\$2; shift 2 ;;
        *) url=\$1; shift ;;
    esac
done
echo "curl \$url" >> "$log"
case "\$url" in
    https://api.github.com/*) file="$release/releases.json" ;;
    *) file="$release/\${url##*/}" ;;
esac
[ -f "\$file" ] || exit 22
cp "\$file" "\$out"
EOF
if [ "$os" = other ]; then
    cat > "$stubs/uname" << 'EOF'
#!/bin/sh
case "${1:-}" in
    -m) echo x86_64 ;;
    *) echo Linux ;;
esac
EOF
    os=linux
fi
chmod 755 "$stubs"/*

# run HOME SCRIPT [ARGS...]: runs scripts/SCRIPT with the stand-ins and HOME as the home folder;
# what it printed is in $work/output.
run() {
    run_home=$1
    script="$root/scripts/$2"
    shift 2
    shell="sh"
    if head -n 1 "$script" | grep -q bash; then shell=bash; fi
    mkdir -p "$run_home"
    HOME=$run_home PATH="$stubs:$PATH" "$shell" "$script" "$@" > "$work/output" 2>&1
}
# succeeds HOME SCRIPT [ARGS...]: run, showing what the script printed when it fails.
succeeds() {
    run "$@" || {
        status=$?
        cat "$work/output" >&2
        return "$status"
    }
}
# fails_with STATUS HOME SCRIPT [ARGS...]: run, expecting that exit status.
fails_with() {
    expected=$1
    shift
    status=0
    run "$@" || status=$?
    [ "$status" = "$expected" ]
}

# Tasks in the data folder, the folder older versions left on Linux, and another app's data.
add_data() {
    mkdir -p "$1/Falog" "$1/another-app"
    echo tasks > "$1/Falog/falog.db"
    echo other > "$1/another-app/data"
    # The older folder's name differs only in case: a file system that ignores case has one folder.
    if [ ! -e "$1/falog" ]; then
        mkdir -p "$1/falog"
        echo window > "$1/falog/app.ron"
    fi
}

# What install.sh and install-release.sh leave on Linux: check_linux_install BIN DATA CONFIG ASSISTANT
check_linux_install() {
    bin=$1 data=$2 config=$3 assistant=$4
    expect "falog is installed" same "$build/falog" "$bin/falog"
    expect "falog-mcp is installed" same "$build/falog-mcp" "$bin/falog-mcp"
    expect "falog can be run" test -x "$bin/falog"
    expect "the app menu entry starts it" has "$data/applications/falog.desktop" "Exec=\"$bin/falog\""
    expect "it starts at sign-in" has "$config/autostart/falog.desktop" "Exec=\"$bin/falog\""
    expect "the sign-in entry is enabled" has "$config/autostart/falog.desktop" "X-GNOME-Autostart-enabled=true"
    expect "the icon is installed" same "$icons/falog.svg" "$data/icons/hicolor/scalable/apps/falog.svg"
    expect "the 256 px icon is installed" same "$icons/png/falog-256.png" "$data/icons/hicolor/256x256/apps/falog.png"
    expect "the MCP server is registered" has "$log" "claude mcp add --scope user falog -- $bin/falog-mcp"
    expect "the assistant workspace is copied" same "$root/assistant/CLAUDE.md" "$assistant/CLAUDE.md"
    expect "the assistant settings are copied" same "$root/assistant/.claude/settings.json" "$assistant/.claude/settings.json"
}

# What uninstalling takes away on Linux: check_linux_removed BIN DATA CONFIG
check_linux_removed() {
    bin=$1 data=$2 config=$3
    expect "falog is removed" missing "$bin/falog"
    expect "falog-mcp is removed" missing "$bin/falog-mcp"
    expect "the app menu entry is removed" missing "$data/applications/falog.desktop"
    expect "the sign-in entry is removed" missing "$config/autostart/falog.desktop"
    expect "the icon is removed" missing "$data/icons/hicolor/scalable/apps/falog.svg"
    expect "the 256 px icon is removed" missing "$data/icons/hicolor/256x256/apps/falog.png"
    expect "the MCP server is unregistered" has "$log" "claude mcp remove --scope user falog"
}

expect "install.sh refuses an unknown option" fails_with 2 "$work/home" install.sh --bogus
expect "uninstall.sh refuses an unknown option" fails_with 2 "$work/home" uninstall.sh --bogus

if [ "$os" = linux ]; then
    echo "== install.sh and uninstall.sh with XDG folders (and a space in the home folder)"
    home="$work/xdg home"
    XDG_BIN_HOME="$home/xdg/bin"
    XDG_DATA_HOME="$home/xdg/data"
    XDG_CONFIG_HOME="$home/xdg/config"
    export XDG_BIN_HOME XDG_DATA_HOME XDG_CONFIG_HOME
    : > "$log"
    expect "install.sh succeeds" succeeds "$home" install.sh --no-voice --no-start
    expect "it builds without voice input" has "$log" "cargo build --release --workspace --no-default-features"
    check_linux_install "$XDG_BIN_HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$home/falog-assistant"
    add_data "$XDG_DATA_HOME"
    : > "$log"
    expect "uninstall.sh succeeds" succeeds "$home" uninstall.sh
    check_linux_removed "$XDG_BIN_HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME"
    expect "the tasks are kept" test -f "$XDG_DATA_HOME/Falog/falog.db"
    expect "it says where the tasks are" has "$work/output" "Your tasks are still in $XDG_DATA_HOME/Falog"
    unset XDG_BIN_HOME XDG_DATA_HOME XDG_CONFIG_HOME

    echo "== install.sh and uninstall.sh --remove-data with the default folders"
    home="$work/home"
    share="$home/.local/share"
    : > "$log"
    expect "install.sh succeeds" succeeds "$home" install.sh --no-voice --no-start --assistant-dir "$home/assistant"
    check_linux_install "$home/.local/bin" "$share" "$home/.config" "$home/assistant"
    add_data "$share"
    : > "$log"
    expect "uninstall.sh --remove-data succeeds" succeeds "$home" uninstall.sh --remove-data
    check_linux_removed "$home/.local/bin" "$share" "$home/.config"
    # The data folder is ~/.local/share/Falog; older versions also left ~/.local/share/falog.
    expect "the data folder is removed" missing "$share/Falog/falog.db"
    expect "the older data folder is removed" missing "$share/falog/app.ron"
    expect "other apps' data is kept" test -f "$share/another-app/data"

    echo "== install-release.sh from a release archive"
    expect "package-linux.sh makes the archive" \
        succeeds "$work/home" package-linux.sh "$build" "$release/falog-linux-x86_64.tar.gz"
    (cd "$release" && sha256sum falog-linux-x86_64.tar.gz > SHA256SUMS)
    printf '[{"tag_name": "v9.1.0-beta.1"}, {"tag_name": "v9.0.0"}]\n' > "$release/releases.json"
    home="$work/release home"
    share="$home/.local/share"
    url=https://github.com/IFafaa/falog/releases
    : > "$log"
    expect "install-release.sh succeeds" succeeds "$home" install-release.sh --no-start
    expect "it downloads the latest release" has "$log" "curl $url/latest/download/falog-linux-x86_64.tar.gz"
    expect "it downloads the checksums" has "$log" "curl $url/latest/download/SHA256SUMS"
    check_linux_install "$home/.local/bin" "$share" "$home/.config" "$home/falog-assistant"
    : > "$log"
    expect "install-release.sh --version succeeds" succeeds "$home" install-release.sh --no-start --version 9.0.0
    expect "it downloads that release" has "$log" "curl $url/download/v9.0.0/falog-linux-x86_64.tar.gz"
    : > "$log"
    expect "install-release.sh --preview succeeds" succeeds "$home" install-release.sh --no-start --preview
    expect "it downloads the newest release" has "$log" "curl $url/download/v9.1.0-beta.1/falog-linux-x86_64.tar.gz"
    add_data "$share"
    : > "$log"
    expect "install-release.sh --uninstall succeeds" succeeds "$home" install-release.sh --uninstall
    check_linux_removed "$home/.local/bin" "$share" "$home/.config"
    expect "the tasks are kept" test -f "$share/Falog/falog.db"
    expect "install-release.sh --uninstall --remove-data succeeds" \
        succeeds "$home" install-release.sh --uninstall --remove-data
    expect "the data folder is removed" missing "$share/Falog/falog.db"
    expect "the older data folder is removed" missing "$share/falog/app.ron"
    expect "other apps' data is kept" test -f "$share/another-app/data"

    echo "== install-release.sh refuses an archive that does not match SHA256SUMS"
    home="$work/tampered home"
    echo "0000000000000000000000000000000000000000000000000000000000000000  falog-linux-x86_64.tar.gz" \
        > "$release/SHA256SUMS"
    expect "install-release.sh fails" fails_with 1 "$home" install-release.sh --no-start
    expect "it says the archive does not match" has "$work/output" "does not match SHA256SUMS"
    expect "it installs nothing" missing "$home/.local/bin/falog"
else
    echo "== install.sh and uninstall.sh on macOS"
    home="$work/mac home"
    app="$home/Applications/Falog.app"
    agent="$home/Library/LaunchAgents/app.falog.Falog.plist"
    data="$home/Library/Application Support/Falog"
    : > "$log"
    expect "install.sh succeeds" succeeds "$home" install.sh --no-voice --no-start
    expect "falog is in the app" same "$build/falog" "$app/Contents/MacOS/falog"
    expect "falog-mcp is in the app" same "$build/falog-mcp" "$app/Contents/MacOS/falog-mcp"
    expect "the app has its version" has "$app/Contents/Info.plist" "<string>$version</string>"
    expect "the app may use the microphone" has "$app/Contents/Info.plist" "NSMicrophoneUsageDescription"
    expect "the app has its icon" same "$icons/falog.icns" "$app/Contents/Resources/falog.icns"
    expect "the app is signed" has "$log" "codesign --force --deep --sign - $app"
    expect "it starts at sign-in" has "$agent" "<string>$app/Contents/MacOS/falog</string>"
    expect "the MCP server is registered" has "$log" "claude mcp add --scope user falog -- $app/Contents/MacOS/falog-mcp"
    expect "the assistant workspace is copied" same "$root/assistant/CLAUDE.md" "$home/falog-assistant/CLAUDE.md"
    mkdir -p "$data"
    echo tasks > "$data/falog.db"
    : > "$log"
    expect "uninstall.sh succeeds" succeeds "$home" uninstall.sh
    expect "the app is removed" missing "$app"
    expect "the sign-in agent is removed" missing "$agent"
    expect "the MCP server is unregistered" has "$log" "claude mcp remove --scope user falog"
    expect "the tasks are kept" test -f "$data/falog.db"
    expect "install.sh succeeds again" succeeds "$home" install.sh --no-voice --no-start
    expect "uninstall.sh --remove-data succeeds" succeeds "$home" uninstall.sh --remove-data
    expect "the data folder is removed" missing "$data"
    expect "install-release.sh is for Linux only" fails_with 1 "$home" install-release.sh --no-start
fi

if [ "$failures" -gt 0 ]; then
    printf '\n%s install checks failed\n' "$failures" >&2
    exit 1
fi
printf '\nAll install checks passed.\n'
