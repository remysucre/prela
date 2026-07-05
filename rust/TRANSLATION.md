# Prela: algebra notation → Rust

The dictionary between Prela's algebra notation — the infix surface of the
original Julia implementation, preserved on the `julia-engine` branch and
still used in the README and comments as the concise way to write queries
on paper — and the executable Rust embedding. "Julia" below refers to that
historic implementation, which doubles as the algebra's reference spec.

## Entity ids — phantom-typed `Id<Entity>`, 0-based

Rust entity ids are **0-based** (internal id = natural key − 1) and
**phantom-typed**: the dense nodes (`VecRel`/`MultiRel`/`Universe`/`Bitset`/
`DenseFold`) are generic over `D: Dense`, and the `schema!`-generated
columns instantiate them with `D = Id<Entity>` — a `repr(transparent)`
wrapper over `usize` carrying the entity tag, so composing through
mismatched entities is a COMPILE error (`Movie::keyword.select(Person::name)`
does not type-check). Scalar value columns (years, sizes, counts, dates,
prices) stay `i64`/`f64` — id columns and number columns are distinct types.
The binary cache (format v2 — `rust/src/format.rs`) stores the FINAL
physical layouts: ids already 0-based with `NO_ID` holes baked in, dates
pre-parsed to yyyymmdd `i64`, strings as offsets+bytes, multi columns as
CSR. The −1 shift, FK hole filling, and date parsing all happen once, in
`regen`; id-valued columns load by BULK REINTERPRET (the cache words read
as `Id<T>` through the transparent layout). The only places the wrapper
shows are raw-loop escapes (`id.idx()` indexes a `Vec` directly) and output
formatting of natural keys (TPC-H orderkey/custkey/partkey/suppkey print as
`id.idx() + 1`); JOB queries print no ids. Universe sizes are unchanged:
max raw id N ⟹ internal ids 0..N-1 ⟹ n = N (= every column's stored
length).

The missing-id sentinel is `Dense::NONE` (`engine::NO_ID` = `usize::MAX`
under the wrapper): FK-valued columns over gappy key spaces fill holes with
it, never 0 (entity 0 is live). Gap checks compare against it
(`Order::customer.filt(|c| c != Dense::NONE)`); "no entity seen
yet" fold states use `Option<Id<E>>` (or `NO_ID` in raw `Vec<usize>` state
where size matters, e.g. dense per-order arrays). Probes are safe `.get()`
lookups — `NONE` (or any out-of-universe id) fails the single bounds check
and emits nothing, so no `unsafe` is needed on the probe paths.

## Schema declarations — `schema!`

