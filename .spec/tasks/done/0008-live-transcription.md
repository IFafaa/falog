# 0008: Live transcription

**Status:** done
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

- [x] Text appears in the composer within a few seconds of starting to speak and keeps updating
- [x] Stopping produces the final transcript (sent or left to edit, per the setting)
- [x] Partials never queue up behind each other
- [x] Short clips no longer repeat words
- [x] Specs updated

## Plan

- `voice.rs`: `Recorder::snapshot`, jobs and results tagged partial/final, the worker keeps only the
  newest pending job (a final one wins), trailing silence.
- `Assistant`: schedule partials while recording, keep `live_text`.
- Panel: show `live_text` while recording and while finishing.

## Outcome

Two worker lanes share one loaded model: a partial lane (half the cores, stale jobs dropped) and a
final lane that starts as soon as the user stops instead of waiting for the preview in progress; no
new preview starts while the final pass runs. The whisper-rs abort callback was tried for cancelling
previews and dropped (it failed every run with error -6). On the CPU under load the first preview
appears a few seconds after speaking starts and the final text about 8 to 9 s after stopping. GPU
(Vulkan) is a setting; it could not be benchmarked fairly because the GPU was busy with a game.
