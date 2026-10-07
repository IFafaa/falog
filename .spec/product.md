# Product

## Problem

Things to do arrive from many channels (calls, messages, meetings, a thought on the way home), for work
and personal life alike: tasks, errands, appointments. Writing each one down is friction, so some are
lost, and the week starts with a long effort to remember what is pending and by when.

## Users

One person organizing their own life. Falog is single-user and local. Their areas are few (typically
Work, Personal, Health, Home, Studies) and their open tasks number in the tens, rarely hundreds.

## Goals

1. **Capture with zero friction.** Describing a request to an assistant, by voice or text, must be
   enough to file a complete task (title, area, context, requester, due date, priority).
2. **Recover context fast.** The Focus view and the `get_agenda` tool answer "what needs my attention
   now?" in seconds.
3. **See everything in one place**, separated by area (work and personal) but never siloed.
4. **Stay out of the way.** Starts with the OS, one window, instant, offline, no account.

## Non-goals (for now)

- Multi-user collaboration, sharing or sync between machines.
- Integrations that pull tasks from Jira, Linear, GitHub and similar (may come later via the assistant).
- Time tracking and billing.
- Mobile apps.

## Vocabulary

| Term | Meaning |
|---|---|
| Task | A unit of work requested by someone. |
| Area | A part of the user's life: "Work", "Personal", "Health", "Home"... Every task may belong to one. |
| Requester | The person who asked for the task. |
| Note | Timestamped entry in a task's activity log. |
| Agenda | Open tasks grouped by urgency, shown in the Focus view (see [domain.md](domain.md#agenda-buckets)). |
| Assistant | Claude acting on tasks: the in-app assistant dock, or any MCP client using `falog-mcp`. |
