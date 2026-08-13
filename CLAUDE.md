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

`Prela.PullStaged.Query` is the query-author API and generates tight loops over
the physical stores in `Prela.Storage`.

The staged implementation is split by concept into package-private `Scalar`,
`Relation`, `Generation`, `Predicate`, `Materialize`, and `Consumer` modules.
`Query` is the single supported query-author import and selectively exports the
safe qualified `Q.` vocabulary. Executor representations live in `Stream`,
`Ops`, and the two substantial `.Internal` modules.

`Prela.Cache` validates and loads cache-format-v2 column files into managed
storage. `Prela.Schema` uses Template Haskell to generate entity tags, checked
loaders, universes, and staged accessors. Foreign indices are resolved through
their target universe in generated code.

Materializers in the `Q.*` API are deliberate pipeline boundaries. The staged
surface uses `Gen` do-notation; its continuation scopes ensure generated storage
is bound once even when a materialized relation is used more than once.

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

See `haskell/AUDIT.md` for the current cleanup status and next improvements, and
`haskell/CACHE.md` for cache invariants.
