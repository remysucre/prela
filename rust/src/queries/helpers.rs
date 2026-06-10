// Terminal-continuation helpers: lex-min folding per output column.

#[inline(always)]
pub fn update<R: Copy + PartialOrd>(slot: &mut Option<R>, v: R) {
    *slot = Some(match *slot {
        Some(x) if x <= v => x,
        _ => v,
    });
}

pub fn fmt1(m: Option<&'static str>) -> String {
    m.map_or("(empty)".into(), |s| s.to_string())
}
pub fn fmt2(m: [Option<&'static str>; 2]) -> String {
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {}", m[0].unwrap(), m[1].unwrap())
}
pub fn fmt3(m: [Option<&'static str>; 3]) -> String {
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {} || {}", m[0].unwrap(), m[1].unwrap(), m[2].unwrap())
}
pub fn fmt4(m: [Option<&'static str>; 4]) -> String {
    if m[0].is_none() { return "(empty)".into(); }
    format!("{} || {} || {} || {}",
            m[0].unwrap(), m[1].unwrap(), m[2].unwrap(), m[3].unwrap())
}
