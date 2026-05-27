# Prela: Purely Algebraic Relational Combinators

Prela is an embedded query language
 based on [Tarski's Algebra of Relations](https://en.wikipedia.org/wiki/Relation_algebra).
Prela queries are concise, clear, and fast.
The language is implemented by direct embedding 
 (a.k.a. [shallow embedding](https://decomposition.al/blog/2015/06/02/embedding-deep-and-shallow/))
 in a host programming language:
 Prela operators are implemented as regular functions in the host language.
The implementation follows [continuation passing style](https://en.wikipedia.org/wiki/Continuation-passing_style),
 which produces highly efficient code when combined with monomorphization and inlining.
We provide two implementations:
 the Julia engine enjoys elegant syntax thanks to operator overloading and multiple dispatch,
 while the Rust engine gives you (slightly) ugly but fast code.

## Examples

Prela queries are readable even to the untrained eye.

Join Order Benchmark [22a](https://github.com/gregrahn/join-order-benchmark/blob/master/22a.sql):

```julia
movie
   → (info → (Info.type == "countries")
           ∧ (Info.info in ("Germany", "German", "USA", "American")))
   ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
   ∧ (production_year > 2008)
   ∧ (kind in ("movie", "episode"))
   : title
   × ((data → (Data.data < "7.0") ∧ (Data.type == "rating")) → Data.data)
   × ((company → (Company.note ≁ r"\(USA\)")
              ∧ (Company.note ~ r"\(200.*\)")
              ∧ (Company.country != "[us]")
              ∧ (Company.type == "production companies")) → Company.name)
```

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
                ∧ ((order ← (late : Li.supplier)) ▷ n_distinct == 1)))
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
