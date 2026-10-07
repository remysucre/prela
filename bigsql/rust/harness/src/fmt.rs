
pub enum V {
    I(i64),
    F(f64),
    S(&'static str),
    Owned(String),
    B(bool),
    T(i64),
    D(i64),
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

pub fn row(vs: Vec<V>) -> String {
    vs.iter().map(V::text).collect::<Vec<_>>().join("\t")
}

pub fn rows(rs: impl IntoIterator<Item = String>) -> String {
    rs.into_iter().collect::<Vec<_>>().join("\n")
}

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
