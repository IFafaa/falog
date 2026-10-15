//! Plain-text renderings of tasks for tool results.

use chrono::NaiveDate;
use falog_core::date;
use falog_core::domain::{Note, Task};
use falog_core::focus;

/// `#12 [Work] Fix login — High · due 2026-10-09 (Fri Oct 9) · repeats every Friday · In progress ·
/// requested by Ana`
pub fn task_line(task: &Task, today: NaiveDate) -> String {
    let mut facts = vec![task.priority.label().to_string()];
    if let Some(due) = task.due {
        facts.push(format!("due {due} ({})", date::describe_due(due, today)));
    }
    if let Some(rule) = task.recurrence {
        facts.push(format!("repeats {rule}"));
    }
    facts.push(task.status.label().to_string());
    if task.is_archived() {
        facts.push("archived".into());
    }
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
    if let Some(rule) = task.recurrence {
        out.push(format!("Repeats: {rule}"));
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

/// Markdown focus summary: headline numbers, open tasks per area, then each non-empty bucket.
pub fn focus(tasks: &[Task], today: NaiveDate) -> String {
    let summary = focus::summary(tasks, today);
    let mut out = format!(
        "# Focus — {}\n\nOpen: {} ({} overdue, {} due today, {} more due this week)\n",
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

    for section in focus::sections(tasks, today)
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

    let done = focus::recently_completed(tasks, today, 7);
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
    use chrono::Weekday;
    use falog_core::Store;
    use falog_core::domain::{
        Area, AreaId, NewTask, NoteId, Priority, Recurrence, Rgb, Status, TaskId, Weekdays,
    };

    #[test]
    fn focus_lists_open_buckets() {
        let store = Store::open_in_memory().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        store
            .create_task(&NewTask {
                due: Some(today),
                ..NewTask::new("Ship hotfix")
            })
            .unwrap();
        let text = focus(&store.tasks().unwrap(), today);
        assert!(text.contains("## Due today (1)"), "{text}");
        assert!(text.contains("Ship hotfix"));
        assert!(!text.contains(focus::Bucket::Overdue.title()));
    }

    fn tuesday() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 6).unwrap()
    }

    /// A task with only the required facts: no area, due date, rule or requester.
    fn bare_task() -> Task {
        let at = tuesday().and_hms_opt(9, 0, 0).unwrap();
        Task {
            id: TaskId(3),
            title: "Call the bank".into(),
            description: String::new(),
            area: None,
            status: Status::Todo,
            priority: Priority::Medium,
            due: None,
            requester: String::new(),
            note_count: 0,
            created_at: at,
            updated_at: at,
            completed_at: None,
            archived_at: None,
            recurrence: None,
        }
    }

    #[test]
    fn task_line_with_only_the_required_facts() {
        assert_eq!(
            task_line(&bare_task(), tuesday()),
            "#3 [No area] Call the bank — Medium · To do"
        );
    }

    #[test]
    fn task_line_with_every_fact() {
        let done_at = tuesday().and_hms_opt(18, 0, 0);
        let task = Task {
            id: TaskId(12),
            title: "Fix login".into(),
            area: Some(Area {
                id: AreaId(1),
                name: "Work".into(),
                color: Rgb::new(0x74, 0xad, 0xe8),
            }),
            status: Status::Done,
            priority: Priority::High,
            due: NaiveDate::from_ymd_opt(2026, 10, 9),
            requester: "Ana".into(),
            completed_at: done_at,
            archived_at: done_at,
            recurrence: Some(Recurrence::Weekly(Weekdays::single(Weekday::Fri))),
            ..bare_task()
        };
        assert_eq!(
            task_line(&task, tuesday()),
            "#12 [Work] Fix login — High · due 2026-10-09 (Fri Oct 9) · repeats every Friday · Done · \
             archived · requested by Ana"
        );
    }

    #[test]
    fn task_line_describes_the_due_date_relative_to_today() {
        let task = Task {
            due: NaiveDate::from_ymd_opt(2026, 10, 1),
            status: Status::Waiting,
            ..bare_task()
        };
        assert_eq!(
            task_line(&task, tuesday()),
            "#3 [No area] Call the bank — Medium · due 2026-10-01 (5d overdue) · Waiting"
        );
    }

    #[test]
    fn details_show_every_field_and_the_activity_log() {
        let task = Task {
            description: "Ask about the fee".into(),
            due: Some(tuesday()),
            requester: "Ana".into(),
            recurrence: Some(Recurrence::Monthly(5)),
            ..bare_task()
        };
        let note = Note {
            id: NoteId(1),
            task_id: task.id,
            body: "they close at 4".into(),
            created_at: task.created_at,
        };
        assert_eq!(
            task_details(&task, &[note], tuesday()),
            "#3 Call the bank\n\
             Area: No area | Status: To do | Priority: Medium\n\
             Due: 2026-10-06 (today)\n\
             Repeats: monthly on day 5\n\
             Requested by: Ana\n\
             Created: 2026-10-06 09:00:00 | Updated: 2026-10-06 09:00:00\n\
             \nDescription:\nAsk about the fee\n\
             \nActivity:\n\
             - 2026-10-06 09:00:00 — they close at 4"
        );
    }

    #[test]
    fn focus_without_open_tasks_says_so() {
        let text = focus(&[], tuesday());
        assert_eq!(
            text,
            "# Focus — Tuesday, October 6, 2026\n\n\
             Open: 0 (0 overdue, 0 due today, 0 more due this week)\n\
             \nNothing open.\n"
        );
    }
}
