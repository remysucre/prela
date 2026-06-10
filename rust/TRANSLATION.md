# Julia → Rust translation guide for Prela queries

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
    let q = restrict(d.movie, /* ... */);
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

// ... more queries
```

## Operators (engine.rs exports)

| Julia               | Rust                                      | Notes |
|---------------------|-------------------------------------------|-------|
| `a → b` (Q ∘ Q)     | `compose(a, b)`                           | a must yield i64 (entity id) |
| `a → s` (Q → SetQ)  | `in_set(a, s)`                            | filter Q by value ∈ s; a yields i64 |
| `s : q`             | `restrict(s, q)`                          | SetQ : Query |
| `(movie → …)`       | `restrict(d.movie, …)`                    | Universe : Query (or wrapped SetQ : projection) |
| `a ∧ b`             | `conj(a, b)`                              | both SetQ; wrap Query with `keys()` |
| `a ∨ b`             | `disj(a, b)`                              | both SetQ |
| `a - b`             | `set_diff(a, b)`                          | both SetQ; wrap Query with `keys()` |
| `a × b × c`         | `prod(prod(a, b), c)`                     | left-nested binary |
| `a == v`            | `eq(a, v)`                                | a: Query; for `Type.field == v` see ELISION |
| `a != v`            | `ne(a, v)`                                |  |
| `>, <, >=, <=`      | `gt`, `lt`, `ge`, `le`                    | Works on i64 and &str (lex) |
| `a in (v1, …)`      | `in_vec(a, vec![v1, …])`                  | named ones live in `super::sets` |
| `a ~ r"…"`          | `regex_match(a, r"…")`                    |  |
| `a ≁ r"…"`          | `regex_not(a, r"…")`                      |  |
| `Keys(q)`           | `keys(q)`                                 | Query → SetQ (forgets values) |
| `Universe`          | `d.movie`, `d.persons`                    | Copy |

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
string field, equals x". Rust must spell out the resolve step with a `compose`:

| Julia                                | Rust                                                                       |
|--------------------------------------|----------------------------------------------------------------------------|
| `keyword == "x"`                     | `eq(compose(&d.movie_keyword, &d.keyword_keyword), "x")`                  |
| `keyword in (...)`                   | `in_vec(compose(&d.movie_keyword, &d.keyword_keyword), vec![...])`         |
| `role == "x"` (cast)                 | `eq(compose(&d.cast_role, &d.roletype_role), "x")`                         |
| `kind == "x"` (movie)                | `eq(compose(&d.movie_kind, &d.kind_kind), "x")`                            |
| `Info.type == "x"`                   | `eq(compose(&d.info_type, &d.infotype_info), "x")`                         |
| `Company.type == "x"`                | `eq(compose(&d.company_type, &d.companytype_kind), "x")`                   |
| `Data.type == "x"`                   | `eq(compose(&d.data_type, &d.infotype_info), "x")` (Data.type points to InfoType) |
| `MovieLink.type == "x"`              | `eq(compose(&d.movielink_type, &d.linktype_link), "x")`                    |
| `PersonInfo.type == "x"`             | `eq(compose(&d.personinfo_type, &d.infotype_info), "x")`                   |
| `CompleteCast.status == "x"`         | `eq(compose(&d.completecast_status, &d.compcasttype_kind), "x")`           |
| `CompleteCast.subject == "x"`        | `eq(compose(&d.completecast_subject, &d.compcasttype_kind), "x")`          |

Same pattern for `~`, `≁`, `>`, `<`, `in`, etc. — the LHS becomes the
`compose(id-rel, primary-rel)` chain.

## Multi-hop traversal

| Julia in context                   | Rust                                                                             |
|------------------------------------|----------------------------------------------------------------------------------|
| `person.name` (cast context)       | `compose(&d.cast_person, &d.person_name)`                                        |
| `person.aka.name` (cast)           | `compose(&d.cast_person, compose(&d.person_aka, &d.akaname_name))`               |
| `Person.aka.name` (person context) | `compose(&d.person_aka, &d.akaname_name)`                                        |
| `character.name` (cast)            | `compose(&d.cast_character, &d.character_name)`                                  |

## Implicit primary on outputs

When the OUTPUT of a query column is an ID (not a string), Julia auto-resolves
to the entity's primary field at print time. In Rust make it explicit:

- `co × title` where `co` yields Company-id → `prod(compose(co, &d.company_name), &d.movie_title)`
- `lk × …` where `lk` yields MovieLink-id → `compose(lk, compose(&d.movielink_type, &d.linktype_link))`
- `info → (gf : Info.info)` → already string-valued, no further resolution.

If a let-bound query is named after an entity (`co`, `lk`, etc.) and used in
output, look at the surrounding `.name`/`.title` — if absent, compose with the
entity's primary string field as above.

## `let` bindings

Julia `let x = …, y = …; body` where `x` is used twice in `body`. In Rust:
define a helper fn that returns a fresh instance:

```rust
fn co_27<'d>(d: &'d Data) -> impl Query<R = i64> + 'd {
    in_set(&d.movie_company, conj(
        keys(ne(&d.company_country, "[pl]")),
        // … the rest of the Company-side conjunction …
    ))
}
```

Use `co_27(d)` once for the conjunct (wrap in `keys(...)`) and once for the
projection (e.g. `compose(co_27(d), &d.company_name)`). Each call builds a
fresh value — that's fine, the structures are cheap.

If `x` is used only once, inline it.

`impl Query<R = i64> + 'd` (or `impl SetQ + 'd`) — the `'d` lifetime ties the
returned value to the borrows it holds on `d`. Add it whenever the helper
borrows from `d` (which is always).

