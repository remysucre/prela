#import "style.typ": *
#import "tar-figure.typ": tar-figure

#show: paper.with(
  title: [Revisiting the Algebraic Foundation of Relational Data],
  authors: (
    (
      name: [Yisu Remy Wang],
      affiliation: [University of California, Los Angeles],
      department: [Department of Computer Science],
      location: [Los Angeles, CA, USA],
    ),
    (
      name: [Paul Talma],
      affiliation: [University of California, Los Angeles],
      department: [Department of Philosophy],
      location: [Los Angeles, CA, USA],
    ),
  ),
  abstract: [
    We revisit Tarski's Algebra of Relations (#noun[TAR]), an old formalism of relations predating Codd's relational algebra by over 100 years, as a new foundation for relational databases.
    We argue #noun[TAR] provides a better abstraction at both the semantic level and the physical level, in the context of modern application code and system architecture.
    To demonstrate the strengths of #noun[TAR], we design and implement Prela, a compositional and controllable query language, and show that queries written in Prela are concise, clear, and efficient.
  ],
)

= Introduction <sec-intro>

Over the past 50 years, Codd's relational model~#c("DBLP:journals/cacm/Codd70") has cemented its status as the foundation underlying database systems.
The relational algebra has therefore become the central abstraction bridging high-level semantics to low-level execution.
However, the world has changed both below and above this abstraction.
At the system level, OLAP engines increasingly adopt column-oriented storage, while transactional systems are built upon a key-value core.
On the application side, researchers are pushing towards higher abstractions like the Entity/Relationship model~#c("DBLP:conf/cidr/Deshpande25", "DBLP:journals/tods/Chen76"), with the goal to better support nested and graph data, and to reconcile the long-standing impedance mismatch between relational schema and application code.
As a result, both the physical layer and the semantic layer have drifted away from the original table-oriented abstraction.
Yet, as if by serendipity, the independent evolution of the physics and the semantics of relational data have led to the same destiny!
This reunion calls for new abstractions that better connect queries to their execution.
In this paper, we revisit an old formalism of relations predating Codd's relational algebra by over 100 years.
This formalism was developed by logicians from De Morgan, Peirce, Schröder, to Tarski and Givant, and is now commonly know as Tarski's Algebra of Relations (#noun[TAR])~#c("DBLP:journals/jsyml/Tarski41", "tarski1987formalization", "Maddux1991-MADTOO").
To demonstrate the potential of #noun[TAR] as a new foundation for modern database systems, we design and implement Prela, a _compositional_ and _controllable_ query language.
Using Prela, the programmer can write concise, clear, and efficient queries, all the while retaining fine-grained control over low-level details of query execution.

The power of Prela is best illustrated with an example.
Consider query 16b from the Join Order Benchmark~#c("DBLP:journals/pvldb/LeisGMBK015"), whose SQL formulation spans 23 clauses, 11 of which are used to specify join conditions.
@fig-job16b shows the complete Prela source for the same query.
We will revisit this query in detail later, but the intent is already clear at a glance: it looks for movies made by American companies with keyword matching "character-name-in-title", then outputs the movie title along with the alias for each cast member.
There are several notable differences between the Prela query and the SQL version.
First, there is no explicit ```sql FROM``` clause in Prela; instead, relations appear as arguments to _relation combinators_ like `.with`, `.and`, and `.select`, revealing the _structure_ of the query.
Similarly, join conditions are not specified by explicit join clauses, but by query structure.
Finally, thanks to Prela's algebraic nature, queries can be written in a compositional way: in the example, every subexpression, including `company.s(country).eq("[us]")`, `keyword.eq(...)`, and `cast.s(person).s(alias).s(text)`, is on its own a valid query, and they appear directly as inputs to relation combinators to build up the larger query.

#figure(kind: image, placement: top,
  {
  ```rust
  movie.with(company.s(country).eq("[us]")
        .and(keyword.eq("character-name-in-title"))
     .select(title
        .and(cast.s(person).s(alias).s(text)))
  ```
  },
  caption: [JOB query 16b in Prela],
) <fig-job16b>

