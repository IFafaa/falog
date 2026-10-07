# 0008: Live transcription

**Status:** doing
**Area:** desktop

## Goal

While the user is dictating, show the words as they are recognized, greyed out in the composer like a
placeholder, the way dictation works in the Claude apps. The final text replaces it when they stop.

## Context

Whisper is not a streaming model. Measured on the target machine (Ryzen 7 5700X, other apps busy):
2.6 s of speech transcribes in ~3.6 s, 5 s in ~5.5 s, so re-transcribing the clip so far every
second or two keeps the preview a few seconds behind the speaker. Very short clips with a small
`audio_ctx` repeat themselves ("X. X."); appending 1 s of silence fixes it at almost no cost.

## Scope

- In: periodic partial transcriptions while recording (only one in flight, stale ones dropped), the
  latest partial shown in the composer, final pass on stop, model loaded as soon as recording starts,
  1 s of trailing silence on every clip.
- Out: true streaming (word by word), a separate smaller model for previews.

## Acceptance criteria

- [ ] Text appears in the composer within a few seconds of starting to speak and keeps updating
- [ ] Stopping produces the final transcript (sent or left to edit, per the setting)
- [ ] Partials never queue up behind each other
- [ ] Short clips no longer repeat words
- [ ] Specs updated

## Plan

- `voice.rs`: `Recorder::snapshot`, jobs and results tagged partial/final, the worker keeps only the
  newest pending job (a final one wins), trailing silence.
- `Assistant`: schedule partials while recording, keep `live_text`.
- Panel: show `live_text` while recording and while finishing.
