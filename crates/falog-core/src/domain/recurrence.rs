//! How a task repeats. Completing a recurring task creates its next occurrence (see
//! [`Store::update_task`](crate::Store::update_task)).

use crate::text::fold;
use chrono::{Datelike, Days, Months, NaiveDate, Weekday};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Recurrence {
    Daily,
    /// On the given weekdays; never empty.
    Weekly(Weekdays),
    /// On this day of the month, 1 to 31; months without it use their last day.
    Monthly(u32),
}

/// A set of weekdays, Monday first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Weekdays(u8);

const ALL_WEEKDAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

impl Weekdays {
    /// Monday to Friday.
    pub const WORKDAYS: Self = Self(0b001_1111);

    pub const fn single(day: Weekday) -> Self {
        Self(1 << day.num_days_from_monday())
    }

    pub const fn contains(self, day: Weekday) -> bool {
        self.0 & (1 << day.num_days_from_monday()) != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn insert(&mut self, day: Weekday) {
        self.0 |= Self::single(day).0;
    }

    pub fn toggle(&mut self, day: Weekday) {
        self.0 ^= Self::single(day).0;
    }

    /// The days in the set, Monday first.
    pub fn iter(self) -> impl Iterator<Item = Weekday> {
        ALL_WEEKDAYS.into_iter().filter(move |day| self.contains(*day))
    }
}

impl Recurrence {
    /// The first occurrence strictly after `date`.
    pub fn next_after(self, date: NaiveDate) -> NaiveDate {
        match self {
            Self::Daily => date + Days::new(1),
            Self::Weekly(days) => (1..=7)
                .map(|ahead| date + Days::new(ahead))
                .find(|day| days.contains(day.weekday()))
                .unwrap_or(date + Days::new(7)),
            Self::Monthly(day) => {
                let this_month = day_in_month(date, day);
                if this_month > date {
                    this_month
                } else {
                    date.with_day(1)
                        .and_then(|first| first.checked_add_months(Months::new(1)))
                        .map_or(date + Days::new(31), |next| day_in_month(next, day))
                }
            }
        }
    }

    /// The first occurrence on `date` or later.
    pub fn first_from(self, date: NaiveDate) -> NaiveDate {
        date.pred_opt().map_or(date, |before| self.next_after(before))
    }

    /// Due date of the occurrence that follows one due on `due` and completed on `today`: the next
    /// one after both, so a late completion does not leave the new task overdue and an early one
    /// does not repeat the same day.
    pub fn following(self, due: Option<NaiveDate>, today: NaiveDate) -> NaiveDate {
        self.next_after(due.map_or(today, |due| due.max(today)))
    }

    /// Parses a rule as typed or dictated, in English or Portuguese: `daily`, `every weekday`,
    /// `every friday`, `toda segunda e quarta`, `weekly`, `monthly on the 5th`, `todo dia 10`, and the
    /// stored form (`weekly:mon,fri`, `monthly:5`). `weekly` and `monthly` without a day repeat on
    /// `anchor`'s weekday or day. Returns `None` when the text is not a rule.
    pub fn parse(input: &str, anchor: NaiveDate) -> Option<Self> {
        let folded = fold(input);
        if let Some(rule) = parse_key(&folded) {
            return Some(rule);
        }
        let text = folded.replace([',', '.', ';'], " ");
        let words: Vec<&str> = text
            .split_whitespace()
            .filter(|word| !FILLER.contains(word))
            .collect();
        if words.is_empty() {
            return None;
        }
        let has = |options: &[&str]| words.iter().any(|word| options.contains(word));

        if has(&["weekday", "weekdays", "workday", "workdays", "util", "uteis"]) {
            return Some(Self::Weekly(Weekdays::WORKDAYS));
        }
        let mut days = Weekdays::default();
        for word in &words {
            if let Some(day) = weekday_named(word) {
                days.insert(day);
            }
        }
        if !days.is_empty() {
            return Some(Self::Weekly(days));
        }
        if let Some(day) = words.iter().find_map(|word| day_of_month(word)) {
            // "every 2 weeks", "a cada 3 dias": intervals are not supported, and the number is not a day.
            let interval = has(&[
                "days", "dias", "weeks", "week", "semanas", "semana", "months", "meses",
            ]);
            return (!interval).then_some(Self::Monthly(day));
        }
        if has(&[
            "daily",
            "day",
            "days",
            "dia",
            "dias",
            "diario",
            "diaria",
            "diariamente",
        ]) {
            return Some(Self::Daily);
        }
        if has(&["weekly", "week", "semana", "semanal", "semanalmente"]) {
            return Some(Self::Weekly(Weekdays::single(anchor.weekday())));
        }
        if has(&["monthly", "month", "mes", "meses", "mensal", "mensalmente"]) {
            return Some(Self::Monthly(anchor.day()));
        }
        None
    }

