/// A `Date`: milliseconds since 1970-01-01T00:00:00Z, read and shown in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub struct JsDate {
    ms: i64,
}

#[allow(dead_code)]
impl JsDate {
    /// `new Date()`.
    pub fn now() -> JsDate {
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        JsDate { ms }
    }

    /// `new Date(ms)`.
    pub fn from_ms(ms: i64) -> JsDate {
        JsDate { ms }
    }

    /// `new Date(year, month, day, hours, minutes, seconds, ms)`: the month counts from
    /// 0 and overflows into the year, as in JavaScript.
    pub fn from_parts(year: i64, month: i64, day: i64, h: i64, mi: i64, s: i64, ms: i64) -> JsDate {
        let y = year + month.div_euclid(12);
        let m = month.rem_euclid(12) + 1;
        let days = js_days_from_civil(y, m, 1) + (day - 1);
        JsDate {
            ms: days * 86_400_000 + h * 3_600_000 + mi * 60_000 + s * 1000 + ms,
        }
    }

    /// `new Date("2024-06-01")`, `new Date("2024-06-01T12:30:00Z")`,
    /// `new Date("2024-06-01T12:30:00.250+02:00")`.
    pub fn parse(text: &str) -> JsDate {
        match js_parse_date(text.trim()) {
            Some(ms) => JsDate { ms },
            None => panic!("Invalid Date: {text:?}"),
        }
    }

    /// `d.getTime()`, `d.valueOf()`.
    pub fn get_time(&self) -> i64 {
        self.ms
    }

    pub fn value_of(&self) -> i64 {
        self.ms
    }

    fn days(&self) -> i64 {
        self.ms.div_euclid(86_400_000)
    }

    fn ms_of_day(&self) -> i64 {
        self.ms.rem_euclid(86_400_000)
    }

    pub fn get_full_year(&self) -> i64 {
        js_civil_from_days(self.days()).0
    }

    /// `d.getMonth()`: 0 to 11.
    pub fn get_month(&self) -> i64 {
        js_civil_from_days(self.days()).1 - 1
    }

    /// `d.getDate()`: the day of the month, 1 to 31.
    pub fn get_date(&self) -> i64 {
        js_civil_from_days(self.days()).2
    }

    /// `d.getDay()`: the day of the week, 0 for Sunday.
    pub fn get_day(&self) -> i64 {
        (self.days() + 4).rem_euclid(7)
    }

    pub fn get_hours(&self) -> i64 {
        self.ms_of_day() / 3_600_000
    }

    pub fn get_minutes(&self) -> i64 {
        self.ms_of_day() / 60_000 % 60
    }

    pub fn get_seconds(&self) -> i64 {
        self.ms_of_day() / 1000 % 60
    }

    pub fn get_milliseconds(&self) -> i64 {
        self.ms_of_day() % 1000
    }

    pub fn get_utc_full_year(&self) -> i64 {
        self.get_full_year()
    }

    pub fn get_utc_month(&self) -> i64 {
        self.get_month()
    }

    pub fn get_utc_date(&self) -> i64 {
        self.get_date()
    }

    pub fn get_utc_day(&self) -> i64 {
        self.get_day()
    }

    pub fn get_utc_hours(&self) -> i64 {
        self.get_hours()
    }

    pub fn get_utc_minutes(&self) -> i64 {
        self.get_minutes()
    }

    pub fn get_utc_seconds(&self) -> i64 {
        self.get_seconds()
    }

    pub fn get_utc_milliseconds(&self) -> i64 {
        self.get_milliseconds()
    }

    /// `d.getTimezoneOffset()`: minutes, 0 in UTC.
    pub fn get_timezone_offset(&self) -> i64 {
        0
    }

