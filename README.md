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
> I'm currently focused on polishing the Julia implementation,
> and there are ~~vibe-ported~~ *experimental* Rust and Zig ports on separate branches.


## Example

Prela queries are readable even to those new to the language. 
Consider Join Order Benchmark [22a](https://github.com/gregrahn/join-order-benchmark/blob/master/22a.sql):

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
counts ⊗ Su.name
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

See [julia/queries.jl](./julia/queries.jl) and [julia/tpch_queries_idiomatic.jl](./julia/tpch_queries_idiomatic.jl) for more examples.

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
