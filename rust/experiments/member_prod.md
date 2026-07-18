# Experiment: `Prod::member` — flat short-circuit vs probe-derived default

**Question.** Is the hand-written `Member` impl on `Prod<A, B>`

```rust
fn member(&self, x) -> bool { self.a.member(x) && self.b.member(x) }   // "spec"
```

measurably faster than the universal probe-derived default it overrides,

```rust
fn member(&self, x) -> bool { self.probe_any(x, |_| true) }            // "gen"
// = a.probe_any(x, |av| b.probe_any(x, |bv| true))
```

including when both legs are `VecRel`s?

**Why it might be.** The gen path (a) threads pair values `(av, bv)` through the
continuations, and (b) on paper degrades to O(|row_A|) B-lookups when B misses:
`a.probe_any` retries the inner `b.probe_any(x, …)` for *every* value in A's
row until one succeeds, so an A-row of fanout F with an empty B-row costs F
B-probes instead of 1. The spec path never builds pairs and does exactly one
`member` per leg.

## Setup

`src/bin/member_bench.rs` (`cargo run --release --bin member_bench`). Universe
2²⁰ keys, 2²³ random lookups per pass (xorshift64\*, fixed seeds), median of 9
passes, spec/gen passes interleaved, hit counts asserted equal across variants,
`black_box` on the key and the accumulated hit count. Release profile as in
Cargo.toml: `lto = "fat"`, `codegen-units = 1`.

Machine: Apple M2, rustc 1.96.0.

## Results

| case                                    | spec ns | gen ns | gen/spec | hit rate |
|-----------------------------------------|--------:|-------:|---------:|---------:|
| VecRel × VecRel, all hits               |    0.60 |   0.60 |    1.01× |    1.000 |
| VecRel × VecRel, 50% A-miss             |    0.59 |   0.59 |    1.01× |    0.500 |
| (VecRel × VecRel) × VecRel, 50% miss    |    0.59 |   0.57 |    0.97× |    0.500 |
| MatSet × MatSet, 70%/70% hit            |   25.89 |  25.88 |    1.00× |    0.490 |
| MatSet × MatSet, A 95% B 50% hit        |   25.04 |  24.48 |    0.98× |    0.475 |
| Bitset × Bitset, 70%/70% hit            |    5.53 |   5.55 |    1.00× |    0.490 |
| MultiRel(F=8) × MultiRel, B 50% empty   |    4.24 |   4.29 |    1.01× |    0.500 |
| MultiRel(F=8) × MultiRel, B 90% empty   |    4.19 |   4.27 |    1.02× |    0.100 |
| MultiRel(F=32) × MultiRel, B 90% empty  |    4.22 |   4.26 |    1.01× |    0.100 |
| VecRel × (MatSet × MatSet)              |   28.49 |  26.91 |    0.94× |    0.489 |

A second run reproduces every row within ±3%; ratios in 0.94–1.03 are noise.

## Conclusion

**No measurable difference in any configuration, VecRel legs included.** With
full inlining (static plan types + fat LTO), LLVM reduces both forms to the
same code:

- **VecRel legs**: both paths compile down to two bounds checks
  (~0.6 ns/lookup ≈ 2 cycles — the loop is effectively a vectorized compare
  over the key stream). The pair value the gen path nominally builds is dead
  and eliminated.
- **MultiRel legs — the theoretical O(F) blow-up does not materialize.** The
  inner `b.probe_any(x, |_| true)` is loop-invariant with respect to A's row
  scan (pure, captures nothing, independent of `av`), so LLVM hoists it: the
  gen path becomes `!row_a.is_empty() && !row_b.is_empty()`, identical to
  spec. The tell: gen time is flat between F=8 and F=32 at 90% B-empty
  (4.27 vs 4.26 ns) — a real re-scan would add ~F−1 row lookups ≈ tens of ns.
- **MatSet / Bitset legs**: both paths are two `contains` / two bit-tests;
  hash probing (~25 ns, cache-miss bound) dwarfs any structural difference.

So the hand-written `Prod::member` override is **not a performance
optimization** under this build configuration. Its justification is
structural: it bounds the legs on `Member` only (`A: Member, B: Member`),
letting member-only nodes like `Disj` appear as conjuncts, whereas the
probe-derived default requires both legs to be `Probe`.

**Caveats.** The equivalence rests on the optimizer: it holds for statically
typed plans under `lto = "fat"`, `codegen-units = 1`. Behind a `dyn` boundary,
without LTO, or with a continuation that isn't loop-invariant, the hoisting
argument no longer applies and the flat form is the only guaranteed-O(1)-per-leg
one. Keeping the override is free insurance; just don't credit it with speed.