    /// `d.toISOString()`: `2024-06-01T12:30:00.250Z`.
    pub fn to_iso_string(&self) -> String {
        let (y, m, d) = js_civil_from_days(self.days());
        let year = if (0..=9999).contains(&y) {
            format!("{y:04}")
        } else if y < 0 {
            format!("-{:06}", -y)
        } else {
            format!("+{y:06}")
        };
        format!(
            "{year}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds(),
            self.get_milliseconds()
        )
    }

    pub fn to_json(&self) -> String {
        self.to_iso_string()
    }

    /// `d.toDateString()`: `Sat Jun 01 2024`.
    pub fn to_date_string(&self) -> String {
        let (y, m, d) = js_civil_from_days(self.days());
        format!(
            "{} {} {d:02} {y:04}",
            JS_WEEKDAYS[self.get_day() as usize],
            JS_MONTHS[(m - 1) as usize]
        )
    }

    /// `d.toTimeString()`: `12:30:00 GMT+0000 (Coordinated Universal Time)`.
    pub fn to_time_string(&self) -> String {
        format!(
            "{:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds()
        )
    }

    /// `d.toUTCString()`: `Sat, 01 Jun 2024 12:30:00 GMT`.
    pub fn to_utc_string(&self) -> String {
        let (y, m, d) = js_civil_from_days(self.days());
        format!(
            "{}, {d:02} {} {y:04} {:02}:{:02}:{:02} GMT",
            JS_WEEKDAYS[self.get_day() as usize],
            JS_MONTHS[(m - 1) as usize],
            self.get_hours(),
            self.get_minutes(),
            self.get_seconds()
        )
    }
}

/// `String(d)`: `Sat Jun 01 2024 12:30:00 GMT+0000 (Coordinated Universal Time)`.
impl std::fmt::Display for JsDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.to_date_string(), self.to_time_string())
    }
}

#[allow(dead_code)]
const JS_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
#[allow(dead_code)]
const JS_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days since 1970-01-01 of a civil date (proleptic Gregorian calendar).
#[allow(dead_code)]
fn js_days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date (year, month 1-12, day) of a number of days since 1970-01-01.
#[allow(dead_code)]
fn js_civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYY-MM-DD`, with an optional `THH:MM[:SS[.mmm]]` and a `Z` or `±HH:MM` offset (a
/// date alone is UTC, as in JavaScript).
#[allow(dead_code)]
fn js_parse_date(text: &str) -> Option<i64> {
    let (date, rest) = match text.find('T') {
        Some(i) => (&text[..i], &text[i + 1..]),
        None => (text, ""),
    };
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next().map_or(Some(1), |p| p.parse().ok())?;
    let d: i64 = parts.next().map_or(Some(1), |p| p.parse().ok())?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut ms = js_days_from_civil(y, m, d) * 86_400_000;
    if rest.is_empty() {
        return Some(ms);
    }
    let (time, offset) = match rest.find(['Z', '+', '-']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut fields = time.split(':');
    let h: i64 = fields.next()?.parse().ok()?;
    let mi: i64 = fields.next()?.parse().ok()?;
    let s_text = fields.next().unwrap_or("0");
    let (s, frac) = match s_text.find('.') {
        Some(i) => (&s_text[..i], &s_text[i + 1..]),
        None => (s_text, ""),
    };
    let s: i64 = s.parse().ok()?;
    let millis: i64 = if frac.is_empty() {
        0
    } else {
        let digits: String = frac.chars().take(3).collect();
        let n: i64 = digits.parse().ok()?;
        n * 10i64.pow(3 - digits.len() as u32)
    };
    if h > 24 || mi > 59 || s > 59 {
        return None;
    }
    ms += h * 3_600_000 + mi * 60_000 + s * 1000 + millis;
    match offset {
        "" | "Z" => Some(ms),
        off => {
            let sign = if off.starts_with('-') { 1 } else { -1 };
            let (oh, om) = off[1..].split_once(':')?;
            let (oh, om): (i64, i64) = (oh.parse().ok()?, om.parse().ok()?);
            Some(ms + sign * (oh * 3_600_000 + om * 60_000))
        }
    }
}
