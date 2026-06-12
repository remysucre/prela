# Pull (external iterator) vs push (CPS) under monomorphization

## Question

The engine (`src/engine.rs`) is push-style: `Drive::drive<K: FnMut(D, R)>` /
`Probe::probe<K: FnMut(R)>` — continuations are loop bodies, and a compose
nest monomorphizes into literal nested loops with no resumption state. Does
an external-iterator (pull) protocol monomorphize to comparable code, given
that std's adapters forward `fold`/`try_fold` (internal iteration)? And how
much does consumption style — raw `next()` via a `for` loop vs `try_fold`
via `.fold()`/`.for_each()` — matter at JOB's chain depths?

## Design

Two protocols over the **same node types and the same eager state** — not a
second engine:

| push (engine.rs) | pull (pull.rs) |
|---|---|
| `Drive::drive(&self, k: FnMut(D, R))` | `Iterate::iter(&self) -> impl Iterator<Item = (D, R)> + '_` (RPITIT) |
| `Probe::probe(&self, x, k)` / `probe_any` | `ProbeIter::probe_iter(&self, x) -> impl Iterator<Item = R> + '_` |
| `Probe::member(x)` (default `probe_any(x, \|_\| true)`) | `ProbeIter::member_p(x)` (default `probe_iter(x).next().is_some()`) |

Every node implements exactly the modes its push twin does (`Disj` stays
probe-only, `Union`/`InvStream`/`GroupBy` stay drive-only), so a pull-mode
error is the same compile error. Faithfulness rule: the pull hot path never
calls push `drive`/`probe`/`probe_any`. Membership inside pull combinators
goes through `member_p`; leaves with a direct test (Universe bound check,
Bitset bit-test, MatSet hash lookup) override it with the *same* test push
uses, and their `probe_iter` is the member-gated `once` shape
(`cond.then_some(x).into_iter()`) so the defaulted `.next().is_some()`
inlines to the direct test anyway. `Prod::member_p` keeps the flat
short-circuit AND. The one place the protocols are structurally different:
push `member` is a bool-returning continuation chain; pull `member_p` must
construct a nested iterator and ask it for one element.

Combinators map 1:1: `Compose::probe_iter(x) = a.probe_iter(x).flat_map(|m|
b.probe_iter(m))`; `Compose::iter = a.iter().flat_map(|(x, m)|
b.probe_iter(m).map(move |r| (x, r)))`; `Prod` probes b per a-value like
push; `Restrict` filters on `b.member_p(v)`; `Diff` on the key.

Pipeline breakers get pull-side builders that consume via
`.fold()`/`.for_each()` — internal iteration, which std forwards through
`fold`/`try_fold`, i.e. pull's best shot at push-shaped loops:
`Fold::build_pull` / `build_buf_pull`, `DenseFold::build_pull`,
`Bitset::over_pull`, `FromQueryPull` (`.collect_p`), plus `PullExt` sugar
(`.fold_p`, `.buf_fold_p`, `.dense_fold_p`, `.count_distinct_p`,
`.collect_p`; `.group_by` and all constructors are shared with push via
`QueryExt`). The `*_next` builder variants and `min_row_pull_next` consume
via a raw `for` loop (desugars to `next()` calls) for the consumption-style
axis.

## Ports

- **JOB** (`src/queries_pull/`, suite `job-pull`): all 113 queries + the
  `6a/method` demo, plans byte-identical to `src/queries/` (verified by
  `diff` modulo the sink rename) — only `min_row` → `min_row_pull`.
- **TPC-H idiomatic** (`src/tpch/idiomatic_pull.rs`, suite `tpch-pull`, QS
  ignored): all 22 queries, plans identical; spellings `.fold(` → `.fold_p(`,
  `.count_distinct()` → `.count_distinct_p()`, `.collect()` → `.collect_p()`,
  `.unwrap_fold(i, op)` → `.iter().fold(i, |a, (_, v)| op(a, v))`,
  `q.drive(|k, v| rows.push((k, v)))` → `rows.extend(q.iter())`, other drive
  sinks → `.iter().for_each(...)`. Oracles come from the base registry via
  `with_overrides`, so passing means byte-equality with the same strings the
  push suite checks. `optimized`/`ddbcheat` are out of scope.
