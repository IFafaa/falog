# Testing

What the automated tests pin, how to run them, and which test guards each bug fixed so far. CI runs all
of them on every push and pull request ([conventions.md](conventions.md#ci)); the quality gate fails
when any does.

## Layers

| Layer | Where | Runs in CI |
|---|---|---|
| Rust unit tests | `#[cfg(test)] mod tests` next to the code in every crate | `test` on Windows, macOS and Ubuntu, with and without default features; `coverage` on Ubuntu |
| MCP behavior | `crates/falog-mcp/src/server.rs`, through `Server::handle_line` like a real client | `test` |
| Site | `site/src/**/*.test.ts` (Vitest) | `site`, then `npm run build` |
| Install scripts | `scripts/test-install.sh`, in temporary home folders | `scripts` on Ubuntu and macOS |

```sh
cargo test --workspace                       # all Rust tests (add --no-default-features without voice)
cd site && npm test                          # site tests (Node 22.12+)
sh scripts/test-install.sh                   # install and uninstall smoke test (macOS, Linux)
.\scripts\check.ps1                          # everything CI checks that runs locally (check.sh elsewhere)
```

## Rules pinned

- **Domain** ([domain.md](domain.md)): every status and priority synonym; due date parsing in both
  languages (keywords, weekdays, numeric forms, two-digit years, the 60-day rollover), urgency and its
  descriptions; the Focus buckets in their order and sorting; recurrence parsing, next dates and the
  rule moving to the next occurrence; archive and restore; task search; area names unique ignoring
  case and accents.
- **Store**: every field round-trips; trimming; completion and archive stamps; notes cascade and bump
  `updated_at`; `data_version` sees other processes' writes; databases from every older schema
  version upgrade keeping every row; private data folders; the migration lock.
- **MCP** ([mcp.md](mcp.md)): exactly the catalog's tools; the task line's exact shape; clearing fields
  with empty values; lenient ids, enums and dates; in-band errors that list the areas; `list_tasks`
  filters, order and limit; the protocol's edges.
- **Calendar**: iCal feeds (time zones, recurrence with exceptions, cancelled and declined events,
  ranges), Google payloads, read-only accounts, the sign-in (PKCE, state check, timeout, cancel),
  config files kept private, event layout, links opened only over http(s), errors that never show a
  secret address.
- **Desktop**: which tasks each view shows (board columns, old done tasks, archive, area filter,
  search, sort orders); the task panel's draft and repeat rules; preferences from older versions; the
  first start's language; every text translated and no stale translation; dates in both languages;
  the toasts; the updater's versions, channels, checksums and badge; dialogs that fit the window; the
  assistant's composer keys, dictation, slash menu, threads and agents.
- **Site**: the roadmap parser, release notes, dates and links; every English text has a Portuguese
  entry with the same placeholders.

## Regression tests

Each fix below has a test that fails when the fix is reverted. A new fix adds its row.

| Fix | What broke | Test |
|---|---|---|
| `cf7cbb3`, `f4973ce` | The database path depended on the working directory | `store::tests::remembers_an_absolute_path` |
| `30d1a99` | The regional format was not read on macOS or from `LC_*` on Linux | `platform::locale::tests::posix_locale_follows_precedence` |
| `b59a8d6` | Claude was not found when Falog was not started from a shell | `platform::paths::tests::searches_the_native_installer_folder_first` |
| `8611baa` | A stopped ACP turn ended the next one | `acp::tests::a_stopped_turn_does_not_end_the_next_one` |
| `4144012` | Claude over ACP loaded every MCP server the user had set up | `acp::tests::claude_over_acp_gets_only_falogs_mcp_server` |
| `5271914` | A dialog taller than the window ran off its bottom | `components::modal::tests::a_dialog_taller_than_the_window_fits_in_it` |
| `9ad0749` | One click changed every segmented control | `components::switch::tests::a_click_changes_only_the_control_it_hits` |
| `2ce4217` | A refresh dropped the events of accounts it skipped | `calendar::tests::a_refresh_keeps_what_it_did_not_or_could_not_fetch` |
| `def032a` | Pasting the wrong Google address gave no hint | `ics::tests::explains_the_other_google_addresses` |
| `073cc1e` | Shift+Enter sent the message instead of breaking the line | `panel::tests::enter_sends_and_shift_enter_breaks_the_line` |
| `06102e8` | ACP agents could not run the area and calendar tools | `acp::tests::knows_every_tool_of_the_mcp_server` |
| `9f5a151` | The migration lock failed while Windows denied access to it | `paths::tests::retries_the_lock_while_access_is_denied_on_windows` |
| `86951fa` | A window saved smaller than the minimum came back unusable | `app::tests::a_window_below_the_minimum_is_too_small` |
| `b62470a` | A command named like a Falog tool was approved as one | `acp::tests::a_command_named_like_a_falog_tool_is_not_one` |
| `458262b` | Any local process could end a Google sign-in | `oauth::tests::turns_away_redirects_without_this_sign_ins_state`, `reads_at_most_8_kb_of_a_request_line` |
| `6a1c2e5` | Event links with any scheme were opened | `model::tests::opens_only_web_links` |
| `a2a26b8` | The agent's folder was in the shared temp dir | `agent::tests::the_workdir_is_never_the_shared_temp_dir` |
| `041714d` | The data folder was readable by other users on Unix | `paths::tests::data_folders_are_private`, `store::tests::the_folders_it_creates_are_private` |
| `6d22535` | `google.json` was readable by other users on Unix | `config::tests::files_are_readable_only_by_the_user_even_over_a_crash_leftover` |
| `130a634` | Two users on one computer shared the single-instance port | `single_instance::tests::each_data_folder_gets_its_own_port` |
| `af9b062` | A corrupt voice model was used | `download::tests::keeps_only_the_expected_file` |
| `9ef0962` | The focus tool kept its old name somewhere | `server::tests::offers_exactly_the_catalog_tools` |
| `fe96ec3` | Keeping the visible tasks between frames could show a stale list | `app::tests::the_visible_tasks_follow_every_change` |
| `626aa33` | Keeping the calendar's events between frames could show stale days | `calendar::tests::shown_events_follow_the_range_and_the_links` |
| `ab4f1be` | A dictation stopped with Enter was sent before it could be read | `assistant::tests::a_dictation_stopped_with_enter_waits_in_the_composer` |
| i18n test | A `tr("…")` in a comment was taken for a UI text | `i18n::tests::the_text_scanner_reads_calls_and_skips_comments` |
| `8a3a6b0` | The releases page stayed loading when GitHub's API refused the site | `site/src/lib/releases.test.ts` ("builds without releases when the API is rate limited") |
| `c9e5862` | Uninstalling with data left the data folder | `scripts/test-install.sh` (`uninstall.sh --remove-data`) |
| `ea431dd` | Closing the browser during a Google sign-in left no way to try again | `calendar::tests::signing_in_again_replaces_the_waiting_attempt` |
| `5326924` | `09/10/26` was read as the year 26 | `date::tests::reads_two_digit_years_in_this_century` |
| `99e45a7` | Archiving one of several tasks said "Archived 1 tasks" | `app::tests::archiving_says_where_the_tasks_went` |

Without a test, on purpose:

- `a24c183`, `7ba899c` (warnings on other systems or without voice): the `lint` job is the test.
- `b339f77` (text only) needs none.
- `d929451`, `98e8c45` (install outside a packaged app's sandbox): the check detects a redirect the
  operating system applies to a probe folder; no fake path can reproduce it.
- `uninstall.ps1` (the Windows side of `c9e5862`): running it stops Falog and unregisters the real
  sign-in entry and MCP server.

## Not covered, and why

Driving the real window (`FalogApp::new`, `Action` dispatch, `check_window_size`), the microphone,
Google's live endpoints, downloading and installing updates, and turning autostart on: each needs a
display, a device, the network or changes to the user's real system. Their rules are tested through the
pure functions they call.
