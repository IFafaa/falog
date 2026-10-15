use crate::protocol::{self, DEFAULT_PROTOCOL_VERSION, Message};
use crate::tools;
use falog_core::Store;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

/// Sent to the client on `initialize`; assistants use it as guidance for the whole session.
const INSTRUCTIONS: &str = "Falog is the user's personal organizer: tasks, errands and appointments for \
work and personal life, usually dictated by voice. Tasks belong to areas the user chose (Work, Personal, \
Health, Home...). When \
they describe a request (\"I got a task\", \"so-and-so asked\", \"I need to\", \"I have a doctor's \
appointment\"), call create_task with a short actionable title, the area, a \
description that keeps ALL the context they gave, the requester, the due date as YYYY-MM-DD (resolve \
relative dates against today) and a priority; for something that repeats (\"every friday\", \"on the 5th \
of each month\"), pass recurrence too. One message may contain several requests: create one task \
each. Before updating a task mentioned by name, find its id with list_tasks. When they ask what to work \
on or to catch up (\"good morning\", \"what's on my plate\", \"what did I do last week\"), call get_focus. \
Statuses: todo, in_progress, waiting (blocked on someone, in review), done. Reply in the user's language.";

#[derive(Debug)]
pub struct Server {
    store: Store,
    calendar: tools::CalendarAccess,
}

impl Server {
    pub fn new(store: Store) -> Self {
        let calendar = tools::CalendarAccess::for_store(&store);
        Self { store, calendar }
    }