- **pull-next** (`src/pull_next.rs`, suite `pull-next`): 6-query subset —
  JOB q1a (shallow), q2a, q29a (deep t6 chain); TPC-H Q1, Q9, Q21 — each
  registered TWICE in the same process: the pull suite's fold-style runner
  (the exact monomorphized function `job-pull`/`tpch-pull` time, looked up
  from the registry) and a raw-`next()` variant with every consuming sink
  swapped to a `for` loop. Same-process pairing removes the cache-context
  confound.

Porting deviations (all cosmetic):
1. Opaque helper signatures (`film_or_warner_co`, `co_28`, `gf_*`, …) keep
   `+ Drive + Probe` alongside `+ Iterate + ProbeIter`: the shared `.in_s`
   constructor bounds its argument by push `Probe`. This is a construction-
   time trait bound only — the pull execution path never calls push methods.
2. `pull_next.rs` redefines the two 1-line tpch formatters locally
   (`tpch::common` is private to the tpch tree) and spells out
   `count_distinct`'s sort-dedup closure to route it through
   `build_buf_pull_next`.

## Methodology

- Machine otherwise idle-ish; same binary for all measurements (fat LTO +
  `codegen-units = 1`, so everything was re-measured after the last code
  change). 3 process runs per suite; the harness runs two timed rounds per
  process; numbers below are round-2 per-query timings from the best run
  (lowest round-2 total).
- Run-to-run variance of round-2 totals: JOB push 5.82–5.97 s, JOB pull
  7.81–8.01 s, TPC-H push 1.167–1.187 s, TPC-H pull 1.212–1.233 s — all
  ≤ 2.5 %, well under the 10 % reporting bar. (Single-digit-ms queries
  bounce a few hundred µs run to run; ratios for those carry ~±0.1.)
- Correctness: `cargo check --all-targets` zero warnings; `cargo test` 18/18
  (incl. new pull tests: compose iterate vs push, Prod `probe_iter` pairing,
  Restrict member semantics, Diff/Disj/Union, all pull builders);
  `job-pull` 114/114 ok, `tpch-pull` 22/22 ok against the same oracles;
  JOB result strings byte-identical to the push suite's output after
  stripping timings (`diff` clean). TPC-H result strings are compared
  byte-exactly against the oracles by the harness itself, and both suites
  pass 22/22 against the same oracle strings.

## Totals (round-2, best of 3 runs)

| suite | push | pull-fold | pull/push |
|---|---:|---:|---:|
| JOB (114 q) | 5.82 s | 7.81 s | **1.34** |
| TPC-H idiomatic (22 q) | 1.167 s | 1.212 s | **1.04** |

pull-next subset (same process, min of 3 runs; push column from the full
push suite for reference):

| query | push (ms) | pull-fold (ms) | pull-next (ms) | next/fold |
|---|---:|---:|---:|---:|
| JOB 1a | 4.70 | 16.20 | 12.20 | 0.75 |
| JOB 2a | 13.60 | 21.00 | 13.50 | 0.64 |
| JOB 29a | 14.40 | 25.40 | 21.00 | 0.83 |
| TPC-H Q1 | 70.50 | 58.20 | 66.70 | 1.15 |
| TPC-H Q9 | 318.90 | 324.60 | 317.10 | 0.98 |
| TPC-H Q21 | 315.80 | 299.50 | 300.80 | 1.00 |

(JOB pull-fold mini-suite timings match the full-suite pull timings within
noise — 2a/fold 21.0 ms vs 21.2 ms — so the two tables are comparable.)

## Per-query: TPC-H idiomatic

| query | push (ms) | pull (ms) | pull/push |
|---|---:|---:|---:|
| 1 | 70.50 | 64.80 | 0.92 |
| 2 | 11.40 | 18.30 | 1.61 |
| 3 | 12.90 | 15.10 | 1.17 |
| 4 | 60.70 | 63.00 | 1.04 |
| 5 | 10.70 | 16.10 | 1.50 |
| 6 | 9.40 | 9.60 | 1.02 |
| 7 | 19.00 | 33.20 | 1.75 |
| 8 | 16.90 | 24.00 | 1.42 |
| 9 | 318.90 | 323.00 | 1.01 |
| 10 | 17.80 | 17.80 | 1.00 |
| 11 | 2.60 | 4.90 | 1.88 |
| 12 | 40.30 | 47.00 | 1.17 |
| 13 | 65.70 | 68.70 | 1.05 |
| 14 | 8.90 | 9.30 | 1.04 |
| 15 | 7.00 | 8.40 | 1.20 |
| 16 | 16.90 | 18.00 | 1.07 |
| 17 | 39.80 | 43.10 | 1.08 |
| 18 | 49.20 | 51.30 | 1.04 |
| 19 | 31.90 | 25.40 | 0.80 |
| 20 | 25.50 | 26.20 | 1.03 |
| 21 | 315.80 | 309.60 | 0.98 |
| 22 | 15.20 | 15.60 | 1.03 |
JOB ratio distribution: median 1.45, mean 1.56, min 0.99, max 3.52;
6 queries ≤ 1.05, 56 in 1.05–1.5, 52 > 1.5.