The schemas live in `src/job_schema.rs` / `src/tpch_schema.rs` as one
`schema!` block each (`src/schema.rs`). Per entity the macro generates the
entity tag (`struct Movie`), the typed columns (loaded from the v2 cache by
the generated `init`), a paren-free leaf HANDLE per field — a ZST named by
the field, implementing `IntoQuery` (its `iq` fetches the `&'static`
column from the `OnceLock` store once, at plan construction) — spelled as
an entity-qualified associated const for every field (`Movie::title`,
`Info::ty` — values usable wherever a relation is expected), re-exported
BARE for fields marked `pub` (`pub struct production_year;` in scope as a
bare name), a bare universe HANDLE when declared `Movie(movie)` (`movie` —
the identity relation over the entity's ids), and a NAVIGATION trait
(`Movie(movie) / MovieNav`) with one method per field,
blanket-implemented for everything that resolves (via `IntoQuery`) to a
query whose value type is the entity's id — so any `Id<Movie>`-valued
chain can continue `.title()`, `.cast()`, … (each ≡ `.select(Movie::field)`).
Columns sit in a global `OnceLock` store, so query fns take no data
argument. The macro also emits
a `MANIFEST` of (entity, field, cache kind) — regen verifies the cache it
writes against it (field names are cache filenames, verbatim).

## Output file template

```rust
// queries: <range from queries.jl>
use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::*;

pub const ENTRIES: &[super::Entry] = &[
    ("2a", "'Doc'", || min_row(q2a())),
    // ... one entry per query, name + oracle from _q("NAME","ORACLE") do ... end
];

fn q2a() -> impl Drive<R: Row> {
    movie.with(/* ... */).select(/* ... */)
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
  as a set operand directly: `.with(rel)`, `a.and(rel)`, `a.minus(rel)` all
  consume `rel` via `member` only.
- `s : q` (set ∘ query) is plain `Compose` — `s.select(q)` — because the
  identity's value IS the key.
- Identity relations send their keys through the value slot of `drive`, so
  the one `Bitset::over` / `MatSet` `FromQuery` impl serves sets and
  value-bearing queries alike (a set contributes its keys, a query its
  values).

## Conjunction IS the product; restriction carries it

Julia aliases `∧` to `⊗` (`algebra.jl`: `∧(a, b) = ⊗(a, b)`), and Rust
mirrors that exactly: `.and(b)` builds the same `Prod` node as `.and(b)`. The
two faces of the one node:

- **member position** (the argument of `.with`, a `.minus` rhs, a nested
  conjunct): `member(Prod)` is the flat short-circuit AND of the per-leg
  `member`s (Julia's `_prod_member`) — no pair value is ever built, so a
  conjunct tree costs exactly what a dedicated Conj node would.
- **drive/probe position**: it is the product and emits nested-pair values.

So a restricted scan is never written `a.and(p).select(b)` (that would compose
on the PAIR value and not type-check); it is `a.with(p).select(b)` — drive `a`,
member-check `p`, probe `b` — the post-unification spelling of Julia's
`a : p → b`.

## Operators (engine.rs::QueryExt)

| Julia               | Rust                                      | Notes |
|---------------------|-------------------------------------------|-------|
| `a → b` (Q ∘ Q)     | `a.select(b)`                                  | bridge = a's value type |
| `a : b` (restrict)  | `a.with(b)`                               | builds the dedicated `Restrict` node — node-for-node with Julia. Keep rows of a whose VALUE is a `member` of b (any probe-able b); a's value flows through unchanged |
| `s : q` (s a set)   | `s.select(q)`                                  | identity relation composes like any other |
| `(movie → …)`       | `movie.select(…)`                           | Universe ∘ Query |
| `a ∧ b`             | `a.and(b)`                                | alias for `⊗` (= `Prod`); in member position the `member` fast path short-circuits flat without building pairs |
| `a ∨ b`             | `a.or(b)`                                 | probe-only membership union (`Disj`); driving it is a COMPILE error |
| (enumerable union)  | `a.union(b)`                              | bag-concat `Union` (drive a then b, NO dedup); Julia has this only as a design note next to `drive(::Disj)` — Rust implements it. Feed it to deduping sinks (`Bitset::over`, `.collect::<MatSet<_>>()`), or materialize first when duplicates would change results |
| `a - b`             | `a.minus(b)`                              | value-bearing `Diff`: a's pairs whose KEY is not a member of b (identity a ⟹ set difference) |
| `a × b × c`         | `a.and(b).and(c)`                             | left-nested binary |
| `l ⩘ r`             | `r.with(l.collect::<MatSet<_>>())`        | left-driving wedge — in BOTH languages a `Restrict` of `r` by `l`'s value-set, with no dedicated node or sugar. Julia: `⩘(l, r) = Restrict(r, l')`, materialized lazily through the mode system (the `Inv` sits in probed position, so `prepare` self-indexes it); Rust has no lazy `Inv` node, so the collect materializes eagerly — visible in the query text |
| `r ← s` (l-compose) | `set.group_by(r).select(s)`               | drive-only `GroupBy`, SEMANTICS GENERALIZED vs Julia: the Rust receiver is a ROW-VALUED set (entity table / restriction of one — in general the pk map Key → row), and `set.group_by(r)` denotes `r.inv().with(set.inv())` — drive the set, probe the key `r` per row, emit (key-value, row). Column values are navigated AFTER grouping, so Julia's column-receiver `r ← s` becomes `set.group_by(r).select(s)` (identical over the full table; Julia's form relied on the entity table being the identity, i.e. pk id = row number) |
| (no Julia form)     | `a.gather(b)`                             | Rust-only: `select` that KEEPS the nesting — collect `b`'s matches into an `SVec` group per driven key instead of flattening (`movie.gather(year)` : `Movie → [year]`; gathers nest for document-shaped results). Grouping is run detection on the driven key — no hashing — so `Drive` requires the `Clustered` marker (leaves and key-preserving combinators have it; `InvStream`/`GroupBy`/`Union` don't, making a scrambled drive a compile error). Keys whose probes miss emit `[]` (left-outer). `q.collect::<T>()` is semantically `gather` at the unit key (whole result as one group, key elided), fused to a straight drive |
| `q ▷ (op, init)`    | `q.fold(init, op)`                        | per-key foldl into an eager cache |
| `q ▷ f` (callable)  | `q.buf_fold(f)`                           | `BufFold` — per-key whole-multiset reduce: buffer each group, cache `f(group)`. For reducers that don't fit foldl's `(S, R) → S` shape; `▷ (vs -> length(unique(vs)))` ⇒ `.count_distinct()`, the `length ∘ unique` instance |
| `a == v`            | `a.eq(v)`                                 | on an entity-valued col, auto-elides to its primary scalar field |
| `a != v`            | `a.ne(v)`                                 |  |
| `>, <, >=, <=`      | `.gt`, `.lt`, `.ge`, `.le`                | Works on i64 and &str (lex) |
| `a in (v1, …)`      | `a.is_in([v1, …])`                        | any `IntoIterator` — arrays, slices, the named set fns in `super::sets` |
| `a ~ r"…"`          | `a.rx(r"…")`                              |  |
| `a ≁ r"…"`          | `a.nrx(r"…")`                             |  |
| `Universe`          | `movie`, `persons`                   | Copy; identity relation over `Id<Movie>` / `Id<Person>` |

