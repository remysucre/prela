// Timestamps are epoch MICROSECONDS in i64 — what DuckDB's TIMESTAMP is,
// so a comparison against a literal is exact and no precision is lost the
// way a yyyymmdd packing would lose it.
//
// A SQL literal becomes a call: `CreationDate >= TIMESTAMP '2024-10-01'`
// is `.ge(ts(2024, 10, 1, 0, 0, 0))`, and `- INTERVAL '1 year'` is
// `add_years(.., -1)`. EXTRACT / DATE_TRUNC have one function each.

pub const US: i64 = 1_000_000;
pub const DAY_US: i64 = 86_400 * US;

/// Days since 1970-01-01 from a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The inverse: (year, month, day) from days since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A TIMESTAMP literal, in epoch microseconds.
pub fn ts(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    days_from_civil(y, mo, d) * DAY_US + ((h * 60 + mi) * 60 + s) * US
}

/// A DATE literal, in epoch microseconds.
pub fn date(y: i64, m: i64, d: i64) -> i64 {
    ts(y, m, d, 0, 0, 0)
}

fn parts(us: i64) -> (i64, i64, i64, i64, i64, i64, i64) {
    let days = us.div_euclid(DAY_US);
    let rem = us.rem_euclid(DAY_US);
    let (y, mo, d) = civil_from_days(days);
    let secs = rem / US;
    (y, mo, d, secs / 3600, (secs / 60) % 60, secs % 60, rem % US)
}

pub fn fmt_ts(us: i64) -> String {
    let (y, mo, d, h, mi, s, frac) = parts(us);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}.{frac:06}")
}

pub fn fmt_date(us: i64) -> String {
    let (y, mo, d, ..) = parts(us);
    format!("{y:04}-{mo:02}-{d:02}")
}

// ----- EXTRACT ----------------------------------------------------------

pub fn year(us: i64) -> i64 {
    parts(us).0
}
pub fn month(us: i64) -> i64 {
    parts(us).1
}
pub fn day(us: i64) -> i64 {
    parts(us).2
}
pub fn hour(us: i64) -> i64 {
    parts(us).3
}
/// ISO day of week, Monday = 1 .. Sunday = 7. (DuckDB's `dayofweek` is
/// Sunday = 0; use `dow` for that one.)
pub fn isodow(us: i64) -> i64 {
    // 1970-01-01 was a Thursday, which is ISO 4.
    (us.div_euclid(DAY_US).rem_euclid(7) + 3) % 7 + 1
}
pub fn dow(us: i64) -> i64 {
    (us.div_euclid(DAY_US).rem_euclid(7) + 4) % 7
}

// ----- DATE_TRUNC -------------------------------------------------------

pub fn trunc_day(us: i64) -> i64 {
    us.div_euclid(DAY_US) * DAY_US
}
pub fn trunc_month(us: i64) -> i64 {
    let (y, m, _) = civil_from_days(us.div_euclid(DAY_US));
    date(y, m, 1)
}
pub fn trunc_year(us: i64) -> i64 {
    let (y, _, _) = civil_from_days(us.div_euclid(DAY_US));
    date(y, 1, 1)
}

// ----- INTERVAL ---------------------------------------------------------

/// Calendar-month arithmetic, clamping the day to the target month's last
/// day the way DuckDB does (2024-01-31 + 1 month = 2024-02-29).
pub fn add_months(us: i64, n: i64) -> i64 {
    let (y, m, d, h, mi, s, frac) = parts(us);
    let t = (y * 12 + (m - 1)) + n;
    let (ny, nm) = (t.div_euclid(12), t.rem_euclid(12) + 1);
    let last = days_from_civil(
        if nm == 12 { ny + 1 } else { ny },
        if nm == 12 { 1 } else { nm + 1 },
        1,
    ) - days_from_civil(ny, nm, 1);
    ts(ny, nm, d.min(last), h, mi, s) + frac
}

pub fn add_years(us: i64, n: i64) -> i64 {
    add_months(us, 12 * n)
}

pub fn add_days(us: i64, n: i64) -> i64 {
    us + n * DAY_US
}

// ----- TIMESTAMPTZ --------------------------------------------------------

fn nth_sunday(y: i64, m: i64, n: i64) -> i64 {
    let first = days_from_civil(y, m, 1);
    let to_sun = (7 - (first + 4).rem_euclid(7)) % 7;
    first + to_sun + 7 * (n - 1)
}

fn last_sunday(y: i64, m: i64) -> i64 {
    let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
    let last = next - 1;
    last - (last + 4).rem_euclid(7)
}

/// `CAST(ts AS TIMESTAMPTZ)` in the oracle's session zone, America/New_York:
/// the UTC instant of a local wall time, in epoch microseconds. Mixing a
/// TIMESTAMP with `CURRENT_TIMESTAMP` (in a `COALESCE`, say) makes this cast
/// implicit, and then a difference of two such values is elapsed time, which
/// is an hour off the wall-clock difference across a DST change.
pub fn ny_to_utc(us: i64) -> i64 {
    let y = year(us);
    let (on, off) = if y >= 2007 {
        (nth_sunday(y, 3, 2), nth_sunday(y, 11, 1))
    } else {
        (nth_sunday(y, 4, 1), last_sunday(y, 10))
    };
    let dst = us >= on * DAY_US + 3 * 3600 * US && us < off * DAY_US + 3600 * US;
    us + if dst { 4 } else { 5 } * 3600 * US
}

