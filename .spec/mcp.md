# MCP server

`falog-mcp` speaks MCP over stdio: one JSON-RPC 2.0 message per line, UTF-8 (a leading BOM is
tolerated). It supports `initialize`, `ping`, `tools/list`, `tools/call`, and returns empty lists for
`resources/list` and `prompts/list`. Notifications get no response.

`initialize` echoes the client's `protocolVersion` and returns `instructions`: guidance assistants
follow for the whole session (create tasks eagerly, keep full context, resolve dates, one task per
request). Keep it in sync with `assistant/CLAUDE.md`.

## Tools

| Tool | Arguments | Result |
|---|---|---|
| `get_agenda` | `company?` | Markdown agenda: headline, per-company counts, non-empty buckets, completed last 7 days |
| `list_companies` | — | One line per company with open count |
| `create_company` | `name`, `color?` | Confirmation |
| `create_task` | `title`, `company?`, `description?`, `priority?`, `due_date?`, `requester?`, `status?` | `Created #id [...]` line |
| `update_task` | `id`, any task field, `note?` | `Updated ...` line |
| `add_note` | `id`, `note` | Confirmation line |
| `list_tasks` | `company?`, `status?`, `search?`, `include_done?`, `limit?` | Count + task lines, by status then urgency |
| `get_task` | `id` | Full details with description and activity log |
| `delete_task` | `id` | Confirmation (assistants should prefer `status: done`) |

## Conventions

- **Lenient input.** Ids accept `12`, `"12"`, `"#12"`; enum-like fields accept numbers or strings;
  statuses and priorities accept synonyms; companies accept unique fragments.
- **Clearing fields.** In `update_task`, an empty `company` or `due_date` clears it; omitted fields are
  untouched.
- **Errors are in-band.** Tool failures return `isError: true` with a message the model can act on
  (e.g. unknown company → list of registered ones). Protocol errors use JSON-RPC error codes.
- **Text output, stable shape.** Every task line is `#id [Company] Title — Priority · due ... · Status ·
  requested by ...` (see `format.rs`). Models rely on it; change it deliberately.
- Tool descriptions are part of the product: they steer the model. Keep them short and directive.

## Adding a tool

1. Schema in `tools/catalog.rs`.
2. Argument struct in `tools/args.rs` (lenient types where models may vary).
3. Handler in `tools/handlers.rs`, using only `Store` methods; rendering in `format.rs`.
4. Test through `Server::handle_line` in `server.rs` and update the table above.
