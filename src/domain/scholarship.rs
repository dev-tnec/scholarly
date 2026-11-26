use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Asia::Jakarta;

pub const CLOSING_SOON_DAYS: i64 = 14;
pub const URGENCY_DAYS: i64 = 30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeadlineType {
    #[default]
    Fixed,
    Rolling,
    Unknown,
}

impl DeadlineType {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "rolling" => Self::Rolling,
            "unknown" => Self::Unknown,
            _ => Self::Fixed,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Rolling => "rolling",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Open,
    ClosingSoon,
    Closed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Buka",
            Self::ClosingSoon => "Segera Tutup",
            Self::Closed => "Tutup",
        }
    }

    /// Head-block pill copy from content-master-template section 4.4.
    pub fn head_label(self) -> &'static str {
        match self {
            Self::Open => "Pendaftaran Dibuka",
            Self::ClosingSoon => "Segera Tutup",
            Self::Closed => "Tutup",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Open => "badge--status-open",
            Self::ClosingSoon => "badge--status-closing",
            Self::Closed => "badge--status-closed",
        }
    }
}

/// Today's date in Asia/Jakarta, PRD section 5: countdown days use Asia/Jakarta.
pub fn today_jakarta(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Jakarta).date_naive()
}

/// Days from `today` (Jakarta) until `close`. Negative when the date has passed.
pub fn days_remaining(close: NaiveDate, today: NaiveDate) -> i64 {
    (close - today).num_days()
}

/// Status from close date and deadline type, never stored, PRD section 5:
/// Tutup when the close date has passed, Segera Tutup when 14 days or fewer
/// remain, Buka otherwise. Rolling is always Buka with no countdown.
pub fn status(close: Option<NaiveDate>, deadline_type: DeadlineType, today: NaiveDate) -> Status {
    match (deadline_type, close) {
        (DeadlineType::Rolling, _) => Status::Open,
        (_, None) => Status::Open,
        (_, Some(close)) => {
            let days = days_remaining(close, today);
            if days < 0 {
                Status::Closed
            } else if days <= CLOSING_SOON_DAYS {
                Status::ClosingSoon
            } else {
                Status::Open
            }
        }
    }
}

/// Countdown days when a countdown applies. None for rolling or a missing
/// close date; unknown deadline types still count down when a date exists.
pub fn countdown_days(
    close: Option<NaiveDate>,
    deadline_type: DeadlineType,
    today: NaiveDate,
) -> Option<i64> {
    match deadline_type {
        DeadlineType::Rolling => None,
        DeadlineType::Fixed | DeadlineType::Unknown => {
            close.map(|c| days_remaining(c, today)).filter(|d| *d >= 0)
        }
    }
}

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
];
const MONTHS_LONG: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

/// Short Indonesian date for cards and tables: 31 Jan 2027.
pub fn fmt_short(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        MONTHS_SHORT[date.month0() as usize],
        date.year()
    )
}

/// Long Indonesian date for prose: 31 Januari 2027.
pub fn fmt_long(date: NaiveDate) -> String {
    format!(
        "{} {} {}",
        date.day(),
        MONTHS_LONG[date.month0() as usize],
        date.year()
    )
}

/// Permalink for a slug: /{slug}/ with trailing slash, PRD section 7.
pub fn detail_path(slug: &str) -> String {
    format!("/{slug}/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const TODAY: NaiveDate = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();

    fn date(days_from_today: i64) -> NaiveDate {
        TODAY + chrono::Duration::days(days_from_today)
    }

    #[test]
    fn status_thresholds() {
        let cases = [
            (13, Status::ClosingSoon),
            (14, Status::ClosingSoon),
            (15, Status::Open),
            (29, Status::Open),
            (30, Status::Open),
            (31, Status::Open),
        ];
        for (offset, want) in cases {
            assert_eq!(
                status(Some(date(offset)), DeadlineType::Fixed, TODAY),
                want,
                "offset {offset}"
            );
        }
        assert_eq!(
            status(Some(date(-1)), DeadlineType::Fixed, TODAY),
            Status::Closed
        );
        assert_eq!(
            status(Some(date(0)), DeadlineType::Fixed, TODAY),
            Status::ClosingSoon
        );
    }

    #[test]
    fn urgency_badge_boundaries() {
        let mut shown = Vec::new();
        for offset in [31, 30, 29, 1] {
            let days = countdown_days(Some(date(offset)), DeadlineType::Fixed, TODAY).unwrap();
            shown.push((0..=URGENCY_DAYS).contains(&days));
        }
        assert_eq!(shown, [false, true, true, true]);
    }

    #[test]
    fn rolling_is_open_without_countdown() {
        assert_eq!(
            status(Some(date(-5)), DeadlineType::Rolling, TODAY),
            Status::Open
        );
        assert_eq!(
            countdown_days(Some(date(10)), DeadlineType::Rolling, TODAY),
            None
        );
    }

    #[test]
    fn unknown_deadline_type() {
        assert_eq!(status(None, DeadlineType::Unknown, TODAY), Status::Open);
        assert_eq!(countdown_days(None, DeadlineType::Unknown, TODAY), None);
        // A known close date drives status and countdown like a fixed deadline.
        assert_eq!(
            countdown_days(Some(date(10)), DeadlineType::Unknown, TODAY),
            Some(10)
        );
        assert_eq!(
            status(Some(date(10)), DeadlineType::Unknown, TODAY),
            Status::ClosingSoon
        );
        assert_eq!(
            status(Some(date(-1)), DeadlineType::Unknown, TODAY),
            Status::Closed
        );
    }

    #[test]
    fn countdown_across_midnight_jakarta() {
        // 30 Jan 2027 23:30 UTC is already 31 Jan 2027 06:30 in Jakarta.
        let now = Utc.with_ymd_and_hms(2027, 1, 30, 23, 30, 0).unwrap();
        let today = today_jakarta(now);
        assert_eq!(today, NaiveDate::from_ymd_opt(2027, 1, 31).unwrap());
        let close = NaiveDate::from_ymd_opt(2027, 1, 31).unwrap();
        assert_eq!(days_remaining(close, today), 0);
        assert_eq!(
            status(Some(close), DeadlineType::Fixed, today),
            Status::ClosingSoon
        );
        // Naive UTC dating would have said 1 day remains.
    }

    #[test]
    fn date_formats() {
        let d = NaiveDate::from_ymd_opt(2027, 1, 31).unwrap();
        assert_eq!(fmt_short(d), "31 Jan 2027");
        assert_eq!(fmt_long(d), "31 Januari 2027");
    }

    #[test]
    fn detail_path_has_trailing_slash() {
        assert_eq!(detail_path("nl-scholarship"), "/nl-scholarship/");
    }
}
