# 0011: Assistant agents (Claude Code, ACP), models, effort and slash commands

**Status:** done
**Area:** desktop

## Goal

The assistant dock is no longer hard-wired to Claude: each thread talks to an *agent*. Claude Code
stays the default, and any agent that speaks the [Agent Client Protocol](https://agentclientprotocol.com)
(Claude through its ACP adapter, Gemini CLI, Codex, or a custom command) can be plugged in on the
user's own subscription. From the composer the user picks the agent, the model and the effort of a
thread, and runs the agent's slash commands by typing `/`.

## Context

The dock spawns `claude --print --input-format stream-json --output-format stream-json` per thread
([architecture.md](../../architecture.md#assistant), `assistant/claude.rs`); threads talk to it
directly. The model is a global setting (Default, Haiku, Sonnet, Opus) and effort cannot be chosen.

ACP is the JSON-RPC 2.0 protocol over stdio that Zed uses for external agents. Checked against the
schema in `agentclientprotocol/agent-client-protocol` (v1 1.24.1; v2 is still alpha):

- Client → agent: `initialize` (`protocolVersion: 1`, `clientCapabilities` with no fs/terminal),
  `session/new` (`cwd`, `mcpServers` with stdio `{name, command, args, env: [{name, value}]}`),
  `session/resume` when `agentCapabilities.sessionCapabilities.resume` is set, else `session/load`
  when `agentCapabilities.loadSession` (it replays history, which we drop), `session/prompt`
  (`prompt: [{type: "text", text}]`, answers `stopReason`), `session/cancel` (notification),
  `session/set_config_option` (`configId`, `value`).
- Agent → client: `session/update` notifications (`agent_message_chunk`, `tool_call`,
  `tool_call_update`, `available_commands_update` with `{name, description, input: {hint}}`,
  `config_option_update`), and the `session/request_permission` request (answered with
  `{outcome: {outcome: "selected", optionId}}`).
- Model and effort are *session config options* (`configOptions` in the session responses), select
  options with `category: "model"` and `"thought_level"`. The older `session/set_model` is gone from
  the schema.

Adapters (npm, checked Oct 2026): `@agentclientprotocol/claude-agent-acp` (was
`@zed-industries/claude-code-acp`, needs Node 22+; honors `_meta.systemPrompt` and
`_meta.claudeCode.options`), `@agentclientprotocol/codex-acp`, and Gemini CLI's own `gemini --acp`
(`--experimental-acp` is deprecated).

Claude Code 2.1 headless takes `--model <alias|name>` and `--effort low|medium|high|xhigh|max`, and
reports its commands in the `init` message (`slash_commands`) and in `system/commands_changed`
(`{name, description, argumentHint}`); a command is run by sending it as a user message.

## Scope

- In:
  - `assistant/agent/`: an `AgentEvent` stream and a `Session` trait that threads talk to; the
    current stream-json code moves behind it as the `claude-code` agent with no regressions (resume,
    stop, MCP tools, live board refresh).
  - A generic ACP client (newline-delimited JSON-RPC over the agent's stdio) with the falog MCP server
    in `mcpServers`, resume/load, prompt, cancel, config options and permission answers that allow
    only falog tools.
  - Agents configured as `{name, command, args, env}`: presets for Claude Code (headless, default),
    Claude via ACP, Gemini CLI and Codex; custom agents added and removed in Settings.
  - Per-thread agent, model and effort, persisted in `threads.json`; new threads start from the last
    choice. Pickers in the composer footer, as in Zed. Changing model or effort of Claude Code
    restarts it with `--resume`; ACP agents switch live.
  - Slash commands: typing `/` lists the agent's commands; picking one inserts it, sending sends it as
    a prompt.
  - The history list shows each thread's agent.
- Out: ACP file system and terminal capabilities (the assistant only needs falog tools), images and
  other content blocks, agent authentication flows (the user signs in once in a terminal), a mode
  picker, installing agents for the user.

## Acceptance criteria

- [x] Claude Code works as before: replies stream, tools run and the board refreshes, Stop works, a
      thread resumes after a restart
- [x] The composer footer shows the thread's agent and model and, when the agent has one, its effort;
      choosing another model or effort applies to the next message, keeping the conversation
- [x] Typing `/` in the composer lists the agent's commands with descriptions; Tab/Enter completes,
      arrows move, Esc closes; sending a command (`/context`) runs it
- [x] An ACP agent (Claude adapter when Node 22+ is available) answers and creates tasks through the
      falog MCP server; any other tool permission request is rejected
- [x] Gemini CLI and Codex presets are listed, and a custom agent can be added and removed in
      Settings; a missing command shows a clear error in the thread (Gemini and Codex themselves
      not run: not installed or signed in on the dev machine)
- [x] Threads remember their agent, model and effort across restarts; history rows show the agent
- [x] Specs updated (`architecture.md`, `design-system.md`, README)
- [x] Tests cover ACP parsing and dispatch (in-memory streams), the Claude Code parser and thread
      settings; `fmt`, `clippy -D warnings`, `test` green with `--no-default-features`, default and
      `--features falog-desktop/gpu`

## Plan

1. Move `claude.rs` to `agent/claude_code.rs` behind `agent::Session`/`AgentEvent`; `Thread` holds a
   `Box<dyn Session>`. Agent registry (`agent/registry.rs`) with the built-in presets and custom
   agents, executable lookup that understands `.cmd` shims on Windows.
2. Claude Code: `--effort`, commands from `init`/`commands_changed`, static model/effort options.
3. `agent/acp.rs`: transport over any `Read`/`Write` (tested with in-memory pipes), handshake state
   machine on the reader thread, permission policy, config options.
4. Thread settings (agent, model, effort) in `ThreadRecord` with serde defaults so old
   `threads.json` files load; `Assistant` keeps per-agent caches of options and commands in
   `<data dir>/assistant/agents.json` together with custom agents and the last choice.
5. Composer footer pickers, slash command popup, history row agent label.
6. Settings: "Agents" list with an add form; the global model setting goes away (per thread now).
7. Live checks: Claude Code headless through the new path (ignored test / debug run), the Claude ACP
   adapter with Node 22+ if available.

Risks: agents differ in how they name MCP tools (`mcp__falog__x`, `x (falog MCP Server)`), so the
permission policy and tool labels match on the falog tool names; ACP adapters started through `npx`
are slow on first run (download).

## Outcome

- `assistant/agent/`: `Session` trait + `AgentEvent`; `claude_code.rs` (the former `claude.rs`, now
  with `--effort`, commands from `init`/`commands_changed` and static model/effort options),
  `acp.rs` (hand-written ACP v1 client), `registry.rs` (presets, custom agents, `PATHEXT`-aware
  command lookup), `live_tests.rs` (ignored end-to-end check against real agents).
  `assistant/agents.rs` persists `agents.json`; `assistant/slash.rs` is the `/` menu;
  `assistant/agent_settings.rs` is the Settings section.
- Decisions:
  - Model and effort come from ACP *session config options* (`category: model / thought_level`);
    the unstable `session/set_model` is gone from the schema. Claude Code headless gets a static
    list of aliases (Default, Fable, Opus, Sonnet, Haiku) and levels (low...max), so it never goes
    stale. Mode options (permission modes) are not shown: Falog answers permissions itself.
  - A thread's agent is fixed after its first message, as in Zed: the conversation lives in the
    agent. New threads start from the last agent/model/effort picked (`agents.json`).
  - Permission policy: allow once for falog tools however the agent spells them, reject everything
    else. Claude's adapter also gets `tools: []`, `settingSources: []` and `strictMcpConfig`
    (without it the user's other MCP servers added 69k tokens of tools per conversation).
  - Stop with ACP sends `session/cancel` and keeps the session; late updates and the stale answer
    to the cancelled prompt are dropped.
  - Agents that do not read `_meta.systemPrompt` get the instructions in front of the first prompt
    of a new session.
  - The global model setting is gone (old `app.ron` files still load); old `threads.json` files
    load as Claude Code threads.
- Verified live (`cargo test -p falog-desktop live -- --ignored`): Claude Code headless with
  `--model haiku --effort low` and `@agentclientprotocol/claude-agent-acp` 0.86 under Node 24 both
  called falog's `create_area` and `list_areas`, ran `/context`, and resumed the conversation in a
  new process (`--resume` / `session/resume`). No Node processes were left behind. UI checked
  with window captures of a dev build on a demo database (pickers, agent menu, `/` menu, history
  rows, Settings › Agents); computer use was not available, so menus were opened by a throwaway
  dev hook and the menu keys (arrows, Tab, Enter, Esc) were not exercised by hand.
- Also fixed on the way: dialogs taller than the window scroll (Settings no longer runs off small
  windows); Send and Stop are icon buttons so the pickers fit the default dock width.
- Follow-ups:
  - Gemini CLI and Codex were not run (not installed / not signed in here); their presets follow
    the documented commands (`gemini --acp`, `npx -y @agentclientprotocol/codex-acp`).
  - Claude's adapter streams the output of local commands like `/context` twice; a dedupe by
    `messageId` would hide it.
  - Claude Code headless did not emit `system/commands_changed` in some runs, so its commands can
    lack descriptions until it does.
  - Agents that run read-only built-in tools without asking permission (Gemini's file reading,
    web search) are not blocked; only tools that ask are.
  - `segmented` gives two controls in the same Settings layout the same widget id (debug builds
    show "First/Second use of widget ID"); it predates this task.
  - Merge with the macOS/Linux branch: route `find_claude` and `registry::find_executable` through
    `platform::paths::find_executable`.