    /// The stored form: `daily`, `weekly:mon,fri`, `monthly:5`.
    pub fn key(self) -> String {
        match self {
            Self::Daily => "daily".into(),
            Self::Weekly(days) => {
                let names: Vec<&str> = days.iter().map(weekday_key).collect();
                format!("weekly:{}", names.join(","))
            }
            Self::Monthly(day) => format!("monthly:{day}"),
        }
    }
}

/// `every day`, `every weekday`, `every Friday`, `every Monday and Thursday`, `monthly on day 5`.
impl fmt::Display for Recurrence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Daily => f.write_str("every day"),
            Self::Weekly(Weekdays::WORKDAYS) => f.write_str("every weekday"),
            Self::Weekly(days) => {
                let names: Vec<&str> = days.iter().map(weekday_name).collect();
                write!(f, "every {}", join_and(&names))
            }
            Self::Monthly(day) => write!(f, "monthly on day {day}"),
        }
    }
}

impl ToSql for Recurrence {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.key().into())
    }
}

impl FromSql for Recurrence {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?;
        parse_key(text).ok_or_else(|| FromSqlError::Other(format!("unknown recurrence \"{text}\"").into()))
    }
}

/// Words that carry no meaning in a rule: `every`, `on the`, `toda`, `de`...
const FILLER: &[&str] = &[
    "every",
    "each",
    "on",
    "the",
    "of",
    "and",
    "at",
    "repeat",
    "repeats",
    "repeating",
    "toda",
    "todo",
    "todas",
    "todos",
    "cada",
    "de",
    "do",
    "da",
    "no",
    "na",
    "nos",
    "nas",
    "e",
    "a",
    "o",
    "as",
    "os",
    "em",
    "repete",
    "repetir",
    "se",
    "sempre",
];

fn parse_key(text: &str) -> Option<Recurrence> {
    if text == "daily" {
        return Some(Recurrence::Daily);
    }
    if let Some(names) = text.strip_prefix("weekly:") {
        let mut days = Weekdays::default();
        for name in names.split(',') {
            days.insert(weekday_named(name.trim())?);
        }
        return (!days.is_empty()).then_some(Recurrence::Weekly(days));
    }
    let day: u32 = text.strip_prefix("monthly:")?.trim().parse().ok()?;
    (1..=31).contains(&day).then_some(Recurrence::Monthly(day))
}

/// `5`, `5th`, `1st` as a day of the month.
fn day_of_month(word: &str) -> Option<u32> {
    let (digits, suffix) = word.split_at(word.find(|c: char| !c.is_ascii_digit()).unwrap_or(word.len()));
    // An ordinal ending: "5th", "1st", "5º".
    let ordinal = suffix == "º" || (suffix.len() <= 2 && suffix.chars().all(|c| c.is_ascii_alphabetic()));
    if !ordinal {
        return None;
    }
    let day: u32 = digits.parse().ok()?;
    (1..=31).contains(&day).then_some(day)
}

