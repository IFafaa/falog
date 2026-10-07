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

### Area

A part of the user's life a task belongs to: "Work", "Personal", "Health", "Home"...
`id`, `name` (unique, compared ignoring case **and** accents), `color` (`#rrggbb`). New areas take the
next color of `PALETTE` (the One Dark player colors). Migration 2 is intentionally empty.

### Note

Append-only activity log entry: `id`, `task_id`, `body`, `created_at`. Deleted with its task.

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

## Agenda buckets

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
