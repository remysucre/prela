// The canonical result format — the contract between a prela port and the
// DuckDB oracle. `tools/oracle.py` formats DuckDB's rows by exactly these
// rules, so a byte-identical result means the two agree.
//
//   row       fields joined by TAB, rows joined by NEWLINE
//   NULL      \N
//   integer   decimal
//   float     six decimal places, always ("{:.6}")
//   bool      true / false
//   string    backslash, tab, newline and carriage return escaped, so a
//             field can never break the row framing
//   timestamp YYYY-MM-DD HH:MM:SS.ffffff   (from epoch microseconds)
//   date      YYYY-MM-DD
//   list      [a, b, c] over the same rules
//
// Rows are compared as a SORTED multiset (see `run`), so a port is free to
// emit them in any order; ORDER BY only has to be reproduced when the query
// also has a LIMIT, where it decides WHICH rows come back.

pub enum V {
    I(i64),
    F(f64),
    S(&'static str),
    Owned(String),
    B(bool),
    /// epoch microseconds, printed as a timestamp
    T(i64),
    /// epoch microseconds, printed as a date
    D(i64),
    /// a duration in microseconds, printed the way Python renders a timedelta
    Iv(i64),
    L(Vec<V>),
    Null,
}

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

impl V {
    pub fn text(&self) -> String {
        match self {
            V::I(x) => x.to_string(),
            V::F(x) => format!("{x:.6}"),
            V::S(s) => esc(s),
            V::Owned(s) => esc(s),
            V::B(b) => b.to_string(),
            V::T(us) => crate::time::fmt_ts(*us),
            V::D(us) => crate::time::fmt_date(*us),
            V::Iv(us) => fmt_delta(*us),
            V::L(vs) => {
                let inner: Vec<String> = vs.iter().map(V::text).collect();
                format!("[{}]", inner.join(", "))
            }
            V::Null => "\\N".to_string(),
        }
    }
}

/// One result row.
pub fn row(vs: Vec<V>) -> String {
    vs.iter().map(V::text).collect::<Vec<_>>().join("\t")
}

/// The whole result.
pub fn rows(rs: impl IntoIterator<Item = String>) -> String {
    rs.into_iter().collect::<Vec<_>>().join("\n")
}

// ----- projecting a nullable column --------------------------------------
//
// A nullable column is a `Set`, so `.select(col)` DROPS the rows where it
// is absent — which is what a filter or a join wants, and the opposite of
// what a projection wants: `SELECT p.Title` must still emit the row, with
// NULL in that field. So a projection of a nullable column is a probe from
// inside the drive (`Probe::get`), and these turn its `Option` into a
// field.

pub fn ostr(o: Option<&'static str>) -> V {
    o.map_or(V::Null, V::S)
}
pub fn oint(o: Option<i64>) -> V {
    o.map_or(V::Null, V::I)
}
pub fn ofloat(o: Option<f64>) -> V {
    o.map_or(V::Null, V::F)
}
pub fn ots(o: Option<i64>) -> V {
    o.map_or(V::Null, V::T)
}
pub fn odate(o: Option<i64>) -> V {
    o.map_or(V::Null, V::D)
}

pub fn fmt_delta(us: i64) -> String {
    let total_days = us.div_euclid(crate::time::DAY_US);
    let rest = us.rem_euclid(crate::time::DAY_US);
    let micros = rest % 1_000_000;
    let secs = rest / 1_000_000;
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);

    let mut out = String::new();
    if total_days != 0 {
        let unit = if total_days == 1 || total_days == -1 { "day" } else { "days" };
        out.push_str(&format!("{total_days} {unit}, "));
    }
    out.push_str(&format!("{h}:{m:02}:{s:02}"));
    if micros != 0 {
        out.push_str(&format!(".{micros:06}"));
    }
    out
}
