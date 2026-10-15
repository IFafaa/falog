//! Dates in the interface language. Month and weekday names come from a table per language; the
//! order of the parts ("Oct 6" or "6 de out") is a translated pattern.

use super::{Language, current, pt, tr, trf};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Weekday};
use falog_core::date::{Urgency, urgency};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// `October`, or `outubro`.
pub fn month_long(date: NaiveDate) -> &'static str {
    let index = date.month0() as usize;
    match current() {
        Language::English => MONTHS[index],
        Language::Portuguese => pt::MONTHS[index],
    }
}

/// `Oct`, or `out`.
pub fn month_short(date: NaiveDate) -> &'static str {
    first_three(month_long(date))
}

/// `Tuesday`, or `terça-feira`.
pub fn weekday_long(date: NaiveDate) -> &'static str {
    let index = date.weekday().num_days_from_monday() as usize;
    match current() {
        Language::English => WEEKDAYS[index],
        Language::Portuguese => pt::WEEKDAYS[index],
    }
}

/// `Tue`, or `ter`, for a weekday on its own.
pub fn weekday_short_of(day: Weekday) -> &'static str {
    let index = day.num_days_from_monday() as usize;
    first_three(match current() {
        Language::English => WEEKDAYS[index],
        Language::Portuguese => pt::WEEKDAYS[index],
    })
}

/// `Tue`, or `ter`.
pub fn weekday_short(date: NaiveDate) -> &'static str {
    first_three(weekday_long(date))
}

/// `Oct 6`
pub fn short(date: NaiveDate) -> String {
    trf(
        "{month} {day}",
        &[("month", &month_short(date)), ("day", &date.day())],
    )
}

/// `Oct 6, 2026`
pub fn short_with_year(date: NaiveDate) -> String {
    trf(
        "{month} {day}, {year}",
        &[
            ("month", &month_short(date)),
            ("day", &date.day()),
            ("year", &date.year()),
        ],
    )
}

/// `Tue, Oct 6`
pub fn with_weekday(date: NaiveDate) -> String {
    trf(
        "{weekday}, {month} {day}",
        &[
            ("weekday", &weekday_short(date)),
            ("month", &month_short(date)),
            ("day", &date.day()),
        ],
    )
}

/// `Oct 6, 14:30`
pub fn short_time(at: NaiveDateTime) -> String {
    format!("{}, {}", short(at.date()), at.format("%H:%M"))
}

/// `Tue, Oct 6, 14:30`
pub fn with_weekday_time(at: NaiveDateTime) -> String {
    format!("{}, {}", with_weekday(at.date()), at.format("%H:%M"))
}

/// `October 2026`
pub fn month_year(date: NaiveDate) -> String {
    capitalized(trf(
        "{month} {year}",
        &[("month", &month_long(date)), ("year", &date.year())],
    ))
}

/// `Tuesday, October 6, 2026`
pub fn long(date: NaiveDate) -> String {
    capitalized(trf(
        "{weekday}, {month} {day}, {year}",
        &[
            ("weekday", &weekday_long(date)),
            ("month", &month_long(date)),
            ("day", &date.day()),
            ("year", &date.year()),
        ],
    ))
}

/// When a task is due, short: `3d overdue`, `today`, `tomorrow`, `Tue, Oct 6`, `Oct 24`, `Jan 5, 2027`.
pub fn describe_due(due: NaiveDate, today: NaiveDate) -> String {
    match urgency(due, today) {
        Urgency::Overdue { days } => trf("{days}d overdue", &[("days", &days)]),
        Urgency::Today => tr("today").to_owned(),
        Urgency::Tomorrow => tr("tomorrow").to_owned(),
        Urgency::ThisWeek => with_weekday(due),
        Urgency::Later if due.year() == today.year() => short(due),
        Urgency::Later => short_with_year(due),
    }
}

