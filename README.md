# Prela: Purely Algebraic Relation Combinators

> "The calculus of relations has an intrinsic charm and beauty which makes it a source of intellectual
> delight to all who become acquainted with it." —Alfred Tarski

Prela is an embedded query language
 based on [Tarski's Algebra of Relations](https://www.cl.cam.ac.uk/teaching/1415/Databases/Tarski_1941.pdf).
Its queries are concise, clear, and fast.
It is implemented by 
 [shallow embedding](https://decomposition.al/blog/2015/06/02/embedding-deep-and-shallow/)
 in a host programming language:
 Prela operators are regular functions in the host.
The implementation follows [continuation-passing style](https://en.wikipedia.org/wiki/Continuation-passing_style)
 which compiles to efficient columnar execution.

> [!NOTE]
> Prela is a research prototype in early development. 
> Expect constant and sweeping changes to both language design and implementation.
> The implementation is in Rust (`rust/`), where the access-mode analysis
> falls out of trait inference and CPS monomorphizes to fused loops.
> Prela started life in Julia with an infix surface syntax; that
> implementation is preserved on the
> [`julia-engine`](../../tree/julia-engine) branch, and its notation lives on
> below as the concise way to write the algebra on paper.


## Example

Prela queries are readable even to those new to the language. 
Consider Join Order Benchmark [22a](https://github.com/gregrahn/join-order-benchmark/blob/master/22a.sql),
written in the algebra's notation:

```julia
movie
   : (info → (Info.type == "countries")
           ∧ (Info.info in ("Germany", "German", "USA", "American")))
   ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
   ∧ (production_year > 2008)
   ∧ (kind in ("movie", "episode"))
   → title
   × (data : (Data.data < "7.0") ∧ (Data.type == "rating") → Data.data)
   × (company : (Company.note ≁ r"\(USA\)")
              ∧ (Company.note ~ r"\(200.*\)")
              ∧ (Company.country != "[us]")
              ∧ (Company.type == "production companies") → Company.name)
```

Intuitively, the query looks for `movies` satisfying several conditions:

- It's German or American
- It's a thriller
- It's produced after 2008
- It's a movie or a TV series episode (which we'll also just call a "movie")

Then, for each such movie, output the following attributes:

- Its title
- Its rating, if lower than 7
- Its production company, if satisfying further conditions

In SQL's way of thinking, `movie` would be in the `FROM` clause (along with other tables involved),
 the predicates between `:` and `→` are in the `WHERE` clause,
 and what comes after `→` are `SELECT`ed.
But unlike SQL, Prela can freely interleave predicates and outputs,
 resulting in more natural queries as shown above.
You can also think of the `(data : ...)` and `(company : ...)` parts as *subqueries*
 which require special syntax in SQL, but are just subexpressions in Prela. 
And instead of explicit conditions,
 joins in Prela are reflected by the *structure* of the query in a navigational style.

## Data Model and Simple (SPJ) Queries

To understand what's going on under the hood, we should first clarify
 the data model used by Prela.
This is where Tarski's Algebra of Relations comes in:
 everything in Prela is a binary relation,
 and a query is built up with operators
 that take in and produce binary relations,
 which I call *relation combinators*.
One way to think about this is a very extreme form
 of normalization: every table with k columns
 is "shredded" into k binary tables, one per column.

With that in mind, let us consider a simplified version of the query above:

```julia
movie : (production_year > 2008) → title
```

In the executable Rust embedding the same query is a method chain —
each combinator below has a method spelling
(`:` is `.in_s`, `→` is `.o`, and so on):

```rust
movie().in_s(production_year().gt(2008)).o(title())
```

Here, `title` and `production_year` are both attributes of
 the same table in the original schema,
 but in Prela, each of them becomes a binary relation,
 mapping every movie (ID) to its title and production year, respectively.
`movie` is also a binary relation, albeit a little special:
 it corresponds to the primary-key ID column of the original table,
 and is the *identity* relation over the IDs;
 in other words, `movie` contains `(i, i)` for every ID `i` in the table.
Overall, each "column table" can be thought of as a map from the primary
 key to its corresponding value.

Binary operators like `>` and `in` are regular Julia functions,
 but overloaded to operate on relations.
For example, `production_year > 2008` returns a binary relation that's
 a subset of `production_year`, such that the second column (the "value" column)
 contains only values greater than 2008.
The same thing happens for `Info.type == "countries"` and `Info.info in ("Germany", "German", "USA", "American")`
 in the full query, as well as for all other predicates.

Next, `:` is the *restriction* combinator:
 it takes two relations, and restricts the last column of the LHS with the first column of the RHS;
 i.e., it is exactly a left-semijoin.[^1]
In this example, `movie : (production_year > 2008)`
 semijoins `movie` with the filterd-out `production_year` relation,
 and since `movie` is the identity relation,
 we're left with the IDs for movies made after 2008.

[^1]:  `:` binds tighter than `→`. 

The `→` combinator is *relational composition*.
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

In math: $R \rightarrow S = \lbrace (x, z) \mid \exists y . (x, y) \in R \land (y, z) \in S \rbrace$.
In standard relational algebra: $R \rightarrow S = \pi_{x, z}(R \Join_{R.y = S.y} S)$ 
 where R's schema is over x and y, and S's schema is over y and z.

Going back to our example, `→` takes two inputs:

- `movie : (production_year > 2008)`, which has all movies after 2008
 and is of type `Movie -> Movie`
- `title` which has type `Movie -> String`

Their composition then produces an output of type `Movie -> String`,
 namely, a mapping from each qualifying movie to its title.

Another (perhaps more natural) way to think about all these is to pretend
 every movie is a JSON object called `movie`,
 and its attributes like `title` and `production_year` are JSON attributes,
 then `→` will feel like field access, and `:` lets you specify filters.

There are two more constructs in our original example,
 the conjunction `∧` and product `×`.
They are in fact exactly the same - `∧` is just an alias of `×`.
The product combinator takes two relations of types `X -> Y` and `X -> Z`,
 and joins them on `X` to produce an output of type `X -> (Y, Z)`.
When used as a conjunction (suggested by the `∧` symbol),
 product allows us to combine different predicates:
 `(production_year > 2008) ∧ (kind in ("movie", "episode"))`
 gives us movies produced after 2008 *and* whose `kind` is either "movie" or "episode".
When spelled `×`, it combines different columns in the output:
 in the full example, we output title, data (with additional filters),
 and company name - the last because
 `company : ... → Company.name` computes a relation of type `Movie -> String`
 mapping each movie to its company's name.

Note that `×` does not exist in Tarski's algebra, as it somewhat "pollutes"
 the "everything is binary" doctrine - we're now allowed to have tuples!
On one hand, `×` is a practical convinience, as we often need to output multiple columns;
 on the other hand, the pollution is contained, as tuples are
 just opaque values stored in a column,
 and all operators still behave as usual over relations containing tuples.[^2]

 [^2]: Mathematically speaking, Tarski's algebra is an *abstract algebra* specified
 by a set of axioms, and there is a chance that the concrete algebra implemented by
 Prela satisfies these axioms. 

## CTEs, UDFs, and Aggregation

Since Prela is directly embedded in the host language,
 we can borrow many constructs from the host to get
 many features that are considered advanced in other query languages,
 *for free*.
Consider TPCH [q21](https://github.com/dragansah/tpch-dbgen/blob/master/tpch-queries/21.sql):

```julia
late = lineitem : (receiptdate > commitdate)
n_distinct = vs -> length(unique(vs))

qualifying = late : (Li.supplier → (Su.nation → Na.name == "SAUDI ARABIA"))
                  ∧ (order → (Ord.status == "F")
                             # EXISTS another supplier on the order (across all lineitems)
                           ∧ ((order ← Li.supplier) ▷ n_distinct > 1)
                             # NOT EXISTS another LATE supplier (only L1 is late)
                           ∧ ((order ← late → Li.supplier) ▷ n_distinct == 1))

counts = (Li.supplier ← qualifying) ▷ ((a, _) -> a + 1, 0)
counts × Su.name
```

The first line assigns the result of a query to the variable `late`,
 which would require CTEs in SQL,
 but is simply a variable assignment in Prela/Julia!
The second line defines the `n_distinct` function to be used later
 in aggregation, which again requires UDFs in SQL,
 but is just a regular anonymous Julia function.

The next new Prela construct excercised by this query is group-by aggregation.
Let's focus on the expression `(order ← Li.supplier) ▷ n_distinct > 1`.
Intuitively, this can be read as
"group the suppliers by their orders, then `COUNT DISTINCT`, and keep the groups
 with more than 1 distinct suppliers.
In SQL it would look like this:

```SQL
  SELECT order, supplier
    FROM lineitem
GROUP BY order
  HAVING COUNT DISTINCT(supplier) > 1
```

To understand how the Prela query works, we shall first introduce one more combinator,
 the inverse `'`: `R'` just flips the columns of `R`, so if `R: X -> Y`,
 then `R': Y -> X`, like how you invert a function. 

Now, back to the query: the "left compose" `←` is short hand for
 "compose with inverse", i.e. `order ← Li.supplier` means `order' → Li.supplier`.
Here, the `order` relation has type `Li -> Order`, so its inverse `order'`
 has type `Order -> Li` and maps each order to the lineitem.
Then, `Li.supplier` has type `Li -> Supplier`,
 so the composition `order' → Li.supplier` has type `Order -> Supplier`,
 mapping each order to its supplier.
Next, the `▷` combinator groups its LHS relation by the first column,
 and computes the aggregate over its second column using the supplied aggregator function. 
In our case, we group the suppliers by order, then count the number of distinct suppliers per group.
Finally, `>` works as before and filters the LHS relation to keep the orders
 that are supplied by more than 1 distinct suppliers,
 which corresponds to the `HAVING` clause in SQL but requires no special treatment in Prela.
`▷` may appear limiting as it "can only group by one attribute", but that is not true -
 grouping by multiple attributes can be achieved by left-composing with a product!
 I'll leave that as an excercise for the reader.

See [rust/src/queries/](./rust/src/queries/) (all 113 JOB queries) and
[rust/src/tpch/common.rs](./rust/src/tpch/common.rs) (TPC-H) for the
executable forms, and [rust/TRANSLATION.md](./rust/TRANSLATION.md) for the
notation ⇄ method-chain dictionary. The original infix-syntax query suites
live on the [`julia-engine`](../../tree/julia-engine) branch.

## Implementation

If implemented naively, Prela would be pretty slow. 
Indeed, the first prototype was around 100x slower than DuckDB!
The naive implementation literally had each combinator take in and produce
 relations, which leads to a lot of materialized intermediates
 that are only filtered down later on.
There are [two standard solutions](https://www.vldb.org/pvldb/vol11/p2209-kersten.pdf) in DB to this problem:
 implement the [iterator model](https://cs-people.bu.edu/mathan/reading-groups/papers-classics/volcano.pdf)
 and vectorize it to make it fast, 
 or compile the query into low-level code running tight loops.
Both approach involve significant effort.
To be honest, I almost gave up at that point, because building a vectorized or compiled DB is much too large of a scope.
Until I remembered a dark magic I picked up from my functional programming days - [continuation-passing style](https://en.wikipedia.org/wiki/Continuation-passing_style) (CPS).
Actually, I never really understood CPS until working on Prela,
 and when it clicked, I literally cried because the idea was so beautiful.[^3]

[^3]: Long before AI psychosis, there was functional programming (FP) psychosis,
which triggers when someone learns about recursion, higher-order function, or monads.
On the other hand,
CPS was the OG AI psychosis, as the term was coined by Sussman and Steele in their [*AI Memo*](https://www.laputan.org/pub/papers/aim-349.pdf). 

### Passing Data Through Continuation

I'll try to explain CPS with a simple example.
Suppose we want to pass a list of numbers through a bunch of `map` operations:

```
xs.map(x -> x + 1).map(x -> x * 2)
```

A "direct style" implementation would have the `map` function take in a list, 
 apply the function to each element, then produce another list.
This means before the final results, we will produce an intermediate list
 from the first `map`.
In other words, a direct-style function takes something and *makes* another thing.

In CPS, a function doesn't *make*, it *does*.

Think of someone who works for you. You tell them: "take this list of numbers, and add one to each of them".
In direct style, they would add one to every number, and hand the result back to you.
In CPS, they say: "OK I'll do that, but also tell me what you will do with the new numbers, and I'll do that too".
Then, you answer by passing in the *continuation* `k` which will get applied after their job is done.

What's powerful about continuations is that they *compose*:
 in our example, if `iter()` *itself* takes a continuation,
 we can fuse together everything into one pass!

Let's look at some code.

First, suppose there's an `iter` function that takes a continuation `k`
 and applies it per `x`:

```
def iter(xs, k):
  for x in xs:
    k(x)
```

A `map` then takes an `iter` and returns another one by *doing* `f`:

```
def map(iter, f):
  xs, k -> iter(xs, x -> k(f(x)))
```

Now, suppose `blah.map(f)` desugars to `map(blah, f)`, then `iter.map(f)`
 becomes `map(f, iter)`.

If we inline the definition, then `iter.map(x -> x + 1).map(x -> x * 2)`
 becomes a brand new iter that fuses both `map`s into one pass:

```
def iter_mapped(xs, k):
  for x in xs:
    k((x + 1) * 2)
```

There's no intermediate list at all: each `x` flows through `+ 1` and `* 2`
 and arrives in `k` before the next `x` is read.
Instead of applying `f` to each `x` and returning the new list,
 `map` applies `f`, then immediately applies the continuation `k` to the result.
Finally, to get the result out, we supply an *collect* continuation:

```
def collect(iter):
  ys = []
  iter(y -> ys.insert(y))
```

We can chain together any number of steps, and after inlining,
 they will all collapse into a *sinlge* pass of iteration over the input without any intermediates.

So far, this is nothing new, and has been known in functional programming for a long time.
For Prela, this means a simple, modular CPS-style engine —
 each combinator a ~3-line method taking an `FnMut` continuation —
 monomorphizes under `rustc` into the same fused loop nests a query compiler
 would emit, without any iterator overhead.
(We measured the alternative: an external-iterator port of the whole engine
 matches CPS on scan-shaped plans but loses 1.3–3.5× on probe-heavy ones —
 see `rust/experiments/pull_vs_push.md` on the `pull-experiment` branch.)
But when combined with vector-based physical data storage,
 something incredible happens:
 *CPS automatically transforms the algebra-style queries to recover columnar query execution*!
The details on that deserves its own article, and I'll defer that to another time.

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

The plots below compare the run time of the Rust implementation — plus the
 retired Julia staged engine (`julia-engine` branch) as a historic series —
 against DuckDB 1.5.3 (1 thread) as baseline, over TPCH and the Join Order
 Benchmark.
On JOB, Rust Prela runs the 113 queries in 5.0s and Julia in 9.6s vs
 DuckDB's 15.3s (3.1× and 1.6× faster, winning 99 and 83 of 113).
On TPC-H SF=1, idiomatic Rust Prela matches DuckDB's vectorized engine
 (0.91s vs 0.86s) and the hand-optimized variant beats it 2× (0.44s);
 the Julia implementation trails at 1.4–3.3s.

<p>
  <img src="./rust/bench/tpch_scatter.png" width="49%" alt="TPC-H SF=1">
  <img src="./rust/bench/job_scatter.png" width="49%" alt="JOB">
</p>
