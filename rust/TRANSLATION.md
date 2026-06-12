# Prela: algebra notation → Rust

The dictionary between Prela's algebra notation — the infix surface of the
original Julia implementation, preserved on the `julia-engine` branch and
still used in the README and comments as the concise way to write queries
on paper — and the executable Rust embedding. "Julia" below refers to that
historic implementation, which doubles as the algebra's reference spec.

## Entity ids — 0-based `usize` in Rust

Rust entity ids are **0-based `usize`**: internal id = natural key − 1.
Ids are opaque dense indexes, so the id domain type is
`usize` throughout the engine (`VecRel`/`MultiRel`/`Universe`/`Bitset`/`DenseFold`
all have `D = usize`); scalar value columns (years, sizes, counts, dates,
prices) stay `i64`/`f64` — id columns and number columns are distinct types.
The binary cache (format v2 — `rust/src/format.rs`) stores the FINAL
physical layouts: ids already 0-based with `NO_ID` holes baked in, dates
pre-parsed to yyyymmdd `i64`, strings as offsets+bytes, multi columns as
CSR. The −1 shift, FK hole filling, and date parsing all happen once, in
`regen` (the historic 1-based pair-stream cache was a Julia-era artifact;
see the julia-engine branch). The only place a +1 survives is output
formatting of natural keys (TPC-H orderkey/custkey/partkey/suppkey); JOB
queries print no ids. Universe sizes are unchanged: max raw id N ⟹
internal ids 0..N-1 ⟹ n = N (= every column's stored length).

The missing-id sentinel is `engine::NO_ID` (= `usize::MAX`): FK-valued
Leaf names match Julia's (`VecRel`/`MultiRel` implementing `Query`, like
`VecRel`/`MultiRel <: Query{D,R}`), with one collapse: Rust's `VecRel` covers
both Julia's `VecRel` AND `SparseRel` — gappy columns use fill values instead
of a separate presence-map type.
`VecRel` columns over gappy key spaces fill holes with `NO_ID`, never 0
(entity 0 is live) — see the `VecRel` invariant in `src/engine.rs`. Gap
checks compare against it (`.ne(NO_ID)` / `== NO_ID`); "no entity seen yet"
fold states use `Option<usize>` (or `NO_ID` where state size matters, e.g.
dense per-order arrays). Probes are safe `.get()` lookups — `NO_ID` (or any
out-of-universe id) fails the single bounds check and emits nothing, so no
`unsafe` is needed on the probe paths.

## Output file template

```rust
// queries: <range from queries.jl>
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
    ("2a", "'Doc'", q2a),
    // ... one entry per query, name + oracle from _q("NAME","ORACLE") do ... end
];

fn q2a(d: &Data) -> String {
    let q = d.movie.o(/* ... */);
    min_row(q)
}

// ... more queries
```

## Sets are identity relations — there is no keyset type

Mirroring Julia's `Unary{D} <: Query{D, D}`, Rust has ONE trait family
(`Query` / `Drive` / `Probe`). A set-shaped node (`Universe`, `Bitset`,
`MatSet`, `Disj`) is an identity relation `D → D`: `drive`
emits `(x, x)`, `probe` yields `x` iff member. Membership is part of the
relation protocol — `member(q, x)` is defined for ANY probe-able query as
`probe_any(x, |_| true)`, with cheaper overrides on the set leaves (Bitset
bit-test, Universe bound check, MatSet hash lookup) and on `Prod` (the
flat short-circuit AND below).

Consequences:

- There is no `.k()` / `keys()` projection. A value-bearing query is used
  as a set operand directly: `.in_s(rel)`, `a.and(rel)`, `a.minus(rel)` all
  consume `rel` via `member` only.
- `s : q` (set ∘ query) is plain `Compose` — `s.o(q)` — because the
  identity's value IS the key.
- Identity relations send their keys through the value slot of `drive`, so
  the one `Bitset::over` / `MatSet` `FromQuery` impl serves sets and
  value-bearing queries alike (a set contributes its keys, a query its
  values).

## Conjunction IS the product; restriction carries it

