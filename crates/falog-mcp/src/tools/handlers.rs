use super::args::*;
use super::calendar::{self, CalendarAccess};
use crate::format;
use anyhow::{Context, Result, anyhow, bail};
use chrono::NaiveDate;
use falog_core::domain::{NewTask, Priority, Rgb, Status, Task, TaskId, TaskPatch};
use falog_core::{Store, date, focus, text::fold};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Runs a tool and returns the text shown to the model.
pub fn call(store: &Store, access: &CalendarAccess, name: &str, args: Value) -> Result<String> {
    match name {
        "list_events" => calendar::list_events(store, access, parse(args)?),
        "create_event" => calendar::create_event(store, access, parse(args)?),
        "update_event" => calendar::update_event(access, parse(args)?),
        "delete_event" => calendar::delete_event(access, parse(args)?),
        "get_focus" => get_focus(store, parse(args)?),
        "list_areas" => list_areas(store),
        "create_area" => create_area(store, parse(args)?),
        "update_area" => update_area(store, parse(args)?),
        "create_task" => create_task(store, parse(args)?),
        "update_task" => update_task(store, parse(args)?),
        "add_note" => add_note(store, parse(args)?),
        "list_tasks" => list_tasks(store, parse(args)?),
        "get_task" => get_task(store, parse(args)?),
        "delete_task" => delete_task(store, parse(args)?),
        "archive_tasks" => archive_tasks(store, parse(args)?),
        "restore_task" => restore_task(store, parse(args)?),
        _ => bail!("unknown tool \"{name}\""),
    }
}

fn parse<T: DeserializeOwned>(args: Value) -> Result<T> {
    let args = if args.is_null() {
        Value::Object(Default::default())
    } else {
        args
    };
    serde_json::from_value(args).context("invalid arguments")
}

fn get_focus(store: &Store, args: FocusArgs) -> Result<String> {
    let tasks = tasks_of(store, args.area.as_deref())?;
    Ok(format::focus(&tasks, date::today()))
}

fn list_areas(store: &Store) -> Result<String> {
    let areas = store.areas()?;
    if areas.is_empty() {
        return Ok(
            "No areas yet. Ask the user what to track (Work, Personal, Health, Home...) and call create_area.".into(),
        );
    }
    let tasks = store.tasks()?;
    let lines: Vec<String> = areas
        .iter()
        .map(|a| {
            let open = tasks
                .iter()
                .filter(|t| t.is_open() && t.area_id() == Some(a.id))
                .count();
            format!("- {} ({open} open)", a.name)
        })
        .collect();
    Ok(lines.join("\n"))
}

fn create_area(store: &Store, args: CreateAreaArgs) -> Result<String> {
    let color = args.color.as_deref().map(str::parse::<Rgb>).transpose()?;
    let area = store.create_area(&args.name, color)?;
    Ok(format!("Created area {} ({})", area.name, area.color))
}

fn update_area(store: &Store, args: UpdateAreaArgs) -> Result<String> {
    let area = store.find_area(&args.area)?;
    let name = non_empty(args.name).unwrap_or(area.name);
    let color = match non_empty(args.color) {
        Some(color) => color.parse::<Rgb>()?,
        None => area.color,
    };
    let area = store.update_area(area.id, &name, color)?;
    Ok(format!("Updated area {} ({})", area.name, area.color))
}

fn create_task(store: &Store, args: CreateTaskArgs) -> Result<String> {
    let area_id = match non_empty(args.area) {
        Some(name) => Some(store.find_area(&name)?.id),
        None => None,
    };
    let task = store.create_task(&NewTask {
        title: args.title,
        description: args.description,
        area_id,
        status: parse_status(args.status)?.unwrap_or_default(),
        priority: parse_priority(args.priority)?.unwrap_or_default(),
        due: parse_due(args.due_date)?.flatten(),
        requester: args.requester,
    })?;
    Ok(format!("Created {}", format::task_line(&task, date::today())))
}

fn update_task(store: &Store, args: UpdateTaskArgs) -> Result<String> {
    let id = args.id.0;
    let area_id = match args.area {
        None => None,
        Some(name) if name.trim().is_empty() => Some(None),
        Some(name) => Some(Some(store.find_area(&name)?.id)),
    };
    let patch = TaskPatch {
        title: non_empty(args.title),
        description: args.description,
        area_id,
        status: parse_status(args.status)?,
        priority: parse_priority(args.priority)?,
        due: parse_due(args.due_date)?,
        requester: args.requester,
    };
    let task = store.update_task(id, &patch)?;
    let mut reply = format!("Updated {}", format::task_line(&task, date::today()));
    if let Some(note) = non_empty(args.note) {
        store.add_note(id, &note)?;
        reply.push_str("\nNote added to the activity log.");
    }
    Ok(reply)
}

