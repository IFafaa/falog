#!/bin/sh
# Installs Falog on Linux from a GitHub release, for the current user. Each release publishes this file
# as install.sh:
#
#   curl -fsSL https://github.com/IFafaa/falog/releases/latest/download/install.sh | sh
#
# It downloads falog-linux-x86_64.tar.gz from the same release, checks it against the release's
# SHA256SUMS and installs:
#   - falog and falog-mcp in ~/.local/bin
#   - an app menu entry and icon, and launch at sign-in (~/.config/autostart)
#   - the MCP server registered with Claude Code (user scope), if the `claude` CLI is available
#   - the assistant workspace in ~/falog-assistant
# and starts Falog. Running it again updates Falog. Options (piped, pass them after `sh -s --`):
#   --version X.Y.Z       install that release instead of this one
#   --assistant-dir DIR   assistant workspace folder (default ~/falog-assistant)
#   --archive FILE        install from a falog-linux-x86_64.tar.gz you already have (not checked)
#   --no-start            do not start Falog
#   --uninstall           remove Falog; add --remove-data to delete your tasks too
#
# To build from source instead, use scripts/install.sh in the repository.
set -eu

repo=IFafaa/falog
name=falog-linux-x86_64
# The release workflow writes the tag of the release this file belongs to; empty means the latest.
release_tag=""

step() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat << EOF
Installs Falog on Linux from a GitHub release.

  curl -fsSL https://github.com/$repo/releases/latest/download/install.sh | sh -s -- [options]

  --version X.Y.Z       install that release instead of this one
  --assistant-dir DIR   assistant workspace folder (default ~/falog-assistant)
  --archive FILE        install from a $name.tar.gz you already have (not checked)
  --no-start            do not start Falog
  --uninstall           remove Falog; add --remove-data to delete your tasks too
EOF
}

download() {
    if command -v curl > /dev/null 2>&1; then
        curl -fsSL --retry 3 -o "$2" "$1"
    elif command -v wget > /dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        die "curl or wget is needed to download Falog."
    fi
}

sha256() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum > /dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        die "sha256sum is needed to check the download."
    fi
}

# Replaces `dest` through a rename, which is safe while the old binary is running.
install_file() {
    mkdir -p "$(dirname "$2")"
    cp "$1" "$2.new"
    chmod "$3" "$2.new"
    mv -f "$2.new" "$2"
}

find_claude() {
    if command -v claude > /dev/null 2>&1; then
        command -v claude
    elif [ -x "$HOME/.local/bin/claude" ]; then
        echo "$HOME/.local/bin/claude"
    fi
}

uninstall() {
    remove_data=$1
    pkill -x falog 2> /dev/null || true
    rm -f "$config/autostart/falog.desktop" "$data/applications/falog.desktop"
    rm -f "$data"/icons/hicolor/*/apps/falog.svg "$data"/icons/hicolor/*/apps/falog.png
    rm -f "$bin/falog" "$bin/falog-mcp"
    claude=$(find_claude)
    if [ -n "$claude" ]; then
        "$claude" mcp remove --scope user falog > /dev/null 2>&1 || true
    fi
    if [ "$remove_data" = 1 ]; then
        rm -rf "$data/Falog" "$data/falog"
        echo "Falog and its data were removed."
    else
        echo "Falog was removed. Your tasks are still in $data/Falog."
    fi
}

