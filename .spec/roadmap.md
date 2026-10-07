# Roadmap

Ideas in rough priority order. Each one gets a spec in [tasks/backlog](tasks/backlog) before work
starts; move it to `tasks/done` when shipped.

| # | Task | Why |
|---|---|---|
| 0001 | [System tray](tasks/backlog/0001-system-tray.md) | Keep Falog running without a window in the taskbar |
| 0002 | [Due date reminders](tasks/backlog/0002-due-date-reminders.md) | Get a Windows notification before things are late |
| 0003 | [App icon](tasks/done/0003-executable-icon.md) | A distinctive icon in Explorer, the taskbar and the Start menu |
| 0004 | [Recurring tasks](tasks/backlog/0004-recurring-tasks.md) | Weekly reports, monthly invoices, standing chores |
| 0005 | [Quick capture hotkey](tasks/backlog/0005-quick-capture-hotkey.md) | File a task from anywhere without the assistant |
| 0006 | [Assistant panel with voice](tasks/done/0006-assistant-panel.md) | Talk to Falog itself instead of a separate Claude window |
| 0007 | [Assistant threads](tasks/done/0007-assistant-threads.md) | Several conversations with their own context |
| 0008 | [Live transcription](tasks/done/0008-live-transcription.md) | See the words while dictating |
| 0009 | [Areas for every part of life](tasks/done/0009-areas.md) | Personal tasks and appointments next to work |
| 0010 | [Calendar view with Google Calendar](tasks/doing/0010-google-calendar.md) | Meetings from several Google accounts next to the tasks |
| 0011 | [Assistant agents](tasks/done/0011-assistant-agents.md) | Claude Code, Gemini, Codex or any ACP agent; model, effort, mode, context and slash commands per thread |
| 0012 | [macOS and Linux](tasks/done/0012-macos-and-linux.md) | The same board on the Mac and the Linux machine |
| 0013 | [Data folder outside AppData](tasks/done/0013-data-outside-appdata.md) | Claude desktop's sandbox must not hide tasks from Falog |
| 0014 | [Google Calendar through secret iCal links](tasks/done/0014-ical-links.md) | Connect a calendar by pasting a link instead of creating an OAuth client |
| 0015 | [Calendars by area, events by voice](tasks/doing/0015-calendar-areas-and-events.md) | Which meetings belong to which job, and booking them through the assistant |
| 0016 | [CI quality gate](tasks/doing/0016-ci-quality-gate.md) | One required check that says the code is correct before it lands on `main` |

Later, unordered: tags/labels, manual ordering within a column, weekly report export (Markdown),
undo for destructive actions, signed macOS builds and Linux packages (Flatpak, `.deb`).
