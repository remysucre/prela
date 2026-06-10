# Does `recursion_relation` make the interpreted engine fast?

**No.** The inference-recursion-limit failure is real and visible, but on
relational workloads it is not where the time goes, and patching
`recursion_relation` does not recover staged-engine behavior on the real
combinator library (unlike on the toy CPS chains in `../why_no_inline.jl`).
Staging is the architecture; this experiment closes the "could we just
patch the interpreter?" question.

Setup: `experiment_recursion.jl`, 2M-node synthetic linked entity
(`val`, random `next`), Apple Silicon, Julia 1.12.6. One process per mode;
`interp-rr` patches `recursion_relation = (a...) -> true` on **557 methods**
(the drive/probe protocol + every continuation closure in the Prela module)
before any inference. `prep` = index/cache builds (all scans through the
engine under test); `scan` = final scan, min of 3, warm.

| query        | interp scan | interp-rr scan | staged scan | interp bytes | interp-rr bytes | staged bytes |
|--------------|------------:|---------------:|------------:|-------------:|----------------:|-------------:|
| chain1       |     0.77 ms |        0.77 ms |     0.77 ms |         64 MB|            64 MB|         48 B |
| chain2       |     3.25 ms |        3.33 ms |     3.47 ms |        256 B |            64 MB|         64 B |
| chain4       |     6.34 ms |        5.91 ms |     6.09 ms |        544 B |            64 MB|         80 B |
| chain8       |    16.93 ms |       15.40 ms |    16.05 ms |       1.8 kB |            64 MB|        112 B |
| wide6        |     3.55 ms |        3.57 ms |     3.55 ms |        64 MB |            64 MB|        144 B |
| chain4+fold  |    16.63 ms |       16.57 ms |  **4.91 ms**|        20 MB |            20 MB|         32 B |
| chain4+fold prep | 122.7 ms |      115.3 ms |    115.6 ms |              |                 |              |

Findings:

1. **The widening is real but wall-clock-invisible on pipeline scans.** The
   interpreted engine demonstrably runs boxed dynamic calls per row — the
   64 MB/scan (= 32 B × 2M rows) of ID boxing is the receipt. (Where the
   boxed value is a small Int, Julia's interned-box cache hides the
   allocation but not the dispatch — that's why deep chains show ~0 bytes.)
   Yet times match the staged engine at every depth: these scans are bound
   by random-access memory latency, and the dispatch + boxing hides
   entirely in the shadow of the cache misses.

2. **`recursion_relation` does not fix the interpreter.** With all 557
   methods patched the allocation profile gets *worse-or-equal* (every
   chain depth now boxes 64 MB/scan) and no query gets faster. On the toy
   chains the patch was sufficient because every method in the tower was
   ours; the real engine's towers cross methods we cannot meaningfully
   patch (Dict iteration/insertion, Base internals) and at this type
   complexity other inlining limits (cost model, union-splitting budgets)
   bind as well. The escape hatch does not scale beyond toys here.

3. **Where the staged engine actually wins: scans over Dict-backed physical
   state.** Iterating a `FoldP` cache is 3.4× faster staged (4.9 vs
   16.6 ms) and garbage-free (32 B vs 20 MB of `Pair` boxes). Hash-insert-
   dominated *builds* are engine-insensitive here (≈116 ms everywhere) —
   consistent with the gen-branch finding that routing builds through
   codegen pays on TPC-H where builds do cheaper per-row work (bitsets,
   dense folds) and hashing doesn't dominate.

## Field results — full JOB suite (113 queries, IMDB, warm, `-t1`)

Both engines pass all 113 oracles (`ENGINE=staged|interp test_td.jl`).
Totals over the whole suite (`bench.jl job`), plus a third run with the
625-method `recursion_relation` patch applied before any compilation:

| engine                  | total   | vs staged (median per-query) |
|-------------------------|--------:|-----------------------------:|
| staged                  |  9.63 s |                          1.0× |
| interp                  | 14.59 s |                         1.75× |
| interp + recursion_relation | 13.95 s |                     1.64× |

Unlike the latency-bound synthetic chains, real JOB queries do light
per-row work (selective filters, folds, membership probes), so the
interpreted engine's overhead shows: 1.5× end-to-end, 1.75× median, up to
25× on small index-heavy queries (6c, 33a/b). The patch claws back ~4%
end-to-end — real, but marginal against a 50% gap.

Bottom line: the interpreted engine stays in-tree as the executable spec
and as `Interp()` for A/B runs, the staged engine stays the default, and
`recursion_relation` is retired as a performance strategy for Prela.
