## Testing

### Goal

Right now `baseline_query.rs` and `tree_query.rs` each hardcode one query, so trying a new query means rewriting a lot of boilerplate. The goal is a small `/testing` directory where a new query is a short definition, and a single runner executes everything and reports whether the join-tree engine matches the baseline and how much less data it loaded.

For every query the test must always end the same way: two `.txt` files with the output rows printed (sorted, same line format), and a diff of the two. An empty diff means the engine is correct.

### Layout

Since the current files are cargo examples, the testing code is an example directory:

```
examples/testing/
    main.rs      // the runner (section 3); declares `mod helpers; mod queries;`
    helpers.rs   // Sink -> prela relations, shared by every query
    queries.rs   // one baseline + one join-tree definition per query
```

Run with: `cargo run --release --features regen --example testing` (optionally followed by `-- <query_name>`).

`tree_query.rs` is the prototype for all of this. Anything in it that is not specific to its one query moves into `helpers.rs`.

### 1. helpers.rs: preprocessing the Sink

General functions that turn the `Sink` contents (`Vec<RecordBatch>` per file) into the in-memory relations the prela query needs, so queries do not repeat this code. These are the pieces already in `tree_query.rs`:

- `ints(batches, col)` / `floats(batches, col)`: pull one column out of all of a table's batches as a plain `Vec` (integers widened to `i64`)
- `domain(ids)`: the relation size, max loaded id + 1
- `key::<E>(n, ids)`: a `SparseUniverse` over only the ids that were actually loaded. This is needed because `VecRel` is dense, so every slot would otherwise count as a member
- `id_col(n, ids, vals)`: a foreign-key column, with `NO_ID` in the slots of rows that were not loaded
- `names(&[...])`: shorthand for building the `Vec<String>` of column names in a `Node`
- `row_count(path)`: the row count from a file's Parquet footer, used for the statistics (the scan already reads this number, so share the code)

Add a helper for the id shift that the cache loader applies (e.g. custkey is 1-based, nationkey and regionkey are 0-based), so each query states it once instead of hand-writing `k - 1`.

### 2. queries.rs: the test queries

Each query is written twice, on the data in `data/tpch/parquet`:

- **baseline**: the prela query over the whole cached schema (`tpch_schema::load`), as in `baseline_query.rs`
- **tree**: the same query as a hand-built join tree (`Node` / `Edge` / `PredicateExpr`), then `traverse`, then helpers to build the relations from the Sink, then the same prela expression run over those relations

**Rules for the queries**

- Allowed: predicates, `.select()`, `.and()` and `.with()` (inner joins and semijoins).
- Not allowed: outer joins and anti-joins, or anything that has to keep rows that have no match. The engine relies on semijoin filtering never losing information the query needs.
- All conditions at a node are ANDed together, and that includes its child edges, since each child's Bloom filter is just one more filter on the node's scan. So an `Or` across two different `.with()` branches cannot be expressed as a tree. An `Or` inside one node's own `PredicateExpr` is fine.
- The tree version uses the same prela expression and the same thresholds as the baseline, over the reduced relations. **Every exact `.with()` stays in it**, because the Bloom filters only approximate the filtering (false positives) and the prela query is what makes the result exact. Use one constant per threshold, shared by both versions.

**How to break a prela query into its join-tree form**

Rules:
1. One node for each table in the query.
2. One edge for every time the query "hops" between two tables based on a foreign key. The parent is the table being filtered, and the child is the table inside its `.with(...)`. The edge's `parent_key` and `child_key` are the two columns the join equates (some pk-fk relation), whichever table holds the foreign key. The child inserts its `child_key` values into a Bloom filter, and the parent probes it with its `parent_key`.
   - `movie.with(data.select(data_ty).eq("bottom 10 rank"))` has an implied hop where `movie.id = data.movie_id`. Here the child holds the FK, so `parent_key = movie.id` and `child_key = data.movie_id`.
   - `order.with(o_customer...)` is the reverse. The parent holds the FK, so `parent_key = o_custkey` and `child_key = c_custkey`.
3. Each predicate goes on the node whose column it tests. In the example above, data would have the predicate data_ty == "bottom 10 rank"
4. Edge keys, predicates and `out_columns` are names as stated in the Parquet file columns on disk.
5. `out_columns` is everything the prela query reads from that table, plus any key prela needs to rebuild the relation, so this includes any pks or fks.
6. Set `materialize: true` on every node for now, since the prela query still probes each table.

