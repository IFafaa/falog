# 0021: Tests for the rules and every fixed bug

**Status:** done
**Area:** core, mcp, calendar, desktop, site, scripts, ci

## Goal

Every rule Falog already has is pinned by an automated test, every bug fixed so far has a regression
test that fails without its fix, and CI runs all of them on every push.

## Context

The specs say "every bug fix comes with a test that fails without it", but a third of the fixes so far
landed without one, and the site and the install scripts had no tests at all. Small details the owner
asked for (Enter while dictating, Shift+Enter, archived tasks off the board, restarting a Google
sign-in) would break silently in a later change.

## Scope

- In: unit tests for documented rules (`domain.md`, `mcp.md`, `design-system.md`, doc comments);
  regression tests for the fix commits; site unit tests; a smoke test of the install and uninstall
  scripts; CI jobs for the site and the scripts; a raised coverage minimum; a list of what is pinned
  in [testing.md](../../testing.md).
- Out: new features, behavior changes (except bugs the tests uncover, fixed in their own commits),
  end-to-end tests that drive the real window.

## Acceptance criteria

- [x] Each fix commit has a regression test, or [testing.md](../../testing.md) says why it cannot have one
- [x] Each regression test was seen failing with its fix reverted
- [x] The rules of `domain.md` and `mcp.md` are pinned by tests
- [x] CI runs the Rust tests on the three systems, the site tests and the script smoke test, all in
      the quality gate
- [x] `MIN_LINE_COVERAGE` raised to the new measurement

## Plan

Three parallel pieces with no shared files: core and MCP; calendar, site, scripts and the CI jobs;
the desktop app. Then merge, measure coverage, raise the minimum and write `testing.md`.

## Outcome

- Rust tests went from about 230 to 447 (core 122, MCP 54, calendar 101, desktop 170); the site gained
  29 Vitest tests and the install scripts a smoke test. Line coverage went from 44.9% to 63.6%;
  `MIN_LINE_COVERAGE` is now 60.
- CI: new `site` and `scripts` jobs in the quality gate; `check.ps1` and `check.sh` run both.
- Bugs the tests found: `09/10/26` read as the year 26; archiving one of several tasks said
  "Archived 1 tasks"; the i18n scanner read `tr` calls inside comments; a stale Portuguese entry.
- Decided: `get_focus` keeps archived tasks under "Completed in the last 7 days" (domain.md).
- Small refactors only to make code testable (visible-task filter, toasts, repeat picker rules,
  update badge texts, first-start language, the sign-in redirect wait, the migration lock).
- Left as they are: the Area sort places tasks without an area under the English "No area"; a
  Google calendar id is accepted from any host ending in `google.com`.