## Named tuple constants — `super::sets`

`kw7()`, `kw8()`, `kw10()`, `voice3()`, `voice4()`, `writer5()`, `genre6()`,
`murder4()`, `nordic8()`, `nordic9()`, `nordic10()`, `link3()`.

Use as: `in_vec(compose(&d.movie_keyword, &d.keyword_keyword), kw8())`.

## Output formatting

- 1 col: `Option<&'static str>` → `fmt1(m)`.
- 2 cols (Prod a × b): `[Option<&'static str>; 2]`, destructure `(a, b)`, → `fmt2`.
- 3 cols (a × b × c → `prod(prod(a,b),c)`): destructure `((a, b), c)`, → `fmt3`.
- 4 cols (a × b × c × d → `prod(prod(prod(a,b),c),d)`): destructure `(((a, b), c), d)`, → `fmt4`.

All outputs in JOB are string-valued (after the primary-field resolution).
Empty results land as `"(empty)"` automatically when `m[0].is_none()`.

## Multi-conjunct nesting

`a ∧ b ∧ c ∧ d` → `conj(a, conj(b, conj(c, d)))` (right-nest). Same for ∨.

## Common patterns

### Movie-rooted (templates 1-5, 11-15, 22)
```rust
fn qXa(d: &Data) -> String {
    let q = restrict(d.movie, restrict(
        conj(/* movie conjuncts */),
        /* projection — usually prod(...) */,
    ));
    // collect mins via q.drive(|_, tuple| { … });
    // return fmtK(m).
}
```

### Movie + cast filter + cast projection (templates 6-10, 16-20)
```rust
fn qXa(d: &Data) -> String {
    let q = restrict(d.movie, restrict(
        conj(/* movie conjuncts */),
        prod(
            compose(&d.movie_cast, restrict(
                conj(/* cast conjuncts as keys(...) */),
                /* cast projection — `person.name`, `character.name`, etc. */,
            )),
            &d.movie_title,
        ),
    ));
    // …
}
```

## Naming
Each query fn: `q<lowercase id>` (e.g. `q2a`, `q11d`, `q22c`).
ENTRIES key is the exact Julia name string ("2a", "11d", "22c").
ENTRIES oracle is the exact second-arg string from `_q("name", "oracle")`.

## Pitfalls
- Always borrow leaves with `&` in operator args (`compose(&d.foo, …)`).
  `d.movie` (Universe) is the only exception — it's Copy.
- `conj(keys(...), keys(...))` — wrap Query in `keys()` when it's used as a
  SetQ conjunct.
- `set_diff(a, keys(b))` — same for the RHS of `-` if it's a Query.
- For `(production_year >= X) ∧ (production_year <= Y)` use
  `conj(keys(ge(&d.movie_production_year, X)), keys(le(&d.movie_production_year, Y)))`
  — each comparison is its own `keys(Filter)` conjunct.
- Don't forget the OUTERMOST `restrict(d.movie, …)` — the query is anchored at
  the movie universe.