fn weekday_named(word: &str) -> Option<Weekday> {
    let word = word.trim_end_matches("-feira").trim_end_matches("-feiras");
    // Plurals: "fridays", "sextas".
    let word = match word {
        "sabados" | "domingos" => word.trim_end_matches('s'),
        _ => word.strip_suffix('s').filter(|w| w.len() > 3).unwrap_or(word),
    };
    let day = match word {
        "monday" | "mon" | "segunda" | "seg" => Weekday::Mon,
        "tuesday" | "tue" | "terca" | "ter" => Weekday::Tue,
        "wednesday" | "wed" | "quarta" | "qua" => Weekday::Wed,
        "thursday" | "thu" | "quinta" | "qui" => Weekday::Thu,
        "friday" | "fri" | "sexta" | "sex" => Weekday::Fri,
        "saturday" | "sat" | "sabado" | "sab" => Weekday::Sat,
        "sunday" | "sun" | "domingo" | "dom" => Weekday::Sun,
        _ => return None,
    };
    Some(day)
}

const fn weekday_key(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

const fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

/// `a`, `a and b`, `a, b and c`.
fn join_and(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// `day` of `date`'s month, or the month's last day when it is shorter.
fn day_in_month(date: NaiveDate, day: u32) -> NaiveDate {
    (1..=day.clamp(1, 31))
        .rev()
        .find_map(|d| date.with_day(d))
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// A Tuesday.
    fn today() -> NaiveDate {
        d(2026, 10, 6)
    }

    fn weekly(days: &[Weekday]) -> Recurrence {
        let mut set = Weekdays::default();
        days.iter().for_each(|day| set.insert(*day));
        Recurrence::Weekly(set)
    }

    #[test]
    fn parses_english() {
        let parse = |text| Recurrence::parse(text, today());
        assert_eq!(parse("daily"), Some(Recurrence::Daily));
        assert_eq!(parse("Every day"), Some(Recurrence::Daily));
        assert_eq!(
            parse("every weekday"),
            Some(Recurrence::Weekly(Weekdays::WORKDAYS))
        );
        assert_eq!(parse("every friday"), Some(weekly(&[Weekday::Fri])));
        assert_eq!(
            parse("Mondays and Thursdays"),
            Some(weekly(&[Weekday::Mon, Weekday::Thu]))
        );
        assert_eq!(
            parse("every mon, wed, fri"),
            Some(weekly(&[Weekday::Mon, Weekday::Wed, Weekday::Fri]))
        );
        assert_eq!(parse("weekly"), Some(weekly(&[Weekday::Tue])));
        assert_eq!(parse("monthly"), Some(Recurrence::Monthly(6)));
        assert_eq!(parse("monthly on the 5th"), Some(Recurrence::Monthly(5)));
        assert_eq!(parse("monthly on day 5"), Some(Recurrence::Monthly(5)));
        assert_eq!(parse("on the 31st of every month"), Some(Recurrence::Monthly(31)));
    }

    #[test]
    fn parses_portuguese() {
        let parse = |text| Recurrence::parse(text, today());
        assert_eq!(parse("todo dia"), Some(Recurrence::Daily));
        assert_eq!(parse("todos os dias"), Some(Recurrence::Daily));
        assert_eq!(parse("diariamente"), Some(Recurrence::Daily));
        assert_eq!(parse("dias úteis"), Some(Recurrence::Weekly(Weekdays::WORKDAYS)));
        assert_eq!(parse("toda sexta-feira"), Some(weekly(&[Weekday::Fri])));
        assert_eq!(
            parse("todas as segundas e quartas"),
            Some(weekly(&[Weekday::Mon, Weekday::Wed]))
        );
        assert_eq!(parse("aos sábados"), Some(weekly(&[Weekday::Sat])));
        assert_eq!(parse("semanal"), Some(weekly(&[Weekday::Tue])));
        assert_eq!(parse("todo mês"), Some(Recurrence::Monthly(6)));
        assert_eq!(parse("todo dia 10"), Some(Recurrence::Monthly(10)));
        assert_eq!(parse("mensal no dia 5"), Some(Recurrence::Monthly(5)));
    }

    #[test]
    fn rejects_what_is_not_a_rule() {
        assert_eq!(Recurrence::parse("", today()), None);
        assert_eq!(Recurrence::parse("someday", today()), None);
        assert_eq!(Recurrence::parse("every 45th", today()), None);
        assert_eq!(Recurrence::parse("every 2 weeks", today()), None);
        assert_eq!(Recurrence::parse("a cada 3 dias", today()), None);
    }

    #[test]
    fn keys_round_trip() {
        for rule in [
            Recurrence::Daily,
            Recurrence::Weekly(Weekdays::WORKDAYS),
            weekly(&[Weekday::Sun]),
            Recurrence::Monthly(31),
        ] {
            assert_eq!(parse_key(&rule.key()), Some(rule), "{}", rule.key());
            assert_eq!(Recurrence::parse(&rule.key(), today()), Some(rule));
        }
        assert_eq!(weekly(&[Weekday::Fri, Weekday::Mon]).key(), "weekly:mon,fri");
        assert_eq!(parse_key("monthly:0"), None);
        assert_eq!(parse_key("weekly:"), None);
    }

    #[test]
    fn finds_the_next_occurrence() {
        assert_eq!(Recurrence::Daily.next_after(today()), d(2026, 10, 7));
        let fridays = weekly(&[Weekday::Fri]);
        assert_eq!(fridays.next_after(today()), d(2026, 10, 9));
        assert_eq!(fridays.next_after(d(2026, 10, 9)), d(2026, 10, 16));
        assert_eq!(fridays.first_from(d(2026, 10, 9)), d(2026, 10, 9));
        let workdays = Recurrence::Weekly(Weekdays::WORKDAYS);
        assert_eq!(workdays.next_after(d(2026, 10, 9)), d(2026, 10, 12));
        assert_eq!(Recurrence::Monthly(5).next_after(today()), d(2026, 11, 5));
        assert_eq!(Recurrence::Monthly(10).next_after(today()), d(2026, 10, 10));
        assert_eq!(Recurrence::Monthly(31).next_after(d(2027, 1, 31)), d(2027, 2, 28));
        assert_eq!(Recurrence::Monthly(31).next_after(d(2027, 2, 28)), d(2027, 3, 31));
    }

    #[test]
    fn follows_from_the_later_of_due_date_and_today() {
        let fridays = weekly(&[Weekday::Fri]);
        // Done on time: the next Friday.
        assert_eq!(
            fridays.following(Some(d(2026, 10, 9)), d(2026, 10, 9)),
            d(2026, 10, 16)
        );
        // Done early: still the Friday after the one it was due.
        assert_eq!(fridays.following(Some(d(2026, 10, 9)), today()), d(2026, 10, 16));
        // Done late: the coming Friday, not one already past.
        assert_eq!(fridays.following(Some(d(2026, 9, 25)), today()), d(2026, 10, 9));
        assert_eq!(fridays.following(None, today()), d(2026, 10, 9));
    }

    #[test]
    fn describes_rules() {
        assert_eq!(Recurrence::Daily.to_string(), "every day");
        assert_eq!(
            Recurrence::Weekly(Weekdays::WORKDAYS).to_string(),
            "every weekday"
        );
        assert_eq!(
            weekly(&[Weekday::Mon, Weekday::Wed, Weekday::Fri]).to_string(),
            "every Monday, Wednesday and Friday"
        );
        assert_eq!(Recurrence::Monthly(1).to_string(), "monthly on day 1");
        assert_eq!(Recurrence::Monthly(22).to_string(), "monthly on day 22");
    }
}
