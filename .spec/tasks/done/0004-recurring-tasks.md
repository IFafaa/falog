# 0004: Recurring tasks

**Status:** done
**Area:** core, mcp, desktop

## Goal

A task can repeat (daily, weekly on given weekdays, monthly on a day). Completing it creates the next
occurrence with the next due date.

## Context

Weekly reports and monthly chores are currently re-dictated every time.

## Scope

- In: recurrence rule on a task, next occurrence on completion, MCP `recurrence` argument
  ("every friday", "monthly on the 5th"), UI field in the task panel, recurrence shown on cards.
- Out: calendar-style exceptions, end dates, intervals ("every 2 weeks").

## Acceptance criteria

- [x] Completing a recurring task creates the next one (same fields, new due date, status `todo`)
- [x] The rule is parsed from natural language in English and Portuguese
- [x] `create_task`/`update_task` accept `recurrence`; `get_task` shows it
- [x] Migration adds the column without touching existing data
- [x] [domain.md](../../domain.md) and [mcp.md](../../mcp.md) updated

## Plan

`Recurrence` in `falog-core::domain` with parsing, the stored form and the next-date rule; migration 4
adds `tasks.recurrence`; `Store::update_task_with_next` creates the occurrence in the same transaction
as the completion. The MCP server takes `recurrence` on `create_task`/`update_task`; the desktop task
panel gets a *Repeats* row and board cards show the rule.

## Outcome

- **Next due date:** the rule's first date after the later of the old due date and today, so a task
  completed late is not born overdue and one completed early does not repeat the same day.
- **The rule moves to the new task.** The completed task stops repeating, so reopening and completing
  it again cannot create a duplicate. Only the transition into `done` repeats; edits never do.
- A recurring task without a due date gets the first occurrence from today.
- Desktop: *Repeats* picker (never, every day, every weekday, every week, every month) with weekday
  toggles or the day of the month; cards show the rule with a repeat icon; the toast after completing
  says when the next one is due. The MCP reply includes a `Next occurrence:` line.
- The demo seed makes the monthly report and the electricity bill repeat.
