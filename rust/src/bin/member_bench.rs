// member_bench — Prod::member: flat short-circuit vs probe-derived default.
//
// Compares, for `Prod<A, B>` (conjunction in member position):
//   spec: `a.member(x) && b.member(x)`      (the hand-written Member impl)
//   gen:  `p.probe_any(x, |_| true)`        (what `member_via_probe!` would give)
//
// Leg shapes exercised:
//   - VecRel × VecRel      (dense columns; member ≡ bounds check either way)
//   - MatSet × MatSet      (hash sets; member ≡ contains either way)
//   - Bitset × Bitset      (bit tests)
//   - MultiRel × MultiRel  (CSR, fanout F on A) — the asymmetric case: the
//     probe-derived path re-probes B once per A-row value until B succeeds,
//     so an empty B row costs O(F) B-lookups instead of O(1).
//   - nested Prod triple over VecRels (tuple threading depth 2)
//
// Run: cargo run --release --bin member_bench

use ahash::AHashSet;
use prela::engine::{Bitset, MatSet, MultiRel, Probe, Prod, Universe, VecRel};
use std::hint::black_box;
use std::time::Instant;

const N: usize = 1 << 20; // universe size (1M keys)
const LOOKUPS: usize = 1 << 23; // lookups per timed pass (~8.4M)
const REPS: usize = 9; // timed passes per variant; median reported

// Deterministic xorshift64* — no rand dep, reproducible key streams.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn keys_in(range: usize, seed: u64) -> Vec<usize> {
    let mut rng = Rng(seed);
    (0..LOOKUPS).map(|_| rng.below(range)).collect()
}

