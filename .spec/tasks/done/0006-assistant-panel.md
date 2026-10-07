# 0006: Assistant panel with voice

**Status:** done
**Area:** desktop

## Goal

Talk to Falog itself. A right dock, like Zed's agent panel, holds a conversation with Claude; the user
types or dictates (push-to-talk) and Claude creates and updates tasks, with every change showing up on
the board immediately. No separate Claude Code window needed.

## Context

Until now tasks were filed by talking to Claude in a Claude Code session that used `falog-mcp`
([mcp.md](../../mcp.md)). That works but requires keeping another app open.

Decisions (2026-10-06, with the user):

- **Brain: the user's Claude subscription through Claude Code headless**, not the API. Falog runs one
  long-lived `claude -p --input-format stream-json --output-format stream-json` process per thread,
  with all built-in tools disabled (`--tools ""`) and only the `falog` MCP server allowed
  (`--strict-mcp-config --allowedTools mcp__falog --permission-mode dontAsk --setting-sources ""`).
  Verified: one process serves many turns; a turn with one tool call takes ~4 s on Haiku.
- **Speech-to-text: Whisper, locally** (whisper.cpp through `whisper-rs`), model
  `ggml-large-v3-turbo-q5_0.bin` (~574 MB) downloaded on demand into `%APPDATA%\Falog\models`.
  Anthropic has no speech-to-text API.
- **Replies are text only.**

## Scope

- In: assistant dock (toggle from status bar, palette, `Ctrl+Shift+A`); streaming replies; tool calls
  rendered as compact cards linking to the task; push-to-talk (`Ctrl+Space` or mic button) with level
  meter; transcription in a worker thread; auto-send after transcription (setting); model download
  with progress; stop a running turn; new thread; settings for model, voice language and auto-send;
  clear errors when `claude` is missing or not signed in.
- Out: spoken replies, resuming threads after restarting Falog, wake word, non-Windows audio testing.

## Acceptance criteria

- [x] "Add a task for Globex to review the payments PR by friday" (typed or spoken) creates the task and
      the board shows it within a second of the reply
- [x] Tool calls appear in the thread; a created/updated task can be opened from its card
- [x] Push-to-talk records, transcribes Portuguese and English correctly, and sends
- [x] The Whisper model downloads once with visible progress and is reused
- [x] Stop interrupts a turn; the next message continues the same conversation
- [x] Missing `claude` CLI or Whisper model produce an actionable message, never a crash
- [x] `architecture.md`, `design-system.md` and the README updated

## Plan

- `assistant/claude.rs`: spawn and talk to Claude Code (stream-json in/out), reader thread mapping
  events to `ClaudeEvent`; `--resume <session>` after a stop.
- `assistant/voice.rs`: `Recorder` (cpal, mono, resampled to 16 kHz), `Transcriber` worker thread
  (whisper-rs), `ModelDownload` (ureq, progress).
- `assistant/panel.rs`: dock UI, thread items, composer.
- `assistant/mod.rs`: `Assistant` state machine tying the three together.
- Assistant system prompt in `assets/assistant-prompt.md`.
- Prefs, actions, shortcuts, status bar button, palette commands, settings section.

## Outcome

Shipped as planned. Verified on a demo database: a Portuguese request created a task and another
completed one with a note, about 4 to 5 s per turn; Whisper transcribed the English sample word for
word. Decisions: Claude Code headless on the user's subscription instead of an API key; Whisper runs
locally (`large-v3-turbo` q5_0) behind the `whisper` feature, with an optional `gpu` (Vulkan) feature;
replies are text only. Follow-ups: threads (0007) and live transcription (0008).