let Movie { title, kind, production_year, episode_nr, keyword, aka,
                company, cast, info, data, complete_cast, link, linked_by, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;

#### Example Query decomposition

let q1b = move || {
        db.movie.with(data.select(data_ty).eq("bottom 10 rank"))
                .select(company.with(company_ty.eq("production companies"))
                               .select(company_note)
                        .and(title)
                        .and(production_year.between(2005, 2010)))}

Node(Movie):
    path: movie.parquet
    children: [Edge(Data), Edge(Company)]
    predicates: [production_year >= 2005, production_year <= 2010]
    out_columns: [<movie id>, title, production_year]
    materialize: true

Edge(Data):
    child: Node(Data)
    parent_key: <movie id>
    child_key: <data movie_id>

Node(Data):
    path: data.parquet
    children: []
    predicates: [<data ty> == "bottom 10 rank"]
    out_columns: [<data id>, <data movie_id>, <data ty>]
    materialize: true

Edge(Company):
    child: Node(Company)
    parent_key: <movie id>
    child_key: <company movie_id>

Node(Company):
    path: company.parquet
    children: []
    predicates: [<company ty> == "production companies"]
    out_columns: [<company id>, <company movie_id>, <company ty>, <company note>]
    materialize: true

The other half of the tree version, after the tree is built:

```
sink = Sink::default()
traverse(order_node, None, &mut sink)

// Sink -> relations, using the helpers (apply the id shift, build key / id_col / VecRel per table)
order, customer, nation, region relations = build from sink

// the same prela expression as the baseline, over these relations, including every exact .with()
q = order_key.with(totalprice.ge(300000.0)).with(...).select(...)
rows = drive q, format each row as a line, sort
```

Note: the chained `.select(...).select(...).eq(...)` used as a predicate above is assumed to compile in prela. Check it first. If it does not, collect the matching nations into a `MatSet` first, the way `baseline_query.rs` does.

**The query interface**

Each query exposes the same shape so the runner can treat all of them identically. Both functions return rows already formatted as sorted `Vec<String>` (one line per row), so the runner never needs to know the row types:

```
Query {
    name: &str,
    tables: Vec<&str>,                    // files the query touches, for the baseline statistic
    baseline: fn() -> Vec<String>,        // formatted rows, sorted
    tree: fn() -> (Vec<String>, usize),   // formatted rows, sorted + rows pushed into the sink
}
```

Queries are defined in `queries.rs` and listed in one place (e.g. `fn all_queries() -> Vec<Query>`), which the runner imports.

### 3. runner (main.rs)

For each query in the list (or just the one named on the command line):

1. Run `baseline()` and `tree()`.
2. Write `testing/out/out_baseline_<name>.txt` and `testing/out/out_tree_<name>.txt`, one row per line. The files are always written, so you can run `diff` yourself on a failure.
3. Compare the two outputs in the program (they are already sorted, so equal vectors means equal files).
4. Print a line with the query name, PASS or FAIL, and the statistics below.
5. If both outputs are empty, print a warning. Two empty files match trivially, so the test proves nothing. Pick thresholds that give non-empty results.

At the end, print a summary table and exit non-zero if any query failed.

### Statistics

Initially we only track how many rows are loaded into memory:

- **baseline rows loaded**: the total rows of every table in the query's `tables` list. The baseline loads each whole table, so this is the sum of `row_count(file)` over those files
- **tree rows loaded**: the total rows pushed into the Sink over the traversal (rows after prefiltering). This only counts nodes with `materialize: true`, which is every node for now
- the ratio between them

Example report (for the Order/Customer/Nation/Region query, the baseline is 15000 + 1500 + 25 + 5 = 16530 rows):

```
query            baseline rows   tree rows   ratio   result
q_asia_orders    16530           ...         ...     PASS
```

This is the number that tells us whether prefiltering pays off. Later additions could be bytes read and rows decoded, but they are out of scope now.

### Queries to implement

## Queries to implement

Notes for the implementer:
- Every predicate below is one of the forms `PredicateExpr` handles: `Cmp` (`==, !=, <, <=, >, >=`), `And`, `Or`, `Not`, `In`. A range is written as an `And` of two `Cmp`s.
- Prela field names (e.g. `s_nation`, `l_order`) are guesses. Check them against `src/tpch_schema.rs`. Prela method names for `Or`, `In` and `le` also need checking.
- Tree column names are the Parquet column names, per the decomposition rules. `out_columns` includes every key and every column used by an exact filter.
- Literal types must match the column: `acctbal`, `totalprice`, `l_quantity`, `ps_supplycost` are `F64`; `p_size` is `I32`; strings are `Str`.
- Thresholds may return empty results on the small tables. Run the baseline first and loosen any threshold that returns no rows.

---

### 1. `q_customer_filter` (single node, no edges)

Building-segment customers with a very high or very low balance.
`tables: customer`

```rust
db.customer
    .with(mktsegment.eq("BUILDING"))
    .with(acctbal.ge(9500.0).or(acctbal.le(-900.0)))
    .select(c_name.and(acctbal))
```

```
Node(Customer):
    path: customer.parquet
    children: []
    predicates: [c_mktsegment == "BUILDING", Or(c_acctbal >= 9500.0, c_acctbal <= -900.0)]
    out_columns: [c_custkey, c_name, c_acctbal, c_mktsegment]
    materialize: true
```

Tests: `And` / `Or` predicates, string and float literals, a node with no children (so no Bloom is built).

---

### 2. `q_asia_suppliers` (chain of 3, parent holds the FK)

Suppliers with a high balance in an Asian nation.
`tables: supplier, nation, region`

```rust
db.supplier
    .with(s_acctbal.ge(5000.0))
    .with(s_nation.select(n_region).select(r_name).eq("ASIA"))
    .select(s_name.and(s_acctbal))
```

```
Node(Supplier):
    path: supplier.parquet
    children: [Edge(Nation)]
    predicates: [s_acctbal >= 5000.0]
    out_columns: [s_suppkey, s_name, s_acctbal, s_nationkey]
    materialize: true

Edge(Nation):  parent_key: s_nationkey   child_key: n_nationkey

Node(Nation):
    path: nation.parquet
    children: [Edge(Region)]
    predicates: []
    out_columns: [n_nationkey, n_regionkey]
    materialize: true

Edge(Region):  parent_key: n_regionkey   child_key: r_regionkey

Node(Region):
    path: region.parquet
    children: []
    predicates: [r_name == "ASIA"]
    out_columns: [r_regionkey, r_name]
    materialize: true
```

Tests: Bloom filters chained upward over three levels, `i32` keys.

---

### 3. `q_nation_big_customers` (child holds the FK, regression)

Nations that have a customer with a very high balance. This is the existing `baseline_query.rs` / `tree_query.rs` query.
`tables: nation, customer`

```rust
let nations: MatSet<_> = db.customer
    .with(acctbal.ge(9900.0))
    .select(c_nation)
    .collect();
db.nation.with(&nations).select(n_region)
```

```
Node(Nation):
    path: nation.parquet
    children: [Edge(Customer)]
    predicates: []
    out_columns: [n_nationkey, n_regionkey]
    materialize: true

Edge(Customer):  parent_key: n_nationkey   child_key: c_nationkey

Node(Customer):
    path: customer.parquet
    children: []
    predicates: [c_acctbal >= 9900.0]
    out_columns: [c_custkey, c_acctbal, c_nationkey]
    materialize: true
```

Tests: the opposite FK direction from query 2. Known-good, so a failure here points at the harness rather than the engine.

---

### 4. `q_nation_two_children` (two sibling children)

Nations that have both a rich supplier and a rich customer.
`tables: nation, supplier, customer`

```rust
let sup_nations: MatSet<_> = db.supplier
    .with(s_acctbal.ge(9000.0))
    .select(s_nation)
    .collect();
let cust_nations: MatSet<_> = db.customer
    .with(acctbal.ge(9500.0))
    .select(c_nation)
    .collect();
db.nation.with(&sup_nations).with(&cust_nations).select(n_region.and(n_name))
```

```
Node(Nation):
    path: nation.parquet
    children: [Edge(Supplier), Edge(Customer)]
    predicates: []
    out_columns: [n_nationkey, n_regionkey, n_name]
    materialize: true

Edge(Supplier):  parent_key: n_nationkey   child_key: s_nationkey
Edge(Customer):  parent_key: n_nationkey   child_key: c_nationkey

Node(Supplier):
    path: supplier.parquet
    children: []
    predicates: [s_acctbal >= 9000.0]
    out_columns: [s_suppkey, s_acctbal, s_nationkey]
    materialize: true

Node(Customer):
    path: customer.parquet
    children: []
    predicates: [c_acctbal >= 9500.0]
    out_columns: [c_custkey, c_acctbal, c_nationkey]
    materialize: true
```

Tests: two Bloom filters on the same parent column. A nation must pass both, so make sure the result is not empty.

---

### 5. `q_lineitem_germany` (largest table, sparse keys, 4 levels)

Large air shipments whose order was placed by a customer in Germany.
`tables: lineitem, orders, customer, nation`

```rust
db.lineitem
    .with(l_quantity.ge(45.0))
    .with(l_shipmode.eq("AIR"))
    .with(l_order.select(o_customer).select(c_nation).select(n_name).eq("GERMANY"))
    .select(l_extendedprice.and(l_discount))
```

```
Node(Lineitem):
    path: lineitem.parquet
    children: [Edge(Orders)]
    predicates: [l_quantity >= 45.0, l_shipmode == "AIR"]
    out_columns: [l_id, l_orderkey, l_quantity, l_shipmode, l_extendedprice, l_discount]
    materialize: true

Edge(Orders):  parent_key: l_orderkey   child_key: o_orderkey

Node(Orders):
    path: orders.parquet
    children: [Edge(Customer)]
    predicates: []
    out_columns: [o_orderkey, o_custkey]
    materialize: true

Edge(Customer):  parent_key: o_custkey   child_key: c_custkey

Node(Customer):
    path: customer.parquet
    children: [Edge(Nation)]
    predicates: []
    out_columns: [c_custkey, c_nationkey]
    materialize: true

Edge(Nation):  parent_key: c_nationkey   child_key: n_nationkey

Node(Nation):
    path: nation.parquet
    children: []
    predicates: [n_name == "GERMANY"]
    out_columns: [n_nationkey, n_name]
    materialize: true
```

Tests: the biggest table (60,175 rows), the deepest chain, the sparse orderkey (keys go to 60,000 with only 15,000 rows), mixed `i32` / `i64` key widths, and two predicates on different columns of one node.

---

### 6. `q_partsupp_branching` (branching tree, In, range via And)

Cheap partsupp rows for certain brands and sizes, supplied by European suppliers.
`tables: partsupp, part, supplier, nation, region`

```rust
db.partsupp
    .with(ps_supplycost.le(50.0))
    .with(ps_part.with(p_brand.in_(["Brand#12", "Brand#23"]))
                 .with(p_size.ge(5))
                 .with(p_size.le(20)))
    .with(ps_supplier.select(s_nation).select(n_region).select(r_name).eq("EUROPE"))
    .select(ps_availqty.and(ps_supplycost))
```

```
Node(PartSupp):
    path: partsupp.parquet
    children: [Edge(Part), Edge(Supplier)]
    predicates: [ps_supplycost <= 50.0]
    out_columns: [ps_id, ps_partkey, ps_suppkey, ps_availqty, ps_supplycost]
    materialize: true

Edge(Part):      parent_key: ps_partkey   child_key: p_partkey
Edge(Supplier):  parent_key: ps_suppkey   child_key: s_suppkey

Node(Part):
    path: part.parquet
    children: []
    predicates: [In(p_brand, ["Brand#12", "Brand#23"]), p_size >= 5, p_size <= 20]
    out_columns: [p_partkey, p_brand, p_size]
    materialize: true

Node(Supplier):
    path: supplier.parquet
    children: [Edge(Nation)]
    predicates: []
    out_columns: [s_suppkey, s_nationkey]
    materialize: true

Edge(Nation):  parent_key: s_nationkey   child_key: n_nationkey

Node(Nation):
    path: nation.parquet
    children: [Edge(Region)]
    predicates: []
    out_columns: [n_nationkey, n_regionkey]
    materialize: true

Edge(Region):  parent_key: n_regionkey   child_key: r_regionkey

Node(Region):
    path: region.parquet
    children: []
    predicates: [r_name == "EUROPE"]
    out_columns: [r_regionkey, r_name]
    materialize: true
```

Tests: a branching tree (one sibling is a leaf, the other a chain of 3), the `In` predicate, a range on an `I32` column, and a root with several selective filters.

---

### Coverage

| Query | Hops | FK held by | Special |
|---|---|---|---|
| 1 `q_customer_filter` | 0 | n/a | And/Or, no Bloom |
| 2 `q_asia_suppliers` | 2 (chain) | parent | chained Blooms |
| 3 `q_nation_big_customers` | 1 | child | regression |
| 4 `q_nation_two_children` | 2 (siblings) | child | two Blooms on one column |
| 5 `q_lineitem_germany` | 3 (chain) | parent | sparse keys, biggest table |
| 6 `q_partsupp_branching` | 4 (branching) | parent | In, range, branching |



### cargo run --release --features regen --example testin

q_customer_filter: PASS  (output 21 vs 21 rows; loaded 21 of 1500 rows)
q_asia_suppliers: PASS  (output 9 vs 9 rows; loaded 15 of 130 rows)
q_nation_big_customers: PASS  (output 7 vs 7 rows; loaded 14 of 1525 rows)
q_nation_two_children: PASS  (output 6 vs 6 rows; loaded 74 of 1625 rows)
q_lineitem_germany: PASS  (output 45 vs 45 rows; loaded 657 of 76700 rows)
q_partsupp_branching: PASS  (output 1 vs 1 rows; loaded 72 of 10130 rows)

query                    baseline rows  tree rows   ratio  out rows  result
q_customer_filter                 1500         21   0.014        21  PASS
q_asia_suppliers                   130         15   0.115         9  PASS
q_nation_big_customers            1525         14   0.009         7  PASS
q_nation_two_children             1625         74   0.046         6  PASS
q_lineitem_germany               76700        657   0.009        45  PASS
q_partsupp_branching             10130         72   0.007         1  PASS