## Per-query: JOB

| query | push (ms) | pull (ms) | pull/push |
|---|---:|---:|---:|
| 1a | 4.70 | 16.40 | 3.49 |
| 1b | 4.70 | 16.10 | 3.43 |
| 1c | 4.60 | 16.20 | 3.52 |
| 1d | 4.30 | 14.90 | 3.47 |
| 2a | 13.60 | 21.60 | 1.59 |
| 2b | 13.00 | 21.70 | 1.67 |
| 2c | 12.50 | 20.60 | 1.65 |
| 2d | 13.90 | 21.80 | 1.57 |
| 3a | 80.00 | 90.90 | 1.14 |
| 3b | 78.00 | 88.30 | 1.13 |
| 3c | 80.90 | 91.90 | 1.14 |
| 4a | 79.70 | 84.90 | 1.07 |
| 4b | 79.60 | 89.20 | 1.12 |
| 4c | 81.40 | 89.60 | 1.10 |
| 5a | 15.70 | 23.20 | 1.48 |
| 5b | 16.00 | 23.10 | 1.44 |
| 5c | 17.70 | 25.10 | 1.42 |
| 6a | 8.80 | 15.20 | 1.73 |
| 6b | 6.00 | 10.80 | 1.80 |
| 6c | 5.20 | 10.70 | 2.06 |
| 6d | 101.60 | 116.30 | 1.14 |
| 6e | 14.30 | 23.60 | 1.65 |
| 6f | 67.10 | 81.40 | 1.21 |
| 7a | 19.80 | 37.40 | 1.89 |
| 7b | 11.30 | 15.30 | 1.35 |
| 7c | 89.60 | 130.40 | 1.46 |
| 8a | 33.40 | 62.20 | 1.86 |
| 8b | 19.70 | 33.70 | 1.71 |
| 8c | 152.10 | 187.10 | 1.23 |
| 8d | 89.00 | 116.70 | 1.31 |
| 9a | 102.20 | 165.00 | 1.61 |
| 9b | 60.90 | 100.10 | 1.64 |
| 9c | 198.20 | 308.50 | 1.56 |
| 9d | 177.10 | 335.20 | 1.89 |
| 10a | 20.40 | 39.10 | 1.92 |
| 10b | 13.20 | 30.00 | 2.27 |
| 10c | 362.30 | 393.40 | 1.09 |
| 11a | 13.80 | 25.10 | 1.82 |
| 11b | 11.40 | 22.20 | 1.95 |
| 11c | 19.20 | 29.10 | 1.52 |
| 11d | 16.10 | 28.80 | 1.79 |
| 12a | 71.80 | 83.60 | 1.16 |
| 12b | 15.30 | 21.70 | 1.42 |
| 12c | 78.60 | 90.80 | 1.16 |
| 13a | 17.60 | 28.20 | 1.60 |
| 13b | 37.40 | 51.80 | 1.39 |
| 13c | 35.50 | 48.10 | 1.35 |
| 13d | 30.80 | 60.80 | 1.97 |
| 14a | 26.20 | 39.00 | 1.49 |
| 14b | 19.90 | 30.10 | 1.51 |
| 14c | 28.20 | 39.60 | 1.40 |
| 15a | 35.20 | 41.80 | 1.19 |
| 15b | 18.50 | 28.90 | 1.56 |
| 15c | 34.80 | 49.10 | 1.41 |
| 15d | 38.00 | 55.00 | 1.45 |
| 16a | 18.00 | 34.50 | 1.92 |
| 16b | 86.80 | 130.60 | 1.50 |
| 16c | 24.60 | 43.10 | 1.75 |
| 16d | 23.30 | 42.80 | 1.84 |
| 17a | 75.70 | 95.30 | 1.26 |
| 17b | 121.60 | 131.50 | 1.08 |
| 17c | 119.10 | 119.20 | 1.00 |
| 17d | 159.60 | 167.50 | 1.05 |
| 17e | 46.30 | 60.30 | 1.30 |
| 17f | 159.20 | 157.10 | 0.99 |
| 18a | 134.10 | 191.20 | 1.43 |
| 18b | 80.50 | 90.10 | 1.12 |
| 18c | 155.50 | 189.00 | 1.22 |
| 19a | 122.70 | 145.50 | 1.19 |
| 19b | 67.30 | 81.90 | 1.22 |
| 19c | 142.60 | 207.10 | 1.45 |
| 19d | 116.10 | 242.50 | 2.09 |
| 20a | 74.00 | 85.30 | 1.15 |
| 20b | 34.90 | 37.20 | 1.07 |
| 20c | 31.50 | 37.90 | 1.20 |
| 21a | 79.40 | 79.20 | 1.00 |
| 21b | 79.50 | 78.70 | 0.99 |
| 21c | 79.60 | 78.80 | 0.99 |
| 22a | 81.50 | 97.50 | 1.20 |
| 22b | 83.20 | 96.80 | 1.16 |
| 22c | 89.80 | 104.50 | 1.16 |
| 22d | 76.70 | 101.70 | 1.33 |
| 23a | 12.40 | 21.50 | 1.73 |
| 23b | 11.80 | 21.20 | 1.80 |
| 23c | 12.60 | 25.10 | 1.99 |
| 24a | 76.60 | 94.80 | 1.24 |
| 24b | 18.20 | 22.30 | 1.23 |
| 25a | 54.60 | 82.10 | 1.50 |
| 25b | 52.00 | 66.30 | 1.27 |
| 25c | 95.40 | 151.30 | 1.59 |
| 26a | 32.10 | 45.10 | 1.40 |
| 26b | 15.60 | 23.00 | 1.47 |
| 26c | 30.90 | 38.50 | 1.25 |
| 27a | 20.60 | 26.60 | 1.29 |
| 27b | 20.60 | 27.30 | 1.33 |
| 27c | 25.70 | 32.50 | 1.26 |
| 28a | 15.00 | 28.60 | 1.91 |
| 28b | 13.30 | 27.70 | 2.08 |
| 28c | 21.00 | 35.30 | 1.68 |
| 29a | 14.40 | 25.20 | 1.75 |
| 29b | 12.90 | 22.10 | 1.71 |
| 29c | 14.10 | 19.40 | 1.38 |
| 30a | 12.30 | 21.80 | 1.77 |
| 30b | 10.20 | 18.70 | 1.83 |
| 30c | 21.00 | 42.00 | 2.00 |
| 31a | 32.00 | 41.70 | 1.30 |
| 31b | 31.20 | 37.60 | 1.21 |
| 31c | 32.50 | 41.60 | 1.28 |
| 32a | 10.40 | 19.90 | 1.91 |
| 32b | 13.10 | 22.90 | 1.75 |
| 33a | 4.40 | 13.20 | 3.00 |
| 33b | 4.70 | 11.70 | 2.49 |
| 33c | 10.20 | 15.00 | 1.47 |
| 6a/method | 8.80 | 15.00 | 1.70 |
## Analysis

