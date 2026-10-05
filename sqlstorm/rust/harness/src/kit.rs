// Small helpers shared by the batches from b142 on: float sums the way
// DuckDB adds DOUBLEs, the NULL-aware MIN/MAX sentinels, ORDER BY ... LIMIT
// with a warning when the cut falls inside a tie, and short names for the
// type-name compositions nearly every query joins through.

use crate::fmt::V;
use crate::schema::*;
use crate::views::*;
use prela::engine::*;
use prela::loader::{Col, Str};

/// Compensated summation. See notes/translation-failures.md 10.
pub fn kahan((s, err): (f64, f64), x: f64) -> (f64, f64) {
    let y = x - err;
    let t = s + y;
    (t, (t - s) - y)
}

/// AVG over a Kahan sum: NULL when nothing was summed.
pub fn fmean(s: (f64, f64), n: i64) -> V {
    if n == 0 { V::Null } else { V::F(s.0 / n as f64) }
}

/// MIN of a timestamp folded from `i64::MAX`: NULL when nothing was folded.
pub fn tmin(x: i64) -> V {
    if x == i64::MAX { V::Null } else { V::T(x) }
}

/// MAX of a timestamp folded from `i64::MIN`: NULL when nothing was folded.
pub fn tmax(x: i64) -> V {
    if x == i64::MIN { V::Null } else { V::T(x) }
}

pub fn ptype_name(db: &'static So) -> Compose<&'static Col<Post, Id<PostType>>, &'static Col<PostType, Str>> {
    (&db.post.post_type).select(&db.post_type.name)
}

pub fn vtype_name(db: &'static So) -> Compose<&'static Col<Vote, Id<VoteType>>, &'static Col<VoteType, Str>> {
    (&db.vote.vote_type).select(&db.vote_type.name)
}

pub fn htype_name(db: &'static So) -> Compose<&'static Col<PostHistory, Id<PostHistoryType>>, &'static Col<PostHistoryType, Str>> {
    (&db.post_history.post_history_type).select(&db.post_history_type.name)
}

/// ORDER BY `key` LIMIT `n` (`n == 0`: no limit). Prints a warning when the
/// rows either side of the cut tie on the key, which is when the query has
/// more than one right answer.
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

/// Every (key, value) a relation drives, in a Vec.
pub fn drain<Q: Drive>(q: Q) -> Vec<(Q::D, Q::R)> {
    let mut v = Vec::new();
    q.drive(|d, r| v.push((d, r)));
    v
}

pub fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

pub fn ucols(db: &'static So, u: Id<User>, cols: &[&str]) -> Vec<V> {
    cols.iter().map(|c| crate::views::user_col(db, u, c)).collect()
}

/// EXTRACT(EPOCH FROM interval) for an interval in microseconds.
pub fn secs(us: i64) -> f64 {
    us as f64 / 1e6
}

/// A materialised list of rows as a relation keyed by position.
pub fn rel<R: Copy>(v: Vec<R>) -> VecRel<usize, R> {
    VecRel::new(v)
}


/// The right side of `LEFT JOIN r ON <condition not mentioning the left>`:
/// every row of `r` if there is one, else the single all-NULL row.
pub fn left_all<R: Copy>(v: Vec<R>) -> VecRel<usize, Option<R>> {
    if v.is_empty() { VecRel::new(vec![None]) } else { VecRel::new(v.into_iter().map(Some).collect()) }
}

// ----- aggregates the b142+ batches keep asking for ----------------------

/// Per user, over `Users LEFT JOIN Posts`: [joined rows, posts, questions,
/// answers, score sum, views present, views sum, latest creation, score > 0,
/// reputation summed over the rows].
pub fn user_posts(db: &'static So) -> Fold<Id<User>, [i64; 10]> {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).opt()))
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN, 0, 0], |a, (r, p)| match p {
            Some((((t, s), v), d)) => [
                a[0] + 1,
                a[1] + 1,
                a[2] + (t == 1) as i64,
                a[3] + (t == 2) as i64,
                a[4] + s,
                a[5] + v.is_some() as i64,
                a[6] + v.unwrap_or(0),
                a[7].max(d),
                a[8] + (s > 0) as i64,
                a[9] + r,
            ],
            None => {
                let mut a = a;
                a[0] += 1;
                a[9] += r;
                a
            }
        })
}

