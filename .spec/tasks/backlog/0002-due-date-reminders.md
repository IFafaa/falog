# 0002: Due date reminders

**Status:** backlog
**Area:** desktop

## Goal

Falog shows a Windows toast in the morning listing tasks due today and overdue, and another one the
day before a high or urgent task is due.

## Context

Forgotten deadlines are the core pain (see [product.md](../../product.md)). The agenda rules in
[domain.md](../../domain.md#agenda-buckets) already compute what is due.

## Scope

- In: one morning summary (time configurable in Settings), day-before reminders, a setting to turn
  reminders off, not repeating a reminder already shown.
- Out: per-task custom reminder times.

## Acceptance criteria

- [ ] At the configured time, a single toast summarizes overdue and due-today counts; clicking opens the Agenda
- [ ] High/urgent tasks due tomorrow get a reminder once
- [ ] Reminders survive restarts without duplicating (store what was sent)
- [ ] Setting to disable reminders
- [ ] Specs updated

## Plan

Needs a "last reminded" record (new migration). Toasts via the `winrt-notification` or `notify-rust`
crate. Depends on 0001 if reminders must fire while the window is closed.
