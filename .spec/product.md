# Product

## Problem

A developer working for several companies receives requests from many channels (calls, chat, meetings).
Writing each one down is friction, so some are lost; people end up chasing them, and every Monday starts
with a long effort to remember what is pending, for whom, and by when.

## Users

One person: the developer. Falog is single-user and local. Their companies are few (typically 2–5) and
their open tasks number in the tens, rarely hundreds.

## Goals

1. **Capture with zero friction.** Describing a request to an assistant, by voice or text, must be
   enough to file a complete task (title, company, context, requester, due date, priority).
2. **Recover context fast.** The Agenda view and the `get_agenda` tool answer "what needs my attention
   now?" in seconds.
3. **See everything in one place**, separated by company but never siloed.
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
| Company | Who the work is for; every task may belong to one. |
| Requester | The person who asked for the task. |
| Note | Timestamped entry in a task's activity log. |
| Agenda | Open tasks grouped by urgency (see [domain.md](domain.md#agenda-buckets)). |
| Assistant | Any MCP client (Claude Code, Claude Desktop...) using `falog-mcp`. |
