# 0007: Assistant threads

**Status:** doing
**Area:** desktop

## Goal

Keep several assistant conversations, like Zed's agent threads: start a new thread, browse the
history, switch between threads and continue any of them later, each with its own context.

## Context

The assistant dock (0006) held a single conversation that "+" erased. Each Claude Code session already
has an id that `--resume` continues, so a thread is just its visible items plus that session id.

## Scope

- In: thread list in the dock (history view) with title, last activity and a running indicator;
  new, open and delete; recent threads in the empty state; threads persisted to
  `<data dir>/assistant/threads.json` and resumed with `--resume` after a restart; threads keep
  running in the background while another one is open; idle background threads release their
  Claude Code process.
- Out: model-generated thread titles (the first message is the title), search, pinning.

## Acceptance criteria

- [ ] "+" starts a new thread without losing the previous one
- [ ] The history view lists threads newest first; clicking one opens it with its messages
- [ ] A thread opened after restarting Falog remembers the earlier conversation
- [ ] A long turn keeps running when switching threads; its row shows it is busy
- [ ] Deleting a thread removes it from the list and from disk
- [ ] Specs updated

## Plan

- `assistant/thread.rs`: `Thread` (items, draft, session, busy) with the event handling that lived in
  `Assistant`; serializable `ThreadRecord`.
- `assistant/history.rs`: load/save the records as JSON.
- `Assistant` owns `Vec<Thread>` plus the active id and history view flag; polls every thread.
- Panel: header with the thread title, history and new-thread buttons; history list view.
