# Design system

Falog follows [Zed](https://github.com/zed-industries/zed)'s UI closely. When in doubt, open Zed and
copy what it does. Reference sources in the Zed repo: `assets/themes/one/one.json` (colors),
`crates/ui` (components, spacing), `crates/workspace` (docks, status bar), `crates/title_bar`.

## Color tokens

Defined in `theme.rs` with Zed's token names; values are Zed's **One Dark** and **One Light**.
Never hard-code colors in UI code; use a token.

| Token | Use |
|---|---|
| `title_bar`, `status_bar` | Window chrome (native caption is tinted to match) |
| `tab_bar`, `panel` | Tab bar, docks (sidebar, task panel), dock headers |
| `editor`, `tab_active` | Main content, active tab |
| `elevated_surface` | Cards, modals, popovers |
| `border` / `border_variant` / `border_focused` | Strong / subtle / focus or selection outlines |
| `element*` | Filled controls (inputs, combos, filled buttons) |
| `ghost_hover`, `ghost_selected` | Transparent controls and list rows |
| `text`, `text_muted`, `text_placeholder`, `text_accent` | Text hierarchy |
| `error`, `warning`, `success`, `hint` | Status colors |

Semantic helpers: `status_color`, `priority_color`, `due_color`. Area colors come from the data.

## Typography

| Use | Font | Size |
|---|---|---|
| Body, buttons | IBM Plex Sans | 14 (13–13.5 in dense rows) |
| Secondary text | IBM Plex Sans | 12–12.5 |
| Headers, titles | IBM Plex Sans SemiBold (`fonts::semibold`) | 13.5–22 |
| Ids, timestamps, commands | Lilex (monospace) | 11.5–13 |

## Metrics

- Title bar 32 px (native), tab bar and dock headers 32 px (`workspace::BAR_HEIGHT`), toolbar 38 px,
  status bar 28 px.
- Buttons and inputs 24 px tall; icons 14 px (12 px in metadata, 15 px for status icons in rows).
- Corner radius: 4 px controls, 6 px cards, 8 px modals.
- List rows 26 px (sidebar) / 30 px (tables, palette). Spacing 4/6/8/12/16.

## Components (`components/`)

| Component | Zed equivalent | Notes |
|---|---|---|
| `button(style, icon, label)` | `Button` | `Ghost`, `Filled`, `Accent` (primary), `Danger` |
| `icon_button`, `icon_toggle` | `IconButton` | 24×24 ghost; toggles show `ghost_selected` |
| `meta`, `badge` | `Label` + icon, `CountBadge` | Card metadata, counters |
| `modal(placement)` | `Modal` | Dimmed backdrop, Esc/backdrop dismiss; `Top` for pickers |
| `switch`, `segmented` | `Switch`, `ToggleButtonGroup` | Settings controls |
| `*_picker` | `DropdownMenu` | Area, status, priority |
| `single_line`, `paint_keybinding` | truncation, `KeyBinding` | Painter helpers |

## Layout

```
┌ native title bar (themed) ─────────────────────────────────────────────────────────┐
│ sidebar │ tab bar: Board·List·Focus·Calendar           🔍 ＋ │ task  │ assistant      │
│ (left   │ toolbar: breadcrumb › view    sort ✓ [search]    │ panel │ thread         │
│  dock)  │ view                                             │       │ ────────────── │
│         │                                                  │       │ composer 🎤 ➤  │
├─────────┴──────────────────────────────────────────────────┴───────┴────────────────┤
│ status bar: sidebar toggle · counts             listening · ✦ · date · ⌘ · ⚙        │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Assistant dock

Modeled on Zed's agent panel. Header (32 px, `tab_bar`) with ✦ and the thread title (its first
message; hover shows the model), then new thread (+), history (clock, toggled), zoom (maximize / minimize, `Shift+Esc`) and close.
Zoomed, the dock fills the window like Zed's zoomed panels: sidebar, tabs, toolbar, views and task
panel are hidden, the status bar stays, and the thread, history and composer keep to a centered
820 px column. The history
view replaces the thread and composer with rows of 44 px: title, then `Agent · N messages · 2h ago` in
`text_muted`; the open thread is `element_selected`, hover is `ghost_hover` with a trash button, and a
running thread shows a spinner instead. The empty state lists the three most recent threads under the
examples, with "View all" when there are more. The thread lists user messages (bordered `element` boxes), plain replies (streamed with a ▍
caret), tool calls as compact bordered cards (spinner → check / warning, action label, first line of
the result; click opens the task, hover shows the arguments) and errors in `error`. The composer is an
`editor`-filled box whose border turns `border_focused` on focus; it swaps to a recording bar
(pulsing `error` dot, timer, level meter), a transcribing spinner, or the model download prompt.
Under the text, the footer has two rows so it fits the default dock width. The first holds the thread's
agent, model, effort and mode pickers (Zed's selectors: `text_muted` label and a chevron,
`ghost_hover` on hover, a menu above with a check on the current value; a picker only shows when
the agent offers that setting, and the agent becomes a plain label once the thread has messages).
The second has the mic, the fast mode flame (an icon toggle, `ghost_selected` while on, like Zed's
burn mode; the tooltip says why it is unavailable), "Ultracode" in `text_placeholder` for Claude
agents with a tooltip on why it is off, and on the right the context ring and icon-only Send or
Stop (tooltips "Send (Enter)", "Stop"). The context ring is 13 px across: a 2 px `border_variant`
track filled clockwise from twelve o'clock in `text_muted`, `warning` from 70 % and `error` from
90 %; its tooltip reads "Context: 12.4k of 200k tokens (6%)". Settings › Assistant lists the agents one per line (name, command line in monospace `text_muted`,
`Installed` in `success` or `Not found` in `text_placeholder` with the setup hint on hover, a trash
button for custom ones); "Add agent" opens a bordered form (name, command, environment).
Typing `/` opens the command menu above the composer (`elevated_surface`, popover shadow, rows of
30 px): `/name` in monospace, the argument hint in `text_placeholder`, the description in
`text_muted`; the selected row is `ghost_selected`. Arrows move, Tab or Enter completes `/name `,
Enter on a complete name sends it, Esc closes.

### Calendar view

Modeled on Google Calendar, painted with Zed tokens. Header: `Today` (filled), previous/next, the
range title (SemiBold 16), then the refresh button, status ("Updated 10:32" or the error in `warning`)
and a Week/Month `segmented`. Week: 52 px hour gutter, 48 px per hour, day names over the day number
(today in a `text_accent` circle), an all-day row with lanes for all-day and multi-day events and for
tasks due that day, timed events as blocks tinted with the calendar color and a 3 px bar in full
color (past events fainter), overlapping events side by side, and an `error` line for now. Month: whole
weeks, timed events as dot + time + title, all-day events as filled bars, "+N more" opens the week.
An event opens a popover (`elevated_surface`) with time, calendar, location, description, Join and
Open in Google Calendar. The sidebar adds a Calendars section with one checkbox per calendar, in its
Google color, grouped by account.

## App icon

An **F whose middle arm is a voice fading out**: the stem and top arm are a line already in the log,
and the middle arm is two bars and a dot getting shorter, like a waveform dying down. What you say
becomes a line in the log. Accent ink (`#74ade8`; at 32 px and up a `#8cbcf0 → #5f9fe2` gradient) on a
rounded One Dark tile (`#31363f → #22262c`, edge `#464b57`). No check mark: every to-do app has one.
The concepts it was picked from are in `docs/icon-concepts` (see task 0003).

Files in `crates/falog-desktop/assets/icon`:

| File | Use |
|---|---|
| `falog.svg` | Master for 32 px and up (256 viewBox, tile 8–248, corner radius 56) |
| `falog-16.svg`, `falog-24.svg` | Redrawn on the pixel grid (2 / 3 px strokes, 1 px gaps), used at exactly 16 and 24 px |
| `png/falog-N.png` | 16–1024 px; `falog-256.png` is the window and taskbar icon (`icons::app_icon`) |
| `falog.ico` | 16–256 px; embedded in `falog.exe` by `build.rs`, used by the Start menu shortcut |
| `falog.icns` | macOS: the tile on Apple's 824/1024 grid with a drop shadow |

Edit the SVGs, never the generated files, then run
`cargo run --release --manifest-path tools/icon/Cargo.toml` and check `docs/icon-preview.png` (dark and
light taskbar backgrounds, 16/24/32 px magnified). A change to the master's geometry goes to the hinted
copies too. On Linux, install the PNGs and the SVG into the `hicolor` theme and use `Icon=falog`.

### Settings

A 780 × 520 modal with a 184 px `panel` column of pages on the left (General, Appearance, Assistant,
Voice, Calendar, Data; icon + label rows of 30 px, `ghost_selected` for the open page) and the page on
the right: its title in SemiBold 16, then rows of title, description in `text_muted` and the control on
the right. Each page is one function in `overlays/settings/` (Calendar in its own file).
`Action::OpenSettingsTab` opens a given page, as the calendar's "Set up" does.

## Rules

- Prefer painting rows by hand (`allocate_exact_size` + painter) for pixel control; use egui widgets for
  inputs and menus.
- `ui.horizontal` inherits the parent's direction. Inside right-to-left layouts, allocate a fixed box
  with `allocate_ui_with_layout(..., Layout::left_to_right(..))` or paint manually.
- Every interactive element shows a hover state and a pointing-hand cursor, and has a tooltip with its
  shortcut when one exists.
- Empty states say what to do next ("Tell your assistant about a request, or press Ctrl+N").
- Verify UI changes with screenshots in both themes (`scripts/seed-demo.ps1` gives realistic data).
