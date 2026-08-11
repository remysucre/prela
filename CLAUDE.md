# Repository guidance

Prela is a research-prototype embedded query language based on Tarski's Algebra
of Relations. The Haskell implementation lives in `haskell/`.

## Data model

Everything is a binary relation: a collection of `(domain, range)` pairs. SQL
tables are shredded into one relation per column, keyed by phantom-typed entity
identifiers such as `Id Movie`. `Id` is abstract and is obtained by checked
lookup in a `Universe Movie`. Missing values and nullable foreign keys are absent
pairs, not sentinel identifiers.

## Haskell architecture

There are two query implementations:

- `Prela.Pull` is the direct, pure reference semantics over ordinary Haskell
  values. `Stream` enumerates pairs and `Lookup` performs keyed access.
- `Prela.PullStaged` has the same relational surface but generates tight loops
  over the physical stores in `Prela.Storage`.

`Prela.Cache` validates and loads cache-format-v2 column files into managed
storage. `Prela.Schema` uses Template Haskell to generate entity tags, checked
loaders, universes, and staged accessors. Foreign indices are resolved through
their target universe in generated code.

Materializers in `Prela.Pull.Materialize` and
`Prela.PullStaged.Materialize` are deliberate pipeline boundaries. The staged
variants use continuation arguments so generated storage is bound once even
when a materialized relation is used more than once.

## Commands

Run these from `haskell/` with GHC 9.10 and Cabal 3.x:

```bash
cabal build all
cabal test prela-test --test-show-details=direct
cabal run prela-demo-pull

# Requires the shared binary TPC-H cache (default ../cache).
cabal run prela-tpch
cabal run prela-tpch -- 1 6 14
```

`PRELA_CACHE` overrides the cache directory and `ROUNDS` controls repetitions.

## Query conventions

- Keep reusable leaves mode-polymorphic (`Mode q => ...` or `SMode q => ...`)
  and select `Stream` or `Lookup` at the boundary.
- Use `collect`, `foldAll`, or another consumer to leave the relation algebra.
- Bind staged materializers through their `with...` continuations to preserve
  sharing.
- Keep generated schema accessors as top-level functions; local signatures are
  important because associated storage types enable `MonoLocalBinds`.
- Treat cache bytes as untrusted. Add validation before constructing a storage
  value whenever the on-disk format grows.

See `haskell/AUDIT.md` for the current cleanup status and next improvements, and
`haskell/CACHE.md` for cache invariants.
