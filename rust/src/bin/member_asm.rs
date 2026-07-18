// member_asm — disassembly fixtures for the member_prod experiment.
//
// Exposes each (leg shape × variant) as a `#[no_mangle] #[inline(never)]`
// symbol so the two member paths can be compared instruction-for-
// instruction under the SAME build profile as member_bench:
//
//   otool -tv target/release/member_asm
//
// spec_* : the hand-written flat short-circuit `a.member(x) && b.member(x)`
// gen_*  : the probe-derived default `p.probe_any(x, |_| true)`

use prela::engine::{MatSet, Member, MultiRel, Probe, Prod, VecRel};
use std::hint::black_box;

type PVec = Prod<VecRel<u32, usize>, VecRel<u32, usize>>;
type PMulti = Prod<MultiRel<u32, usize>, MultiRel<u32, usize>>;
type PSet = Prod<MatSet<usize>, MatSet<usize>>;

#[unsafe(no_mangle)]
#[inline(never)]
pub fn spec_vecrel(p: &PVec, x: usize) -> bool {
    p.member(x)
}
#[unsafe(no_mangle)]
#[inline(never)]
pub fn gen_vecrel(p: &PVec, x: usize) -> bool {
    p.probe_any(x, |_| true)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn spec_multirel(p: &PMulti, x: usize) -> bool {
    p.member(x)
}
#[unsafe(no_mangle)]
#[inline(never)]
pub fn gen_multirel(p: &PMulti, x: usize) -> bool {
    p.probe_any(x, |_| true)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn spec_matset(p: &PSet, x: usize) -> bool {
    p.member(x)
}
#[unsafe(no_mangle)]
#[inline(never)]
pub fn gen_matset(p: &PSet, x: usize) -> bool {
    p.probe_any(x, |_| true)
}

fn main() {
    // Keep every symbol reachable so LTO can't drop them; results printed
    // so the calls aren't dead either.
    let pv = Prod {
        a: VecRel::<u32, usize>::new(vec![1, 2, 3]),
        b: VecRel::<u32, usize>::new(vec![4, 5]),
    };
    let pm = Prod {
        a: MultiRel::<u32, usize>::from_csr(Vec::leak(vec![0u32, 2, 2, 3]), Vec::leak(vec![7, 8, 9])),
        b: MultiRel::<u32, usize>::from_csr(Vec::leak(vec![0u32, 0, 1, 1]), Vec::leak(vec![5])),
    };
    let ps = Prod {
        a: MatSet { set: [0usize, 2].into_iter().collect() },
        b: MatSet { set: [0usize, 1].into_iter().collect() },
    };
    let x = black_box(0usize);
    println!(
        "{} {} {} {} {} {}",
        spec_vecrel(black_box(&pv), x),
        gen_vecrel(black_box(&pv), x),
        spec_multirel(black_box(&pm), x),
        gen_multirel(black_box(&pm), x),
        spec_matset(black_box(&ps), x),
        gen_matset(black_box(&ps), x),
    );
}