The reader may be surprised to learn that the query in @fig-job16b[figure] is in fact regular Rust code.
Indeed, `movie`, `company`, `country`, etc., are Rust variables, and combinators like `.with`, `.and`, `.select`, etc., are Rust functions.
In other words, Prela is an _embedded query language_, or more simply, a _library_.
This means Prela queries seamlessly integrate with application code, and the programmer can pass in regular Rust functions as UDFs.
Under the hood, every Prela query is also a query plan, since each relational combinator corresponds precisely to an operator in #noun[TAR].
A common complaint in SQL is the difficulty of guiding the query optimizer when it fails to find an efficient plan; by contrast, the Prela user has complete control over all aspect of query planning, including join ordering, operator pushdown, materialization, and selection of physical data structures.
This level of control is increasingly relevant in the age of AI, as it puts the user---human or machine---back in the driver seat of performance tuning.
Nevertheless, this great power need not come with great responsibility, and Prela/#noun[TAR] can just as well serve as the _intermediate representation_ underlying a query optimizer---the implementation of which we leave as future work.
Finally, Prela queries are surprisingly "closer to the metal" than even the standard relational algebra, thanks again to the foundation of #noun[TAR].
In our running example, variables like `country` and `keyword` are backed directly by vectors in a column store, and relation combinators compile to efficient operations over the columns.

#figure(kind: image,
  tar-figure,
  caption: [Conventional database abstractions
    (top route) v.s. unified abstraction based on #noun[TAR] (bottom route)],
) <fig-compare-abstractions>

In summary, @fig-compare-abstractions[figure] compares the architecture of a traditional database with one based on #noun[TAR].
Unlike the layers of abstractions that pull the programmer away from the data, #noun[TAR] serves as the _universal representation_ shared by the programmer, the storage engine, the execution engine, and the optimizer, bringing each component closer to each other and enabling cross-cutting optimizations.

= Related Work <sec-related-work>

#runin["What goes around comes around"][
Despite countless attempts at replacement, the relational model has stood the test of time~#c("stonebraker2005what").
This paper does not try to supplant the relational model, but rather to refine and evolve it to meet modern needs.
#noun[TAR]---despite being a century older---can be understood as an evolution of the relational algebra towards column stores and modern application code; the Prela language can also be considered a prototype demonstrating how SQL can be extended and made more flexible.
In particular, #noun[TAR] and Prela stand upon the two pillars of physical and logical independence: the relation-centric #noun[TAR] assumes an abstract data model allowing for different physical implementations including dense vectors, hash maps, or bit sets; the corresponding Prela query accepts type annotations to specify any of these data structures.
Queries can be declared as views and used as inputs to other queries, should the logical schema change; addition and deletion of orthogonal attributes and relationships also will not break existing queries.
We believe ideas of #noun[TAR] and Prela can be incorporated into mainstream relational databases to simplify their design and implementation.]

#runin[#noun[TAR] in theory][
The connection between Tarski and Codd has been noted as early as the first #noun[PODS] conference in 1982, when Imielinski and Lipski~#c("DBLP:journals/jcss/ImielinskiL84") demonstrated how to emulate Codd's relational algebra with Tarski's _cylindric algebras_~#c("henkin1971cylindric"), an extension of Boolean algebras related to #noun[TAR].]

#noun[TAR] is a language for specifying operations on binary relations.
While its focus on binary relations lies at the core of our approach, #noun[TAR]'s expressive power is limited: it is equivalent to the three-variable fragment of first-order logic, and hence is strictly less expressive than Codd's relational algebra~#c("DBLP:conf/csl/Bussche01").
// The issue is that binary relations only carry information about pairs of elements, whereas we often wish to correlate several elements (for example, matching movies with their title and year).

In response, Gyssens, Saxton and Van Gucht~#c("DBLP:books/mk/freytagMV91/GyssensSG91") proposed modeling relational algebra by augmenting #noun[TAR] with a _tagging_ operator that introduces tuple values.
// enrich with product, aka fork
We prefer to enrich #noun[TAR] with a _fork_ operator, corresponding to Prela's product combinator `.and`, which is more natural for writing queries.
The resulting _fork algebra_, which we call #noun[TAR+], recovers the full power of Codd's relational algebra~#c("haebererForkAlgebras1997").
However, just as SQL extends Codd's algebra with operators for aggregation, ordering, and the like, Prela extends #noun[TAR+] to support the functionality of a modern query language.

Database theorists have also used #noun[TAR] to analyze graph and tree query languages like Regular Path Queries, XPath, and SPARQL~#c("DBLP:conf/icdt/FletcherGLBGVW11", "DBLP:journals/jacm/LibkinMV16", "DBLP:journals/igpl/SurinxFGLBGVW15", "DBLP:journals/jlap/HellingsWGG22").
This is thanks to how #noun[TAR]'s main operator, the relational composition, conveniently models path traversal.
Nevertheless, none of these languages were built upon #noun[TAR] _a priori_.
Van den Bussche~#c("DBLP:conf/csl/Bussche01") gives an introduction and survey of how Tarski's ideas have been applied to database theory.

