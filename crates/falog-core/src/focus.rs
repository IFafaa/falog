//! Groups open tasks by how urgently they need attention.
//!
//! This powers the desktop "Focus" view and the MCP `get_focus` tool, which is how the user
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
            archived_at: None,
            recurrence: None,
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

    fn ids(tasks: &[&Task]) -> Vec<i64> {
        tasks.iter().map(|t| t.id.0).collect()
    }

    fn completed(id: i64, on: NaiveDate) -> Task {
        Task {
            completed_at: on.and_hms_opt(12, 0, 0),
            ..task(id, Status::Done, None, Priority::Medium)
        }
    }

    #[test]
    fn checks_the_buckets_in_order() {
        // Tuesday; the week ends on Sunday the 11th.
        let today = day(6);
        let cases = [
            (Status::InProgress, Some(day(5)), Some(Bucket::Overdue)),
            (Status::Waiting, Some(day(5)), Some(Bucket::Overdue)),
            (Status::InProgress, Some(day(6)), Some(Bucket::DueToday)),
            (Status::Waiting, Some(day(6)), Some(Bucket::DueToday)),
            (Status::InProgress, Some(day(11)), Some(Bucket::DueThisWeek)),
            (Status::Waiting, Some(day(11)), Some(Bucket::DueThisWeek)),
            (Status::InProgress, Some(day(12)), Some(Bucket::InProgress)),
            (Status::InProgress, None, Some(Bucket::InProgress)),
            (Status::Waiting, Some(day(12)), Some(Bucket::Waiting)),
            (Status::Waiting, None, Some(Bucket::Waiting)),
            (Status::Todo, Some(day(12)), Some(Bucket::Upcoming)),
            (Status::Todo, None, Some(Bucket::Backlog)),
            (Status::Done, Some(day(5)), None),
            (Status::Done, Some(day(6)), None),
            (Status::Done, None, None),
        ];
        for (status, due, expected) in cases {
            let task = task(1, status, due, Priority::Medium);
            assert_eq!(Bucket::of(&task, today), expected, "{status:?} due {due:?}");
        }
    }

    #[test]
    fn the_week_ends_on_sunday() {
        let sunday = day(11);
        let monday = task(1, Status::Todo, Some(day(12)), Priority::Medium);
        assert_eq!(Bucket::of(&monday, sunday), Some(Bucket::Upcoming));
        let next_sunday = task(2, Status::Todo, Some(day(18)), Priority::Medium);
        assert_eq!(Bucket::of(&next_sunday, day(12)), Some(Bucket::DueThisWeek));
        assert_eq!(Bucket::of(&next_sunday, sunday), Some(Bucket::Upcoming));
    }

    #[test]
    fn done_and_archived_tasks_are_in_no_bucket() {
        let archived = Task {
            archived_at: day(5).and_hms_opt(19, 0, 0),
            ..task(2, Status::Done, Some(day(2)), Priority::Urgent)
        };
        let tasks = vec![task(1, Status::Done, Some(day(2)), Priority::Urgent), archived];
        assert!(sections(&tasks, day(6)).iter().all(|s| s.tasks.is_empty()));
    }

    #[test]
    fn lists_every_bucket_in_order_even_when_empty() {
        let buckets: Vec<Bucket> = sections(&[], day(6)).iter().map(|s| s.bucket).collect();
        assert_eq!(buckets, Bucket::ALL);
        let titles: Vec<&str> = Bucket::ALL.iter().map(|b| b.title()).collect();
        assert_eq!(
            titles,
            [
                "Overdue",
                "Due today",
                "Due this week",
                "In progress",
                "Waiting on others",
                "Upcoming",
                "Backlog"
            ]
        );
    }

    #[test]
    fn sorts_overdue_by_due_date_first() {
        let tasks = vec![
            task(1, Status::Todo, Some(day(1)), Priority::Low),
            task(2, Status::Todo, Some(day(3)), Priority::Urgent),
            task(3, Status::Todo, Some(day(1)), Priority::High),
            task(4, Status::Todo, Some(day(1)), Priority::High),
        ];
        let overdue = &sections(&tasks, day(6))[0];
        assert_eq!(overdue.bucket, Bucket::Overdue);
        assert_eq!(ids(&overdue.tasks), [3, 4, 1, 2]);
    }

    #[test]
    fn sorts_other_buckets_by_priority_then_due_date_then_id() {
        let tasks = vec![
            task(1, Status::Todo, Some(day(30)), Priority::Medium),
            task(2, Status::Todo, Some(day(31)), Priority::High),
            task(3, Status::Todo, Some(day(20)), Priority::Medium),
            task(4, Status::Todo, Some(day(20)), Priority::Medium),
        ];
        let upcoming = &sections(&tasks, day(6))[5];
        assert_eq!(upcoming.bucket, Bucket::Upcoming);
        assert_eq!(ids(&upcoming.tasks), [2, 3, 4, 1]);
    }

    #[test]
    fn earlier_due_dates_come_first_and_none_last() {
        assert_eq!(cmp_due(Some(day(1)), Some(day(2))), Ordering::Less);
        assert_eq!(cmp_due(Some(day(2)), None), Ordering::Less);
        assert_eq!(cmp_due(None, Some(day(2))), Ordering::Greater);
        assert_eq!(cmp_due(None, None), Ordering::Equal);
    }

    #[test]
    fn recently_completed_covers_the_window_most_recent_first() {
        let today = day(8);
        let tasks = vec![
            completed(1, day(1)),
            completed(2, day(8)),
            completed(3, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()),
            completed(4, day(4)),
            task(5, Status::Todo, None, Priority::Medium),
        ];
        assert_eq!(ids(&recently_completed(&tasks, today, 7)), [2, 4, 1]);
        assert_eq!(ids(&recently_completed(&tasks, today, 0)), [2]);
    }

    #[test]
    fn summary_counts_each_kind() {
        let today = day(6);
        let tasks = vec![
            task(1, Status::Waiting, Some(day(1)), Priority::Medium),
            task(2, Status::Todo, Some(day(6)), Priority::Medium),
            task(3, Status::InProgress, Some(day(6)), Priority::Medium),
            task(4, Status::Todo, Some(day(11)), Priority::Medium),
            task(5, Status::InProgress, None, Priority::Medium),
            task(6, Status::Todo, Some(day(12)), Priority::Medium),
            completed(7, day(6)),
            completed(8, NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        ];
        assert_eq!(
            summary(&tasks, today),
            Summary {
                open: 6,
                overdue: 1,
                due_today: 2,
                due_this_week: 1,
                completed_last_7_days: 1,
            }
        );
        assert_eq!(summary(&[], today), Summary::default());
    }
}
