use crate::fmt::V;
use prela::engine::*;

pub fn kahan((s, err): (f64, f64), x: f64) -> (f64, f64) {
    let y = x - err;
    let t = s + y;
    (t, (t - s) - y)
}

pub fn fmean(s: (f64, f64), n: i64) -> V {
    if n == 0 { V::Null } else { V::F(s.0 / n as f64) }
}

pub fn top_n<X, T: Ord>(mut v: Vec<X>, key: impl Fn(&X) -> T, n: usize) -> Vec<X> {
    v.sort_by_key(|x| key(x));
    if n > 0 && n < v.len() && key(&v[n - 1]) == key(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    if n > 0 {
        v.truncate(n);
    }
    v
}

pub fn drain<Q: Drive>(q: Q) -> Vec<(Q::D, Q::R)> {
    let mut v = Vec::new();
    q.drive(|d, r| v.push((d, r)));
    v
}

pub fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

pub fn rel<R: Copy>(v: Vec<R>) -> VecRel<usize, R> {
    VecRel::new(v)
}

pub fn left_all<R: Copy>(v: Vec<R>) -> VecRel<usize, Option<R>> {
    if v.is_empty() { VecRel::new(vec![None]) } else { VecRel::new(v.into_iter().map(Some).collect()) }
}

pub fn fkey(x: f64) -> i64 {
    let b = x.to_bits() as i64;
    b ^ (((b >> 63) as u64) >> 1) as i64
}

pub fn ranked<X, K: Ord>(mut v: Vec<X>, key: impl Fn(&X) -> K, dense: bool) -> Vec<(X, i64)> {
    v.sort_by_key(|x| key(x));
    let mut out: Vec<(X, i64)> = Vec::with_capacity(v.len());
    let mut last: Option<K> = None;
    let (mut r, mut d) = (0i64, 0i64);
    for (i, x) in v.into_iter().enumerate() {
        let k = key(&x);
        if last.as_ref() != Some(&k) {
            r = i as i64 + 1;
            d += 1;
        }
        last = Some(k);
        out.push((x, if dense { d } else { r }));
    }
    out
}

pub fn like(s: &str, p: &str) -> bool {
    let (s, p): (Vec<char>, Vec<char>) = (s.chars().collect(), p.chars().collect());
    let (mut i, mut j, mut star, mut mark) = (0, 0, None, 0);
    while i < s.len() {
        if j < p.len() && p[j] == '%' {
            star = Some(j);
            mark = i;
            j += 1;
        } else if j < p.len() && (p[j] == '_' || p[j] == s[i]) {
            i += 1;
            j += 1;
        } else if let Some(st) = star {
            j = st + 1;
            mark += 1;
            i = mark;
        } else {
            return false;
        }
    }
    p[j..].iter().all(|&c| c == '%')
}

pub fn round_dec(x: f64, places: i32) -> f64 {
    let p = 10f64.powi(places);
    let scaled = format!("{:.9}", x).parse::<f64>().unwrap() * p;
    scaled.round() / p
}