/// The first three letters of a name, its abbreviation in both languages.
fn first_three(name: &'static str) -> &'static str {
    name.char_indices().nth(3).map_or(name, |(end, _)| &name[..end])
}

/// `text` with its first letter in upper case, for a date that starts a heading.
fn capitalized(text: String) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::set;

    #[test]
    fn writes_dates_in_either_language() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 3).unwrap();
        set(Language::English);
        assert_eq!(with_weekday(date), "Tue, Mar 3");
        assert_eq!(long(date), "Tuesday, March 3, 2026");
        set(Language::Portuguese);
        assert_eq!(with_weekday(date), "ter, 3 de mar");
        assert_eq!(long(date), "Terça-feira, 3 de março de 2026");
        assert_eq!(month_year(date), "Março de 2026");
        set(Language::English);
    }

    /// `f` of the first day of each month of 2026, in `language`.
    fn each_month(language: Language, f: fn(NaiveDate) -> &'static str) -> Vec<&'static str> {
        set(language);
        let names = (1..=12)
            .map(|month| f(NaiveDate::from_ymd_opt(2026, month, 1).unwrap()))
            .collect();
        set(Language::English);
        names
    }

    /// `f` of each day of the week from Monday, October 5, 2026, in `language`.
    fn each_weekday(language: Language, f: fn(NaiveDate) -> &'static str) -> Vec<&'static str> {
        set(language);
        let names = (5..=11).map(|d| f(crate::testing::day(d))).collect();
        set(Language::English);
        names
    }

    #[test]
    fn names_months_and_weekdays_in_either_language() {
        assert_eq!(
            each_month(Language::English, month_short),
            [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
            ]
        );
        assert_eq!(each_month(Language::Portuguese, month_long)[4], "maio");
        assert_eq!(
            each_month(Language::Portuguese, month_short),
            [
                "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"
            ]
        );
        assert_eq!(
            each_weekday(Language::English, weekday_short),
            ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        );
        // Abbreviations cut after three letters, not three bytes: "sábado" is "sáb".
        assert_eq!(
            each_weekday(Language::Portuguese, weekday_short),
            ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"]
        );
        assert_eq!(each_weekday(Language::Portuguese, weekday_long)[5], "sábado");
        set(Language::Portuguese);
        assert_eq!(weekday_short_of(Weekday::Sat), "sáb");
        set(Language::English);
        assert_eq!(weekday_short_of(Weekday::Sat), "Sat");
    }

    #[test]
    fn writes_short_dates_and_times_in_either_language() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        let at = date.and_hms_opt(14, 30, 0).unwrap();
        set(Language::English);
        assert_eq!(short(date), "Oct 6");
        assert_eq!(short_with_year(date), "Oct 6, 2026");
        assert_eq!(short_time(at), "Oct 6, 14:30");
        assert_eq!(with_weekday_time(at), "Tue, Oct 6, 14:30");
        assert_eq!(month_year(date), "October 2026");
        set(Language::Portuguese);
        assert_eq!(short(date), "6 de out");
        assert_eq!(short_with_year(date), "6 de out de 2026");
        assert_eq!(short_time(at), "6 de out, 14:30");
        assert_eq!(with_weekday_time(at), "ter, 6 de out, 14:30");
        set(Language::English);
    }

    #[test]
    fn describes_when_a_task_is_due_in_either_language() {
        use crate::testing::day;
        // Thursday, October 15, 2026.
        let today = day(15);
        let due = |date| describe_due(date, today);
        let next_year = NaiveDate::from_ymd_opt(2027, 1, 5).unwrap();
        set(Language::English);
        assert_eq!(due(day(12)), "3d overdue");
        assert_eq!(due(today), "today");
        assert_eq!(due(day(16)), "tomorrow");
        assert_eq!(due(day(21)), "Wed, Oct 21", "within six days");
        assert_eq!(due(day(22)), "Oct 22");
        assert_eq!(due(next_year), "Jan 5, 2027");
        set(Language::Portuguese);
        assert_eq!(due(day(12)), "3d de atraso");
        assert_eq!(due(today), "hoje");
        assert_eq!(due(day(16)), "amanhã");
        assert_eq!(due(day(21)), "qua, 21 de out");
        assert_eq!(due(day(22)), "22 de out");
        assert_eq!(due(next_year), "5 de jan de 2027");
        set(Language::English);
    }
}