fn add_note(store: &Store, args: AddNoteArgs) -> Result<String> {
    store.add_note(args.id.0, &args.note)?;
    let task = store.require_task(args.id.0)?;
    Ok(format!(
        "Note added to {}",
        format::task_line(&task, date::today())
    ))
}

fn list_tasks(store: &Store, args: ListTasksArgs) -> Result<String> {
    let mut tasks = tasks_of(store, args.area.as_deref())?;
    tasks.retain(|t| t.is_archived() == args.archived);
    match parse_status(args.status)? {
        Some(status) => tasks.retain(|t| t.status == status),
        None if !args.include_done && !args.archived => tasks.retain(Task::is_open),
        None => {}
    }
    if let Some(query) = non_empty(args.search) {
        tasks.retain(|t| t.matches(&query));
    }
    if tasks.is_empty() {
        return Ok("No tasks found.".into());
    }
    tasks.sort_by(|a, b| {
        let column = |t: &Task| Status::ALL.iter().position(|s| *s == t.status);
        column(a).cmp(&column(b)).then_with(|| focus::by_urgency(a, b))
    });

    let today = date::today();
    let limit = args.limit.unwrap_or(100).max(1);
    let mut lines = vec![format!("{} task(s); today is {today}", tasks.len())];
    lines.extend(
        tasks
            .iter()
            .take(limit)
            .map(|t| format!("- {}", format::task_line(t, today))),
    );
    if tasks.len() > limit {
        lines.push(format!("... and {} more", tasks.len() - limit));
    }
    Ok(lines.join("\n"))
}

fn get_task(store: &Store, args: TaskArgs) -> Result<String> {
    let task = store.require_task(args.id.0)?;
    let notes = store.notes(task.id)?;
    Ok(format::task_details(&task, &notes, date::today()))
}

fn delete_task(store: &Store, args: TaskArgs) -> Result<String> {
    let task = store.delete_task(args.id.0)?;
    Ok(format!(
        "Deleted #{} [{}] {}",
        task.id,
        task.area_name(),
        task.title
    ))
}

fn archive_tasks(store: &Store, args: ArchiveTasksArgs) -> Result<String> {
    let ids: Vec<TaskId> = if args.all_done {
        tasks_of(store, args.area.as_deref())?
            .iter()
            .filter(|t| t.status == Status::Done && !t.is_archived())
            .map(|t| t.id)
            .collect()
    } else if args.ids.is_empty() {
        bail!("pass the ids of the tasks to archive, or all_done=true");
    } else {
        for id in &args.ids {
            let task = store.require_task(id.0)?;
            if task.status != Status::Done {
                bail!("#{} is not done; only finished tasks can be archived", task.id);
            }
        }
        args.ids.iter().map(|id| id.0).collect()
    };
    let archived = store.archive_tasks(&ids)?;
    Ok(match archived {
        0 => "Nothing to archive: no done tasks on the board.".into(),
        1 => "Archived 1 task. It stays in the Archive view until restored.".into(),
        n => format!("Archived {n} tasks. They stay in the Archive view until restored."),
    })
}

fn restore_task(store: &Store, args: TaskArgs) -> Result<String> {
    let task = store.restore_task(args.id.0)?;
    Ok(format!("Restored {}", format::task_line(&task, date::today())))
}

/// All tasks, or only those of the area matching `area`.
fn tasks_of(store: &Store, area: Option<&str>) -> Result<Vec<Task>> {
    let mut tasks = store.tasks()?;
    if let Some(name) = area.filter(|a| !a.trim().is_empty()) {
        let area = store.find_area(name)?;
        tasks.retain(|t| t.area_id() == Some(area.id));
    }
    Ok(tasks)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

fn parse_status(value: Option<String>) -> Result<Option<Status>> {
    Ok(non_empty(value).map(|v| v.parse()).transpose()?)
}

fn parse_priority(value: Option<String>) -> Result<Option<Priority>> {
    Ok(non_empty(value).map(|v| v.parse()).transpose()?)
}

/// `None` = not given, `Some(None)` = clear the due date.
fn parse_due(value: Option<String>) -> Result<Option<Option<NaiveDate>>> {
    let Some(raw) = value else { return Ok(None) };
    if matches!(
        fold(&raw).as_str(),
        "" | "none" | "null" | "no due date" | "sem prazo"
    ) {
        return Ok(Some(None));
    }
    date::parse_due(&raw, date::today())
        .map(|due| Some(Some(due)))
        .ok_or_else(|| anyhow!("invalid due date \"{raw}\"; use YYYY-MM-DD"))
}