Julia aliases `∧` to `⊗` (`algebra.jl`: `∧(a, b) = ⊗(a, b)`), and Rust
mirrors that exactly: `.and(b)` builds the same `Prod` node as `.x(b)`. The
two faces of the one node:

- **member position** (the argument of `.in_s`, a `.minus` rhs, a nested
  conjunct): `member(Prod)` is the flat short-circuit AND of the per-leg
  `member`s (Julia's `_prod_member`) — no pair value is ever built, so a
  conjunct tree costs exactly what a dedicated Conj node would.
- **drive/probe position**: it is the product and emits nested-pair values.

So a restricted scan is never written `a.and(p).o(b)` (that would compose
on the PAIR value and not type-check); it is `a.in_s(p).o(b)` — drive `a`,
member-check `p`, probe `b` — the post-unification spelling of Julia's
`a : p → b`.

## Operators (engine.rs::QueryExt)

| Julia               | Rust                                      | Notes |
|---------------------|-------------------------------------------|-------|
| `a → b` (Q ∘ Q)     | `a.o(b)`                                  | bridge = a's value type |
| `a : b` (restrict)  | `a.in_s(b)`                               | builds the dedicated `Restrict` node — node-for-node with Julia. Keep rows of a whose VALUE is a `member` of b (any probe-able b); a's value flows through unchanged |
| `s : q` (s a set)   | `s.o(q)`                                  | identity relation composes like any other |
| `(movie → …)`       | `d.movie.o(…)`                            | Universe ∘ Query |
| `a ∧ b`             | `a.and(b)`                                | alias for `⊗` (= `Prod`); in member position the `member` fast path short-circuits flat without building pairs |
| `a ∨ b`             | `a.or(b)`                                 | probe-only membership union (`Disj`); driving it is a COMPILE error |
| (enumerable union)  | `a.union(b)`                              | bag-concat `Union` (drive a then b, NO dedup); Julia has this only as a design note next to `drive(::Disj)` — Rust implements it. Feed it to deduping sinks (`Bitset::over`, `.collect::<MatSet<_>>()`), or materialize first when duplicates would change results |
| `a - b`             | `a.minus(b)`                              | value-bearing `Diff`: a's pairs whose KEY is not a member of b (identity a ⟹ set difference) |
| `a × b × c`         | `a.x(b).x(c)`                             | left-nested binary |
| `l ⩘ r`             | `r.in_s(l.collect::<MatSet<_>>())`        | left-driving wedge — in BOTH languages a `Restrict` of `r` by `l`'s value-set, with no dedicated node or sugar. Julia: `⩘(l, r) = Restrict(r, l')`, materialized lazily through the mode system (the `Inv` sits in probed position, so `prepare` self-indexes it); Rust has no lazy `Inv` node, so the collect materializes eagerly — visible in the query text |
| `r ← s` (l-compose) | `s.group_by(r)`                           | drive-only `GroupBy`: drive `s`, probe `r` per row for the group key, emit (r-value, s-value). RECEIVER = DRIVEN SIDE — Julia's infix argument order is a surface artifact; in method position the flip reads naturally ("group s by r") |
| `q ▷ (op, init)`    | `q.fold(init, op)`                        | per-key foldl into an eager cache |
| `q ▷ f` (callable)  | `q.buf_fold(f)`                           | `BufFold` — per-key whole-multiset reduce: buffer each group, cache `f(group)`. For reducers that don't fit foldl's `(S, R) → S` shape; `▷ (vs -> length(unique(vs)))` ⇒ `.count_distinct()`, the `length ∘ unique` instance |
| `a == v`            | `a.eq(v)`                                 | for `Type.field == v` see ELISION |
| `a != v`            | `a.ne(v)`                                 |  |
| `>, <, >=, <=`      | `.gt`, `.lt`, `.ge`, `.le`                | Works on i64 and &str (lex) |
| `a in (v1, …)`      | `a.in_v(vec![v1, …])`                     | named ones live in `super::sets` |
| `a ~ r"…"`          | `a.rx(r"…")`                              |  |
| `a ≁ r"…"`          | `a.nrx(r"…")`                             |  |
| `Universe`          | `d.movie`, `d.persons`                    | Copy; identity relation over 0..n |

## No hidden materialization — `collect` names the physical type

Every index/set build is visible in the query text. `q.collect()` (the
`FromQuery` mirror of `Iterator::collect`/`FromIterator`) drives `q` once into
the physical structure named by the target type — turbofish inline
(`.collect::<MatSet<_>>()`) or a `let` annotation
(`let idx: HashIdx<_, _> = (…).collect();`). This is the ONLY way a stream
becomes probe-side state; a drive-only node (`InvStream`, `GroupBy`, `Union`)
in probe position is a compile error, and the fix is an explicit `collect`
where Julia's `prepare` would auto-index through the mode system. `Bitset`
is deliberately NOT a `FromQuery` target: it needs the universe size `n` —
part of the physical choice — so it keeps `Bitset::over(universe, q)`, taking
the `Universe` itself (self-documenting, typo-proof) rather than a bare `n`.

The scalar comparisons (`.eq`/`.ne`/`.gt`/`.lt`/`.ge`/`.le`/`.in_v`/`.rx`/
`.nrx`/`.during`/`.between`) are all captured-closure forms of `.filt`: each
builds `Filter<A, F>` where `F` is a plain `Fn(A::R) -> bool` closure held
directly — there is no predicate trait layer (Julia: `Filter(a, pred)` with
any callable). Relation-valued restriction is the separate `Restrict` node
(`.in_s`), consuming its rhs via `member` only — the same `Filter`/`Restrict`
split as Julia's algebra.

## Schema fields → `d.<field>`

**Bare names in MOVIE context** (also valid as `Movie.<field>`):
`title→movie_title`, `production_year→movie_production_year`,
`episode_nr→movie_episode_nr`, `kind→movie_kind`, `info→movie_info`,
`keyword→movie_keyword`, `data→movie_data`, `company→movie_company`,
`complete_cast→movie_complete_cast`, `link→movie_link`,
`linked_by→movie_linked_by`, `aka→movie_aka`, `cast→movie_cast`.

**Bare names in CAST context**:
`note→cast_note`, `role→cast_role`, `character→cast_character`,
`person→cast_person`.

**Bare names in PERSON context**:
`name→person_name`, `gender→person_gender`, `aka→person_aka`,
`info→person_info`, `name_pcode_cf→person_name_pcode`.

**Qualified entity.field**:
| Julia                         | Rust                  |
|-------------------------------|-----------------------|
| Person.name                   | d.person_name         |
| Person.gender                 | d.person_gender       |
| Person.aka                    | d.person_aka          |
| Person.info                   | d.person_info         |
| Person.name_pcode_cf          | d.person_name_pcode   |
| Keyword.keyword               | d.keyword_keyword     |
| Kind.kind                     | d.kind_kind           |
| RoleType.role                 | d.roletype_role       |
| Character.name                | d.character_name      |
| Company.country               | d.company_country     |
| Company.name                  | d.company_name        |
| Company.note                  | d.company_note        |
| Company.type                  | d.company_type        |
| CompanyType.kind              | d.companytype_kind    |
| Info.info                     | d.info_info           |
| Info.type                     | d.info_type           |
| Info.note                     | d.info_note           |
| InfoType.info                 | d.infotype_info       |
| Data.data                     | d.data_data           |
| Data.type                     | d.data_type           |
| PersonInfo.info               | d.personinfo_info     |
| PersonInfo.type               | d.personinfo_type     |
| PersonInfo.note               | d.personinfo_note     |
| AkaName.name                  | d.akaname_name        |
| AkaTitle.title                | d.akatitle_title      |
| MovieLink.target              | d.movielink_target    |
| MovieLink.type                | d.movielink_type      |
| LinkType.link                 | d.linktype_link       |
| CompleteCast.status           | d.completecast_status |
| CompleteCast.subject          | d.completecast_subject|
| CompCastType.kind             | d.compcasttype_kind   |

## CRITICAL — Primary-field elision

Julia writes `keyword == "x"` but means "the keyword id, resolved to its primary
string field, equals x". Rust must spell out the resolve step with a compose:

| Julia                                | Rust                                                                       |
|--------------------------------------|----------------------------------------------------------------------------|
| `keyword == "x"`                     | `(&d.movie_keyword).o(&d.keyword_keyword).eq("x")`                         |
| `keyword in (...)`                   | `(&d.movie_keyword).o(&d.keyword_keyword).in_v(vec![...])`                 |
| `role == "x"` (cast)                 | `(&d.cast_role).o(&d.roletype_role).eq("x")`                               |
| `kind == "x"` (movie)                | `(&d.movie_kind).o(&d.kind_kind).eq("x")`                                  |
| `Info.type == "x"`                   | `(&d.info_type).o(&d.infotype_info).eq("x")`                               |
| `Company.type == "x"`                | `(&d.company_type).o(&d.companytype_kind).eq("x")`                         |
| `Data.type == "x"`                   | `(&d.data_type).o(&d.infotype_info).eq("x")` (Data.type points to InfoType) |
| `MovieLink.type == "x"`              | `(&d.movielink_type).o(&d.linktype_link).eq("x")`                          |
| `PersonInfo.type == "x"`             | `(&d.personinfo_type).o(&d.infotype_info).eq("x")`                         |
| `CompleteCast.status == "x"`         | `(&d.completecast_status).o(&d.compcasttype_kind).eq("x")`                 |
| `CompleteCast.subject == "x"`        | `(&d.completecast_subject).o(&d.compcasttype_kind).eq("x")`                |

Same pattern for `~`, `≁`, `>`, `<`, `in`, etc. — the LHS becomes the
`id-rel.o(primary-rel)` chain.

## Multi-hop traversal

| Julia in context                   | Rust                                                                             |
|------------------------------------|----------------------------------------------------------------------------------|
| `person.name` (cast context)       | `(&d.cast_person).o(&d.person_name)`                                             |
| `person.aka.name` (cast)           | `(&d.cast_person).o((&d.person_aka).o(&d.akaname_name))`                         |
| `Person.aka.name` (person context) | `(&d.person_aka).o(&d.akaname_name)`                                             |
| `character.name` (cast)            | `(&d.cast_character).o(&d.character_name)`                                       |

## Implicit primary on outputs

When the OUTPUT of a query column is an ID (not a string), Julia auto-resolves
to the entity's primary field at print time. In Rust make it explicit:

- `co × title` where `co` yields Company-id → `co.o(&d.company_name).x(&d.movie_title)`
- `lk × …` where `lk` yields MovieLink-id → `lk.o((&d.movielink_type).o(&d.linktype_link))`
- `info → (gf : Info.info)` → already string-valued, no further resolution.

If a let-bound query is named after an entity (`co`, `lk`, etc.) and used in
output, look at the surrounding `.name`/`.title` — if absent, compose with the
entity's primary string field as above.

## `let` bindings

Julia `let x = …, y = …; body` where `x` is used twice in `body`. In Rust:
define a helper fn that returns a fresh instance:

```rust
fn co_27<'d>(d: &'d Data) -> impl Query<D = usize, R = usize> + Drive + Probe + 'd {
    (&d.movie_company).in_s(
        (&d.company_country).ne("[pl]")
            .and(/* … the rest of the Company-side conjunction … */)
    )
}
```

Use `co_27(d)` once as a conjunct (`.and(co_27(d))` — no projection needed)
and once for the projection (e.g. `co_27(d).o(&d.company_name)`). Each call
builds a fresh value — that's fine, the structures are cheap.

If `x` is used only once, inline it.

`impl Query<D = usize, R = usize> + Drive + Probe + 'd` — the `'d` lifetime
ties the returned value to the borrows it holds on `d`. Add it whenever the
helper borrows from `d` (which is always). Value-bearing projections name
the value's type (e.g. `R = &'static str`). Conjunct-tree helpers (rooted
at `.and` / `.minus`) are consumed via `member` only, so they leave `R`
opaque: `impl Query<D = usize> + Probe + 'd` (the pair-valued `R` of a `Prod`
is an implementation detail).