    /// Serves newline-delimited JSON-RPC until `input` ends.
    pub fn serve(&self, input: impl BufRead, mut output: impl Write) -> io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(response) = self.handle_line(&line) {
                writeln!(output, "{response}")?;
                output.flush()?;
            }
        }
        Ok(())
    }

    pub fn handle_line(&self, line: &str) -> Option<Value> {
        // Some shells (Windows PowerShell 5.1) prefix piped input with a UTF-8 BOM.
        let line = line.trim_start_matches('\u{feff}');
        match serde_json::from_str::<Message>(line) {
            Ok(message) => self.handle(message),
            Err(err) => Some(protocol::failure(
                Value::Null,
                protocol::PARSE_ERROR,
                format!("invalid JSON: {err}"),
            )),
        }
    }

    fn handle(&self, message: Message) -> Option<Value> {
        let id = message.id?;
        let Some(method) = message.method else {
            return Some(protocol::failure(id, protocol::INVALID_REQUEST, "missing method"));
        };
        let response = match method.as_str() {
            "initialize" => protocol::success(id, Self::initialize(&message.params)),
            "ping" => protocol::success(id, json!({})),
            "tools/list" => protocol::success(id, json!({ "tools": tools::catalog() })),
            "tools/call" => protocol::success(id, self.call_tool(&message.params)),
            "resources/list" => protocol::success(id, json!({ "resources": [] })),
            "prompts/list" => protocol::success(id, json!({ "prompts": [] })),
            other => protocol::failure(
                id,
                protocol::METHOD_NOT_FOUND,
                format!("method not found: {other}"),
            ),
        };
        Some(response)
    }

    fn initialize(params: &Value) -> Value {
        let version = params["protocolVersion"]
            .as_str()
            .unwrap_or(DEFAULT_PROTOCOL_VERSION);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "falog", "version": env!("CARGO_PKG_VERSION") },
            "instructions": INSTRUCTIONS,
        })
    }

    /// Tool failures are reported in the result (`isError`) so the model can read and recover.
    fn call_tool(&self, params: &Value) -> Value {
        let name = params["name"].as_str().unwrap_or_default();
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
        match tools::call(&self.store, &self.calendar, name, arguments) {
            Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
            Err(err) => {
                json!({ "content": [{ "type": "text", "text": format!("Error: {err:#}") }], "isError": true })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> Server {
        Server::new(Store::open_in_memory().unwrap())
    }

    fn call(server: &Server, id: i64, tool: &str, args: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
                           "params": { "name": tool, "arguments": args } });
        server.handle_line(&line.to_string()).unwrap()["result"].clone()
    }

    fn text(result: &Value) -> &str {
        result["content"][0]["text"].as_str().unwrap()
    }

    /// Calls a tool that must succeed and returns its text.
    fn ok(server: &Server, tool: &str, args: Value) -> String {
        let result = call(server, 1, tool, args);
        assert_ne!(result["isError"], true, "{tool}: {}", text(&result));
        text(&result).to_owned()
    }

    /// Calls a tool that must fail in-band and returns its error text.
    fn fails(server: &Server, tool: &str, args: Value) -> String {
        let result = call(server, 1, tool, args);
        assert_eq!(result["isError"], true, "{tool}: {}", text(&result));
        text(&result).to_owned()
    }

    #[test]
    fn initializes_with_client_version() {
        let response = server()
            .handle_line(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
            )
            .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(response["result"]["serverInfo"]["name"], "falog");
    }

    #[test]
    fn ignores_notifications_and_rejects_unknown_methods() {
        let server = server();
        assert!(
            server
                .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );
        let response = server
            .handle_line(r#"{"jsonrpc":"2.0","id":2,"method":"nope"}"#)
            .unwrap();
        assert_eq!(response["error"]["code"], protocol::METHOD_NOT_FOUND);
        let response = server.handle_line("{not json").unwrap();
        assert_eq!(response["error"]["code"], protocol::PARSE_ERROR);
    }

    #[test]
    fn tolerates_a_byte_order_mark() {
        let response = server()
            .handle_line("\u{feff}{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}")
            .unwrap();
        assert_eq!(response["result"], json!({}));
    }

    /// Every tool mcp.md documents, in the order `tools/list` offers them.
    const TOOLS: [&str; 16] = [
        "get_focus",
        "list_areas",
        "create_area",
        "update_area",
        "create_task",
        "update_task",
        "add_note",
        "list_tasks",
        "archive_tasks",
        "restore_task",
        "get_task",
        "delete_task",
        "list_events",
        "create_event",
        "update_event",
        "delete_event",
    ];

    // Regression: the focus tool was renamed from get_agenda to get_focus; the catalog and the
    // dispatcher must offer and answer the same names, and the old one no longer exists.
    #[test]
    fn offers_exactly_the_catalog_tools() {
        let server = server();
        let response = server
            .handle_line(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#)
            .unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, TOOLS);
        for tool in tools {
            assert!(
                tool["description"].as_str().is_some_and(|d| !d.is_empty()),
                "{tool}"
            );
            assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        }
        for name in TOOLS {
            let result = call(&server, 4, name, json!({}));
            assert!(
                !text(&result).contains("unknown tool"),
                "{name}: {}",
                text(&result)
            );
        }
        let old = call(&server, 5, "get_agenda", json!({}));
        assert_eq!(old["isError"], true);
        assert_eq!(text(&old), "Error: unknown tool \"get_agenda\"");
    }

    #[test]
    fn creates_updates_and_reads_tasks() {
        let server = server();
        call(&server, 1, "create_area", json!({ "name": "Work Café" }));
        let created = call(
            &server,
            2,
            "create_task",
            json!({ "title": "Fix Google login", "area": "work", "priority": "high",
                    "due_date": "2026-10-09", "requester": "Ana" }),
        );
        assert!(
            text(&created).starts_with("Created #1 [Work Café] Fix Google login"),
            "{}",
            text(&created)
        );

        let updated = call(
            &server,
            3,
            "update_task",
            json!({ "id": "#1", "status": "doing", "note": "repro'd" }),
        );
        assert!(text(&updated).contains("In progress"), "{}", text(&updated));

        let details = call(&server, 4, "get_task", json!({ "id": 1 }));
        assert!(text(&details).contains("repro'd"));
        assert!(text(&call(&server, 5, "get_focus", json!({}))).contains("Fix Google login"));
    }

    #[test]
    fn archives_done_tasks_out_of_the_list() {
        let server = server();
        call(
            &server,
            1,
            "create_task",
            json!({ "title": "Ship v1", "status": "done" }),
        );
        call(&server, 2, "create_task", json!({ "title": "Plan v2" }));
        let refused = call(&server, 3, "archive_tasks", json!({ "ids": [2] }));
        assert!(
            text(&refused).contains("only finished tasks"),
            "{}",
            text(&refused)
        );

        let archived = call(&server, 4, "archive_tasks", json!({ "all_done": true }));
        assert!(
            text(&archived).starts_with("Archived 1 task"),
            "{}",
            text(&archived)
        );
        let board = text(&call(&server, 5, "list_tasks", json!({ "include_done": true }))).to_owned();
        assert!(!board.contains("Ship v1") && board.contains("Plan v2"), "{board}");
        let archive = text(&call(&server, 6, "list_tasks", json!({ "archived": true }))).to_owned();
        assert!(
            archive.contains("Ship v1") && archive.contains("archived"),
            "{archive}"
        );

        call(&server, 7, "restore_task", json!({ "id": 1 }));
        let board = text(&call(&server, 8, "list_tasks", json!({ "include_done": true }))).to_owned();
        assert!(board.contains("Ship v1"), "{board}");
    }

    #[test]
    fn completing_a_recurring_task_schedules_the_next() {
        let server = server();
        let created = call(
            &server,
            1,
            "create_task",
            json!({ "title": "Send the weekly report", "due_date": "2099-01-02",
                    "recurrence": "toda sexta" }),
        );
        assert!(
            text(&created).contains("repeats every Friday"),
            "{}",
            text(&created)
        );

        let done = call(&server, 2, "update_task", json!({ "id": 1, "status": "done" }));
        assert!(
            text(&done)
                .contains("Next occurrence: #2 [No area] Send the weekly report — Medium · due 2099-01-09"),
            "{}",
            text(&done)
        );
        let details = text(&call(&server, 3, "get_task", json!({ "id": 2 }))).to_owned();
        assert!(details.contains("Repeats: every Friday"), "{details}");

        let stopped = call(
            &server,
            4,
            "update_task",
            json!({ "id": 2, "recurrence": "never" }),
        );
        assert!(!text(&stopped).contains("repeats"), "{}", text(&stopped));
        let refused = call(
            &server,
            5,
            "update_task",
            json!({ "id": 2, "recurrence": "sometimes" }),
        );
        assert_eq!(refused["isError"], true);
    }

    #[test]
    fn recolors_and_renames_areas() {
        let server = server();
        call(&server, 1, "create_area", json!({ "name": "Home" }));
        let updated = call(
            &server,
            2,
            "update_area",
            json!({ "area": "home", "color": "#39ff14" }),
        );
        assert_eq!(text(&updated), "Updated area Home (#39ff14)");
        let renamed = call(
            &server,
            3,
            "update_area",
            json!({ "area": "hom", "name": "House" }),
        );
        assert_eq!(text(&renamed), "Updated area House (#39ff14)");
        let bad = call(
            &server,
            4,
            "update_area",
            json!({ "area": "medi", "color": "green" }),
        );
        assert_eq!(bad["isError"], true);
    }

    #[test]
    fn calendar_tools_explain_when_nothing_is_connected() {
        let server = server();
        let result = call(&server, 1, "list_events", json!({}));
        assert_eq!(result["isError"], true);
        assert!(
            text(&result).contains("no calendars are connected"),
            "{}",
            text(&result)
        );
        let result = call(
            &server,
            2,
            "create_event",
            json!({ "title": "Call", "start": "2026-10-08T15:00" }),
        );
        assert!(text(&result).contains("Settings › Calendar"), "{}", text(&result));
        let result = call(&server, 3, "create_event", json!({ "start": "2026-10-08T15:00" }));
        assert!(text(&result).contains("invalid arguments"), "{}", text(&result));
    }

    #[test]
    fn reports_tool_errors_in_band() {
        let server = server();
        let result = call(
            &server,
            1,
            "create_task",
            json!({ "title": "X", "area": "nowhere" }),
        );
        assert_eq!(result["isError"], true);
        assert!(text(&result).contains("not found"));
        let result = call(&server, 2, "create_task", json!({}));
        assert_eq!(result["isError"], true);
    }

    /// Task #1 in Work with every field set; due in 2099 so the line does not depend on today.
    fn full_task(server: &Server) {
        ok(server, "create_area", json!({ "name": "Work" }));
        ok(
            server,
            "create_task",
            json!({ "title": "Send the report", "area": "Work", "description": "Numbers from Q3",
                    "priority": "high", "due_date": "2099-01-02", "requester": "Ana",
                    "status": "waiting", "recurrence": "every friday" }),
        );
    }

    #[test]
    fn empty_fields_clear_the_area_due_date_and_recurrence() {
        let server = server();
        full_task(&server);
        let updated = ok(
            &server,
            "update_task",
            json!({ "id": 1, "area": "", "due_date": "", "recurrence": "" }),
        );
        assert_eq!(
            updated,
            "Updated #1 [No area] Send the report — High · Waiting · requested by Ana"
        );
    }

    #[test]
    fn omitted_fields_are_left_alone() {
        let server = server();
        full_task(&server);
        let updated = ok(
            &server,
            "update_task",
            json!({ "id": 1, "title": "Send the Q3 report" }),
        );
        assert_eq!(
            updated,
            "Updated #1 [Work] Send the Q3 report — High · due 2099-01-02 (Jan 2, 2099) · repeats every \
             Friday · Waiting · requested by Ana"
        );
        assert!(ok(&server, "get_task", json!({ "id": 1 })).contains("Numbers from Q3"));
        // An empty title keeps the old one too.
        let updated = ok(&server, "update_task", json!({ "id": 1, "title": " " }));
        assert!(updated.contains("Send the Q3 report"), "{updated}");
    }

    #[test]
    fn update_task_appends_a_note() {
        let server = server();
        full_task(&server);
        let updated = ok(
            &server,
            "update_task",
            json!({ "id": 1, "note": "the numbers are late" }),
        );
        assert!(
            updated.ends_with("\nNote added to the activity log."),
            "{updated}"
        );
        let details = ok(&server, "get_task", json!({ "id": 1 }));
        assert!(details.contains("\nActivity:\n- "), "{details}");
        assert!(details.ends_with(" — the numbers are late"), "{details}");
        let blank = ok(&server, "update_task", json!({ "id": 1, "note": "  " }));
        assert!(!blank.contains("Note added"), "{blank}");
    }

    #[test]
    fn unknown_areas_list_the_registered_ones() {
        let server = server();
        assert_eq!(
            fails(&server, "create_task", json!({ "title": "X", "area": "work" })),
            "Error: area \"work\" not found (no areas registered yet)"
        );
        ok(&server, "create_area", json!({ "name": "Work" }));
        ok(&server, "create_area", json!({ "name": "Home" }));
        ok(&server, "create_task", json!({ "title": "X" }));
        let expected = "Error: area \"school\" not found; registered areas: Home, Work";
        for (tool, args) in [
            ("create_task", json!({ "title": "Y", "area": "school" })),
            ("update_task", json!({ "id": 1, "area": "school" })),
            ("list_tasks", json!({ "area": "school" })),
            ("get_focus", json!({ "area": "school" })),
            ("archive_tasks", json!({ "all_done": true, "area": "school" })),
            ("update_area", json!({ "area": "school", "name": "Study" })),
        ] {
            assert_eq!(fails(&server, tool, args), expected, "{tool}");
        }
    }

    #[test]
    fn ambiguous_areas_list_the_candidates() {
        let server = server();
        ok(&server, "create_area", json!({ "name": "Work Projects" }));
        ok(&server, "create_area", json!({ "name": "Work Admin" }));
        assert_eq!(
            fails(&server, "create_task", json!({ "title": "X", "area": "work" })),
            "Error: \"work\" matches more than one area: Work Admin, Work Projects"
        );
        let created = ok(&server, "create_task", json!({ "title": "X", "area": "admin" }));
        assert!(created.starts_with("Created #1 [Work Admin] X"), "{created}");
    }

    /// Two areas and seven tasks in every status; #6 is archived.
    fn board() -> Server {
        let server = server();
        ok(&server, "create_area", json!({ "name": "Work" }));
        ok(&server, "create_area", json!({ "name": "Home" }));
        for task in [
            json!({ "title": "Write the report", "area": "Work" }),
            json!({ "title": "Review the PR", "area": "Work", "status": "waiting", "priority": "high",
                    "due_date": "2099-01-02", "requester": "Bruno" }),
            json!({ "title": "Fix the sink", "area": "Home", "status": "in_progress", "priority": "low" }),
            json!({ "title": "Pay the rent", "area": "Home", "status": "done" }),
            json!({ "title": "Book flights", "area": "Home", "priority": "urgent" }),
            json!({ "title": "Old invoice", "area": "Work", "status": "done" }),
            json!({ "title": "Renew the passport", "area": "Work", "due_date": "2099-03-01" }),
        ] {
            ok(&server, "create_task", task);
        }
        ok(&server, "archive_tasks", json!({ "ids": [6] }));
        server
    }

    /// The ids of the task lines of a `list_tasks` reply, in order.
    fn listed(server: &Server, args: Value) -> Vec<i64> {
        ok(server, "list_tasks", args)
            .lines()
            .filter_map(|line| line.strip_prefix("- #"))
            .map(|line| line.split(' ').next().unwrap().parse().unwrap())
            .collect()
    }

    #[test]
    fn list_tasks_shows_open_tasks_by_column_then_urgency() {
        let server = board();
        let reply = ok(&server, "list_tasks", json!({}));
        let header = format!("5 task(s); today is {}", falog_core::date::today());
        assert_eq!(reply.lines().next(), Some(header.as_str()));
        assert_eq!(listed(&server, json!({})), [5, 7, 1, 3, 2]);
        assert_eq!(
            listed(&server, json!({ "include_done": true })),
            [5, 7, 1, 3, 2, 4]
        );
    }

    #[test]
    fn list_tasks_filters_by_status_area_and_search() {
        let server = board();
        assert_eq!(listed(&server, json!({ "status": "done" })), [4]);
        assert_eq!(listed(&server, json!({ "status": "em review" })), [2]);
        assert_eq!(
            listed(&server, json!({ "status": "todo", "area": "work" })),
            [7, 1]
        );
        assert_eq!(listed(&server, json!({ "area": "home" })), [5, 3]);
        assert_eq!(listed(&server, json!({ "search": "home" })), [5, 3]);
        assert_eq!(listed(&server, json!({ "search": "BRUNO" })), [2]);
        assert_eq!(listed(&server, json!({ "search": "#2" })), [2]);
        assert_eq!(listed(&server, json!({ "search": "rent" })), [] as [i64; 0]);
        assert_eq!(
            listed(&server, json!({ "search": "rent", "include_done": true })),
            [4]
        );
        assert_eq!(
            ok(&server, "list_tasks", json!({ "search": "nothing like it" })),
            "No tasks found."
        );
    }

    #[test]
    fn list_tasks_shows_archived_tasks_only_when_asked() {
        let server = board();
        assert_eq!(listed(&server, json!({ "archived": true })), [6]);
        let reply = ok(&server, "list_tasks", json!({ "archived": true }));
        assert!(
            reply.ends_with("- #6 [Work] Old invoice — Medium · Done · archived"),
            "{reply}"
        );
        assert!(!listed(&server, json!({ "status": "done" })).contains(&6));
    }

    #[test]
    fn list_tasks_limits_the_lines_and_counts_the_rest() {
        let server = board();
        let reply = ok(&server, "list_tasks", json!({ "limit": 2 }));
        let lines: Vec<&str> = reply.lines().collect();
        assert_eq!(lines.len(), 4, "{reply}");
        assert!(lines[0].starts_with("5 task(s)"), "{reply}");
        assert_eq!(lines[3], "... and 3 more");
        // At least one line is always shown.
        let reply = ok(&server, "list_tasks", json!({ "limit": 0 }));
        assert!(reply.ends_with("\n... and 4 more"), "{reply}");
        assert_eq!(listed(&server, json!({ "limit": 0 })), [5]);
        assert!(!ok(&server, "list_tasks", json!({ "limit": 5 })).contains("more"));
    }

    #[test]
    fn get_focus_covers_every_area_or_one() {
        let server = board();
        let today = falog_core::date::today();
        let focus = ok(&server, "get_focus", json!({}));
        assert!(
            focus.starts_with(&format!(
                "# Focus — {}\n\nOpen: 5 (0 overdue, 0 due today, 0 more due this week)\nBy area: \
                 Work: 3 · Home: 2\n",
                falog_core::date::format_long(today)
            )),
            "{focus}"
        );
        assert!(
            focus.contains(
                "\n## Backlog (2)\n- #5 [Home] Book flights — Urgent · To do\n- #1 [Work] Write the \
                 report — Medium · To do\n"
            ),
            "{focus}"
        );
        assert!(
            focus.ends_with(&format!(
                "\n## Completed in the last 7 days (1)\n- #4 [Home] Pay the rent — completed {today}\n"
            )),
            "{focus}"
        );

        let home = ok(&server, "get_focus", json!({ "area": "home" }));
        assert!(
            home.contains("Open: 2 (") && home.contains("By area: Home: 2\n"),
            "{home}"
        );
        assert!(
            home.contains("Book flights") && !home.contains("Write the report"),
            "{home}"
        );
        assert!(home.contains("Pay the rent"), "{home}");
    }

    #[test]
    fn add_note_confirms_with_the_task_line() {
        let server = board();
        assert_eq!(
            ok(
                &server,
                "add_note",
                json!({ "id": "#3", "note": " bought the part " })
            ),
            "Note added to #3 [Home] Fix the sink — Low · In progress"
        );
        assert!(ok(&server, "get_task", json!({ "id": 3 })).ends_with(" — bought the part"));
        assert_eq!(
            fails(&server, "add_note", json!({ "id": 3, "note": " " })),
            "Error: note cannot be empty"
        );
        assert_eq!(
            fails(&server, "add_note", json!({ "id": 99, "note": "x" })),
            "Error: task #99 not found"
        );
        let missing = fails(&server, "add_note", json!({ "id": 3 }));
        assert!(missing.starts_with("Error: invalid arguments: "), "{missing}");
    }

    #[test]
    fn delete_task_names_what_it_deleted() {
        let server = board();
        assert_eq!(
            ok(&server, "delete_task", json!({ "id": 1 })),
            "Deleted #1 [Work] Write the report"
        );
        assert_eq!(
            fails(&server, "delete_task", json!({ "id": 1 })),
            "Error: task #1 not found"
        );
        assert_eq!(
            fails(&server, "get_task", json!({ "id": 1 })),
            "Error: task #1 not found"
        );
    }

    #[test]
    fn archive_tasks_takes_every_done_task_of_an_area() {
        let server = board();
        ok(
            &server,
            "create_task",
            json!({ "title": "Close the sprint", "area": "Work", "status": "done" }),
        );
        assert_eq!(
            ok(
                &server,
                "archive_tasks",
                json!({ "all_done": true, "area": "work" })
            ),
            "Archived 1 task. It stays in the Archive view until restored."
        );
        assert_eq!(listed(&server, json!({ "archived": true })), [6, 8]);
        assert_eq!(listed(&server, json!({ "status": "done" })), [4]);
        assert_eq!(
            ok(&server, "archive_tasks", json!({ "all_done": true })),
            "Archived 1 task. It stays in the Archive view until restored."
        );
        assert_eq!(
            ok(&server, "archive_tasks", json!({ "all_done": true })),
            "Nothing to archive: no done tasks on the board."
        );
        assert_eq!(
            fails(&server, "archive_tasks", json!({})),
            "Error: pass the ids of the tasks to archive, or all_done=true"
        );
    }

    #[test]
    fn restore_task_puts_it_back_in_done() {
        let server = board();
        assert_eq!(
            ok(&server, "restore_task", json!({ "id": "#6" })),
            "Restored #6 [Work] Old invoice — Medium · Done"
        );
        assert_eq!(listed(&server, json!({ "status": "done" })), [4, 6]);
        assert_eq!(
            fails(&server, "restore_task", json!({ "id": 99 })),
            "Error: task #99 not found"
        );
        ok(&server, "restore_task", json!({ "id": 4 }));
        assert_eq!(
            ok(&server, "archive_tasks", json!({ "ids": [4, "#6"] })),
            "Archived 2 tasks. They stay in the Archive view until restored."
        );
    }
}