// One timed pass over the key stream; returns (ns_per_lookup, hits).
fn time_pass(f: &mut dyn FnMut() -> u64) -> (f64, u64) {
    let t = Instant::now();
    let hits = f();
    let ns = t.elapsed().as_nanos() as f64 / LOOKUPS as f64;
    (ns, black_box(hits))
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

// Interleave spec/gen passes (same cache/branch environment for both),
// report per-variant medians.
fn run_case<P: Probe<D = usize>>(name: &str, p: &P, keys: &[usize]) {
    let mut spec = || {
        let mut hits = 0u64;
        for &x in keys {
            hits += p.member(black_box(x)) as u64;
        }
        hits
    };
    let mut generic = || {
        let mut hits = 0u64;
        for &x in keys {
            hits += p.probe_any(black_box(x), |_| true) as u64;
        }
        hits
    };
    // warmup
    let h_spec = spec();
    let h_gen = generic();
    assert_eq!(h_spec, h_gen, "{name}: variants disagree on hit count");

    let (mut ts, mut tg) = (Vec::new(), Vec::new());
    for _ in 0..REPS {
        ts.push(time_pass(&mut spec).0);
        tg.push(time_pass(&mut generic).0);
    }
    let (ms, mg) = (median(ts), median(tg));
    println!(
        "{name:<44} spec {ms:6.2} ns  gen {mg:6.2} ns  gen/spec {:5.2}x  (hit rate {:.3})",
        mg / ms,
        h_spec as f64 / LOOKUPS as f64
    );
}

fn vecrel(n: usize) -> VecRel<u32, usize> {
    VecRel::new((0..n).map(|i| i as u32).collect())
}

fn matset(n: usize, keep_pct: u64, seed: u64) -> MatSet<usize> {
    let mut rng = Rng(seed);
    let set: AHashSet<usize> = (0..n).filter(|_| rng.next() % 100 < keep_pct).collect();
    MatSet { set }
}

fn bitset(n: usize, keep_pct: u64, seed: u64) -> Bitset<usize> {
    let mut rng = Rng(seed);
    let mut b = Bitset::empty(Universe::<usize>::new(n));
    for i in 0..n {
        if rng.next() % 100 < keep_pct {
            b.set(i);
        }
    }
    b
}

/// CSR MultiRel: every row has `fanout` values with probability
/// `nonempty_pct`, else 0 values. Backing arrays are leaked ('static).
fn multirel(n: usize, fanout: usize, nonempty_pct: u64, seed: u64) -> MultiRel<u32, usize> {
    let mut rng = Rng(seed);
    let mut offsets = Vec::with_capacity(n + 1);
    let mut values = Vec::new();
    offsets.push(0u32);
    for i in 0..n {
        if rng.next() % 100 < nonempty_pct {
            for j in 0..fanout {
                values.push((i * fanout + j) as u32);
            }
        }
        offsets.push(values.len() as u32);
    }
    MultiRel::from_csr(Vec::leak(offsets), Vec::leak(values))
}

fn main() {
    println!(
        "Prod::member — flat `a.member && b.member` (spec) vs probe-derived \
         `probe_any(x, |_| true)` (gen)\nuniverse {N}, {LOOKUPS} lookups/pass, \
         median of {REPS} passes\n"
    );

    let keys_hit = keys_in(N, 1); // all in-universe
    let keys_half = keys_in(2 * N, 2); // ~50% out of range (VecRel misses)

    // --- VecRel legs -----------------------------------------------------
    {
        let p = Prod { a: vecrel(N), b: vecrel(N) };
        run_case("VecRel x VecRel, all hits", &p, &keys_hit);
        run_case("VecRel x VecRel, 50% A-miss", &p, &keys_half);
    }
    // Nested triple: tuple threading two levels deep on the gen path.
    {
        let p = Prod {
            a: Prod { a: vecrel(N), b: vecrel(N) },
            b: vecrel(N),
        };
        run_case("(VecRel x VecRel) x VecRel, 50% miss", &p, &keys_half);
    }

    // --- MatSet legs (hash membership) ------------------------------------
    {
        let p = Prod { a: matset(N, 70, 3), b: matset(N, 70, 4) };
        run_case("MatSet x MatSet, 70%/70% hit", &p, &keys_hit);
        let p = Prod { a: matset(N, 95, 5), b: matset(N, 50, 6) };
        run_case("MatSet x MatSet, A 95% B 50% hit", &p, &keys_hit);
    }

    // --- Bitset legs -------------------------------------------------------
    {
        let p = Prod { a: bitset(N, 70, 7), b: bitset(N, 70, 8) };
        run_case("Bitset x Bitset, 70%/70% hit", &p, &keys_hit);
    }

    // --- Bitset LEAF: hand-written bit-test member vs probe-derived --------
    // (`Bitset::probe_any` is `member(x) && k(x)`, so gen should reduce to
    // the same bit test; measured at several densities and one
    // cache-hostile size.)
    {
        run_case("Bitset leaf, 70% set", &bitset(N, 70, 17), &keys_hit);
        run_case("Bitset leaf, 10% set", &bitset(N, 10, 18), &keys_hit);
        run_case("Bitset leaf, 100% set", &bitset(N, 100, 19), &keys_hit);
        run_case("Bitset leaf, 50% key out-of-range", &bitset(N, 70, 20), &keys_half);
        let big = 1 << 26; // 64M keys → 8 MB of mask words, spills L2
        run_case("Bitset leaf 64M keys (8MB), 70% set", &bitset(big, 70, 21), &keys_in(big, 22));
    }

    // --- MultiRel legs: the asymmetric case --------------------------------
    // A always has `fanout` values; B's row is empty half (or 90%) of the
    // time. gen re-probes B for every A value when B misses.
    {
        let p = Prod {
            a: multirel(N, 8, 100, 9),
            b: multirel(N, 1, 50, 10),
        };
        run_case("MultiRel(F=8) x MultiRel, B 50% empty", &p, &keys_hit);
        let p = Prod {
            a: multirel(N, 8, 100, 11),
            b: multirel(N, 1, 10, 12),
        };
        run_case("MultiRel(F=8) x MultiRel, B 90% empty", &p, &keys_hit);
        let p = Prod {
            a: multirel(N, 32, 100, 13),
            b: multirel(N, 1, 10, 14),
        };
        run_case("MultiRel(F=32) x MultiRel, B 90% empty", &p, &keys_hit);
    }

    // --- Mixed: cheap A, hash B --------------------------------------------
    {
        let p = Prod {
            a: vecrel(N),
            b: Prod { a: matset(N, 70, 15), b: matset(N, 70, 16) },
        };
        run_case("VecRel x (MatSet x MatSet)", &p, &keys_hit);
    }
}