When the same binding recurs across query templates (it does for the
company/link bindings of templates 21 and 27), the helper lives once in
`queries/helpers.rs` under a descriptive name — `film_or_warner_co`,
`follow_link` — instead of per-file copies.

Query families that differ only in constants (e.g. q2a–q2d's country code)
are one parameterized fn plus one-line wrappers:

```rust
fn q2(d: &Data, country: &'static str) -> String { /* the query */ }

fn q2a(d: &Data) -> String { q2(d, "[de]") }
fn q2b(d: &Data) -> String { q2(d, "[nl]") }
```

Only do this when the bodies are otherwise identical — structurally
different siblings stay separate.

## Named tuple constants — `super::sets`

`kw7()`, `kw8()`, `kw10()`, `voice3()`, `voice4()`, `writer5()`, `genre6()`,
`murder4()`, `nordic8()`, `nordic9()`, `nordic10()`, `link3()`.

Use as: `(&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8())`.

## Output formatting — query tails

Every query ends with `min_row(q)` (from `super::helpers`). It drives the
query once, accumulates the lexicographic minimum of each output column
independently (the JOB `MIN(...)` projection), and renders the columns
joined with `" || "` — or `"(empty)"` if no row survived.

Column shapes are handled by the `Row` trait: `&'static str`, `i64`, and
nested `Prod` tuples thereof (`((a, b), c)` for `a × b × c`, etc.), so any
arity and any str/int column mix works with the same one-line tail.

## Multi-conjunct nesting

`a ∧ b ∧ c ∧ d` → `a.and(b).and(c).and(d)` (left-chained; the operands are
used via `member` only, so association doesn't matter). Same for `.or`.
When the chain restricts a DRIVEN relation, each conjunct can equally be
its own restriction — `u.in_s(a).in_s(b)` ≡ `u.in_s(a.and(b))` (identical
member order and short-circuit).

## Common patterns

### Movie-rooted (templates 1-5, 11-15, 22)
```rust
fn qXa(d: &Data) -> String {
    let q = d.movie.in_s(
        /* movie conjunct tree: a.and(b).and(c)… (member-checked) */
    ).o(/* projection — usually a .x(…) product */);
    min_row(q)
}
```

### Movie + cast filter + cast projection (templates 6-10, 16-20)
```rust
fn qXa(d: &Data) -> String {
    let q = d.movie.in_s(
        /* movie conjunct tree */
    ).o(
        (&d.movie_cast).in_s(
            /* cast conjunct tree */
        ).o(/* cast projection — `person.name`, etc. */)
        .x(&d.movie_title)
    );
    min_row(q)
}
```

## Naming
Each query fn: `q<lowercase id>` (e.g. `q2a`, `q11d`, `q22c`).
ENTRIES key is the exact Julia name string ("2a", "11d", "22c").
ENTRIES oracle is the exact second-arg string from `_q("name", "oracle")`.

## Pitfalls
- Always borrow leaves with `&` as method receivers (`(&d.foo).o(…)`).
  `d.movie` (Universe) is the only exception — it's Copy.
- Conjuncts need NO projection: `a.and(b)` consumes `b` via `member`, so a
  value-bearing filter (`(&d.movie_production_year).gt(2000)`) is a valid
  operand as-is. Same for `.minus`'s RHS and `.in_s`'s argument.
- A conjunct tree is member-position ONLY. To compose or drive past it,
  hoist it into the upstream restriction: `x.in_s(a.and(b)).o(body)`, never
  `x.o(a.and(b).o(body))` — `.and` is the product, so the latter would try
  to compose on the pair value (compile error at best).
- `.or` cannot be driven (no `Drive` impl, by design — Julia's `∨` is
  probe-only). The enumerable union is `.union` (bag-concat, no dedup).
- For `(production_year >= X) ∧ (production_year <= Y)` use
  `(&d.movie_production_year).ge(X).and((&d.movie_production_year).le(Y))`
  — each comparison is its own Filter conjunct.
- Don't forget the OUTERMOST `d.movie.in_s(…)` / `d.movie.o(…)` — the query
  is anchored at the movie universe.
