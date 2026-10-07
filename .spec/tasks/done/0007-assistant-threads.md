# 0007: Assistant threads

**Status:** done
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

- [x] "+" starts a new thread without losing the previous one
- [x] The history view lists threads newest first; clicking one opens it with its messages
- [x] A thread opened after restarting Falog remembers the earlier conversation
- [x] A long turn keeps running when switching threads; its row shows it is busy
- [x] Deleting a thread removes it from the list and from disk
- [x] Specs updated

## Plan

- `assistant/thread.rs`: `Thread` (items, draft, session, busy) with the event handling that lived in
  `Assistant`; serializable `ThreadRecord`.
- `assistant/history.rs`: load/save the records as JSON.
- `Assistant` owns `Vec<Thread>` plus the active id and history view flag; polls every thread.
- Panel: header with the thread title, history and new-thread buttons; history list view.

## Outcome

`Thread` (items, draft, Claude Code process, `--resume` id) and `history.rs` (JSON, newest first, at
most 100, atomic writes, corrupt files backed up) replace the single conversation. Falog opens on a
fresh thread like Zed; empty threads are dropped when switching; switching ends the idle processes of
the other threads. Saved two seconds after a change and whenever eframe saves. Thread switching,
restart and deletion are covered by unit tests; the panel was built without a live screenshot
because the window could not be captured while occluded, so check it by hand on the next run.
Follow-ups: model-generated titles, search.
