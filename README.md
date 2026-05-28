# Prela: Purely Algebraic Relation Combinators

Prela is an embedded query language
 based on [Tarski's Algebra of Relations](https://en.wikipedia.org/wiki/Relation_algebra).
Prela queries are concise, clear, and fast.
The language is implemented by direct embedding 
 (a.k.a. [shallow embedding](https://decomposition.al/blog/2015/06/02/embedding-deep-and-shallow/))
 in a host programming language:
 its operators are regular functions in the host.
The implementation follows [continuation-passing style](https://en.wikipedia.org/wiki/Continuation-passing_style),
 which produces highly efficient code when compiled.
We provide two prototypes:
 the Julia engine enjoys elegant syntax thanks to operator overloading and multiple dispatch,
 while the Rust engine gives you (slightly) uglier but fast code.

> [!NOTE]
> Prela is a research prototype in early development. 
> Expect constant and sweeping changes to both language design and implementation.


## Example

Prela queries are readable even to the untrained eye. 
Consider Join Order Benchmark [22a](https://github.com/gregrahn/join-order-benchmark/blob/master/22a.sql):

```julia
movie
   : (info → (Info.type == "countries")
           ∧ (Info.info in ("Germany", "German", "USA", "American")))
   ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
   ∧ (production_year > 2008)
   ∧ (kind in ("movie", "episode"))
   → title
   × ((data → (Data.data < "7.0") ∧ (Data.type == "rating")) → Data.data)
   × ((company → (Company.note ≁ r"\(USA\)")
              ∧ (Company.note ~ r"\(200.*\)")
              ∧ (Company.country != "[us]")
              ∧ (Company.type == "production companies")) → Company.name)
```

Intuitively, the query looks for `movies` satisfying several conditions:

- It's German or American
- It's a thriller
- It's produced after 2008
- It's a movie or a TV series episode (which we'll also just call a "movie")

Then, for each such movie, output the following attributes:

- Its title
- Its rating data, if lower than 7
- The name of its production company, if satisfying further conditions

In SQL's way of thinking, `movie` would be in the `FROM` clause (along with other tables involved),
 the predicates between `:` and `→` are in the `WHERE` clause,
 and what comes after `→` are `SELECT`ed.
But unlike SQL, Prela can freely interleave predicates and outputs,
 resulting in more natural queries as shown above.
And instead of explicit conditions,
 joins in Prela are reflected by the *structure* of the query in a navigational style.

## Data Model

To understand what's going on under the hood, we should first clarify
 the data model used by Prela.
This is where Tarski's Algebra of Relations comes in:
 everything in Prela is a binary relation,
 and a query is built up with operators
 that take in and produce binary relations,
 called *relation combinators*.
One way to think about this is a very extreme form
 of normalization: every table with k columns
 is "shredded" into k binary tables, one per column.

With that in mind, let us consider a simplified version of the query above:

```julia
movie : (production_year > 2008) → title
```

Here, `title` and `production_year` are both attributes of
 the same table in the original schema,
 but in Prela, each of them becomes a binary relation,
 mapping every movie (ID) to its title and production year, respectively.
`movie` can be thought of as a unary relation storing all the movie IDs;
 but if we strictly following the "everything is a *binary* relation" doctrine,
 a unary relation of type `T` is just a (degenerate) binary relation of type `() -> T`
 where `()` is the unit type.

Binary operators like `>` and `in` are regular Julia functions,
 but overloaded to operate on relations.
For example, `production_year > 2008` returns a binary relation that's
 a subset of `production_year`, such that the second column (the "value" column)
 contains only values greater than 2008.
The same thing happens for `Info.type == "countries"` and `Info.info in ("Germany", "German", "USA", "American")`
 in the full query, as well as for all other predicates.

Next, the `:` operator is the *restriction* operator:
 it takes two relations, and restricts the last column of the LHS with the first column of the RHS.
In this example, since `movie` is unary (we won't insist on the pedantry of binary relations), there's
 only one column over the movie IDs;
 on the other hand, the first column of `production_year > 2008` contains the IDs of all movies
 produced after 2008.
So `movie : (production_year > 2008)` is a unary relation over the IDs of all post-2008 movies.

The `→` operator is *relational composition*.
It is the workhorse of both Prela and Tarski's Algebra of Relations.
The key to understanding relational composition is to view relations as 
 a generalization of functions:
 a binary relation of type `(X, Y)`
 generalizes a function of type `X -> Y`
 by allowing multiple different "output" (`Y`) values
 per "input" (`X`).
For this reason, we will abuse `X -> Y` to denote the type of a binary relation.

From this perspective, it is then natural to see `→`
 as a generalization of the function composition ($f \circ g$):
 `R → S` first "applies" `R` to each `x` to get a bunch of `y`,
 then for each `y`, apply `S` to get a bunch of `z`.
In code:

```julia
function R → S(x)
for y in R[x]:
  for z in S[z]:
    print((x, z))
end
```

In math: $R \rightarrow S = \{(x, z) \mid \exists_y . (x, y) \in R \land (y, z) \in S \}$.

In standard relational algebra: $R \rightarrow S = \pi_{x, z}(R \Join{R.y = S.y} S)$ 
 where R's schema is over x and y, and S's schema is over y and z.

Going back to our example, the `→` operator takes two inputs, namely 
`movie : (production_year > 2008)`, which has all movies after 2008,
 and `title` which has type `Movie -> String`.
Recall that a unary relation really has type `() -> T`,
 so the first input is really a `() -> Movie`.
The composition then produces an output of type `() -> String`,
 namely, a unary relation containing strings,
 which are all the movie titles.

Another (perhaps more natural) way to think about all these is to pretend
 every movie is a JSON object called `movie`,
 and its attributes like `title` and `production_year` are JSON attributes,
 then `→` will feel like field access, and `:` lets you specify filters.

There are two more constructs in our original example,
 the conjunction `∧` and product `×`.

A conjunction takes two relations of types `X -> Y` and `X -> Z`,
 and *intersects* their domain `X` to produce a relation of type `X -> ()`.
This allows us to combine different predicates:
 `(production_year > 2008) ∧ (kind in ("movie", "episode"))`
 gives us movies produced after 2008 *and* whose `kind` is either "movie" or "episode".

The product is similar to conjunction but preserves the "output types":
 with `R: X -> Y` and `S: X -> Z`, 
 `R × S` has type `X -> (Y, Z)`, 
 and is exactly the join of `R` and `S` on their first column.
This allows us to combine different columns in the output:
 in the full example, we output title, data (with additional filters),
 and company name - the last because
 `company : ... → Company.name` computes a relation of type `Movie -> String`
 mapping each movie to its company's name.

Note that `×` does not exist in Tarski's algebra, as it somewhat "pollutes"
 the "everything is binary" doctrine - we're now allowed to have tuples!
On one hand, `×` is a practical convinience, as we often need to output multiple columns;
 on the other hand, the pollution is contained, as tuples are
 just opaque values stored in a column,
 and all operators still behave as usual over relations containing tuples.

TPCH [q21](https://github.com/dragansah/tpch-dbgen/blob/master/tpch-queries/21.sql):

```julia
late = lineitem ∧ (receiptdate > commitdate)
n_distinct = vs -> length(unique(vs))
qualifying = (late
    ∧ (Li.supplier → supplier ∧ (Su.nation → Na.name == "SAUDI ARABIA"))
    ∧ (order → (orders ∧ (Ord.status == "F"))
                # EXISTS another supplier on the order (across all lineitems)
                ∧ ((order ← Li.supplier) ▷ n_distinct > 1)
                # NOT EXISTS another LATE supplier (only L1 is late)
                ∧ ((order ← (late → Li.supplier)) ▷ n_distinct == 1)))
counts = (Li.supplier ← qualifying) ▷ ((a, _) -> a + 1, 0)
counts ⊗ Su.name
```

In the examples, constructs like `movie`, `Info.type` are regular Julia variables of type
 `Relation`, and operators like `→`, `∧`, and `in` are regular Julia functions overloaded
 to operate on relations.
Notably, tables are decomposed into [sixth normal form](https://en.wikipedia.org/wiki/Sixth_normal_form),
 so `keyword` is a *relation* mapping each movie ID to a string.
The overhead of "joining back together" the decomposed columns is eliminated by continuation passing style
 which produces code that co-iterates the column tables.

Directly embedding Prela like this allows one to freely intermix queries with
 code of the host language to extend the reach of Prela,
 both in terms of expressiveness and performance.
For example, the Prela version of [TPCH Q13](https://github.com/dragansah/tpch-dbgen/blob/master/tpch-queries/13.sql)
 uses Julia code to implement `LEFT JOIN` semantics (Prela currently [has no `NULL`s](https://arxiv.org/abs/2307.15751)):

```julia
let live_orders = orders ∧ (Ord.comment ≁ r"special.*requests"),
    # Per-customer order count (only for customers with at least one match)
    count_per_cust = (Ord.customer ← (live_orders → date)) ▷ ((a, _) -> a + 1, 0)
    # Build the c_count → custdist distribution. Customers with no matching
    # orders get c_count = 0 (LEFT JOIN semantic).
    dist = Dict{Int, Int}()
    n_with = 0
    Prela.drive(count_per_cust, (_, c) -> begin
        dist[c] = get(dist, c, 0) + 1
        n_with += 1
    end)
    dist[0] = customer.n - n_with
    InlineRel{Int, Int}([k => v for (k, v) in dist])
end
```

Unlike SQL's user-defined functions, Prela "UDF"s are inlined and compiled together with the outer query
 without penalizing performance.
User can also swap out parts of the query with custom kernels to squeeze out extra performance,
 as exercised in the [Rust TPCH queries](./rust/src).

See [julia/queries.jl](./julia/queries.jl) and [julia/tpch_queries.jl](./julia/tpch_queries.jl) for more examples,
 or the corresponding Rust versions under [rust/src](./rust/src).

## Benchmark

**Take performance numbers with a grain of salt**, Prela differs from
 traditional databases in many ways.
For example, there is no query optimizer in Prela,
 as queries are executed as-is;
 in other words, a Prela query *is* a query plan.
And because Prela's data model is based on binary relations,
 the schema is closer to Entity/Relationship than traditional
 relational schema. 
We've taken liberty refactoring the JOB schema to make Prela queries
 natural, but that also lead to stronger performance
 because the indexes turn out to suit the queries better.
Overall, the main takeaway from the numbers is that 
*the simplicity of Prela does not hold it back from running fast*.

The plots below compare the run time of the Rust and Julia implementation
 against DuckDB (1 thread) as baseline,
 over TPCH and the Join Order Benchmark.

<p>
  <img src="./rust/bench/tpch_scatter.png" width="49%" alt="TPC-H SF=1">
  <img src="./rust/bench/job_scatter.png" width="49%" alt="JOB">
</p>
