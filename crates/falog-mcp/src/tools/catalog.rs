use serde_json::{Value, json};

/// Tool definitions returned by `tools/list`.
pub fn catalog() -> Value {
    let task_id = json!({
        "type": ["integer", "string"],
        "description": "Task id, e.g. 12 or \"#12\""
    });
    let status = json!({
        "type": "string",
        "enum": ["todo", "in_progress", "waiting", "done"],
        "description": "waiting = blocked on someone else (review, an answer, a deploy)"
    });
    let priority = json!({ "type": "string", "enum": ["low", "medium", "high", "urgent"] });

    json!([
        {
            "name": "get_focus",
            "description": "What needs attention: overdue, due today, due this week, in progress, waiting, upcoming, backlog, plus tasks completed in the last 7 days. Use it when the user wants to catch up or plan their day/week.",
            "inputSchema": { "type": "object", "properties": {
                "area": { "type": "string", "description": "Only this area" }
            }}
        },
        {
            "name": "list_areas",
            "description": "Registered areas with their number of open tasks.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "create_area",
            "description": "Registers a new area, a part of the user's life (Work, Personal, Health, Home...). Only when the user confirms it is new (not a nickname of an existing one).",
            "inputSchema": { "type": "object", "properties": {
                "name": { "type": "string" },
                "color": { "type": "string", "description": "Optional hex color, e.g. #74ade8" }
            }, "required": ["name"] }
        },
        {
            "name": "update_area",
            "description": "Renames an area or changes its color. Turn color names into hex yourself (neon green #39ff14, purple #b477cf, blue #74ade8...).",
            "inputSchema": { "type": "object", "properties": {
                "area": { "type": "string", "description": "Current area name or a unique part of it" },
                "name": { "type": "string", "description": "New name" },
                "color": { "type": "string", "description": "New hex color, e.g. #74ade8" }
            }, "required": ["area"] }
        },
        {
            "name": "create_task",
            "description": "Creates a task and returns it with its id.",
            "inputSchema": { "type": "object", "properties": {
                "title": { "type": "string", "description": "Short and actionable, starting with a verb, e.g. 'Fix Google login on Android'" },
                "area": { "type": "string", "description": "Area name or a unique part of it; must already exist. Personal errands and appointments go to a personal area" },
                "description": { "type": "string", "description": "Full context: what was asked, details, links, acceptance criteria, open questions. Keep everything that helps the user remember later." },
                "priority": priority,
                "due_date": { "type": "string", "description": "YYYY-MM-DD (also accepts 'today', 'tomorrow', 'friday', DD/MM)" },
                "requester": { "type": "string", "description": "Who asked for it" },
                "status": status
            }, "required": ["title"] }
        },
        {
            "name": "update_task",
            "description": "Changes only the given fields. Use status=done to complete a task. Can append a note to the activity log in the same call.",
            "inputSchema": { "type": "object", "properties": {
                "id": task_id,
                "title": { "type": "string" },
                "area": { "type": "string", "description": "New area; empty string removes it" },
                "description": { "type": "string", "description": "Replaces the whole description" },
                "priority": priority,
                "due_date": { "type": "string", "description": "YYYY-MM-DD; empty string removes the due date" },
                "requester": { "type": "string" },
                "status": status,
                "note": { "type": "string", "description": "Entry for the activity log, e.g. 'client changed the scope: ...'" }
            }, "required": ["id"] }
        },
        {
            "name": "add_note",
            "description": "Adds a timestamped note to a task's activity log (progress, decisions, new information) without touching the description.",
            "inputSchema": { "type": "object", "properties": {
                "id": task_id,
                "note": { "type": "string" }
            }, "required": ["id", "note"] }
        },
        {
            "name": "list_tasks",
            "description": "Lists or searches tasks. Open tasks only unless include_done or status=done. Archived tasks are left out unless archived=true, which lists only them.",
            "inputSchema": { "type": "object", "properties": {
                "area": { "type": "string" },
                "status": status,
                "search": { "type": "string", "description": "Matches title, description, requester or area" },
                "include_done": { "type": "boolean" },
                "archived": { "type": "boolean", "description": "List archived tasks instead of the ones on the board" },
                "limit": { "type": "integer", "description": "Default 100" }
            }}
        },
        {
            "name": "archive_tasks",
            "description": "Puts done tasks away from the board, keeping them (and their notes) in the Archive. Use it when the user wants to clean up finished work; only done tasks can be archived.",
            "inputSchema": { "type": "object", "properties": {
                "ids": { "type": "array", "items": task_id, "description": "Tasks to archive" },
                "all_done": { "type": "boolean", "description": "Archive every done task instead of ids" },
                "area": { "type": "string", "description": "With all_done: only this area" }
            }}
        },
        {
            "name": "restore_task",
            "description": "Takes an archived task back to the board's Done column.",
            "inputSchema": { "type": "object", "properties": { "id": task_id }, "required": ["id"] }
        },
        {
            "name": "get_task",
            "description": "All details of a task, including its description and activity log.",
            "inputSchema": { "type": "object", "properties": { "id": task_id }, "required": ["id"] }
        },
        {
            "name": "delete_task",
            "description": "Permanently deletes a task. Prefer status=done for finished work; delete only when the user asks (e.g. created by mistake).",
            "inputSchema": { "type": "object", "properties": { "id": task_id }, "required": ["id"] }
        },
        {
            "name": "list_events",
            "description": "Meetings and other events from the user's Google calendars for a day or a range (default: today), with their area, join link, and the calendar and id that update_event and delete_event need. Use it with get_focus when the user catches up, and before booking to check they are free.",
            "inputSchema": { "type": "object", "properties": {
                "from": { "type": "string", "description": "First day, YYYY-MM-DD (default today)" },
                "to": { "type": "string", "description": "Last day, inclusive, YYYY-MM-DD (default: same as from)" },
                "area": { "type": "string", "description": "Only the calendars of this area" }
            }}
        },
        {
            "name": "create_event",
            "description": "Creates an event in a Google calendar. The account comes from the area (each area is linked to its email in Falog); pass calendar (an email) when the area has several or none. Times are local, YYYY-MM-DDTHH:MM.",
            "inputSchema": { "type": "object", "properties": {
                "title": { "type": "string" },
                "start": { "type": "string", "description": "YYYY-MM-DDTHH:MM, or YYYY-MM-DD with all_day" },
                "end": { "type": "string", "description": "YYYY-MM-DDTHH:MM (or the last day with all_day); default start + duration_minutes" },
                "duration_minutes": { "type": "integer", "description": "Used when end is missing; default 30" },
                "all_day": { "type": "boolean" },
                "area": { "type": "string", "description": "Area name or a unique part of it" },
                "calendar": { "type": "string", "description": "Account email (or part of it) to create the event in" },
                "description": { "type": "string" },
                "location": { "type": "string" },
                "attendees": { "type": "array", "items": { "type": "string" }, "description": "Guests' emails; Google sends them the invitation" },
                "meet": { "type": "boolean", "description": "Attach a Google Meet link" }
            }, "required": ["title", "start"] }
        },
        {
            "name": "update_event",
            "description": "Changes an event found with list_events: pass its calendar and id, and only the fields to change. When moving it, pass start and end together.",
            "inputSchema": { "type": "object", "properties": {
                "calendar": { "type": "string", "description": "The event's calendar, as list_events shows it" },
                "id": { "type": "string" },
                "title": { "type": "string" },
                "start": { "type": "string", "description": "YYYY-MM-DDTHH:MM" },
                "end": { "type": "string", "description": "YYYY-MM-DDTHH:MM" },
                "description": { "type": "string" },
                "location": { "type": "string" }
            }, "required": ["calendar", "id"] }
        },
        {
            "name": "delete_event",
            "description": "Deletes an event found with list_events (Google tells the guests). Only when the user explicitly asks to cancel or delete it.",
            "inputSchema": { "type": "object", "properties": {
                "calendar": { "type": "string" },
                "id": { "type": "string" }
            }, "required": ["calendar", "id"] }
        }
    ])
}
