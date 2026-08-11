# Haskell implementation audit

The executor internals are thoughtful and well measured, but too many implementation constraints leak into semantics and query authoring. The most “Haskelly” improvement is not replacing `do` blocks—it is making invalid states unrepresentable and hiding optimizer machinery behind a smaller API.

## Highest-priority issues

### 1. Sparse universes are semantically inconsistent

A sparse universe enumerates only live IDs, but keyed access accepts every in-range ID in both `Prela.Push.Ops` and `Prela.PullStaged.Ops`. The tests explicitly preserve this discrepancy in `test/Spec.hs`.

That means the same relation denotes different sets depending on how it is accessed. The justification—dead IDs cannot arrive—is unenforced because `Id(..)` is public.

Recommended changes:

- Immediately make sparse lookup check the validity bit.
- Make `Id` abstract, exposing checked construction and an explicitly named `unsafeId`.
- Longer-term, distinguish validated `LiveId e` values if eliminating that bit test is measurably important.

The unstaged Pull implementation already gives sparse universes consistent enumeration and lookup semantics, so it provides the correct reference behavior.

### 2. `noId` leaks a storage sentinel into relational semantics

`noId` is physically useful, but it is presently a valid `Id e` value exposed publicly in `Prela.Id`. Consequently, grouping a nullable foreign key creates a real `noId` group, which the tests also preserve.

Keep `-1` in the cache representation, but decode absence at the relational boundary. A nullable reference should be represented as presence plus value—like `SparseCol`—rather than as an ordinary domain value. Then “missing” consistently means no pair.

### 3. Schema invariants are implicit and fragile

An entity’s size comes from its first field, and a sparse entity’s validity comes from holes in its first foreign key. Nothing validates that the remaining columns have the same extent.

Introduce an explicit `Universe e` or `Domain e` value loaded once per entity, containing its size and optional validity. Every column should be checked against it at load time. This would also let `foldDenseOuter` and `withDenseOuter` accept evidence of a dense universe instead of a raw `Int`; currently their correctness precondition exists only in a comment.

The cache reader should similarly validate:

- Word64-to-Int conversions.
- Negative or overflowing sizes.
- Monotonic CSR and string offsets.
- Final offsets against their corresponding payload sizes.

These checks must happen before constructing unsafe views. Current bounds arithmetic in `Prela.Cache` can itself overflow on malformed input.

## API and ergonomics

### 4. The public API exposes almost the whole implementation

`import Prela` reexports storage internals, while the Push modules have no export lists. PullStaged has no umbrella module, yet exports constructors such as `Producer(..)`, `Stream(..)`, `Lookup(..)`, and even mutable table internals.

Provide three curated facades:

- `Prela.Push`
- `Prela.Pull`
- `Prela.Staged`

Move representation constructors, `filtPKV`, mutable tables, unsafe storage access, and generation helpers under `.Internal`. This also solves much of the naming problem: internal modules can use ordinary names such as `Producer.filter` and `Stream.filter` through qualified imports instead of acronyms or elaborate overloading.

### 5. Staging mechanics dominate staged queries

Users currently see `CodeQ`, `[|| … ||]`, `lam1`, continuation-shaped materializers, a separate splice module, and `-fno-full-laziness`.

The continuation encoding is technically sound, but it should be hidden behind a generation monad:

```haskell
q s = runGen $ do
  totals <- fold step zero grouped
  used   <- bitset domain selected
  pure (render totals used)
```

Internally this can remain exactly the existing CPS implementation. This is a good use of `do`: it expresses generation-time sequencing and sharing, while generated runtime queries remain pure.

For predicates, introduce a staged scalar wrapper so the common case becomes:

```haskell
gt 1980 year
eq "EUROPE" regionName
gt (code cutoff) year
```

That keeps the same function name while accepting literals ergonomically and retaining an explicit escape hatch for dynamic generated expressions.

### 6. Backend duplication signals a missing abstraction—but merge cautiously

The three predicate modules repeat the same algebra, and all 22 TPC-H queries have separate Push and staged versions. Ultimately, a finally-tagless surface with an associated scalar representation could let one query definition target Push, Pull, and staged Pull.

Do not merge the hot executor internals yet. First unify literals, predicates, schema storage, and differential tests. Only then prototype shared query definitions and require Core and allocation equivalence before adopting them.

## Concrete smaller fixes

### 7. The unstaged Pull reference has two avoidable problems

- `sfold` is a lazy left fold. Make its accumulator strict by default and expose a separately named lazy fold only if needed.
- `index` preserves order with repeated `old ++ [new]`, making large groups quadratic. Accumulate by cons and reverse each bucket once.

These are small, conventional Haskell fixes with no architectural risk.

### 8. Performance guarantees need to become tests

The existing probes are excellent, but `design/CoreProbe.hs` and `design/StagedProbe.hs` are not built automatically. The ordinary test suite covers only five small staged queries and no direct unstaged Pull operator suite.

Add:

- Small randomized differential tests across Push, Pull, and PullStaged.
- Dedicated tests for sparse universes, nullable references, `union`, `limit`, `anyOf`, lockstep skipping, and every materializer.
- Automated Core assertions and allocation ceilings for representative query shapes.
- The 22-query oracle run as an optional slow CI group.
- A GHC-version matrix, because several guarantees depend on simplifier behavior.

## What should remain

Keep the existential pull `Producer`, CPS consumers, `ST` materializers, top-level generated schema accessors, zero-copy cache storage, and explicit materialization boundaries. These have concrete performance justification. The numerous `do` blocks in `ST`, `IO`, and Template Haskell are idiomatic; removing them would mostly obscure sequencing.

Move the long benchmark histories out of source comments into design notes, retaining only short invariants and links. Several current comments refer to `MODES.md` and `FUSION.md`, which are deleted in the current worktree, and the root README presents only the Rust interface.

## Verification performed

- The test binary passes.
- The staged 22-query TPC-H run matches the recorded oracles.
- A forced build revealed four unused-binder warnings in generated Q9, Q16, and Q21 code.
- `cabal check` reports that the package would be rejected because its license and important upper bounds are missing.

## Recommended implementation order

1. Establish semantic parity and validate loader invariants.
2. Add differential correctness, Core, and allocation regression tests.
3. Introduce curated public facades and hide internals.
4. Add the `Gen` and staged-scalar ergonomic layer.
5. Prototype shared backend-independent query definitions only after the regression gates exist.
