use crate::info::Rule;
use prela::engine::*;

// `flag` if `on`, else 0. Used to pack conditions into a bitmask.
pub fn bit(on: bool, flag: u32) -> u32 {
    flag * on as u32
}

// Rule-table lookup: for each row, pick the highest-priority rule whose conditions match.
pub fn matching<Q, T, O, F>(rows: Q, rules: &MultiRel<usize, Rule<O>>, id: F) -> HashIdx<usize, (T, O)>
where
    Q: Drive<R = (T, usize, u32)>,
    T: Copy,
    O: Copy + 'static,
    F: Fn(T) -> usize,
{
    // Join rows to rules by key, keep rules whose mask matches the row's facts.
    rows.key_by(|(_, k, _): (T, usize, u32)| k)
        .and(rules)
        .filt(|((_, _, f), r): ((T, usize, u32), Rule<O>)| f & r.mask == r.want)
        // Per row id, keep the rule with the lowest prio number.
        .key_by(move |((t, _, _), _): ((T, usize, u32), Rule<O>)| id(t))
        .fold(None, |a: Option<(T, Rule<O>)>, ((t, _, _), r): ((T, usize, u32), Rule<O>)| match a {
            Some(b) if b.1.prio <= r.prio => Some(b),
            _ => Some((t, r)),
        })
        .flat_map(|a: Option<(T, Rule<O>)>| a.map(|(t, r)| (t, r.out)))
        .collect()
}
