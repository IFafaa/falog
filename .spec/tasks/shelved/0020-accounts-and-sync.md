# 0020: Accounts and sync across devices

**Status:** shelved (not pursued; no code was written)
**Area:** core, desktop

## Goal

Sign in with an email and see the same tasks, areas and notes on every device: the work laptop, the
home desktop, later a phone.

## Context

Falog keeps everything in one SQLite file on one machine. People who use more than one computer want
the same organizer on each. How accounts work (sign-in method, where data is stored and how it is
protected, conflict handling, offline use) is not decided yet; this task records the goal until it is.

## Scope

- In: to be defined.
- Out: sharing tasks with other people.

## Acceptance criteria

- [ ] A design is agreed before any code is written

## Outcome

Shelved: Falog stays a single-machine organizer whose data never leaves the computer. Revisit only
with a design for sign-in, storage, protection of the data and conflict handling.
