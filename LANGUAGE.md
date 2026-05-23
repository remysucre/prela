Everything in Prela is a relation of arity at most 2.
Such a relation can be thought of as a (finite) function
 that can return multiple results for the same input.

Prela is typed, and the same name can be used for different relations thanks to type inference.

Let's look at what the JOB schema looks like in Prela:

```
movie: Movie

info: Movie -> Info

Info : {
  info : String,
  type : InfoType,
  note : String
}
```

Compare with SQL:

```
CREATE TABLE movie_info (
    id integer NOT NULL PRIMARY KEY,
    movie_id integer NOT NULL,
    info_type_id integer NOT NULL,
    info text NOT NULL,
    note text
);
```

`movie: Movie` declares the name `movie` as an (abstract) unary relation
 representing the universe of all `Movie` entities.
There is an abstract `Movie` type instead of the integer ID `movie_id`,
 and we're explicit that `info` returns the `Info` of a `Movie`.
This makes is clear that `info` is a foreign key relationship between `Info` and `Movie`.
The "struct" `Info` is in fact shorthand for declaring 3 relations:
 `info: Info -> String, type: Info -> InfoType, note: Info -> String`.
This allows us to "access fields" with simple relational composition (introduced formally soon):
 `movie.info.note`.

Sequential composition / join `.`: `r . s` with `r: x -> y, s: y -> z` is the relational composition
 `t: x -> z`.
If `s: y`, then `t: x -> y` (range restriction); if `r: y`, then `t: y -> z` (domain restriction).
If they are both unary, then same as intersection.

Intersection `&`: `r: x -> y & s: x -> z` is the intersections of their keys, i.e. a set over `x`.

Set difference `-`: `r - s` is `r` with keys not in `s`'s domain. Same precedence as `&`,
 left-associative. Prela has no NULLs or 3VL — fully normalized binary relations mean a
 missing value is simply absent, so SQL's `IS NULL` is expressed as `- r`. E.g. inside
 `company.(...)`, `& type == "production companies" - note` reads "matching companies that
 have no note".

Select `:`: same as sequential composition, but requiring lhs to be unary (domain reistriction).

Disjunction `|`: `r | s` is the union of compatible relations. Used between
 predicates it reads as OR, e.g. `info ~ r"^Japan:.*200" | info ~ r"^USA:.*200"`.

Predicates are applied to the range of each relation:
 `r < 3` with `r: x -> y` filters `r` by `y < 3`.

Because the fundamental data model of Prela is over unary/binary relations,
 creating "tuples" requires a bit more machinery.

Parallel composition `,`: strictly binary. `r: x -> y, s: x -> z` returns `t: x -> (y, z)`,
 a 2-tuple per `x`. Tuple members are accessed positionally: `t.0` is `r` with domain
 restricted to those shared with `s`, and `t.1` is `s` similarly restricted.
For more than two components, nest by association: `a, b, c` parses as `(a, b), c` and
 lands as `x -> ((y_a, y_b), y_c)`. Access: `.0.0`, `.0.1`, `.1`.

Per-`x` semantics: the cross product of tuple sets from each side. If `r` yields multiple
 `y` values and `s` yields multiple `z` values for the same `x`, every combination is emitted.

Primary field: each type has a designated primary field, defaulting to the first field
 of the struct. By convention, single-field lookup types use a name matching the type
 (`Keyword.keyword`, `Kind.kind`), while multi-field types pick a semantic name
 (`Movie.title`).

Predicate elision: when a predicate compares an entity-typed expression to a scalar
 literal (e.g. `keyword == "sequel"` where `keyword: Movie -> Keyword`), Prela auto-
 traverses to the entity's primary field. So `keyword == "sequel"` is sugar for
 `keyword.keyword == "sequel"`.

Returning entities: there is no explicit unwrap operator. To return a scalar, compose
 to the relevant scalar relation. To return an entity, just include it in a `,` or `:`;
 display layers render the entity via its primary field.

Aggregation `min, max, sum, ...`: `agg(r)` where `r: x -> y` groups by `x` and aggregates over `y`.

Here's JOB q22a:

```
movie.(
    info.(type == "countries" & info in ("Germany", "German", "USA", "American"))
  & keyword in ("murder", "murder-in-title", "blood", "violence")
  & production_year > 2008
  & kind in ("movie", "episode")
  : title
  , data.(data < "7.0" & type == "rating")
  , company.(
       note !~ r"\(USA\)" &
       note ~ r"\(200.*\)" &
       country != "[us]" &
       type == "production companies"
    )
)
```

## Related work

Prela sits in a small family of navigational query languages over typed schemas:

- **DAPLEX** (Shipman, 1981) and **FQL** — functional data model: entities are first-class
  and relations are multi-valued functions between them. Closest in spirit to Prela's
  struct sugar + dotted composition.
- **CQL** (Categorical Query Language, Spivak/Wisnesky) — ER schemas as categories;
  queries compose functorially.
- **XPath / XQuery** — `book/author/name` reads like `book.author.name`; inline predicates
  `[year > 2008]` mirror Prela's filtering. Multi-valued by default.
- **jq** — `.author.name` for JSON; same navigational ergonomics but stream + callback
  (`select(...)`, `any(.)`) rather than algebraic operators. Predicate composition is
  `and`/`or` over booleans, not `&`/`|` over relations.
- **SPARQL** — fundamentally binary (RDF triples); same data model, pattern-matching
  surface rather than algebraic composition.
- **Cypher** (Neo4j) — `MATCH (a)-[r]->(b)` graph patterns; ER worldview, pattern-shaped.
- **Tarski's calculus of relations** and **allegory theory** — algebraic foundations for
  `.`, `&`, `|`, transpose.

The distinctive bet: stay strictly binary, drop the SQL `JOIN`/`FROM`/`SELECT` skeleton,
and lean on a small algebraic operator set so navigation reads like a path language while
remaining closed under composition — XPath/DAPLEX ergonomics on a Tarski/Codd foundation.