main() {
    assistant_dir="$HOME/falog-assistant"
    archive=""
    start=1
    action=install
    remove_data=0
    while [ $# -gt 0 ]; do
        case "$1" in
            --version)
                [ $# -ge 2 ] || die "--version needs a version, like 0.2.0"
                release_tag="v${2#v}"
                shift 2
                ;;
            --assistant-dir)
                [ $# -ge 2 ] || die "--assistant-dir needs a folder"
                assistant_dir=$2
                shift 2
                ;;
            --archive)
                [ $# -ge 2 ] || die "--archive needs a file"
                archive=$2
                shift 2
                ;;
            --no-start) start=0; shift ;;
            --uninstall) action=uninstall; shift ;;
            --remove-data) remove_data=1; shift ;;
            -h | --help) usage; exit 0 ;;
            *) die "unknown option: $1" ;;
        esac
    done

    [ "$(uname -s)" = Linux ] ||
        die "this installs Falog on Linux; for Windows and macOS see https://github.com/$repo#install"

    bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
    data="${XDG_DATA_HOME:-$HOME/.local/share}"
    config="${XDG_CONFIG_HOME:-$HOME/.config}"

    if [ "$action" = uninstall ]; then
        uninstall "$remove_data"
        return
    fi

    case "$(uname -m)" in
        x86_64 | amd64) ;;
        *) die "the Linux download is built for x86_64 only; build from source on $(uname -m): https://github.com/$repo/blob/main/docs/install/linux.md" ;;
    esac

    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    if [ -z "$archive" ]; then
        if [ -n "$release_tag" ]; then
            base="https://github.com/$repo/releases/download/$release_tag"
            step "Downloading Falog ${release_tag#v}"
        else
            base="https://github.com/$repo/releases/latest/download"
            step "Downloading the latest Falog"
        fi
        archive="$tmp/$name.tar.gz"
        download "$base/$name.tar.gz" "$archive" || die "could not download $base/$name.tar.gz"
        download "$base/SHA256SUMS" "$tmp/SHA256SUMS" || die "could not download $base/SHA256SUMS"

        step "Checking the download"
        expected=$(awk -v f="$name.tar.gz" '$2 == f || $2 == "*" f { print $1 }' "$tmp/SHA256SUMS")
        [ -n "$expected" ] || die "SHA256SUMS has no line for $name.tar.gz"
        actual=$(sha256 "$archive")
        [ "$actual" = "$expected" ] ||
            die "$name.tar.gz does not match SHA256SUMS (expected $expected, got $actual); try again"
    fi

    tar -xzf "$archive" -C "$tmp"
    src="$tmp/$name"
    if [ ! -f "$src/falog" ] || [ ! -f "$src/falog-mcp" ]; then
        die "$archive is not a Falog Linux download"
    fi

    exe="$bin/falog"
    mcp="$bin/falog-mcp"
    step "Installing to $bin"
    pkill -x falog 2> /dev/null || true
    install_file "$src/falog" "$exe" 755
    install_file "$src/falog-mcp" "$mcp" 755

    (cd "$src/icons" && find . -type f) | while IFS= read -r icon; do
        install_file "$src/icons/$icon" "$data/icons/$icon" 644
    done
    if command -v gtk-update-icon-cache > /dev/null 2>&1; then
        gtk-update-icon-cache -q -t "$data/icons/hicolor" 2> /dev/null || true
    fi

    # Exec quoting as in the desktop entry spec (the path rarely needs it, but $HOME may have spaces).
    # shellcheck disable=SC2016 # the backtick and $ are characters to escape, not expansions
    exec_line="Exec=\"$(printf '%s' "$exe" | sed 's/\\/\\\\\\\\/g; s/["`$]/\\\\&/g; s/%/%%/g')\""

    step "App menu entry and launch at sign-in"
    mkdir -p "$data/applications" "$config/autostart"
    {
        grep -v '^Exec=' "$src/falog.desktop"
        echo "$exec_line"
    } > "$data/applications/falog.desktop"
    {
        grep -v '^Exec=' "$src/falog.desktop"
        echo "$exec_line"
        echo "X-GNOME-Autostart-enabled=true"
    } > "$config/autostart/falog.desktop"
    if command -v update-desktop-database > /dev/null 2>&1; then
        update-desktop-database -q "$data/applications" 2> /dev/null || true
    fi

    # The download is built on an older Ubuntu so it runs on most distributions; say what is missing
    # (usually the ALSA library) instead of failing silently from the app menu.
    missing=$(ldd "$exe" 2>&1 | grep 'not found' || true)
    if [ -n "$missing" ]; then
        warn "Falog cannot start until these are installed with your package manager:"
        printf '%s\n' "$missing" >&2
    fi

    step "Registering the MCP server with Claude Code"
    claude=$(find_claude)
    if [ -n "$claude" ]; then
        "$claude" mcp remove --scope user falog > /dev/null 2>&1 || true
        "$claude" mcp add --scope user falog -- "$mcp" || warn "claude mcp add failed."
    else
        warn "The 'claude' CLI was not found. Once it is installed, register the server with:"
        printf '  claude mcp add --scope user falog -- "%s"\n' "$mcp"
    fi

    step "Assistant workspace at $assistant_dir"
    mkdir -p "$assistant_dir/.claude"
    cp "$src/assistant/CLAUDE.md" "$assistant_dir/"
    cp "$src/assistant/.claude/settings.json" "$assistant_dir/.claude/"

    case ":$PATH:" in
        *":$bin:"*) ;;
        *) warn "$bin is not on your PATH; the app menu entry works, but add it to run falog from a terminal." ;;
    esac

    if [ "$start" = 1 ] && [ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]; then
        step "Starting Falog"
        nohup "$exe" > /dev/null 2>&1 &
    fi
    echo "Done. Falog is in your app menu. Uninstall with:"
    echo "  curl -fsSL https://github.com/$repo/releases/latest/download/install.sh | sh -s -- --uninstall"
}

# Everything runs from main, so a download cut short cannot run half a script.
main "$@"
