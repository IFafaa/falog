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

Semantic helpers: `status_color`, `priority_color`, `due_color`. Company colors come from the data.

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
| `*_picker` | `DropdownMenu` | Company, status, priority |
| `single_line`, `paint_keybinding` | truncation, `KeyBinding` | Painter helpers |

## Layout

```
┌ native title bar (themed) ─────────────────────────────────────────────────────────┐
│ sidebar │ tab bar: Board · List · Agenda              🔍 ＋ │ task  │ assistant      │
│ (left   │ toolbar: breadcrumb › view    sort ✓ [search]    │ panel │ thread         │
│  dock)  │ view                                             │       │ ────────────── │
│         │                                                  │       │ composer 🎤 ➤  │
├─────────┴──────────────────────────────────────────────────┴───────┴────────────────┤
│ status bar: sidebar toggle · counts             listening · ✦ · date · ⌘ · ⚙        │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Assistant dock

Modeled on Zed's agent panel. Header (32 px, `tab_bar`) with ✦ title, model name, new thread and
close. The thread lists user messages (bordered `element` boxes), plain replies (streamed with a ▍
caret), tool calls as compact bordered cards (spinner → check / warning, action label, first line of
the result; click opens the task, hover shows the arguments) and errors in `error`. The composer is an
`editor`-filled box whose border turns `border_focused` on focus; it swaps to a recording bar
(pulsing `error` dot, timer, level meter), a transcribing spinner, or the model download prompt.

## Rules

- Prefer painting rows by hand (`allocate_exact_size` + painter) for pixel control; use egui widgets for
  inputs and menus.
- `ui.horizontal` inherits the parent's direction. Inside right-to-left layouts, allocate a fixed box
  with `allocate_ui_with_layout(..., Layout::left_to_right(..))` or paint manually.
- Every interactive element shows a hover state and a pointing-hand cursor, and has a tooltip with its
  shortcut when one exists.
- Empty states say what to do next ("Tell your assistant about a request, or press Ctrl+N").
- Verify UI changes with screenshots in both themes (`scripts/seed-demo.ps1` gives realistic data).
