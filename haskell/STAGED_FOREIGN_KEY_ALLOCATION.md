# Staged foreign-key allocation diagnosis

## Summary

The staged executor successfully fuses scans, filters, scalar products, keyed
membership tests, and dense reducers into direct loops. An allocation audit
found that the large historical allocation in TPC-H Q4 and Q5 was not caused by
retaining `Stream` or `Producer` values at runtime, nor by the bitset or
dense-fold materializers. It came from driven foreign-key traversal through the
word-backed `SparseCol` representation.

Before the fix, each successful foreign-key read crossed two optional-value
boundaries:

```text
stored Word64
    -> Maybe Int
    -> Maybe (Id target)
    -> downstream stream continuation
```

At `-O1`, GHC does not eliminate these constructors across the nested producer
continuations. A successful driven foreign-key read therefore allocates about
32--33 bytes. Keyed membership probes generally do not have this problem:
`probeAny` keeps the read and its Boolean consumer in one expression, so the
temporary values cancel.

The implemented fix generates a direct foreign-key branch which reads the
stored word, tests the hole sentinel, validates the target universe, and calls
`yield` or `skip` without constructing either `Maybe` value. A specialized
reference-column leaf supplies direct `Stream` and `Lookup` implementations;
generated query source and handwritten query definitions are unchanged.

## Measurement context

These measurements were taken on 2026-08-12 using:

- TPC-H scale factor 1;
- the trusted mmap loader;
- GHC 9.10.3;
- an explicit `-O1` build;
- generated functions compiled with `-fno-full-laziness`; and
- RTS allocation counters around the query body, excluding schema loading.

Temporary diagnostic programs lived under `/tmp` and were removed from the
working tree. No diagnostic instrumentation was retained; the executor change
described below is the production implementation.

## Implementation outcome

`SMode.referenceColumn` is now the physical leaf for generated one-valued
reference accessors. `Prela.Schema` emits that leaf automatically. Multi-valued
references retain the general CSR-plus-resolution path.

An inlinable storage eliminator, `withSparseIntAt`, handles both checked boxed
storage and trusted word-backed storage without constructing a new `Maybe` on
the hot path. The reference leaf then validates source liveness and target
liveness/range before passing an internal `Id` newtype directly to the generated
continuation.

At `-O1`, using the same SF1 cache and exact allocation counters around the
query body:

| Measurement | Before | After |
|---|---:|---:|
| Q4 query-body allocation | about 125--126 MB | 2.45 MB |
| Q5 query-body allocation | about 75--76 MB | 0.022 MB |
| complete 22-query allocation | 2.47 GB | 1.41 GB |
| warm 22-query runtime | median 3.77 s | 2.61 s |

The complete suite therefore lost approximately 1.06 GB, or 43%, of cumulative
query-body allocation. Q4 lost about 98% and Q5 more than 99%. Warm Q4 measured
approximately 133--142 ms after the change, and warm Q5 approximately 162--171
ms. All 22 queries matched their oracles in both verification rounds.

Focused tests now cover:

- holes;
- out-of-range raw target indices;
- dead targets in sparse universes;
- direct scan versus keyed access; and
- checked boxed versus trusted word-backed reference storage.

## Query-plan follow-up

With the reference path fixed, the remaining allocation belonged mostly to
explicit physical plans: large dense states, hash aggregates, result lists, and
sorting. The staged queries were subsequently rewritten to make the intended
algorithms more explicit:

- Q2 hoists selective supplier/part predicates, aggregates only eligible
  partsupp rows, keeps a bounded top 100, and attaches wide payload columns
  afterward;
- Q10 and Q15 use dense folds over bounded entity ids;
- Q12 tests the selective receipt-date predicate before its other conjuncts;
- Q18 selects a bounded top 100 before customer/name projection;
- Q20 restricts the `(part, supplier)` shipment aggregate to a precomputed
  bitset of forest-named parts;
- Q21 packs two small supplier state machines into one dense `Int` and retains
  only its ordered top 100; and
- Q3 also performs its final top 10 inside generated code.

The executor now exposes an explicit bounded `topK` materializer. Its mutable
buffer is capped by the requested result count and the retained rows re-enter
the stream pipeline, so later projections still fuse normally. This is a
physical operation chosen in query source; the compiler does not infer it.

Candidate rewrites for Q3/Q8/Q11/Q13/Q19 which added predicate bitsets or a
different outer-fold plan were benchmarked and rejected when they were slower
or did not reduce allocation. The written query remains the plan rather than a
target for speculative optimizer rewrites.

After the retained query rewrites:

| Measurement | After reference leaf | After query rewrites |
|---|---:|---:|
| complete 22-query allocation | 1.41 GB | 0.98 GB |
| warm 22-query runtime | 2.61 s | 2.16--2.18 s |

The query pass removed another 427 MB (about 30%) from the post-leaf suite.
Relative to the original staged implementation, total cumulative allocation is
down about 60% and warm runtime about 43%.

