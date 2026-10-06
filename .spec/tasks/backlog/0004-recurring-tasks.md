# 0004: Recurring tasks

**Status:** backlog
**Area:** core, mcp, desktop

## Goal

A task can repeat (daily, weekly on given weekdays, monthly on a day). Completing it creates the next
occurrence with the next due date.

## Context

Weekly reports and monthly chores are currently re-dictated every time.

## Scope

- In: recurrence rule on a task, next occurrence on completion, MCP `recurrence` argument
  ("every friday", "monthly on the 5th"), UI field in the task panel, recurrence shown on cards.
- Out: calendar-style exceptions, end dates.

## Acceptance criteria

- [ ] Completing a recurring task creates the next one (same fields, new due date, status `todo`)
- [ ] The rule is parsed from natural language in English and Portuguese
- [ ] `create_task`/`update_task` accept `recurrence`; `get_task` shows it
- [ ] Migration adds the column without touching existing data
- [ ] [domain.md](../../domain.md) and [mcp.md](../../mcp.md) updated
