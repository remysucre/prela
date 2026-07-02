# prela sandbox

A standalone playground for writing and running prela queries against a
small, hand-seeded database — no cache files, no JOB/TPC-H data required.

```bash
cd prela/sandbox
cargo run
```

## Layout

- `src/schema.rs` — declares a small "company" schema (`Department`,
  `Employee`, `Skill`, `Project`) with the `schema!` macro from
  `prela::schema`, and `load()` seeds it directly in memory (`VecRel::new`,
  `MultiRel::from_csr`) instead of reading a binary cache.
- `src/main.rs` — a handful of example queries over that schema, with an
  operator quick-reference in the header comment.

## Extending it

- **New entities/fields**: add to the `schema! { ... }` block in
  `schema.rs`, then add matching columns to the `Store { ... }` literal in
  `load()`. Field types: `str`, `i64`, `f64`, a bare entity name (FK),
  `Multi<T>` (multi-valued FK or `Multi<str>`). See
  `prela/rust/src/schema.rs` for the full macro reference and
  `prela/rust/src/job_schema.rs` for a larger worked example.
- **New queries**: write them in `main.rs` (or a new module) using the
  combinators from `prela::engine::QueryExt` — `.select`, `.with`, `.and`,
  `.eq`/`.gt`/`.lt`/`.rx`/…, `.group_by` + `.fold`/`.dense_fold`. Every
  query ends in `.drive(|domain, value| ...)` or `.probe(id, |value| ...)`.