The final 0.98 GB is about 2.13 times the optimized Rust suite's 0.46 GB and
1.13 times the idiomatic Rust suite's 0.87 GB. GHC heap allocation and Rust
global-allocator requests are not identical accounting systems, so these ratios
are directional rather than byte-for-byte ABI comparisons.

## Initial allocation comparison

Across all 22 TPC-H queries, cumulative query-body allocation was:

| Implementation | Allocation |
|---|---:|
| staged Haskell | 2.47 GB |
| Rust, idiomatic plans | 870 MB |
| Rust, optimized plans | 461 MB |

This comparison includes explicit materializer storage and result construction,
so it does not by itself identify a fusion failure. Several scan-shaped queries
show that the basic fusion path works:

| Query | Staged Haskell | Rust optimized |
|---|---:|---:|
| Q1 | about 156 KB | about 16 KB |
| Q6 | about 5.4 KB | 27 B |
| Q7 | below RTS sampling granularity | about 1.3 KB |
| Q12 | below RTS sampling granularity | 425 B |
| Q17 | below RTS sampling granularity | about 67 KB |

Q6 scans roughly six million lineitems while combining date, discount, and
quantity predicates with two projected doubles and a scalar fold. Allocating
only a few kilobytes for the entire query is strong evidence that the normal
scan/filter/product/fold pipeline is fused.

The suspicious cases were queries whose physical result was too small to
justify their allocation:

| Query | Staged Haskell | Rust optimized |
|---|---:|---:|
| Q4 | 126 MB | 0.75 MB |
| Q5 | 75 MB | about 1 KB |

Q4 builds a bitset and a tiny priority-count table. Q5 reduces into a dense
nation table with only 25 possible slots. Neither result can explain tens of
megabytes of allocation.

## Focused Q5 isolation

Q5 was reduced one operator at a time. Each variant ran over the same
date-qualified lineitems.

| Variant | Result cardinality | Allocation |
|---|---:|---:|
| date restriction followed by `count` | 910,519 | approximately zero |
| compose through supplier to nation | 910,519 | 61.99 MB |
| re-key the same rows with `groupBy` | 910,519 | 62.02 MB |
| add a dense count fold | 25 | 62.02 MB |
| add projected price/discount and revenue reducer | 25 | 62.02 MB |
| use Q5's full same-nation key | 7,243 input rows | 76.03 MB |
| complete Q5 | 5 result rows | 76.03 MB |

This establishes three things:

1. The date-filtered producer is allocation-free.
2. The allocation appears when the driven rows navigate foreign-key columns.
3. `groupBy`, the dense materializer, projected scalar products, and the
   reducer add essentially no further allocation.

The simple supplier-to-nation path performs two successful foreign-key reads
per qualifying row:

```text
910,519 rows * 2 reads * about 32 bytes = about 58.3 MB
```

That accounts for nearly all of the measured 61.99 MB. Q5's additional
customer/nation navigation explains the increase to approximately 76 MB.

## Q4 cross-check

Q4 provides an even cleaner confirmation:

| Variant | Cardinality | Allocation |
|---|---:|---:|
| late-lineitem predicate followed by `count` | 3,793,296 | approximately zero |
| compose each late line through `liOrder` | 3,793,296 | 125.33 MB |
| complete Q4 | 5 result rows | 125.26 MB |

The added operation is one successful foreign-key read per late lineitem:

```text
125.33 MB / 3,793,296 reads = about 33 bytes per read
```

The complete query allocates no more than the isolated navigation. Its bitset
construction and grouped priority count are therefore not the source of the
regression.

## Former execution path

Schema generation represents a reference field structurally as:

```haskell
compose
  (compose (universe source) (sparseColumn rawIndices))
  (resolveId targetUniverse)
```

The trusted loader stores the raw indices in `SparseWordCol`, backed by a
storable vector of `Word64`. A hole is represented by `maxBound`.

The first optional-value boundary is `sparseAt`:

```haskell
sparseAt (SparseWordCol values) i
  | i < 0 || i >= V.length values = Nothing
  | value == maxBound             = Nothing
  | otherwise                     = Just (fromIntegral value)
  where
    value = values V.! i
```

The staged `Lookup.at` implementation for sparse columns calls `sparseAt` and
pattern-matches on its `Maybe`. The value is then sent through a nested
`Bind`. `resolveId` calls `lookupId`, which returns another `Maybe`, and sends
the resulting `Id` to the next continuation.

The generated Core contains direct loops and no runtime `Stream` tree, so
staging itself is working. The problem is narrower: at `-O1`, case-of-constructor
cancellation does not cross all of these generated continuation boundaries.
The successful path retains boxed optional values and integers.

The keyed probe path is different. `probeAny` nests the accepting predicates
directly:

```haskell
probeAny a key (\middle -> probeAny b middle accept)
```

There is no driven one-row stream between the read and the consumer. GHC can
normally reduce the optional read to a branch, which is why restrictions and
membership tests remain cheap.

## Required generated code

A trusted word-backed reference read should generate code equivalent to:

