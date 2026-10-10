# 0009: Areas for every part of life

**Status:** done
**Area:** core

## Goal

Group tasks by **area** ("Work", "Personal", "Health", "Home"...), so personal tasks, errands and
appointments live next to work in one organizer.

## Context

Falog's grouping only fit work. The user also wants to track personal tasks and appointments, so the
grouping becomes "area": an ongoing sphere of
responsibility. It leaves room for projects inside an area later.

## Scope

- In: the area entity everywhere (database, `falog-core` API, MCP tools and arguments, UI labels,
  assistant prompt, scripts, docs).
- Out: projects inside areas, a time of day on due dates (appointments), a default "Personal" area.

## Acceptance criteria

- [x] Tasks belong to areas in the database, the core API, the MCP tools and the UI
- [x] MCP tools are `list_areas` / `create_area`, and task tools take `area`
- [x] The assistant is told to file personal requests under a personal area
- [x] Specs updated (`product.md`, `domain.md`, `mcp.md`, `design-system.md`, `architecture.md`)
- [x] `fmt`, `clippy`, `test` green

## Plan

1. `falog-core`: the `areas` table and `tasks.area_id`; `Area`/`AreaId` and the store API.
2. MCP: tool names, argument names, descriptions, server instructions, output text.
3. Desktop: labels, the areas dialog, palette commands, the sidebar filter.
4. Assistant prompt and `assistant/CLAUDE.md`; demo seed with personal areas; README and specs.

## Outcome

- `Area`, `AreaId` and the store API (`areas()`, `find_area()`...); MCP tools `list_areas` and
  `create_area`, and every task tool takes `area`.
- The sidebar header button is a plus ("Add area"), and the dialog focuses the new area field.
- The assistant prompt, the `~/falog-assistant` workspace and the demo seed use areas such as Work,
  Home, Studies and Health. Whether the model picks the right area was checked by reading the prompt.
- Follow-ups: a time of day on due dates for appointments; projects inside areas.
