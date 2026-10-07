//! Plain-text renderings of tasks for tool results.

use chrono::NaiveDate;
use falog_core::agenda;
use falog_core::date;
use falog_core::domain::{Note, Task};

/// `#12 [Work] Fix login — High · due 2026-10-09 (Fri Oct 9) · In progress · requested by Ana`
pub fn task_line(task: &Task, today: NaiveDate) -> String {
    let mut facts = vec![task.priority.label().to_string()];
    if let Some(due) = task.due {
        facts.push(format!("due {due} ({})", date::describe_due(due, today)));
    }
    facts.push(task.status.label().to_string());
    if !task.requester.is_empty() {
        facts.push(format!("requested by {}", task.requester));
    }
    format!(
        "#{} [{}] {} — {}",
        task.id,
        task.area_name(),
        task.title,
        facts.join(" · ")
    )
}

pub fn task_details(task: &Task, notes: &[Note], today: NaiveDate) -> String {
    let mut out = vec![
        format!("#{} {}", task.id, task.title),
        format!(
            "Area: {} | Status: {} | Priority: {}",
            task.area_name(),
            task.status,
            task.priority
        ),
    ];
    if let Some(due) = task.due {
        out.push(format!("Due: {due} ({})", date::describe_due(due, today)));
    }
    if !task.requester.is_empty() {
        out.push(format!("Requested by: {}", task.requester));
    }
    let mut timestamps = format!("Created: {} | Updated: {}", task.created_at, task.updated_at);
    if let Some(done) = task.completed_at {
        timestamps.push_str(&format!(" | Completed: {done}"));
    }
    out.push(timestamps);
    if !task.description.is_empty() {
        out.push(format!("\nDescription:\n{}", task.description));
    }
    if !notes.is_empty() {
        out.push("\nActivity:".into());
        out.extend(notes.iter().map(|n| format!("- {} — {}", n.created_at, n.body)));
    }
    out.join("\n")
}

/// Markdown agenda: headline numbers, open tasks per area, then each non-empty bucket.
pub fn agenda(tasks: &[Task], today: NaiveDate) -> String {
    let summary = agenda::summary(tasks, today);
    let mut out = format!(
        "# Agenda — {}\n\nOpen: {} ({} overdue, {} due today, {} more due this week)\n",
        date::format_long(today),
        summary.open,
        summary.overdue,
        summary.due_today,
        summary.due_this_week,
    );

    let mut per_area: Vec<(&str, usize)> = Vec::new();
    for task in tasks.iter().filter(|t| t.is_open()) {
        match per_area.iter_mut().find(|(name, _)| *name == task.area_name()) {
            Some((_, count)) => *count += 1,
            None => per_area.push((task.area_name(), 1)),
        }
    }
    per_area.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    if !per_area.is_empty() {
        let parts: Vec<String> = per_area.iter().map(|(name, n)| format!("{name}: {n}")).collect();
        out.push_str(&format!("By area: {}\n", parts.join(" · ")));
    }

    for section in agenda::sections(tasks, today)
        .iter()
        .filter(|s| !s.tasks.is_empty())
    {
        out.push_str(&format!(
            "\n## {} ({})\n",
            section.bucket.title(),
            section.tasks.len()
        ));
        for task in &section.tasks {
            out.push_str(&format!("- {}\n", task_line(task, today)));
        }
    }

    let done = agenda::recently_completed(tasks, today, 7);
    if !done.is_empty() {
        out.push_str(&format!("\n## Completed in the last 7 days ({})\n", done.len()));
        for task in done {
            let on = task.completed_on().map(|d| d.to_string()).unwrap_or_default();
            out.push_str(&format!(
                "- #{} [{}] {} — completed {on}\n",
                task.id,
                task.area_name(),
                task.title
            ));
        }
    }
    if summary.open == 0 {
        out.push_str("\nNothing open.\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use falog_core::Store;
    use falog_core::domain::NewTask;

    #[test]
    fn agenda_lists_open_buckets() {
        let store = Store::open_in_memory().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        store
            .create_task(&NewTask {
                due: Some(today),
                ..NewTask::new("Ship hotfix")
            })
            .unwrap();
        let text = agenda(&store.tasks().unwrap(), today);
        assert!(text.contains("## Due today (1)"), "{text}");
        assert!(text.contains("Ship hotfix"));
        assert!(!text.contains(agenda::Bucket::Overdue.title()));
    }
}
