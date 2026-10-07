#!/usr/bin/env bash
# Removes what install.sh set up on macOS or Linux. Your tasks database is kept unless --remove-data.
#
#   ./scripts/uninstall.sh [--remove-data]
set -euo pipefail

remove_data=0
case "${1:-}" in
    --remove-data) remove_data=1 ;;
    "") ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
esac

pkill -x falog 2> /dev/null || true

case "$(uname -s)" in
    Darwin)
        rm -f "$HOME/Library/LaunchAgents/app.falog.Falog.plist"
        rm -rf "$HOME/Applications/Falog.app"
        data="$HOME/Library/Application Support/Falog"
        ;;
    Linux)
        bin="${XDG_BIN_HOME:-$HOME/.local/bin}"
        share="${XDG_DATA_HOME:-$HOME/.local/share}"
        config="${XDG_CONFIG_HOME:-$HOME/.config}"
        rm -f "$config/autostart/falog.desktop" "$share/applications/falog.desktop"
        rm -f "$share/icons/hicolor/scalable/apps/falog.svg" "$share/icons/hicolor/512x512/apps/falog.png"
        rm -f "$bin/falog" "$bin/falog-mcp"
        data="$share/Falog"
        ;;
    *)
        echo "This script uninstalls Falog on macOS and Linux; on Windows use scripts/uninstall.ps1." >&2
        exit 1
        ;;
esac

claude="$(command -v claude || true)"
[ -z "$claude" ] && [ -x "$HOME/.local/bin/claude" ] && claude="$HOME/.local/bin/claude"
if [ -n "$claude" ]; then
    "$claude" mcp remove --scope user falog || true
fi

if [ "$remove_data" = 1 ]; then
    rm -rf "$data"
    # eframe keeps window and UI preferences in a folder of its own on Linux.
    [ "$(uname -s)" = Linux ] && rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/falog"
    echo "Falog and its data were removed."
else
    echo "Falog was removed. Your tasks are still in $data."
fi
