# Haskell implementation audit

The Haskell tree contains only the staged executor over validated column
storage. Obsolete backends and design prototypes have been removed.

## Changes completed

### Identifier and universe semantics

- `Id` is abstract. Its constructor and the old sentinel value are no longer
  public.
- `lookupId` is the normal construction boundary and validates both range and
  sparse-universe liveness.
- Sparse universes now mean the same thing when enumerated and when used for
  keyed lookup.
- Nullable foreign keys are represented as absence (`Maybe`/`SparseCol`) in
  storage. A cache hole never becomes an `Id` or a relational value.
- Generated schema accessors resolve stored foreign indices through the target
  universe, dropping missing, out-of-range, and dead references.

### Checked cache and staged code

- The cache reader uses managed `ByteString` reads and checked slicing/indexing;
  it no longer reinterprets mapped memory.
- Header counts, file lengths, integer conversions, string offsets, and CSR
  offsets are validated before storage is constructed.
- Schema loading verifies that every column belonging to an entity has the same
  extent.
- Generated record projection uses liftable field tags and ordinary typed class
  dispatch. It does not use a typed-code coercion.
- Staged physical leaves use checked vector/array operations after explicit
  bounds validation.

### API and maintenance

- `Prela.PullStaged.Query` is the single supported staged query-author import;
  the concept modules are package-private and `Stream`/`Ops` form the executor
  layer.
- The package exposes no Push modules or Push executable.
- Tests construct identifiers through universes and model missing references as
  absent pairs.
- `Prela.Storage` exposes abstract storage types, checked construction and safe
  inspection only. Physical stores, cache-only constructors, key arrays, and
  hash-table operations live in `Prela.Storage.Internal`.
- Staged `Stream`/`Lookup` types are abstract. Existential `Producer` state and
  plan constructors live in `Stream.Internal`, and mutable materializer helpers
  are no longer part of the public API.

## Remaining worthwhile improvements

### Make dense materializers universe-directed

`foldDense`, `foldDenseOuter`, and `bitset` still accept a raw extent. Accepting
`Universe e` would make their domain assumption explicit and avoid repeating a
size that the schema already knows.

### Reduce staging ceremony

Implemented. Package-private `Scalar`, `Relation`, `Generation`, `Predicate`,
`Materialize`, and `Consumer` modules each own one part of the staged
implementation. `Prela.PullStaged.Query` is the explicit public façade and
selectively exports the author vocabulary. Predicates accept ordinary literals, scalar
tuple destructuring and common byte operations stay quote-free, materializers
use `do` notation, and complete queries use `Q.query`/`Q.compile`. All 22 TPC-H
builders are quote-free. They return typed rows or scalars; ordinary `renderN`
functions perform result sorting, limiting, and formatting after compilation.
Typed quotation remains confined to the façade implementation and low-level
executor modules. Generated products use only `pair`/`onPair`; larger products
nest rather than expanding into an arity-specific family of tuple combinators.

### Automate performance contracts

The correctness suite now checks IDs, sparse-universe parity, cache validation,
nullable references, and staged schema queries. The next
useful gate is automated Core/allocation checking for representative fused
queries, plus the existing 22-query TPC-H oracle run as a slow test target.

## Verification expected for this change

- Build the library, tests, and staged TPC-H executable.
- Run the test suite.
- Scan active source, tests, applications, and TPC-H code for unchecked
  operation imports or calls.
- Run the TPC-H oracle suite when the local cache is available.