### Predicate position — `→` and `:` translate verbatim

In member position (a conjunct of a `.with`/`.and` tree, a `.minus` rhs)
hopping an edge into a predicate tree with `→` and restricting the edge
with `:` are semantically interchangeable — membership through `Compose`
(`probe_any` threads the existential through the hop) and through
`Restrict` (`member` on the restricted edge) ask the same question, and
the suites pin both paths. The translation does NOT get to choose: it
preserves the operator the Julia source wrote.

- `edge → tree` ⇒ `edge.select(tree)` — or the nav method when the hop is a
  single field (`company → (Company.country == "[de]")` ⇒
  `company.country().eq("[de]")`; nav IS compose).
- `edge : tree` ⇒ `edge.with(tree)`.

JOB 22a's first conjunct — Julia `info → (Info.type == "countries") ∧
(Info.info in (…))` — is therefore

```rust
info.select(Info::ty.eq("countries")
     .and(Info::info.is_in([…])))
```

while its subqueries `data : (…) ∧ (…) → Data.data` are genuine
restrictions and stay `data.with(…).text()`.

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

The scalar comparisons (`.eq`/`.ne`/`.gt`/`.lt`/`.ge`/`.le`/`.is_in`/`.rx`/
`.nrx`/`.during`/`.between`) are all captured-closure forms of `.filt`: each
builds `Filter<A, F>` where `F` is a plain `Fn(A::R) -> bool` closure held
directly — there is no predicate trait layer (Julia: `Filter(a, pred)` with
any callable). Relation-valued restriction is the separate `Restrict` node
(`.with`), consuming its rhs via `member` only — the same `Filter`/`Restrict`
split as Julia's algebra.

## Schema fields → leaf handles and navigation

A field is spelled two ways, by position in the chain:

- **Root** (the start of a predicate or projection chain, or argument
  position): a paren-free leaf handle — the bare ZST for `pub` fields,
  the `Entity::field` associated const otherwise. No parens: the handle
  is a VALUE, resolved to its column via `IntoQuery` when the plan is
  built.
- **Mid-chain** (after anything `Id<E>`-valued): a NAVIGATION method —
  `keyword.text()`, `cast.person().name()` — one method per field on
  the entity's generated nav trait, regardless of `pub`. Nav methods keep
  their parens; only the chain's root is bare.

**Bare handles** (fields marked `pub` in `src/job_schema.rs`):
`title`, `kind`, `production_year`, `episode_nr`, `keyword`,
`company`, `cast`, `info`, `data`, `complete_cast`, `link`,
`linked_by`, `aka` (Movie — every Movie edge/attr is bare); `person`,
`role`, `character` (Cast); `gender`, `alias`, `bio`,
`name_pcode_cf` (Person); `country` (Company); `target` (MovieLink);
`status`, `subject` (CompleteCast).

**Entity-qualified roots** (names that collide across entities): the
lookup-table labels are uniformly `text` (`Keyword::text`,
`Kind::text`, `RoleType::text`, `Character::text`,
`CompanyType::text`, `InfoType::text`, `Data::text`,
`AkaName::text`, `AkaTitle::text`, `LinkType::text`,
`CompCastType::text`); plus `Person::name` / `Company::name`,
`Cast::note` / `Company::note` / `Info::note` / `PersonInfo::note`,
`Company::ty` / `Info::ty` / `Data::ty` / `PersonInfo::ty` /
`MovieLink::ty`, and `Info::info` / `PersonInfo::info`.

(Former names: `type_` is now `ty` everywhere — no raw-keyword underscore,
and field names ARE the cache filenames (`Info_ty.bin`). `Person.info` →
`bio`, `Person.aka` → `alias`; each lookup table's label column →
`text`.)

## Primary-field elision — comparisons auto-navigate to the primary

Julia writes `keyword == "x"` and means "the keyword id, resolved to its
label, equals x". Rust elides identically: a comparison on an entity-valued
query auto-navigates to that entity's PRIMARY (first-declared) scalar field
before comparing. So `keyword.eq("x")` ≡ `keyword.text().eq("x")` — the
`.text()` is implied. Driven by the `Field`/`Primary` traits in engine.rs;
the elision is by-construction transparent (same plan as the explicit nav).

| Julia                                | Rust                       |
|--------------------------------------|----------------------------|
| `keyword == "x"`                     | `keyword.eq("x")`          |
| `keyword in (...)`                   | `keyword.is_in([...])`     |
| `role == "x"` (cast)                 | `role.eq("x")`             |
| `kind == "x"` (movie)                | `kind.eq("x")`             |
| `Info.type == "x"`                   | `Info::ty.eq("x")`         |
| `Data.type == "x"`                   | `Data::ty.eq("x")`         |
| `CompleteCast.status == "x"`         | `status.eq("x")`           |

Applies to every comparator (`eq/ne/gt/lt/ge/le/in_v/is_in/rx/nrx/during/
between`). Two caveats:

- **Only the PRIMARY field elides.** A non-primary scalar still needs the
  explicit hop: `Company.country` is `company.country()` (country is not
  Company's first field), and any navigation used as OUTPUT keeps its nav
  method (`cast.person().name()`).
- **Entities without a scalar primary don't elide** (their first field is an
  entity ref: Cast, MovieLink, CompleteCast, PartSupp, Order, Lineitem) — so
  `.eq` on their ids won't compile. Compare ids with the non-eliding escape
  hatch `.filt`, e.g. `Order::customer.filt(|c| c != Dense::NONE)`.

## Multi-hop traversal

| Julia in context                   | Rust                                |
|------------------------------------|-------------------------------------|
| `person.name` (cast context)       | `person.name()`                   |
| `person.aka.name` (cast)           | `person.alias().text()`           |
| `Person.aka.name` (person context) | `alias.text()`                    |
| `character.name` (cast)            | `character.text()`                |
| `cast.person.name` (movie context) | `cast.person().name()`            |

(`person` is Cast's `person` FK column; `persons` is the Person
universe — the bare universe name is plural exactly because bare `person`
is the hot Cast handle.)

## Implicit primary on outputs

When the OUTPUT of a query column is an ID (not a string), Julia
auto-resolves to the entity's primary field at print time. In Rust make it
explicit — navigate to the string column:

- `co × title` where `co` yields Company-id → `co.name().and(title)`
- `lk × …` where `lk` yields MovieLink-id → `lk.ty().text()`
- `info → (gf : Info.info)` → already string-valued, no further resolution.

If a let-bound query is named after an entity (`co`, `lk`, etc.) and used in
output, look at the surrounding `.name`/`.title` — if absent, navigate to
the entity's label column as above.

## `let` bindings

Julia `let x = …, y = …; body` where `x` is used twice in `body`. In Rust:
define a helper fn that returns a fresh instance:

```rust
fn co_27() -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    company.with(country.ne("[pl]")
            .and(/* … the rest of the Company-side conjunction … */))
}
```

Use `co_27()` once as a conjunct (`.and(co_27())` — no projection needed)
and once for the projection (e.g. `co_27().name()`). Each call
builds a fresh value — that's fine, the structures are cheap. The columns
are `&'static` (they live in the schema's `OnceLock` store), so no lifetime
parameter is needed.

