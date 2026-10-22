use crate::schema::{AiRule, FaceRule, MoverRule, WeaponRule};
use prela::engine::*;
use prela::loader::Col;

// `flag` if `on`, else 0. Used to pack conditions into a bitmask.
pub fn bit(on: bool, flag: u32) -> u32 {
    flag * on as u32
}

// The columns every rule table has: a rule matches facts when (facts & mask) == want; lowest prio wins.
pub trait Rules: Sized + 'static {
    fn prio(&self) -> &Col<Self, u8>;
    fn mask(&self) -> &Col<Self, u32>;
    fn want(&self) -> &Col<Self, u32>;
}

macro_rules! rule_tables {
    ($($t:ty),*) => {$(
        impl Rules for $t {
            fn prio(&self) -> &Col<Self, u8> {
                &self.prio
            }
            fn mask(&self) -> &Col<Self, u32> {
                &self.mask
            }
            fn want(&self) -> &Col<Self, u32> {
                &self.want
            }
        }
    )*};
}
rule_tables!(AiRule, WeaponRule, MoverRule, FaceRule);

// Rule-table lookup: for each row, the highest-priority rule whose conditions match.
// Rows are (row, rule key, fact bits); `by_key` lists each key's rules; `id` names the row
// in the result, a dense id below `n`.
pub fn matching<Q, T, R, F, K>(
    rows: Q,
    rules: &R,
    by_key: &MultiRel<usize, Row<R>>,
    id: F,
    n: usize,
) -> MultiRel<K, (T, Row<R>)>
where
    Q: Drive<R = (T, usize, u32)>,
    T: Copy + 'static,
    R: Rules,
    F: Fn(T) -> K,
    K: Dense,
{
    // Join rows to the rules for their key, keep rules whose mask matches the row's facts.
    rows.key_by(|(_, k, _): (T, usize, u32)| k)
        .and(by_key)
        .key_by(|(_, r)| r)
        .and(rules.mask().and(rules.want()).and(rules.prio()))
        .filt(|(((_, _, f), _), ((mask, want), _)): (((T, usize, u32), Row<R>), ((u32, u32), u8))| f & mask == want)
        // Per row id, keep the rule with the lowest prio number (the first listed on ties).
        .key_by(move |(((t, _, _), _), _): (((T, usize, u32), Row<R>), ((u32, u32), u8))| id(t))
        .dense_fold(n, None, |a: Option<(T, Row<R>, u8)>, (((t, _, _), r), (_, prio)): (((T, usize, u32), Row<R>), ((u32, u32), u8))| match a {
            Some(b) if b.2 <= prio => Some(b),
            _ => Some((t, r, prio)),
        })
        // Unwrap the winner: (row, rule).
        .flat_map(|a: Option<(T, Row<R>, u8)>| a.map(|(t, r, _)| (t, r)))
        .collect()
}

// Decision table: choice_key(key, facts) -> the best matching rule, for every combination of `bits` fact bits.
pub fn decisions<R: Rules>(
    rules: &R,
    by_key: &MultiRel<usize, Row<R>>,
    bits: u32,
) -> MultiRel<usize, Row<R>> {
    let n = by_key.n_dom() << bits;
    // Every rule against every fact combination; keep (key, facts) alongside.
    by_key
        .cross(Universe::<usize>::new(1 << bits))
        .and(Same::<(usize, usize)>::new())
        .key_by(|((r, _), _): ((Row<R>, usize), (usize, usize))| r)
        .and(rules.mask().and(rules.want()).and(rules.prio()))
        .filt(|(((_, f), _), ((mask, want), _)): (((Row<R>, usize), (usize, usize)), ((u32, u32), u8))| f as u32 & mask == want)
        // Lowest prio per (key, facts).
        .key_by(move |((_, (k, f)), _): (((Row<R>, usize), (usize, usize)), ((u32, u32), u8))| choice_key(k, f, bits))
        .dense_fold(n, None, |a: Option<(Row<R>, u8)>, (((r, _), _), (_, prio)): (((Row<R>, usize), (usize, usize)), ((u32, u32), u8))| match a {
            Some(b) if b.1 <= prio => Some(b),
            _ => Some((r, prio)),
        })
        .flat_map(|a: Option<(Row<R>, u8)>| a.map(|(r, _)| r))
        .collect()
}

// (rule key, facts) -> decision-table key, for `bits` fact bits.
pub fn choice_key(key: usize, facts: usize, bits: u32) -> usize {
    (key << bits) + facts
}

// A rule table's key column as an index: key -> its rules, in table order.
pub fn by_key<R: 'static>(key: &Col<R, usize>) -> MultiRel<usize, Row<R>> {
    key.inv().collect()
}