#runin[#noun[TAR] in practice][
There have been sporadic efforts to bring the theory of #noun[TAR] into practice.
In 1992, Paredaens et al.~#c("DBLP:journals/sigmod/ParedaensBAGGTGSS92") presented #noun[GOOD], a "Graph-Oriented Object Database", and envisioned an implementation over a data model based on #noun[TAR].
The #noun[GOOD] system itself, however, was built atop a standard relational engine.
A follow-on technical report by Sarathy and Van Gucht~#c("sarathy1993implementation") describes an implementation of the #noun[GOOD] model in the #noun[IUGQL] query language.
More recently, Libkin et al. developed #noun[TriAL]~#c("DBLP:journals/tods/LibkinRSV18"), a language for recursive queries over RDF data based on a _triple algebra_ related to #noun[TAR].
Compared to these languages, Prela focuses on traditional non-recursive queries over relational data.
Prela also uses #noun[TAR] directly both as the surface syntax and the underlying storage abstraction, whereas the aforementioned systems all come with a separate, declarative query syntax.
This indirection complicates system design, obstructs performance tuning, and obscures the beauty of #noun[TAR].]

#runin[Dataframes][
Dataframe libraries like #noun[pandas]~#c("mckinney2010pandas") and #noun[polars]~#c("polars") provide an interface to tabular data based on relational algebra.
As such, these libraries also struggle to express multi-table queries succinctly, as each join operation must specify the join condition explicitly.
At the engine level, #noun[pandas] executes queries as-is, albeit _eagerly_ -- every dataframe operation immediately produces a result when it is called.
Other more optimized engines like #noun[polars] generate and optimize query plans, much like traditional OLAP databases.
Prela's engine can be thought of as implementing a lazy, push-based execution model.
Operators do not materialize intermediate results unless necessary, and the unoptimized queries already run fast.]

#runin[SQL extensions][
The secret behind the longevity of SQL is its ability to absorb ideas proposed in new query languages.
Recent work by Shute, Zheng, and Kudtarkar~#c("DBLP:conf/cidr/ShuteZK26") proposed SQL extensions to better support semantic data modeling and graph workloads.
Their proposal lead to several design points strikingly similar to those taken by Prela.
For example, they also support implicit joins without specifying join conditions, as well as multi-hop join/access syntax similar to the last line of @fig-job16b[figure].
With Prela, we aim to demonstrate that such extensions to SQL can be grounded on the firm theoretical foundation of #noun[TAR], and that they are not merely syntactic sugar, but can produce tangible performance benefits when connected to the underlying execution engine.]

#runin[Functional data models][
Several functional data models have been proposed over the years~#c("DBLP:conf/sigmod/BunemanF79", "Gray2005FunctionalApproach", "DBLP:journals/cj/KulkarniA86", "DBLP:conf/sigmod/Shipman79"), and a recent position paper by Dittrich~#c("DBLP:conf/edbt/Dittrich26") envisions a data model and query language where "everything is a function".
He then proposes a _function algebra_ where the operators are higher-order functions consuming and producing functions.
Prela can be considered a realization of that vision over #noun[TAR]: a relation in #noun[TAR] is precisely a (multi-valued) function.
Prela also goes beyond the syntax and semantics of the query language itself and explores the synergy between high-level abstraction and low-level execution.]

#runin[MonetDB and the DSM][
  MonetDB~#c("bonczMonetDBX100HyperPipelining2005") was one of the first systems to support columnar execution.
  It applies the _Decomposition Storage Model_~#c("copelandDecompositionStorageModel1985"), on which tables are stored in column-major order, to main memory.
  In its original implementation, MonetDB translated SQL queries into MIL, an intermediate language strikingly similar to #noun[TAR]~#c("bonczMILPrimitivesQuerying1999").
  MIL forms an algebra over binary relations and includes operators for selection, joins, grouping, aggregation, and the like.
  By storing tables as contiguous columns, MonetDB greatly reduced memory and I/O bandwidth on OLAP workloads~#c("manegoldOptimizingDatabaseArchitecture2000").
  And by lifting this columnar layout into the algebra, it achieved a simple and uniform data representation.
]

// TODO: shorten volcano exposition
However, the original MonetDB engine executes queries in an _operator-at-a-time_ fashion.
This prevents operator pipelining and results in large intermediate data structures.
The MIL algebra was therefore superseded by the X100 algebra and vectorized engine~#c("DBLP:conf/cidr/BonczZN05").
X100 follows the basic structure of the Volcano system~#c("DBLP:journals/tkde/Graefe94"), on which relational operators implement a `.next_tuple` method.
// When `.next_tuple` is called on a selection operator, for example, it will call `.next_tuple` on its child operator and check whether the returned tuple satisfies its selection predicate.
// If it does, the selection operator will return that tuple.
// Otherwise, it will continue to call `.next_tuple` on its child.
The Volcano model avoids materializing intermediate data structures, since only one tuple is "in flight" at any moment.
However, `.next_tuple` involves significant dynamic interpretation overhead.
// TODO: conceptually implements
X100 operators instead implement a `.next_batch` method that processes and returns a batch of $N = 1,000$ tuples at a time, thereby amortizing interpretation overhead.
Thus, MonetDB/X100 strikes a balance between the low interpretation overheads of columnar execution and the low memory overheads of tuple-at-a-time execution.