**Where pull matches push — and why.** TPC-H is within 4 % in aggregate and
within noise on most queries. These plans are scan-shaped: one drive over a
dense leaf (`VecRel`/`Universe`), a stack of `Filter`/`Restrict`/`Compose`
adapters, one group-fold at the bottom. The pull builders consume with
`.for_each()`/`.fold()`, and std's adapters (`flat_map`, `filter`, `map`)
forward `fold`/`try_fold` all the way down to the leaf slice iterator — so
the monomorphized result is the same fused loop nest the CPS engine emits,
modulo register allocation. The two heavy queries (Q9 325 ms, Q21 300 ms)
land at 1.01x and 0.98x. Pull even *wins* on Q1 (0.92x) and Q19 (0.80x):
identical algorithms, the codegen lottery just rolls differently when the
accumulator is threaded through `fold`'s return value instead of a `FnMut`
capture.

**Where pull diverges.** JOB is 1.34x in aggregate, median 1.45x, worst
3.5x — and the TPC-H outliers (Q2 1.61, Q5 1.50, Q7 1.75, Q8 1.42, Q11
1.88, all small in absolute terms) are exactly the JOB-shaped ones:
join-dense plans dominated by *probe-side* work. Two structural reasons:

1. **Membership tests.** Push `member` is `probe_any(x, |_| true)` — a
   bool-returning continuation chain that short-circuits as plain `&&`
   control flow. Pull `member_p` is `probe_iter(x).next().is_some()`: every
   `Restrict`/`Diff` member test constructs a nested
   `flat_map`/`filter`-of-`Option` state machine, asks it for one element,
   and throws it away. It inlines, but LLVM has to materialize the
   `Option<Item>` plumbing and the adapters' resumption invariants instead
   of a bare branch tree. JOB's hot loops are `Universe`-drives where every
   element runs a 3–7-leg conjunct of such tests.