/// `PostTypes LEFT JOIN Posts` grouped by the type: [posts, score sum, views
/// present, views sum], every type included.
pub fn type_left_posts(db: &'static So) -> Fold<Id<PostType>, [i64; 4]> {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    db.post_type
        .group_by(Ident::<PostType>::new())
        .select(of_type.select((&db.post.score).and((&db.post.view_count).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, v)) => [a[0] + 1, a[1] + s, a[2] + v.is_some() as i64, a[3] + v.unwrap_or(0)],
            None => a,
        })
}

pub fn stats_by_type(db: &'static So) -> Fold<Str, [i64; 12]> {
    let Post { score, view_count, answer_count, comment_count, favorite_count, creation_date, .. } = &db.post;
    db.post
        .group_by(ptype_name(db))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN, 0], |a, (((((s, v), an), cc), f), d)| {
            [
                a[0] + 1,
                a[1] + s,
                a[2] + v.is_some() as i64,
                a[3] + v.unwrap_or(0),
                a[4] + an.is_some() as i64,
                a[5] + an.unwrap_or(0),
                a[6] + cc,
                a[7] + f.is_some() as i64,
                a[8] + f.unwrap_or(0),
                a[9] + (s > 0) as i64,
                a[10].max(d),
                0,
            ]
        })
}

pub fn tname(db: &'static So, t: Id<PostType>) -> V {
    V::S(db.post_type.name.get(t).unwrap())
}

/// Per tag over `Tags LEFT JOIN Posts ON p.Tags LIKE '%' || t.TagName || '%'`:
/// [posts, views present, views sum, score sum, questions, answers].
pub fn tag_stats(db: &'static So) -> Fold<Id<Tag>, [i64; 6]> {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    db.tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select((&db.post.view_count).opt().and(&db.post.score).and(&db.post.post_type_id)).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((v, s), t)) => [a[0] + 1, a[1] + v.is_some() as i64, a[2] + v.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64],
            None => a,
        })
}

pub fn distinct_some<T: Ord + Copy>(v: impl IntoIterator<Item = Option<T>>) -> i64 {
    let mut x: Vec<T> = v.into_iter().flatten().collect();
    x.sort_unstable();
    x.dedup();
    x.len() as i64
}

pub fn own_votes(db: &'static So) -> HashIdx<Id<Post>, Id<Vote>> {
    let Vote { user, post, .. } = &db.vote;
    db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect()
}

pub fn fkey(x: f64) -> i64 {
    let b = x.to_bits() as i64;
    b ^ (((b >> 63) as u64) >> 1) as i64
}

/// CURRENT_TIMESTAMP as a UTC instant, epoch microseconds.
pub fn now_utc() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as i64
}

/// COUNT(DISTINCT p.Id) per user over `Users LEFT JOIN Posts`.
pub fn user_distinct_posts(db: &'static So) -> Fold<Id<User>, i64> {
    db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64)
}

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product):
/// [rows with a post, score sum, gold, silver, bronze, class sum, badge id sum, rows].
pub fn user_posts_badges(db: &'static So) -> Fold<Id<User>, [i64; 8]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).select((&db.badge.class).and(&db.badge.origid)).opt()))
        .fold([0i64; 8], |a, (s, b)| {
            let c = b.map(|b| b.0);
            [
                a[0] + s.is_some() as i64,
                a[1] + s.unwrap_or(0),
                a[2] + (c == Some(1)) as i64,
                a[3] + (c == Some(2)) as i64,
                a[4] + (c == Some(3)) as i64,
                a[5] + c.unwrap_or(0),
                a[6] + b.map_or(0, |b| b.1),
                a[7] + 1,
            ]
        })
}

/// Sorts by `key` and pairs each row with its `RANK()` (or `DENSE_RANK()`)
/// over that order.
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




/// `CAST(ts AS VARCHAR)`: seconds, then the fraction with its trailing zeros
/// dropped (none at all when it is zero).
pub fn ts_text(us: i64) -> String {
    let s = crate::time::fmt_ts(us);
    let (head, frac) = s.split_at(19);
    let frac = frac.trim_end_matches('0');
    if frac == "." { head.to_string() } else { format!("{head}{frac}") }
}


/// SQL `LIKE`: '%' matches any run, '_' any one character, no escape.
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
