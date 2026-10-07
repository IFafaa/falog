# 0009: Areas instead of companies

**Status:** doing
**Area:** core

## Goal

Group tasks by **area** (an employer, a client, "Personal", "Health", "Home"...) instead of by
company, so personal tasks and appointments live next to work without pretending to be a company.

## Context

Falog started as a board for a developer working for several companies, and every task could belong
to a company. The user also wants to track personal tasks and appointments, which do not belong to a
company. "Area" follows Things 3 and the PARA method: an ongoing sphere of responsibility. It leaves
room for projects inside an area later.

## Scope

- In: rename the entity everywhere (database, `falog-core` API, MCP tools and arguments, UI labels,
  assistant prompt, scripts, docs); a migration that keeps existing companies and their tasks; the
  sidebar filter preference survives the rename.
- Out: projects inside areas, a time of day on due dates (appointments), a default "Personal" area.

## Acceptance criteria

- [ ] An existing database opens with its companies as areas and every task still attached
- [ ] MCP tools are `list_areas` / `create_area`, and task tools take `area`
- [ ] The UI says "Areas" everywhere it said "Companies" (sidebar, pickers, dialogs, palette)
- [ ] The assistant files personal requests under a personal area
- [ ] Specs updated (`product.md`, `domain.md`, `mcp.md`, `design-system.md`, `architecture.md`)
- [ ] Tests cover the migration; `fmt`, `clippy`, `test` green

## Plan

1. `falog-core`: migration 2 renames `companies` → `areas` and `tasks.company_id` → `area_id`
   (SQLite `ALTER TABLE ... RENAME` keeps ids and foreign keys).
2. Rename `Company`/`CompanyId` and the store API to `Area`/`AreaId` across the workspace.
3. MCP: tool names, argument names, descriptions, server instructions, output text.
4. Desktop: labels, the areas dialog, palette commands, `#[serde(alias = "company")]` on the filter.
5. Assistant prompt and `assistant/CLAUDE.md`; demo seed with a personal area; README and specs.

Risk: MCP clients registered with the old tool names keep working only after they reload the tool
list (Claude Code does on its next session).

## Outcome

Filled when done.
