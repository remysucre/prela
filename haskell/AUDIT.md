# Haskell implementation audit

The Haskell tree now has two implementations: a direct pure pull semantics and
a staged pull executor over validated column storage. The old callback-based
Push backend and its obsolete design prototypes have been removed.

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

- `Prela.PullStaged` is now an umbrella module parallel to `Prela.Pull`.
- The package exposes no Push modules or Push executable.
- Tests and the Pull demo construct identifiers through universes and model
  missing references as absent pairs.
- The unstaged `index` materializer now accumulates in linear time rather than
  appending to each group repeatedly.

## Remaining worthwhile improvements

### Narrow the public storage surface

`Prela.Storage` still exports representation constructors because generated
schema code and low-level experiments use them. A cleaner package split would
move constructors, hash-table internals, and producer machinery to `.Internal`
modules while keeping checked smart constructors and query combinators public.

### Make dense materializers universe-directed

`foldDense`, `foldDenseOuter`, and `bitset` still accept a raw extent. Accepting
`Universe e` would make their domain assumption explicit and avoid repeating a
size that the schema already knows.

### Reduce staging ceremony

Staged queries still expose `CodeQ`, typed quotes, `lam1`, and
continuation-shaped materializers. A small generation monad and staged-literal
wrapper could preserve the current implementation while presenting a more
ordinary query surface.

### Automate performance contracts

The correctness suite now checks IDs, sparse-universe parity, cache validation,
nullable references, plain Pull behavior, and staged schema queries. The next
useful gate is automated Core/allocation checking for representative fused
queries, plus the existing 22-query TPC-H oracle run as a slow test target.

## Verification expected for this change

- Build the library, Pull demo, tests, and staged TPC-H executable.
- Run the test suite and Pull demo.
- Scan active source, tests, applications, and TPC-H code for unchecked
  operation imports or calls.
- Run the TPC-H oracle suite when the local cache is available.