```haskell
let raw = foreignKeys[index]
in if raw == holeWord
     then skip nextState
     else
       let targetIndex = fromIntegral raw
       in if targetIndex < targetExtent && targetIsLive targetIndex
            then yield sourceId (Id targetIndex) nextState
            else skip nextState
```

This representation has no intermediate `Maybe Int`, no
`Maybe (Id target)`, and no runtime query object. `Id` is a `newtype`, so its
constructor should erase after the validation branch is inlined.

Bounds checks must remain correct. In particular, the generated condition must
reject negative indices when the intermediate type is `Int`, reject indices at
or beyond the target extent, and consult the target liveness mask for sparse
target entities.

## Implementation options

### Option 1: specialize the existing low-level leaves

The smallest change is to make the staged `SparseWordCol` producer inspect its
vector and sentinel directly instead of calling `sparseAt`, and to make
`resolveId.at` validate the universe directly instead of calling `lookupId`.

This removes both named `Maybe` boundaries while preserving the existing
schema-generated composition. The boxed `SparseCol` constructor used by tests
and checked/small inputs can retain its ordinary `Maybe` branch. The hot trusted
branch should be selected by pattern matching once in generated code.

Advantages:

- small implementation change;
- no public API change;
- preserves the existing relational decomposition; and
- directly targets the measured constructors.

Risks:

- the two direct leaves are still connected by nested one-row stream machinery;
- future changes could reintroduce an optimization barrier; and
- the complete foreign-key invariant remains split across two leaves.

### Option 2: add a specialized reference-column leaf (implemented)

The more robust design is a low-level executor operation such as:

```haskell
referenceColumn
  :: CodeQ (Universe source)
  -> CodeQ (SparseCol source Int)
  -> CodeQ (Universe target)
  -> q (Id source) (Id target)
```

Schema generation would use this operation for reference fields. Its driven
implementation would perform the raw read, hole test, and target validation in
one producer step. Its keyed implementation would perform the same operation
as one direct `probeAny` or one-row `at` branch.

Advantages:

- one physical leaf for what is already one schema relation;
- no intermediate stream or optional-value representation;
- source and target validation are visible in one place;
- robust at `-O1`, rather than relying on cross-combinator simplification; and
- the public `Id` constructor remains abstract.

The specialized leaf is the implemented solution. It is not a query optimizer
rewrite: a declared foreign-key field is already one logical leaf. This only
gives that leaf a direct physical implementation.

## Semantic constraints

Any fix must preserve the following behavior:

- a cache hole produces no relational pair;
- a raw target index outside the target universe produces no pair;
- a target index marked dead by a sparse universe produces no pair;
- scanning preserves the source relation's traversal order;
- keyed access and scanning agree on presence and value;
- no unchecked `Id` can escape the validated executor boundary; and
- the checked and trusted storage constructors remain observationally
  equivalent.

The optimization must not turn a missing or invalid reference into a fabricated
identifier merely to avoid allocating `Maybe`.

## Verification plan

### Correctness tests

Add focused staged tests for:

- a present reference;
- a hole;
- an out-of-range raw target index;
- a reference to a dead target in a sparse universe;
- scan/keyed parity; and
- both `SparseCol` and `SparseWordCol` storage forms.

Then run the existing Haskell test suite and all 22 TPC-H oracle checks.

### Allocation contracts

Add focused allocation checks around generated functions. The important
relationships are:

```text
allocation(dateCount)      ~= allocation(composeCount)
allocation(lateCount)      ~= allocation(lateOrderCount)
allocation(simpleCount)    ~= allocation(simpleDense)
allocation(simpleDense)    ~= allocation(simpleRevenue)
```

The first two are the new regression gates: adding a functional foreign-key
navigation must not add allocation proportional to the number of successful
rows.

Allow a small constant tolerance for RTS bookkeeping, materializer setup, and
result construction. Do not assert exact zero bytes or an exact global total.

### Generated-code inspection

For at least one focused reference scan, inspect optimized Core or STG and
confirm that the successful row loop does not allocate:

- `Just`;
- a boxed intermediate `Int`;
- a one-row list or stream value; or
- an intermediate pair used only to cross the reference boundary.

The generated program should contain a raw vector read followed by sentinel and
universe branches leading directly to the downstream continuation.

### Performance check

Re-run Q4, Q5, and the complete suite at `-O1`. Based on the isolation above:

- Q4 should lose approximately 125 MB of cumulative allocation;
- Q5 should lose approximately 60--75 MB; and
- runtime should improve or remain stable, with less GC pressure.

The exact residual totals will include explicit physical state and output
construction. Success means eliminating allocation proportional to successful
foreign-key reads, not forcing every query to report zero allocation.

## Further work

Turn the focused allocation relationships above into an automated slow
regression target. The current implementation has semantic tests and was
manually checked against optimized allocation counters, but exact heap totals
should not be part of the ordinary correctness suite because RTS bookkeeping
varies across compiler and runtime versions.
