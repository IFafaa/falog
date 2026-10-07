//! Groups open tasks by how urgently they need attention.
//!
//! This powers the desktop "Agenda" view and the MCP `get_agenda` tool, which is how the user
//! gets back up to speed on Monday morning.

use crate::date::end_of_week;
use crate::domain::{Status, Task};
use chrono::{Days, NaiveDate};
use std::cmp::Ordering;

/// Each open task falls into exactly one bucket, checked in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bucket {
    Overdue,
    DueToday,
    DueThisWeek,
    InProgress,
    Waiting,
    Upcoming,
    Backlog,
}

impl Bucket {
    pub const ALL: [Self; 7] = [
        Self::Overdue,
        Self::DueToday,
        Self::DueThisWeek,
        Self::InProgress,
        Self::Waiting,
        Self::Upcoming,
        Self::Backlog,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Overdue => "Overdue",
            Self::DueToday => "Due today",
            Self::DueThisWeek => "Due this week",
            Self::InProgress => "In progress",
            Self::Waiting => "Waiting on others",
            Self::Upcoming => "Upcoming",
            Self::Backlog => "Backlog",
        }
    }

    /// The bucket of an open task, or `None` for completed tasks.
    pub fn of(task: &Task, today: NaiveDate) -> Option<Self> {
        if !task.is_open() {
            return None;
        }
        let bucket = match task.due {
            Some(due) if due < today => Self::Overdue,
            Some(due) if due == today => Self::DueToday,
            Some(due) if due <= end_of_week(today) => Self::DueThisWeek,
            _ if task.status == Status::InProgress => Self::InProgress,
            _ if task.status == Status::Waiting => Self::Waiting,
            Some(_) => Self::Upcoming,
            None => Self::Backlog,
        };
        Some(bucket)
    }
}

#[derive(Debug)]
pub struct Section<'a> {
    pub bucket: Bucket,
    pub tasks: Vec<&'a Task>,
}

/// All buckets in order (possibly empty), each sorted for the order work should happen.
pub fn sections(tasks: &[Task], today: NaiveDate) -> Vec<Section<'_>> {
    let mut sections: Vec<Section<'_>> = Bucket::ALL
        .iter()
        .map(|&bucket| Section {
            bucket,
            tasks: Vec::new(),
        })
        .collect();
    for task in tasks {
        if let Some(bucket) = Bucket::of(task, today) {
            let index = Bucket::ALL.iter().position(|b| *b == bucket).unwrap_or_default();
            sections[index].tasks.push(task);
        }
    }
    for section in &mut sections {
        if section.bucket == Bucket::Overdue {
            section
                .tasks
                .sort_by(|a, b| cmp_due(a.due, b.due).then_with(|| by_urgency(a, b)));
        } else {
            section.tasks.sort_by(|a, b| by_urgency(a, b));
        }
    }
    sections
}

/// Tasks completed in the last `days` days, most recent first.
pub fn recently_completed(tasks: &[Task], today: NaiveDate, days: u64) -> Vec<&Task> {
    let since = today - Days::new(days);
    let mut done: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.completed_on().is_some_and(|d| d >= since))
        .collect();
    done.sort_by_key(|t| std::cmp::Reverse(t.completed_at));
    done
}

/// Headline numbers for a set of tasks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub open: usize,
    pub overdue: usize,
    pub due_today: usize,
    pub due_this_week: usize,
    pub completed_last_7_days: usize,
}

pub fn summary(tasks: &[Task], today: NaiveDate) -> Summary {
    let mut summary = Summary {
        completed_last_7_days: recently_completed(tasks, today, 7).len(),
        ..Summary::default()
    };
    for task in tasks.iter().filter(|t| t.is_open()) {
        summary.open += 1;
        match Bucket::of(task, today) {
            Some(Bucket::Overdue) => summary.overdue += 1,
            Some(Bucket::DueToday) => summary.due_today += 1,
            Some(Bucket::DueThisWeek) => summary.due_this_week += 1,
            _ => {}
        }
    }
    summary
}

/// Earlier due dates first; tasks without a due date last.
pub fn cmp_due(a: Option<NaiveDate>, b: Option<NaiveDate>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Highest priority first, then nearest due date, then oldest.
pub fn by_urgency(a: &Task, b: &Task) -> Ordering {
    b.priority
        .cmp(&a.priority)
        .then_with(|| cmp_due(a.due, b.due))
        .then(a.id.cmp(&b.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Priority, TaskId};
    use chrono::NaiveDateTime;

    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, n).unwrap()
    }

    fn task(id: i64, status: Status, due: Option<NaiveDate>, priority: Priority) -> Task {
        let at: NaiveDateTime = day(1).and_hms_opt(9, 0, 0).unwrap();
        Task {
            id: TaskId(id),
            title: format!("task {id}"),
            description: String::new(),
            area: None,
            status,
            priority,
            due,
            requester: String::new(),
            note_count: 0,
            created_at: at,
            updated_at: at,
            completed_at: (status == Status::Done).then_some(day(5).and_hms_opt(18, 0, 0).unwrap()),
        }
    }

    #[test]
    fn buckets_open_tasks_by_urgency() {
        let today = day(6);
        let tasks = vec![
            task(1, Status::Todo, Some(day(2)), Priority::Medium),
            task(2, Status::InProgress, Some(day(6)), Priority::Medium),
            task(3, Status::Todo, Some(day(9)), Priority::Medium),
            task(4, Status::InProgress, None, Priority::Medium),
            task(5, Status::Waiting, Some(day(30)), Priority::Medium),
            task(6, Status::Todo, Some(day(30)), Priority::Medium),
            task(7, Status::Todo, None, Priority::Medium),
            task(8, Status::Done, Some(day(2)), Priority::Medium),
        ];
        let ids: Vec<Vec<i64>> = sections(&tasks, today)
            .iter()
            .map(|s| s.tasks.iter().map(|t| t.id.0).collect())
            .collect();
        assert_eq!(
            ids,
            vec![vec![1], vec![2], vec![3], vec![4], vec![5], vec![6], vec![7]]
        );
    }

    #[test]
    fn sorts_by_priority_then_due_date() {
        let today = day(6);
        let tasks = vec![
            task(1, Status::Todo, None, Priority::Low),
            task(2, Status::Todo, None, Priority::Urgent),
            task(3, Status::Todo, None, Priority::High),
        ];
        let backlog = &sections(&tasks, today)[6];
        let ids: Vec<i64> = backlog.tasks.iter().map(|t| t.id.0).collect();
        assert_eq!(ids, vec![2, 3, 1]);
    }

    #[test]
    fn summarizes_counts() {
        let today = day(6);
        let tasks = vec![
            task(1, Status::Todo, Some(day(2)), Priority::Medium),
            task(2, Status::Todo, Some(day(6)), Priority::Medium),
            task(3, Status::Done, None, Priority::Medium),
        ];
        let summary = summary(&tasks, today);
        assert_eq!(summary.open, 2);
        assert_eq!(summary.overdue, 1);
        assert_eq!(summary.due_today, 1);
        assert_eq!(summary.completed_last_7_days, 1);
    }
}