If `x` is used only once, inline it.

`impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe` — value-bearing
projections name the value's type (e.g. `R = &'static str`). Conjunct-tree
helpers (rooted at `.and` / `.minus`) are consumed via `member` only, so
they leave `R` opaque: `impl Query<D = Id<Info>> + Probe` (the pair-valued
`R` of a `Prod` is an implementation detail).

When the same binding recurs across query templates (it does for the
company/link bindings of templates 21 and 27), the helper lives once in
`queries/helpers.rs` under a descriptive name — `film_or_warner_co`,
`follow_link` — instead of per-file copies.

Query families that differ only in constants (e.g. q2a–q2d's country code)
are one parameterized fn plus one-line wrappers:

```rust
fn q2(cc: &'static str) -> String { /* the query */ }

fn q2a() -> String { q2("[de]") }
fn q2b() -> String { q2("[nl]") }
```

Only do this when the bodies are otherwise identical — structurally
different siblings stay separate.

## Named tuple constants — `super::sets`

`kw7()`, `kw8()`, `kw10()`, `voice3()`, `voice4()`, `writer5()`, `genre6()`,
`murder4()`, `nordic8()`, `nordic9()`, `nordic10()`, `link3()`.

Use as: `keyword.text().is_in(kw8())`. (The set fns are local
plan-returning helpers, not schema handles — they keep their parens.)

## Output formatting — query tails

Query fns return their PLAN — `fn qXa() -> impl Drive<R: Row>` — the bare
combinator expression, no formatting. `min_row(q)` (from `queries::helpers`)
is applied at the registry: each `ENTRIES` row is
`(name, oracle, || min_row(qXa()))` (the non-capturing closure coerces to
the `fn() -> String` entry type). `min_row` drives the query once,
accumulates the lexicographic minimum of each output column independently
(the JOB `MIN(...)` projection), and renders the columns joined with
`" || "` — or `"(empty)"` if no row survived.

Column shapes are handled by the `Row` trait: `&'static str`, `i64`, and
nested `Prod` tuples thereof (`((a, b), c)` for `a × b × c`, etc.), so any
arity and any str/int column mix works with the same one-line tail.

## Multi-conjunct nesting

`a ∧ b ∧ c ∧ d` → `a.and(b).and(c).and(d)` (FLAT and left-chained; the
operands are used via `member` only, so association doesn't matter — but the
flat chain mirrors Julia's flat `_prod_member`, so never nest
`.and(a.and(b))`). Same for `.or`. When the chain restricts a DRIVEN
relation, write ONE restriction over the ∧-tree — `u.with(a.and(b))`,
mirroring Julia's single `:` — not chained restrictions.
`u.with(a).with(b)` is member-equivalent (identical order and
short-circuit) but reserved for genuinely sequential restriction, e.g. the
`⩘` wedge or a restriction applied after composition.

## Common patterns

### Movie-rooted (templates 1-5, 11-15, 22)
```rust
fn qXa() -> impl Drive<R: Row> {
    movie
        .with(/* movie conjunct tree: a.and(b).and(c)… (member-checked) */)
        .select(/* projection — usually a .and(…) product */)
}
```

### Movie + cast filter + cast projection (templates 6-10, 16-20)
```rust
fn qXa() -> impl Drive<R: Row> {
    movie
        .with(/* movie conjunct tree */)
        .select(cast
             .with(/* cast conjunct tree */)
             .person().name()      // cast projection — navigation
         .and(title))
}
```

## Layout
A continuation conjunct (`.and`/`.or`/`.minus` on its own line) is indented
one space past the `.` of the method call it continues — e.g. the `.and`
siblings of a `.with(…)` sit one space in from `.with`, NOT aligned under the
first conjunct. This keeps deep conjunctions compact; see any multi-conjunct
query (e.g. `q22a`).

## Naming
Each query fn: `q<lowercase id>` (e.g. `q2a`, `q11d`, `q22c`).
ENTRIES key is the exact Julia name string ("2a", "11d", "22c").
ENTRIES oracle is the exact second-arg string from `_q("name", "oracle")`.

## Pitfalls
- Leaf handles are ZST VALUES, not fns — no parens at the root
  (`Movie::title.rx(…)`, `keyword.text()`), no `&` borrows or `d`
  plumbing; the handle resolves to its `&'static` column via `IntoQuery`
  when the plan is built.
- BINDING CAPTURE: a bare handle in scope makes any same-named local a
  unit-struct PATTERN, not a fresh binding — `let kind = …`, a closure
  param `|info|`, a match arm fail to compile ("interpreted as a unit
  struct, not a new binding"). Rename such locals (`let kd = …`). Same
  reason `assert_eq!`/`assert_ne!` break under a glob import of a schema
  exporting bare `kind` (the core macros bind `let kind` internally) —
  test modules import schema names selectively.
- Conjuncts need NO projection: `a.and(b)` consumes `b` via `member`, so a
  value-bearing filter (`production_year.gt(2000)`) is a valid operand
  as-is. Same for `.minus`'s RHS and `.with`'s argument.
- A conjunct tree is member-position ONLY. To compose or drive past it,
  hoist it into the upstream restriction: `x.with(a.and(b)).select(body)`, never
  `x.select(a.and(b).select(body))` — `.and` is the product, so the latter would try
  to compose on the pair value (compile error at best).
- `.or` cannot be driven (no `Drive` impl, by design — Julia's `∨` is
  probe-only). The enumerable union is `.union` (bag-concat, no dedup).
- For `(production_year >= X) ∧ (production_year <= Y)` use
  `production_year.ge(X).and(production_year.le(Y))`
  — each comparison is its own Filter conjunct.
- A nav method needs the matching entity-id value: `.text()` exists only
  where the chain is valued in an entity whose schema declares `text`. On a
  string-valued (or wrong-entity) query it does not resolve (compile
  error), which is the type system catching a wrong hop.
- Don't forget the OUTERMOST `movie.with(…)` / `movie.select(…)` — the
  query is anchored at the movie universe.
