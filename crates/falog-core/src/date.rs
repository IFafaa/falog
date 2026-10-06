//! Calendar helpers and natural-language due dates.
//!
//! Due dates usually arrive dictated ("by friday", "amanhã", "15/10"), so parsing accepts
//! ISO dates, day-first numeric dates and English/Portuguese keywords.

use crate::text::fold;
use chrono::{Datelike, Days, Local, Months, NaiveDate, NaiveDateTime, Timelike, Weekday};

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// Local time truncated to whole seconds.
pub fn now() -> NaiveDateTime {
    let now = Local::now().naive_local();
    now.with_nanosecond(0).unwrap_or(now)
}

/// Sunday of the (Monday-based) week containing `date`.
pub fn end_of_week(date: NaiveDate) -> NaiveDate {
    date + Days::new(6 - u64::from(date.weekday().num_days_from_monday()))
}

/// The next `weekday` on or after `from`.
pub fn next_weekday(from: NaiveDate, weekday: Weekday) -> NaiveDate {
    let ahead = (7 + weekday.num_days_from_monday() - from.weekday().num_days_from_monday()) % 7;
    from + Days::new(u64::from(ahead))
}

/// Parses a due date relative to `today`. Returns `None` when the input is not a date.
///
/// Supported: `2026-10-09`, `09/10/2026`, `09/10` (day first), `today`, `tomorrow`,
/// `friday`, `next friday`, `next week`, `end of week`, `end of month`, and the
/// Portuguese equivalents (`hoje`, `amanhã`, `sexta`, `semana que vem`...).
pub fn parse_due(input: &str, today: NaiveDate) -> Option<NaiveDate> {
    let mut text = fold(input);
    for prefix in [
        "by ", "on ", "until ", "due ", "ate ", "na ", "no ", "pra ", "para ", "dia ",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.trim().to_string();
        }
    }
    if text.is_empty() {
        return None;
    }

    if let Some(date) = parse_keyword(&text, today) {
        return Some(date);
    }
    if let Some(date) = parse_numeric(&text, today) {
        return Some(date);
    }
    if let Some(day) = text
        .strip_prefix("next ")
        .or_else(|| text.strip_prefix("proxima "))
    {
        return weekday_named(day).map(|wd| next_weekday(today + Days::new(1), wd));
    }
    weekday_named(&text).map(|wd| next_weekday(today, wd))
}

fn parse_keyword(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let date = match text {
        "today" | "hoje" => today,
        "tomorrow" | "amanha" => today + Days::new(1),
        "day after tomorrow" | "depois de amanha" => today + Days::new(2),
        "next week" | "semana que vem" | "proxima semana" => next_weekday(today + Days::new(1), Weekday::Mon),
        "end of week" | "end of the week" | "fim da semana" | "final da semana" => {
            let friday = end_of_week(today) - Days::new(2);
            friday.max(today)
        }
        "end of month" | "end of the month" | "fim do mes" | "final do mes" => {
            let first = today.with_day(1)?;
            first.checked_add_months(Months::new(1))? - Days::new(1)
        }
        _ => return None,
    };
    Some(date)
}

fn parse_numeric(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    for format in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d/%m/%y"] {
        if let Ok(date) = NaiveDate::parse_from_str(text, format) {
            return Some(date);
        }
    }
    let (day, month) = text.split_once(['/', '-', '.'])?;
    let (day, month) = (day.parse().ok()?, month.parse().ok()?);
    let date = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    // "05/01" said in December means January of next year.
    if date < today - Days::new(60) {
        return NaiveDate::from_ymd_opt(today.year() + 1, month, day);
    }
    Some(date)
}

fn weekday_named(text: &str) -> Option<Weekday> {
    let text = text.trim_end_matches("-feira").trim_end_matches(" feira");
    let weekday = match text {
        "monday" | "mon" | "segunda" | "seg" => Weekday::Mon,
        "tuesday" | "tue" | "terca" | "ter" => Weekday::Tue,
        "wednesday" | "wed" | "quarta" | "qua" => Weekday::Wed,
        "thursday" | "thu" | "quinta" | "qui" => Weekday::Thu,
        "friday" | "fri" | "sexta" | "sex" => Weekday::Fri,
        "saturday" | "sat" | "sabado" | "sab" => Weekday::Sat,
        "sunday" | "sun" | "domingo" | "dom" => Weekday::Sun,
        _ => return None,
    };
    Some(weekday)
}

