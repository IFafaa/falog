use super::args::*;
use crate::format;
use anyhow::{Context, Result, anyhow, bail};
use chrono::NaiveDate;
use falog_core::domain::{NewTask, Priority, Rgb, Status, Task, TaskPatch};
use falog_core::{Store, agenda, date, text::fold};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Runs a tool and returns the text shown to the model.
pub fn call(store: &Store, name: &str, args: Value) -> Result<String> {
    match name {
        "get_agenda" => get_agenda(store, parse(args)?),
        "list_companies" => list_companies(store),
        "create_company" => create_company(store, parse(args)?),
        "create_task" => create_task(store, parse(args)?),
        "update_task" => update_task(store, parse(args)?),
        "add_note" => add_note(store, parse(args)?),
        "list_tasks" => list_tasks(store, parse(args)?),
        "get_task" => get_task(store, parse(args)?),
        "delete_task" => delete_task(store, parse(args)?),
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

fn get_agenda(store: &Store, args: AgendaArgs) -> Result<String> {
    let tasks = tasks_of(store, args.company.as_deref())?;
    Ok(format::agenda(&tasks, date::today()))
}

fn list_companies(store: &Store) -> Result<String> {
    let companies = store.companies()?;
    if companies.is_empty() {
        return Ok(
            "No companies registered yet. Ask the user for their names and call create_company.".into(),
        );
    }
    let tasks = store.tasks()?;
    let lines: Vec<String> = companies
        .iter()
        .map(|c| {
            let open = tasks
                .iter()
                .filter(|t| t.is_open() && t.company_id() == Some(c.id))
                .count();
            format!("- {} ({open} open)", c.name)
        })
        .collect();
    Ok(lines.join("\n"))
}

fn create_company(store: &Store, args: CreateCompanyArgs) -> Result<String> {
    let color = args.color.as_deref().map(str::parse::<Rgb>).transpose()?;
    let company = store.create_company(&args.name, color)?;
    Ok(format!("Created company {} ({})", company.name, company.color))
}

fn create_task(store: &Store, args: CreateTaskArgs) -> Result<String> {
    let company_id = match non_empty(args.company) {
        Some(name) => Some(store.find_company(&name)?.id),
        None => None,
    };
    let task = store.create_task(&NewTask {
        title: args.title,
        description: args.description,
        company_id,
        status: parse_status(args.status)?.unwrap_or_default(),
        priority: parse_priority(args.priority)?.unwrap_or_default(),
        due: parse_due(args.due_date)?.flatten(),
        requester: args.requester,
    })?;
    Ok(format!("Created {}", format::task_line(&task, date::today())))
}

fn update_task(store: &Store, args: UpdateTaskArgs) -> Result<String> {
    let id = args.id.0;
    let company_id = match args.company {
        None => None,
        Some(name) if name.trim().is_empty() => Some(None),
        Some(name) => Some(Some(store.find_company(&name)?.id)),
    };
    let patch = TaskPatch {
        title: non_empty(args.title),
        description: args.description,
        company_id,
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
    let mut tasks = tasks_of(store, args.company.as_deref())?;
    match parse_status(args.status)? {
        Some(status) => tasks.retain(|t| t.status == status),
        None if !args.include_done => tasks.retain(Task::is_open),
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
        column(a).cmp(&column(b)).then_with(|| agenda::by_urgency(a, b))
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
        task.company_name(),
        task.title
    ))
}

/// All tasks, or only those of the company matching `company`.
fn tasks_of(store: &Store, company: Option<&str>) -> Result<Vec<Task>> {
    let mut tasks = store.tasks()?;
    if let Some(name) = company.filter(|c| !c.trim().is_empty()) {
        let company = store.find_company(name)?;
        tasks.retain(|t| t.company_id() == Some(company.id));
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
