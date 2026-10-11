# Domain

## Entities

### Task

| Field | Type | Rules |
|---|---|---|
| `id` | `TaskId` | Assigned by SQLite, shown as `#12` |
| `title` | text | Required, trimmed; short and actionable |
| `description` | text | Full context; may be long |
| `area` | `Option<Area>` | Deleting an area keeps its tasks with no area |
| `status` | `Status` | Default `todo` |
| `priority` | `Priority` | Default `medium` |
| `due` | `Option<NaiveDate>` | Local calendar date, no time |
| `requester` | text | Who asked |
| `note_count` | number | Derived |
| `created_at`, `updated_at` | local datetime | `updated_at` also bumps when a note is added |
| `completed_at` | `Option` | Set when moving into `done`, cleared when moving out |
| `archived_at` | `Option` | Set when a done task is archived (put away from the board); cleared by restoring it or moving it out of `done` |
| `recurrence` | `Option<Recurrence>` | How it repeats; see [Recurring tasks](#recurring-tasks). Migration 4 |

### Area

A part of the user's life a task belongs to: "Work", "Personal", "Health", "Home"...
`id`, `name` (unique, compared ignoring case **and** accents), `color` (`#rrggbb`). New areas take the
next color of `PALETTE` (the One Dark player colors). Migration 2 is intentionally empty.

### Note

Append-only activity log entry: `id`, `task_id`, `body`, `created_at`. Deleted with its task.

## Archive

Only done tasks can be archived. Archived tasks keep every field and their notes; they leave the Board,
List, Focus and Calendar views and `list_tasks`, and live in the Archive view until restored. Moving an
archived task out of `done` restores it too, since an open task belongs on the board.

## Recurring tasks

A `Recurrence` is `Daily`, `Weekly(days)` (one or more weekdays; Monday to Friday reads "every
weekday") or `Monthly(day)` (1 to 31; shorter months use their last day). It is stored as text:
`daily`, `weekly:mon,fri`, `monthly:5`.

- **Completing** a recurring task (moving it into `done` from any other status) creates the next
  occurrence in the same transaction: same title, description, area, priority, requester and rule,
  status `todo`, due on the rule's first date after the later of the old due date and today. Done late,
  the new task is not born overdue; done early, it does not repeat the same day.
- The rule **moves** to the new task: the completed one no longer repeats, so reopening and completing
  it again creates nothing. Editing an open or done task never creates an occurrence.
- A recurring task **without a due date** is due on the rule's first date from today.
- `Recurrence::parse(text, anchor)` reads English and Portuguese: `daily`, `todo dia`, `every weekday`,
  `dias úteis`, `every friday`, `toda segunda e quarta`, `monthly on the 5th`, `todo dia 10`, and the
  stored form. `weekly` and `monthly` without a day use the anchor's weekday or day (the due date,
  else today). Intervals (`every 2 weeks`) and end dates are not supported and do not parse.

## Status

| Key | Label | Meaning |
|---|---|---|
| `todo` | To do | Not started |
| `in_progress` | In progress | Being worked on |
| `waiting` | Waiting | Blocked on someone else: review, an answer, a deploy |
| `done` | Done | Finished |

Parsing accepts the key plus English and Portuguese synonyms (`doing`, `em review`, `feito`...),
because input is often dictated.

## Priority

`low` < `medium` < `high` < `urgent`, stored as ranks 1–4. Parsing accepts synonyms and ranks.

## Due dates

`date::parse_due(text, today)` understands:

- ISO `2026-10-09`, day-first `09/10/2026`, `09/10/26`, `09/10` (a day-month more than 60 days in the
  past rolls over to next year);
- `today`, `tomorrow`, `day after tomorrow`, `next week` (next Monday), `end of week` (Friday),
  `end of month`, plus `hoje`, `amanhã`, `semana que vem`, `fim do mês`...;
- weekday names in both languages (`friday`, `sexta`): the next occurrence, today included;
  `next friday` skips today.

Assistants should still send ISO dates; the parser is a safety net and powers the UI's due field.

## Focus buckets

Each **open** task belongs to exactly one bucket, checked in this order:

1. **Overdue**: due before today
2. **Due today**
3. **Due this week**: due by Sunday of the current (Monday-based) week
4. **In progress**: status `in_progress`, no due date this week
5. **Waiting on others**: status `waiting`, no due date this week
6. **Upcoming**: due after this week
7. **Backlog**: everything else

Within a bucket: priority (highest first), then due date, then id. Overdue sorts by due date first.
"Completed in the last 7 days" is reported separately.

## Matching

`text::fold` lowercases and strips diacritics. Area lookup (`Store::find_area`) accepts an exact folded
name or a fragment matching exactly one area; otherwise it fails listing the candidates.
Task search (`Task::matches`) checks `#id`, title, description, requester and area name.