/// The local wall time of a UTC instant in America/New_York.
pub fn utc_to_ny(u: i64) -> i64 {
    let y = year(u);
    let (on, off) = if y >= 2007 {
        (nth_sunday(y, 3, 2), nth_sunday(y, 11, 1))
    } else {
        (nth_sunday(y, 4, 1), last_sunday(y, 10))
    };
    let dst = u >= on * DAY_US + 7 * 3600 * US && u < off * DAY_US + 6 * 3600 * US;
    u - if dst { 4 } else { 5 } * 3600 * US
}

/// ICU's "add n days" to an instant: keep the wall clock time across a DST
/// change, except where that wall time does not exist (the skipped hour),
/// where the unadjusted instant stands.
fn tz_add_days(ub: i64, n: i64) -> i64 {
    let m = ub + n * DAY_US;
    let adj = m + ((m - utc_to_ny(m)) - (ub - utc_to_ny(ub)));
    if adj < m && utc_to_ny(adj).rem_euclid(DAY_US) != utc_to_ny(ub).rem_euclid(DAY_US) { m } else { adj }
}

/// `a - b` for two TIMESTAMPTZ values, in microseconds, as DuckDB's ICU
/// extension computes it: whole days first, added to `b` on the local wall
/// clock, then the elapsed time from there to `a`. So the hour a DST change
/// adds or removes only shows when it falls in that last stretch. Checked
/// against DuckDB on every post's `LastActivityDate - CreationDate`.
pub fn tz_sub(a: i64, b: i64) -> i64 {
    let (ua, ub) = (ny_to_utc(a), ny_to_utc(b));
    let t = |n: i64| if n == 0 { ub } else { tz_add_days(ub, n) };
    let mut n = (a - utc_to_ny(ub)).div_euclid(DAY_US);
    while t(n + 1) <= ua {
        n += 1;
    }
    while n > 0 && t(n) > ua {
        n -= 1;
    }
    n * DAY_US + ua - t(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_literals() {
        assert_eq!(fmt_ts(0), "1970-01-01 00:00:00.000000");
        assert_eq!(fmt_ts(ts(2024, 10, 1, 12, 34, 56)), "2024-10-01 12:34:56.000000");
        assert_eq!(fmt_date(ts(2009, 3, 5, 22, 28, 34)), "2009-03-05");
        // pre-epoch, to catch truncating division
        assert_eq!(fmt_ts(ts(1969, 12, 31, 23, 59, 59)), "1969-12-31 23:59:59.000000");
    }

    #[test]
    fn extract_and_trunc() {
        let t = ts(2024, 10, 1, 12, 34, 56);
        assert_eq!((year(t), month(t), day(t), hour(t)), (2024, 10, 1, 12));
        assert_eq!(fmt_ts(trunc_month(t)), "2024-10-01 00:00:00.000000");
        assert_eq!(fmt_ts(trunc_year(t)), "2024-01-01 00:00:00.000000");
        // 1970-01-01 was a Thursday: ISO 4, DuckDB dayofweek 4
        assert_eq!((isodow(0), dow(0)), (4, 4));
        // 2024-10-06 was a Sunday
        let sun = date(2024, 10, 6);
        assert_eq!((isodow(sun), dow(sun)), (7, 0));
    }

    #[test]
    fn new_york_offsets() {
        let h = 3600 * US;
        let off = |t: i64| (ny_to_utc(t) - t) / h;
        assert_eq!(off(ts(2021, 3, 14, 1, 30, 0)), 5);
        assert_eq!(off(ts(2021, 3, 14, 2, 30, 0)), 5);
        assert_eq!(off(ts(2021, 3, 14, 3, 30, 0)), 4);
        assert_eq!(off(ts(2021, 11, 7, 0, 30, 0)), 4);
        assert_eq!(off(ts(2021, 11, 7, 1, 30, 0)), 5);
        assert_eq!(off(ts(2006, 4, 2, 2, 30, 0)), 5);
        assert_eq!(off(ts(2006, 10, 29, 1, 30, 0)), 5);
        assert_eq!(off(ts(2006, 7, 1, 0, 0, 0)), 4);
    }

    #[test]
    fn interval_arithmetic() {
        assert_eq!(fmt_date(add_years(date(2024, 10, 1), -1)), "2023-10-01");
        assert_eq!(fmt_date(add_months(date(2024, 1, 31), 1)), "2024-02-29");
        assert_eq!(fmt_date(add_months(date(2023, 1, 31), 1)), "2023-02-28");
        assert_eq!(fmt_date(add_months(date(2024, 3, 15), -4)), "2023-11-15");
        assert_eq!(fmt_date(add_days(date(2024, 2, 28), 2)), "2024-03-01");
    }
}