/// How close a due date is, relative to today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    Overdue { days: i64 },
    Today,
    Tomorrow,
    ThisWeek,
    Later,
}

pub fn urgency(due: NaiveDate, today: NaiveDate) -> Urgency {
    match (due - today).num_days() {
        days if days < 0 => Urgency::Overdue { days: -days },
        0 => Urgency::Today,
        1 => Urgency::Tomorrow,
        2..=6 => Urgency::ThisWeek,
        _ => Urgency::Later,
    }
}

/// Short human description: `3d overdue`, `today`, `tomorrow`, `Fri Oct 9`, `Oct 24`, `Jan 5, 2027`.
pub fn describe_due(due: NaiveDate, today: NaiveDate) -> String {
    match urgency(due, today) {
        Urgency::Overdue { days } => format!("{days}d overdue"),
        Urgency::Today => "today".to_string(),
        Urgency::Tomorrow => "tomorrow".to_string(),
        Urgency::ThisWeek => due.format("%a %b %-d").to_string(),
        Urgency::Later if due.year() == today.year() => format_short(due),
        Urgency::Later => due.format("%b %-d, %Y").to_string(),
    }
}

/// `Oct 6`
pub fn format_short(date: NaiveDate) -> String {
    date.format("%b %-d").to_string()
}

/// `Tuesday, October 6, 2026`
pub fn format_long(date: NaiveDate) -> String {
    date.format("%A, %B %-d, %Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    const TUESDAY: (i32, u32, u32) = (2026, 10, 6);

    fn today() -> NaiveDate {
        d(TUESDAY.0, TUESDAY.1, TUESDAY.2)
    }

    #[test]
    fn parses_keywords_in_both_languages() {
        assert_eq!(parse_due("today", today()), Some(today()));
        assert_eq!(parse_due("Amanhã", today()), Some(d(2026, 10, 7)));
        assert_eq!(parse_due("next week", today()), Some(d(2026, 10, 12)));
        assert_eq!(parse_due("semana que vem", today()), Some(d(2026, 10, 12)));
        assert_eq!(parse_due("end of week", today()), Some(d(2026, 10, 9)));
        assert_eq!(parse_due("fim do mês", today()), Some(d(2026, 10, 31)));
    }

    #[test]
    fn parses_weekdays() {
        assert_eq!(parse_due("friday", today()), Some(d(2026, 10, 9)));
        assert_eq!(parse_due("até sexta-feira", today()), Some(d(2026, 10, 9)));
        assert_eq!(parse_due("tuesday", today()), Some(today()));
        assert_eq!(parse_due("next tuesday", today()), Some(d(2026, 10, 13)));
        assert_eq!(parse_due("segunda", today()), Some(d(2026, 10, 12)));
    }

    #[test]
    fn parses_numeric_dates() {
        assert_eq!(parse_due("2026-11-03", today()), Some(d(2026, 11, 3)));
        assert_eq!(parse_due("20/10/2026", today()), Some(d(2026, 10, 20)));
        assert_eq!(parse_due("15/10", today()), Some(d(2026, 10, 15)));
        assert_eq!(parse_due("05/01", today()), Some(d(2027, 1, 5)));
    }

    #[test]
    fn rejects_non_dates() {
        assert_eq!(parse_due("", today()), None);
        assert_eq!(parse_due("someday", today()), None);
        assert_eq!(parse_due("31/02", today()), None);
    }

    #[test]
    fn week_bounds() {
        assert_eq!(end_of_week(today()), d(2026, 10, 11));
        assert_eq!(end_of_week(d(2026, 10, 11)), d(2026, 10, 11));
    }

    #[test]
    fn describes_due_dates() {
        assert_eq!(describe_due(d(2026, 10, 1), today()), "5d overdue");
        assert_eq!(describe_due(today(), today()), "today");
        assert_eq!(describe_due(d(2026, 10, 7), today()), "tomorrow");
        assert_eq!(describe_due(d(2026, 10, 9), today()), "Fri Oct 9");
        assert_eq!(describe_due(d(2026, 10, 24), today()), "Oct 24");
        assert_eq!(describe_due(d(2027, 1, 5), today()), "Jan 5, 2027");
    }
}
