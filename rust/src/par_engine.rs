use ahash::AHashMap as HashMap;
use std::hash::Hash;

use crate::engine::{Compose, Dense, Drive, Fold, InvStream, Probe, VecRel};
use crate::sched::{Reduce, run_interval};

/// parallel drive: `k` adds each pair to a partial result `A`; split-off pieces start from `rd.init()` and are merged back in range order
pub trait DrivePar: Drive + Sync {
    /// drive `k` over `self`, accumulating into partial result `acc`.
    /// split pieces are merged using `rd`
    fn drive_par<A, Rd, K>(&self, rd: &Rd, acc: &mut A, k: &K)
    where
        A: Send,
        Rd: Reduce<A>,
        K: Fn(&mut A, Self::D, Self::R) + Sync;
}

/// forward through references, since plans are built from them
impl<T: DrivePar + ?Sized> DrivePar for &T {
    #[inline(always)]
    fn drive_par<A, Rd, K>(&self, rd: &Rd, acc: &mut A, k: &K)
    where
        A: Send,
        Rd: Reduce<A>,
        K: Fn(&mut A, T::D, T::R) + Sync,
    {
        (**self).drive_par(rd, acc, k);
    }
}

/// create one interval over the vecrel
impl<D: Dense + Sync, R: Copy + Sync> DrivePar for VecRel<D, R> {
    #[inline(always)]
    fn drive_par<A, Rd, K>(&self, rd: &Rd, acc: &mut A, k: &K)
    where
        A: Send,
        Rd: Reduce<A>,
        K: Fn(&mut A, D, R) + Sync,
    {
        let v = &self.v[..];
        run_interval(
            rd,
            acc,
            0,
            v.len(),
            |_| (),
            |acc, _, i| k(acc, D::from_idx(i), v[i]),
        );
    }
}

impl<A: DrivePar, B: Probe<D = A::R> + Sync> DrivePar for Compose<A, B> {
    #[inline(always)]
    fn drive_par<Ac, Rd, K>(&self, rd: &Rd, acc: &mut Ac, k: &K)
    where
        Ac: Send,
        Rd: Reduce<Ac>,
        K: Fn(&mut Ac, A::D, B::R) + Sync,
    {
        self.a.drive_par(rd, acc, &|acc: &mut Ac, x, m| {
            self.b.probe(m, |r| k(acc, x, r))
        });
    }
}

/// swap each pair of the parallel drive
impl<Q: DrivePar> DrivePar for InvStream<Q>
where
    Q::R: Eq + Hash,
{
    #[inline(always)]
    fn drive_par<A, Rd, K>(&self, rd: &Rd, acc: &mut A, k: &K)
    where
        A: Send,
        Rd: Reduce<A>,
        K: Fn(&mut A, Q::R, Q::D) + Sync,
    {
        self.q.drive_par(rd, acc, &|acc: &mut A, d, r| k(acc, r, d));
    }
}

impl<D: Copy + Eq + Hash, S: Copy> Fold<D, S> {
    /// parallel `build`: each piece folds into its own map; maps merge per key with `combine(earlier, later)`
    pub fn build_par<Q, OP, CB>(q: Q, init: S, op: OP, combine: CB) -> Self
    where
        Q: DrivePar<D = D>,
        D: Send,
        S: Send + Sync,
        OP: Fn(S, Q::R) -> S + Sync,
        CB: Fn(S, S) -> S + Sync,
    {
        let rd = (
            || HashMap::<D, S>::new(),
            |m: &mut HashMap<D, S>, later: HashMap<D, S>| {
                for (d, s) in later {
                    m.entry(d).and_modify(|e| *e = combine(*e, s)).or_insert(s);
                }
            },
        );
        let mut m = rd.init();
        q.drive_par(&rd, &mut m, &|m: &mut HashMap<D, S>, d, v| {
            let s = m.entry(d).or_insert(init);
            *s = op(*s, v);
        });
        Fold { cache: m }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::QueryExt;

    fn par_people() -> (VecRel<usize, usize>, VecRel<usize, usize>) {
        crate::sched::init(4, std::time::Duration::from_micros(100));
        let people = VecRel::new((0..2_000_000).map(|i| i % 1000).collect());
        let country = VecRel::new((0..1000).map(|c| c % 7).collect());
        (people, country)
    }

    #[test]
    fn drive_par_compose_matches_drive() {
        let (people, country) = par_people();
        let q = (&people).select(&country);
        let mut want = Vec::new();
        q.drive(|d, r| want.push((d, r)));
        let rd = (Vec::new, |a: &mut Vec<_>, b: Vec<_>| a.extend(b));
        let before = crate::sched::splits();
        let mut got = Vec::new();
        q.drive_par(&rd, &mut got, &|a: &mut Vec<_>, d, r| a.push((d, r)));
        assert_eq!(got, want);
        assert!(crate::sched::splits() > before);
    }

    #[test]
    fn fold_build_par_matches_build() {
        let (people, country) = par_people();
        let q = || (&people).select(&country).inv();
        let want = Fold::build(q(), (0u64, 0usize), |(n, s), p| (n + 1, s + p));
        let got = Fold::build_par(
            q(),
            (0u64, 0usize),
            |(n, s), p| (n + 1, s + p),
            |(n1, s1), (n2, s2)| (n1 + n2, s1 + s2),
        );
        assert_eq!(got.cache, want.cache);
        assert_eq!(got.cache.len(), 7);
    }
}