2. **Probe-position products.** `Prod::probe_iter`/`Compose::probe_iter`
   build nested `flat_map` iterators *per probed key* (JOB projections are
   `note × title × year` triples probed per surviving movie); push just
   nests three closures.

**Does chain depth correlate?** Only weakly. The deep t5/t6 chains (29a
1.7x, 30c 2.0x, 33a 3.0x) are bad, but the *worst* offenders are the
shallow template-1 queries (1a–1d, 3.4–3.5x) — short chains of cheap
predicates over small intermediate sets, i.e. ~all plumbing, ~no payload.
And the deep-but-regex-bound queries (17b–17f, 21a–21c, 10c) sit at
0.99–1.08x because regex matching swamps protocol overhead. The predictor
is not depth but **the fraction of per-tuple time spent in protocol
plumbing vs real work** (regex, string compares, hashing).

**The next()-loop penalty — there isn't one; sometimes it's a bonus.** The
expected result was `for`-loop consumption (raw `next()`, resumption state
live across iterations) losing to `.fold()` (internal iteration). Measured,
same process, same plans: on TPC-H's flat group-folds they are equal (Q9
0.98, Q21 1.00) or fold modestly wins (Q1: next is 1.15x of fold). But on
the JOB min_row shapes the `for` loop *beats* `.fold()` by 17–36 % (q2a:
13.5 ms vs 21.0 ms — the for-loop variant matches push's 13.6 ms exactly).
With everything force-inlined, `FlatMap::fold`'s fully-fused body appears
to lose more to register pressure in the deep nests than the next()-based
loop loses to resumption state. So at these depths consumption style is a
±40 % shape-dependent wash, not the decisive axis — the decisive cost is
the probe/member protocol above.

**Compile time.** Full-crate release rebuild (fat LTO, codegen-units = 1,
touch src/main.rs): 25.8 s without the pull modules → 54.6–55.2 s with
them, ~2.1x — every plan now monomorphizes under both protocols, and the
pull side instantiates the std adapter tower per node. Roughly: the pull
protocol costs as much compile time as the entire rest of the engine.

## Verdict: is CPS necessary in Rust?

For *driving* — scanning a plan into a fold — no. `Iterator` with internal-
iteration consumption (and even, at these depths, plain `for` loops) is a
faithful, equally-fast compilation target: rustc+LLVM erase the protocol
exactly as they erase the closures, and TPC-H lands within noise. A pull
engine would have been fine for that half of the workload, and `Iterator`
buys interop (`collect`, `extend`, adapter reuse) that the CPS traits
re-implement by hand.

For *probing* — and above all for membership, which is most of what a
worst-case-optimal-ish JOB plan does per tuple — push is structurally
better in Rust today. `probe_any`'s `FnMut(R) -> bool` short-circuit has no
zero-cost `Iterator` analogue: `member_p` must build an iterator to drop
it, and that costs a real, robust 1.3–3.5x on membership-dense queries
(JOB 5.8 s → 7.8 s). The asymmetric design this experiment suggests is
push (or at least bool-returning visitors) for the probe/member protocol,
with either protocol fine on the drive side; symmetric pull costs ~34 % on
JOB, and symmetric push is what the engine already does. CPS is not
*necessary* — but for this engine's probe-heavy half it is the cheaper
abstraction, and it is also ~2x cheaper to compile.
