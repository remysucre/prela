# Repository guidance

Prela is a research-prototype embedded query language based on Tarski's Algebra
of Relations. The Haskell implementation lives in `haskell/`.

## Data model

Everything is a binary relation: a collection of `(domain, range)` pairs. SQL
tables are shredded into one relation per column, keyed by phantom-typed entity
identifiers such as `Id Movie`. `Id` is abstract and is obtained by checked
lookup in a `Universe Movie`. Missing values and nullable foreign keys are absent
pairs, not sentinel identifiers.

## Commands

Run these from `haskell/` with GHC 9.10 and Cabal 3.x:

```bash
cabal build all
cabal test prela-test --test-show-details=direct

# Requires the shared binary TPC-H cache (default ../cache).
cabal run prela-tpch
cabal run prela-tpch -- 1 6 14
```

`PRELA_CACHE` overrides the cache directory and `ROUNDS` controls repetitions.

## Query conventions

- Give reusable generated schema leaves `Relation` signatures; executor modules
  select `Stream` or `Lookup` internally.
- Use `collect`, `foldAll`, or another consumer to leave the relation algebra.
- Bind materializers with `Gen` do-notation to preserve generated sharing.
- Keep generated schema accessors as top-level functions; local signatures are
  important because associated storage types enable `MonoLocalBinds`.
- Treat cache bytes as untrusted. Add validation before constructing a storage
  value whenever the on-disk format grows.
