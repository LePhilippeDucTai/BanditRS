//! UTC timestamps formatted the way Python's `datetime` renders them.

use std::time::{SystemTime, UNIX_EPOCH};

/// A broken-down UTC date/time with microsecond precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub microsecond: u32,
}

impl UtcDateTime {
    /// Current time.
    pub fn now() -> UtcDateTime {
        let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        UtcDateTime::from_unix(d.as_secs() as i64, d.subsec_micros())
    }

    /// Build from seconds since the Unix epoch.
    pub fn from_unix(secs: i64, microsecond: u32) -> UtcDateTime {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        UtcDateTime {
            year,
            month,
            day,
            hour: (rem / 3600) as u32,
            minute: ((rem % 3600) / 60) as u32,
            second: (rem % 60) as u32,
            microsecond,
        }
    }

    /// `time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())`.
    pub fn iso_z(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `str(datetime.datetime.now(datetime.timezone.utc))`: microseconds are
    /// omitted when zero, the offset is rendered as `+00:00`.
    pub fn python_str(&self) -> String {
        if self.microsecond == 0 {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}+00:00",
                self.year, self.month, self.day, self.hour, self.minute, self.second
            )
        } else {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}+00:00",
                self.year, self.month, self.day, self.hour, self.minute, self.second, self.microsecond
            )
        }
    }
}

/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_known_dates() {
        let dt = UtcDateTime::from_unix(0, 0);
        assert_eq!(dt.iso_z(), "1970-01-01T00:00:00Z");
        assert_eq!(dt.python_str(), "1970-01-01 00:00:00+00:00");
        // 2026-09-08T12:34:56Z
        let dt = UtcDateTime::from_unix(1_788_870_896, 123_456);
        assert_eq!(dt.iso_z(), "2026-09-08T12:34:56Z");
        assert_eq!(dt.python_str(), "2026-09-08 12:34:56.123456+00:00");
        // leap day
        let dt = UtcDateTime::from_unix(951_782_400, 0);
        assert_eq!(dt.iso_z(), "2000-02-29T00:00:00Z");
    }
}
