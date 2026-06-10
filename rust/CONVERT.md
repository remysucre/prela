# Free-fn → method-chain conversion guide

Every existing query in `src/queries/t*.rs` uses free-function syntax
(`compose(a, b)`, `restrict(s, q)`, …). The extension traits in
`src/engine.rs` (`QueryExt`, `SetQExt`) now expose the same operators as
**method calls with short names**. Rewrite every call.

## Name mapping (1:1)

| free fn                  | method form               |
|--------------------------|---------------------------|
| `restrict(s, q)`         | `s.o(q)`                  |
| `compose(a, b)`          | `a.o(b)`                  |
| `conj(a, b)`             | `a.and(b)`                |
| `disj(a, b)`             | `a.or(b)`                 |
| `set_diff(a, b)`         | `a.minus(b)`              |
| `prod(a, b)`             | `a.x(b)`                  |
| `keys(q)`                | `q.k()`                   |
| `eq(q, v)`               | `q.eq(v)`                 |
| `ne(q, v)`               | `q.ne(v)`                 |
| `gt(q, v)`               | `q.gt(v)`                 |
| `lt(q, v)`               | `q.lt(v)`                 |
| `ge(q, v)`               | `q.ge(v)`                 |
| `le(q, v)`               | `q.le(v)`                 |
| `in_vec(q, vs)`          | `q.in_v(vs)`              |
| `in_set(q, s)`           | `q.in_s(s)`               |
| `regex_match(q, re)`     | `q.rx(re)`                |
| `regex_not(q, re)`       | `q.nrx(re)`               |

Note `restrict` and `compose` both map to `.o` — Rust's trait dispatch picks
the right implementation (SetQ::o vs Query::o) from the receiver's traits.
**Same algebra, one method name.**

## Parentheses rule

`&` binds looser than `.`, so any leaf borrow as a method receiver needs
parens:

```rust
// before
compose(&d.movie_keyword, &d.keyword_keyword)
// after
(&d.movie_keyword).o(&d.keyword_keyword)
```

Universe `d.movie` is `Copy` — no parens needed:
```rust
restrict(d.movie, ...)
    →   d.movie.o(...)
```

When the receiver is itself a method-chain or a function call (e.g.
`co_21(d)`), no parens needed:
```rust
compose(co_21(d), &d.company_name)
    →   co_21(d).o(&d.company_name)
```

## Nested calls collapse to chains

```rust
// before
restrict(d.movie, restrict(
    conj(
        keys(eq(compose(&d.movie_keyword, &d.keyword_keyword), "X")),
        keys(compose(&d.movie_company, eq(&d.company_country, "[de]"))),
    ),
    &d.movie_title,
))

// after
d.movie.o(
    (&d.movie_keyword).o(&d.keyword_keyword).eq("X").k()
        .and((&d.movie_company).o((&d.company_country).eq("[de]")).k())
        .o(&d.movie_title)
)
```

The inner-to-outer nesting of free-function calls becomes left-to-right
chaining of methods. Indentation should follow the same shape: each `.o(…)`
or `.and(…)` step on a new line, with the body indented inside the parens.

## What stays the same

- Function names (`fn q2a`, etc.) and signatures.
- `pub const ENTRIES` arrays.
- The `min_row(q)` query tail (see TRANSLATION.md — output columns fold via
  the `Row` trait, no per-query destructuring or format helpers).
- The closure / helper-fn pattern for shared sub-queries (`gf_25ab`,
  `helpers::film_or_warner_co`, etc.) — only the bodies of those helpers
  need the operator rename.
- `super::sets::*` imports and the named-tuple constants (`kw8()`, etc.).

## Style

- One blank line between fns. No doc comments on individual queries.
- Match the indentation style of the existing file you're editing.
- Don't reorder ENTRIES; conversion is rewrite-in-place.
- Don't change identifiers.

## Verification (do not run cargo yourself)

After every fn is converted, the file must still:
- compile (parens balanced, methods exist on the receiver type),
- be the same arity / fn signature,
- preserve the same query shape (so oracles still match).

The integration step (cargo build / cargo run) runs after all six chunks
are converted; that's where any mistake will surface as a compile error or
oracle diff.