Prela demonstrates the viability of retaining the columnar abstraction at the storage, execution, and logical level without incurring memory or execution overheads.

// TODO: materialization is not inherent to columnar/binary algebra
// removing interpretation + memory? overheads: vec/compilation

#figure(
  kind: table,
  table(
    columns: 3,
    align: left + horizon,
    inset: (x: 4pt, y: 2pt),
    stroke: (x, y) => (
      left: if x > 0 { 0.5pt } else { none },
      top: if y == 1 { 0.5pt } else { none },
    ),
    [Name], [Syntax], [Semantics],
    noun[compose], [`r.select(t)` / `r.s(t)`], $pi_(r.1, t.2)(r njoin_(r.2 = t.1) t)$,
    noun[product], [`r.and(t)`], $r njoin_(r.1 = t.1) t$,
    noun[predicate], [`r.eq(v)`, `r.lt(v)`, ...], [$sigma_(r.2 = v)(r)$, $sigma_(r.2 < v)(r)$, ...],
    noun[filter], [`r.filt(p)`], $sigma_(p(r.2))(r)$,
    noun[restrict], [`r.with(t)`], $r times.l_(r.2 = t.1) t$,
    noun[map], [`r.map(f)`], $pi_(r.1, f(r.2))(r)$,
    noun[gather], [`r.gather(t)`], [_nested compose_#footnote[Nested compose is not expressible in standard relational algebra;
      formally, the semantics of `r.gather(t)`
      is $\{\(x, dot.double(union.big)_(#box(height: 0pt, move(dy: -1pt, $script(y in r[x])$))) t[y]\) | x in r.1\}$
      where $dot.double(union.big)$ is the _bag union_ and $r[x]$ is the
      bag of $y$ such that $(x, y) in r$ (similar for $t[y]$).]],
    noun[invert], [`r.inv()`], $pi_(r.2, r.1)(r)$,
    noun[group by], [`r.group_by(t)`], $pi_(t.2, t.1)(t times.l_(t.1 = r.2) r)$,
  ),
  caption: [Relational operators in Prela],
) <tab-operators>

= The Prela Language

This section defines the data model, syntax, and semantics of the Prela query language.
@tab-operators provides a reference of #noun[TAR] operators, their meaning, and their spelling in Prela.

== Data Model <subsec-data-model>

The fundamental datatype of Prela is the _binary_ relation.
Although restricting relations to be binary may appear limiting, it is simple to recover the flexibility of multi-column tables with ones over only two columns: every relation with $k$ columns is represented with $k$ binary relations, each mapping the row number to the corresponding column entry.
@fig-decomposition shows an example: a `movie` relation with columns `title`, `year`, and `keyword` decomposes to 4 binary relations: the first maps each ID to its row, and the other 3 map each row to its title, year, and keyword, respectively.
The special treatment of the ID column will make sense when we join across tables.

Decomposing a wide table into many binary tables may appear expensive: while the original table had $k$ columns, the decomposed tables now span a total of $2k$ columns.
However, since one column of each binary relation simply contains the row number, that column need not be explicitly stored, and each binary relation can be represented by a vector!
In @sec-implementation[section] we will also show how Prela's implementation eliminates the additional joins needed to "re-assemble" a wide table from its columns.
In other words, the "binarization" takes a detour to arrive at the same physical representation found in column stores.
This detour is nevertheless necessary, as it allows us to query the columns in a compositional way.

#figure(kind: image,
  {
    rel-table(
      columns: 4,
      [ID], [title], [year], [keyword],
      ..header-gap(4),
      [646], [The Godfather], [1972], [Crime],
      [478], [Seven Samurai], [1954], [War],
      [583], [Casablanca], [1942], [Romance],
    )
    v(0.6em)
    let t(head, ..rows) = rel-table(
      columns: 2,
      ..head,
      ..header-gap(2),
      ..rows,
    )
    stack(
      dir: ltr,
      spacing: 2.5pt,
      t(([ID], [\#]), [646], [0], [478], [1], [583], [2]),
      t(([\#], [title]), [0], [The Godfather], [1], [Seven Samurai], [2], [Casablanca]),
      t(([\#], [year]), [0], [1972], [1], [1954], [2], [1942]),
      t(([\#], [keyword]), [0], [Crime], [1], [War], [2], [Romance]),
    )
  },
  caption: [A wide table (top) and its decomposition (bottom)],
) <fig-decomposition>

== Select, Project, and Join

Prela queries are composed of operators applied to relational arguments.
The most important operator in Prela is the _relational composition_ `r.select(t)` which is equivalent to the relational algebra expression $pi_(r.1, t.2)(r njoin_(r.2 = t.1) t)$, where $r.i$ indicates the $i$-th column of $r$.
Intuitively, relational composition generalizes function composition in the same way (binary) relations generalize functions: the composition $g compose f$ is sometimes written $f; g$ and first applies $f$, then $g$.
If the relation `r` maps each `x` value to some `y` values, and if `t` maps each `y` to some `z`s, then `r.select(t)` first maps `x` via `r` to get `y`s, then maps each `y` via `t` to get `z`s.
Using our example in @fig-decomposition[figure], if we call the binary relations `movie`, `title`, `year`, and `keyword` from left to right, then `movie.select(title)` returns a binary relation mapping each movie ID to its title, and similarly for `movie.select(year)` and `movie.select(keyword)`.

#figure(kind: image,
  {
    let t(head, ..rows) = rel-table(
      columns: 2,
      ..head,
      ..header-gap(2),
      ..rows,
    )
    grid(
      columns: (auto, 1fr, auto),
      t(([\#], [company]), [0], [657], [1], [188], [2], [353]),
      [],
      stack(
        dir: ltr,
        spacing: 2.5pt,
        t(([ID], [\#]), [657], [0], [188], [1], [353], [2]),
        t(([\#], [name]), [0], [Paramount], [1], [Toho], [2], [Warner Bros.]),
        t(([\#], [country]), [0], [USA], [1], [Japan], [2], [USA]),
      ),
    )
  },
  caption: [Left: decomposed `company` column from the `movie`
    table; right: decomposed columns from the `movie_company`
    table],
) <fig-left-decomposed>

Composition also plays the role of joins in Prela.
Consider a `movie_company` table with three columns: the company ID, name, and country, each of which becomes a binary relation in Prela as shown on the right of @fig-left-decomposed[figure].
We also add a `company` column to the `movie` table linking each movie to the ID of its production company.
Now, we can look up the country of a movie's production company with `movie.s(company).s(id2row).s(country)`, where `.s` is shorthand for `.select`, and `id2row` is the relation mapping company IDs to row numbers.
Because joining via foreign keys almost always require "resolving" the key to its row, Prela automatically inserts `.s(id2row)` to key/foreign key joins, similar to how programming languages like Rust automatically dereference pointers upon a field access.
This allows us to simply write `movie.s(company).s(country)` which is pronounced "movie's company's country".
This is also what happened on the last line of @fig-job16b[figure].

Given that composition "throws away" the join attributes, the reader may be wondering if there are certain SQL queries that Prela cannot express.
Our answer is two-fold.
On one hand, real world queries almost always join on key/foreign keys, and keys rarely serve any other purpose --- it does not make sense to aggregate over them.
So dropping the keys after the join is not a real sacrifice.
On the other hand, it is true that the original #noun[TAR] is strictly less expressive than Codd's relational algebra.
One approach to bridging this gap is to introduce the so-called _pairing operators_ to #noun[TAR]~#c("DBLP:books/mk/freytagMV91/GyssensSG91").
In Prela, we propose the _product_ operator `.and` as an equivalent yet more practical extension.
`r.and(t)` joins the relations on their first column, i.e., $r njoin_(r.1 = t.1) t$.
Crucially, the output is still a _binary_ relation, whose first column is the intersection of $r.1$ and $t.1$, and the second column is the Cartesian product of $r.2$ and $t.2$ for each matching $r.1 = t.1$.
For example, `title.and(year)` is the relation mapping 0 to (The Godfather, 1972), 1 to (Seven Samurai, 1954), and 2 to (Casablanca, 1942).
Combining with `.select`, the expression `movie.select(title.and(year).and(keyword))` implements the SQL query ```sql SELECT title, year, keyword FROM movie```.

The next Prela operator exercised by @fig-job16b[figure] is the _predicate_ `r.eq(v)` which returns all rows of `r` whose right column equals `v`.
Other predicates like `.gt` (greater than), and `.rx` (regular expression match) work the same way.
There is also a generic `r.filt(p)` operator that filters `r` by any Boolean function `p`.
Going back to the example, `keyword.eq("character-name-in-title")` returns rows in `keyword` (which maps a movie to its keywords) such that the keyword is "character-name-in-title"; `company.s(country).eq("[us]")` first composes `company` (mapping a movie to its company) with `country` (mapping a company to its country) to get a mapping from each movie to its country, then keeps only those in the US.
Finally, the `.and` operator joins the filtered relations, keeping only American movies with character name in their titles.

The last operator used in our example is the _restriction_ `r.with(t)` which is exactly the left-semijoin $r times.l_(r.2 = t.1) t$, and it emulates SQL's ```sql WHERE``` clause.#footnote[Unfortunately `where` is a reserved keyword in Rust.]
A happy accident is that `.and` doubles as "logical conjunction" inside `.with`, because the semijoin ignores the second column of its right-hand-side.

#figure(kind: image,
  grid(
    columns: (55%, 45%),
    ```sql
    SELECT keyword, min(year)
      FROM movie
     GROUP BY keyword
    HAVING min(year) > 1950
    ```,
    ```rust
    movie.group_by(keyword)
         .gather(year)
         .map(min)
         .gt(1950)
    ```,
  ),
  caption: [Grouping and aggregation in SQL and Prela],
) <fig-grouping>

== UDFs, Structured Output, and Aggregation <subsec-grouping>

Just like the original form of Codd's relational algebra, #noun[TAR] was designed to model first order logic, and requires extensions to support practical workloads.
Prela supports "generalized projection" with the `.map` operator: `r.map(f)` maps the function `f` over each value in the second column of `r`.
One advantage of embedding Prela in Rust is to allow arbitrary Rust code to be used as UDFs.
In particular, the query can construct structured outputs.
For example, the following query returns a list of `Movie` structs instead of tuples:

#[
#show "|": set text(fill: black)
```rust
movie.select(title.and(year))
     .map(|(id,(t,y))| Movie{title: t, year: y})
```
]

Another extension allows Prela to return _nested_ outputs: `r.gather(t)` is like `r.select(t)`, but maps each distinct `r.1` to a list of `t.2` values.
For example, `movie.gather(keyword)` returns a binary relation mapping each movie ID to all the keywords associated with that movie.
Together with `.map`, we can use `.gather` to implement aggregation: `movie.gather(year).map(min)` returns the minimum year associated with each movie, by mapping the list function `min` over the list of years per movie.

To support grouping, we need to introduce one last operation from the original #noun[TAR]: the _inverse_ `t.inv()` simply flips the two columns of `t`.
We can now implement Prela's `r.group_by(t)` as `t.inv().with(r.inv())`, or $pi_(t.2, t.1)(t times.l_(t.1 = r.2) r)$ in relational algebra.
For example, the first line of the Prela query in @fig-grouping[figure] desugars to `keyword.inv().with(movie.inv())`.
First, `keyword.inv()` flips the `keyword` relation to map each keyword to the rows it appears in; restricting with `movie.inv()` is a no-op, but would be meaningful if `movie` were already restricted by other predicates.
Line 2 composes the result from line 1 with `year` to obtain a mapping from keyword to years.
Line 3 aggregates with `min` to find the earliest year associated with each keyword.
Finally, the last line `.gt(1950)` implements the ```sql HAVING``` clause in the SQL query.
Grouping by multiple columns can be achieved with the help of `.and`, for example `movie.group_by(keyword.and(year))`.
To aggregate over multiple columns, we can pass in a function that reduces all of them at once.

== `NULL` Handling

Missing values are ubiquitous in real world data and must therefore be handled gracefully by any database system~#c("toussaintTroublesNullsViews2022").
SQL explicitly represents missing values with a special `NULL` marker.
Unfortunately, SQL's implementation of `NULL` results in a three-valued logic: a comparison like `1997 < NULL` evaluates to `unknown` rather than `true` or `false`~#c("libkinSQLNullsTwoValued2023").
This three-valued logic has long been a source of bugs and confusion.

Prela takes a hybrid approach to missing data.
Base relations do not explicitly represent missing data.
Instead, missing data is simply omitted.
However, nullable columns may be explicitly marked as such using an `.opt` operator.
This lifts the column's type from `T` to `Option<T>`.
The semantics are then derived from Rust's `Option` semantics.
In particular, the logic remains two-valued: `None`--`Some<T>` comparisons evaluate to false and `None`--`None` comparisons treat `None` as a syntactic constant (so that `None == None` is `true`). 

== Discussion

Looking back at the example in @fig-job16b[figure], we see another, more intuitive way to understand the query: pretend that there are `movie` objects with field `company`, `keyword`, `title`, and `cast`, where `company` is its own object with field `country`, and similarly for `cast`, `person`, and `alias`.
Then the Prela query appears to access fields and apply predicates to them.
This is not accidental.
Central to the design of both #noun[TAR] and Prela is the focus on binary relations which closely model both attributes and relationships _à la_ Entity/Relationship.
Modern business logic usually start life as an E/R diagram, and the application code---commonly written in some object-oriented language---naturally mirrors the E/R model.
Built with a set of compositional operators, Prela queries blend into the surrounding application, obviating the need for ORMs.

Prela's readability and control are particularly relevant in the age of AI agents.
On the one hand, Prela provides unmediated access to the engine's operators, surfacing numerous opportunities for optimization.
As agents write an increasing share of queries, we expect them to exploit these opportunities to enable faster execution.
On the other hand, it remains important for humans to read and understand these queries.
We believe that Prela's clear and concise queries reduce the cognitive burden on the programmer seeking to understand their agent's query code.


= Implementation <sec-implementation>

In @subsec-data-model we explained how the apparent storage overhead of binary relations is eliminated by columnar storage.
It may appear a computational overhead remains, when we need to _reassemble_ a table from its columns.
Consider the following SQL query:

```sql
SELECT ID, title, keyword FROM movie
```

It outputs the ID, title, and keyword of each movie.
In a conventional database the query requires only a simple scan over the `movie` table.
Specifically, the execution over a column store would look something like this:

```python
for i in 0..n: print(movie[i],title[i],keyword[i])
```

In the above, a single loop _co-iterates_ over the selected table columns.
The corresponding Prela query is `movie.select(title.and(year))`.
Implemented naively, the Prela query would require two joins, one each for `.and` and `.select`.
In general, the number of joins grows proportionally with the number of columns involved in a Prela query, which is rather expensive.

The textbook solution to remove intermediate state during query execution is the iterator model~#c("DBLP:journals/tkde/Graefe94").
Iterators incur overhead like method calls and other bookkeeping operations, and two standard remedies are vectorization and query compilation~#c("DBLP:journals/pvldb/KerstenLKNPB18").
Unfortunately, both approaches require significant engineering effort that would exceed the scope of a prototype like Prela.
To achieve good performance while keeping Prela's implementation simple and modular, we turn to a technique from functional compilers --- deforestation in continuation-passing style (CPS)~#c("DBLP:conf/fpca/GillLJ93").

#figure(kind: image,
  ```rust
  lhs.select(rhs).drive(k) =
      lhs.drive(|x, y| rhs.probe(y, |z| k(x, z)))
  lhs.and(rhs).probe(x, k) =
      lhs.probe(x, |y| rhs.probe(x, |z| k((y, z))))
  col.drive(k) = for i in 0..col.len: k(i, col[i])
  col.probe(i, k) = k(col[i])
  ```,
  caption: [CPS protocol for `.select`, `.and`, and columns],
) <fig-cps>

The basic idea of CPS is for combinators to push computation down from the query root to its leaves instead of pulling data up from leaves to root.
More specifically, Prela queries can be accessed in two ways.
Calling `query.drive(k)` applies the closure `k` (the _continuation_) to each row of `query`.
Calling `query.probe(key, k)` looks up `key` in `query` and applies `k` to each resulting value.
For example, `movie.s(title).drive(print)` prints each `(id, title)` pair, while `movie.s(title).probe(3, print)` prints the title of the movie with ID 3.
Since every combinator receives and returns binary relations, `.drive` and `.probe` are well-defined for all Prela queries.
@fig-cps shows the definition of `.drive` and `.probe` for `.select`, `.and`, as well as input columns.

Let us unfold the definitions for our example query.
We start by driving the query with a continuation `k` which may print each result, or append it to a buffer:

```rust
movie.select(title.and(keyword)).drive(k)
```

To save space we will abbreviate `movie`, `title`, `keyword`, `.drive`, and `.probe` with `m`, `t`, `w`, `.drv`, and `.prb`, respectively.
Expanding `.drive` on `.select` drives its left-hand-side and probes its right-hand-side:

```rust
m.drv(|x, y| t.and(w).prb(y, |z| k(x, z)))
```

As `m` is an input column, driving it expands to a loop:

```rust
for i in 0..m.len: t.and(w).prb(i, |z| k(m[i], z))
```

Next, probing `.and` propagates to probing its arguments:

```rust
for i in 0..m.len:
    t.prb(i, |y| w.prb(i, |z| k(m[i], (y, z))))
```

Finally, probing `title` and `keyword` expand to array accesses:

```rust
for i in 0..m.len: k(m[i], (t[i], w[i]))
```

At this point, we recover exactly the fused co-iteration shown earlier in this section!
Note how we use the term _expand_: in Prela's implementation, all we had to do is annotate the definition of `.drive` and `.probe` for each combinator with `#[inline]`, and the Rust compiler would automatically fuse the operations into tight loops.

The main motivation for implementing Prela in continuation-passing style is to derive a self-contained, easy-to-understand prototype that can guide the construction of more full-fledged systems.
On one hand, relying on the Rust compiler has unacceptably high latency for queries that must be compiled on-the-fly; on the other hand, data warehouses repeatedly execute the same query over gradually-changing data, which can amortize the compilation cost~#c("DBLP:journals/pvldb/RenenHPVDNLSKK24").
Another advantage of embedding Prela in Rust is to allow arbitrary Rust code to be used as UDFs, as described in @subsec-grouping[section].
We pose a challenge to future research to match Prela's flexibility and run time performance, while incurring much lower compilation overhead.

= Evaluation

In @sec-intro[section] we promised that Prela leads to concise, clear, and efficient queries.
Clarity is subjective, and measuring conciseness in lines of code only tells part of the story.
We therefore invite the reader to compare Prela queries to their SQL counterparts to judge for themselves.#footnote[The source code for Prela and example queries are available at #link("https://prela-lang.org").]
Nevertheless, we share some "quick numbers" here: the Prela code for all 113 queries in the Join Order Benchmark~#c("DBLP:journals/pvldb/LeisGMBK015") (JOB) total \~1000 lines, while the original SQL queries span over 4000 lines; for TPC-H~#c("tpch"), it takes Prela \~300 lines to express all 22 queries, whereas the SQL queries span \~600 lines.
Prela achieves more savings on JOB because the queries there join together many tables, and the implicit/structural joins in Prela avoids spelling out the join conditions, as demonstrated in @fig-job16b[figure].

Does the brevity of Prela queries sacrifice their performance?
To answer this question, we benchmark Prela against DuckDB v1.5.3 on JOB and TPC-H.
Care must be taken to interpret our results.
On one hand, Prela is an early prototype and has not implemented the myriad of optimizations found in mature systems; on the other, Prela is also not weighed down by a large set of features.
In addition, Prela has no query optimizer, but offers more flexibility for performance tuning.
One way to understand our experiments is that they gauge the _performance ceiling_ of a database system built on a foundation of #noun[TAR], assuming such a system implements a competent query optimizer as well as low-level optimizations.

All experiments run on an M2 MacBook Air with 16 GiB of RAM.
Data is pre-loaded into memory.
Since Prela is currently single-threaded, we also run DuckDB on a single thread for fairness.
Each query is executed 5 times and the median run time is reported.
We ran the TPC-H experiments at scale factors 1 through 8 and observed similar results; @fig-perf[figure] reports SF=1 results.

#figure(kind: image,
  grid(
    columns: (1fr, 1fr),
    image("../../rust/bench/job_scatter.pdf", width: 100%),
    image("../../rust/bench/tpch_scatter.pdf", width: 100%),
  ),
  caption: [Performance comparison of Prela and DuckDB:
    each data point represents a query, where the $x$-coordinate is DuckDB's
    run time, and the $y$-coordinate is Prela's run time],
) <fig-perf>

Summaries of the results are collected in @fig-perf[figure].
Prela executes all 113 queries of JOB in 8.68s, a 4.5$times$ speedup over DuckDB's 38.82s.
Prela's advantage is mainly due to data representation: because JOB's primary keys are contiguous integers, the ID to row mapping degenerates to the identity after sorting; instead of materializing the relation, we represent this mapping with the identity function which is optimized away during compilation.
And because each column is stored as a vector indexed by the row number, joins degenerate to fast array accesses.
We tried to port the same optimizations back to DuckDB but were not successful.
The only user-accessible index provided by DuckDB is the hashing-based adaptive radix tree~#c("DBLP:conf/icde/LeisK013"), and its query optimizer ignored every foreign key index.
DuckDB frequently picks bushy join plans, whereas Prela follows left-deep plans shaped by the query structure and uses index nested loop joins to avoid allocation.
For TPC-H, the plot shows two sets of queries: the circular dots represent _idiomatic_ queries which are written in the most natural way without regard to performance.
These queries run in 1.13s, which is 1.11$times$ faster than DuckDB's 1.26s.
It is easy to further improve the performance of the Prela queries.
By directly rewriting the queries, we were able to speed them up by reordering joins, introducing materialization points, and specifying data structures like bit sets for materialized intermediates.
The tuned queries take 0.44s, 2.86$times$ faster than DuckDB.

The experiments show how Prela empowers the programmer to fine tune query performance, and a future query optimizer can achieve results on par or exceeding that of current systems.

= Conclusion

We have presented the design and implementation of Prela, a query language based on #noun[TAR].
At the core of Prela is the binary relation which connects columnar storage with entity/relationship semantics, resulting in succinct and efficient queries.
Our experiments show there is strong potential for a database system based on #noun[TAR] to outperform state-of-the-art OLAP systems.
Future work include parallelization, fast compilation, and query optimization of #noun[TAR], as well as consideration of transactional workloads.

#bibliography("../references.bib", title: [References], style: "association-for-computing-machinery")


#show: appendix

= Appendix <app-a>

== Subtitle

Text.
