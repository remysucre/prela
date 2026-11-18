## Join Tree Engine Summary 

The join_tree.rs engine consists of several components which allow it to bridge the gap between Parquet's row-group oriented format and the in-memory column relations which Prela's engine expect. In addition, the Join Tree engine does a post-order traversal of the join-tree of the query, filtering the relations and only loading into memory only columns which could possibly contribute to the final join output, just reducing the size of any intermediate tables formed. 


## Helpful Structures

### Bloom 
This is the structure for the Bloom filter that we use for the semijoin reductions. 

### PredicateExpr
This is effectively an AST structure of a predicate of the form of (Col op Literal), and also conjuncts like OR, AND, NOT, and IN are supported. (rx and nrx is not supported currently). The predicate expression is evaluated by just traversing this AST. The predicate operates on a Parquet `RecordBatch` and looks up/ references columns of the table via helper methods.

### ColumnFilter 
This is a way to wrap a `Bloom` and `PredicateExpr` into the same thing -- it is effectively a type which returns an `eval` -- a function that takes in a RecordBatch and returns a BooleanArray, which is what is needed as input to create a Parquet RowFilter. Inside `scan`, it is easy to use a ColumnFilter object to create an `ArrowPredicateFn` because it knows which columns to read and now has an easily accessible function defining the filter (eval).

### Sink
This is the accumulator which will collect all the rows/relations we load into memory. Later on, this Sink will get processed into the `VecRel` structures that is expected by the Prela engine. See the `testing/helpers`.

## Join Tree Structure 

A join tree consists of `Node`s and `Edge`s, which are defined below. 

```
pub struct Edge {
    pub child: Node,
    pub parent_key: String,
    pub child_key: String,
}

pub struct Node {
    pub path: String, // the Parquet file; its schema comes from the footer
    pub children: Vec<Edge>,
    pub predicates: Vec<PredicateExpr>,
    pub out_columns: Vec<String>, // what we actually need to produce
    pub materialize: bool,        // false if the node is a pure filter
}
```

#### Example Decomposition - query 1b
```
let Movie { title, kind, production_year, episode_nr, keyword, aka,
                company, cast, info, data, complete_cast, link, linked_by, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
```

```
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
```


#### Converting query to join-tree

1. Create a node for each table that appears in the query. 

2. Create an edge for every time there is a `.select()` or `.with()` expression that (implicitly) references a different table. (In Prela you can automatically "map" to a different table by using a foreign key, like in `db.movie.with(data.select(data_ty...))`, there is an implicit pk-fk join between the tables movie and data). The `parent_key` is the name of the column in the parent, and the `child_key` is the name of the column in the child that is being used for the join. It is assumed that the relationship between the keys is some pk-fk relationship or fk-pk.
3. A node contains the predicates applied on its table.
4. Edge keys, predicates and `out_columns` are names as stated in the Parquet file columns on disk.
5. `out_columns` is everything the prela query reads from that table, plus any key prela needs to rebuild the relation, so this includes any pks or fks. (Currently we have to leave all the query execeution to the engine but if we do the join ourselves later on we can do get rid of this).
6. Set `materialize: true` on every node for now, since the prela query still probes each table.

### Traversal Pseudocode

- Caution: there's two things called "parent_key". In the argument to traversal, "parent_key" is the key that this node needs to push values into its own created bloom filter which it will return to its parent. In Edge, the parent_key refers to which column to use as the probe for the Bloom filter that is put into our reader filter as we read the current node. 

```
def traverse(node, parent_key, sink):
    """Loads all needed data into the sink. Returns a Bloom filter
    over parent_key for its parent to use. Returns None for the root or if useless."""

    filters = compile(node.predicates)          

    for edge in node.children:
        bloom = traverse(edge.child, edge.child_key, sink)
        if bloom is not None:
            filters.append(ColumnFilter.bloom(edge.parent_key, bloom))

    # all selections are now known, so read this node as selectively as possible
    cols = node.out_columns + [parent_key] + [keys of its own edges]
    reader = scan(node.path, filters, cols)  # builds the RowFilter and the output mask

    bloom = Bloom.with_capacity(row_count) if parent_key else None
    for batch in reader:                        # only rows passing all filters arrive here
        if node.materialize:
            sink.push(node, batch.project(node.out_columns))
        if bloom:
            bloom.insert_all(batch[parent_key])

    return bloom.finish() if bloom else None
```


### Testing 

`cargo run --release --features regen --example testing ` 

In testing/queries.rs, there are 6 queries defined both as prela queries and manually decomposed into the join-tree equivalent. The main.rs then runs the queries in two different ways: 1. by directly loading the full schema and then running the engine, and 2. via the join-tree engine, where the sink output is processed into the equivalent form, and then with the prela engine. We keep track of how many rows were loaded from each relation. 

Some statistics from the run (the testing code is not yet validated/checked).

### Results

Raw output:

```text
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
```

Summary:

| Query | Baseline rows | Tree rows | Ratio | Output rows | Result |
|---|---:|---:|---:|---:|:---:|
| `q_customer_filter` | 1,500 | 21 | 0.014 | 21 | PASS |
| `q_asia_suppliers` | 130 | 15 | 0.115 | 9 | PASS |
| `q_nation_big_customers` | 1,525 | 14 | 0.009 | 7 | PASS |
| `q_nation_two_children` | 1,625 | 74 | 0.046 | 6 | PASS |
| `q_lineitem_germany` | 76,700 | 657 | 0.009 | 45 | PASS |
| `q_partsupp_branching` | 10,130 | 72 | 0.007 | 1 | PASS |


### Obtaining Testing Data 

Run the following script to get a small duckDB dataset.

```
import duckdb

OUT_DIR = "data/tpch/parquet"  # adjust path as needed relative to where you run this

con = duckdb.connect()
con.execute("INSTALL tpch")
con.execute("LOAD tpch")
con.execute("CALL dbgen(sf=0.01)")  # scale factor — 0.01 is a tiny/fast dataset

# regen.rs wants DOUBLE for money and ISO yyyy-mm-dd VARCHAR for dates;
# duckdb's tpch generator uses DECIMAL(15,2) and native DATE, so cast both.
selects = {
    "region": "SELECT * FROM region",
    "nation": "SELECT * FROM nation",
    "supplier": """
        SELECT s_suppkey, s_name, s_address, s_nationkey, s_phone,
               CAST(s_acctbal AS DOUBLE) AS s_acctbal, s_comment
        FROM supplier
    """,
    "customer": """
        SELECT c_custkey, c_name, c_address, c_nationkey, c_phone,
               CAST(c_acctbal AS DOUBLE) AS c_acctbal, c_mktsegment, c_comment
        FROM customer
    """,
    "part": """
        SELECT p_partkey, p_name, p_mfgr, p_brand, p_type, p_size, p_container,
               CAST(p_retailprice AS DOUBLE) AS p_retailprice, p_comment
        FROM part
    """,
    "partsupp": """
        SELECT CAST(row_number() OVER () AS BIGINT) AS ps_id,
               ps_partkey, ps_suppkey, ps_availqty,
               CAST(ps_supplycost AS DOUBLE) AS ps_supplycost, ps_comment
        FROM partsupp
    """,
    "orders": """
        SELECT o_orderkey, o_custkey, o_orderstatus,
               CAST(o_totalprice AS DOUBLE) AS o_totalprice,
               CAST(o_orderdate AS VARCHAR) AS o_orderdate,
               o_orderpriority, o_clerk, o_shippriority, o_comment
        FROM orders
    """,
    "lineitem": """
        SELECT CAST(row_number() OVER () AS BIGINT) AS l_id,
               l_orderkey, l_partkey, l_suppkey, l_linenumber,
               CAST(l_quantity AS DOUBLE) AS l_quantity,
               CAST(l_extendedprice AS DOUBLE) AS l_extendedprice,
               CAST(l_discount AS DOUBLE) AS l_discount,
               CAST(l_tax AS DOUBLE) AS l_tax,
               l_returnflag, l_linestatus,
               CAST(l_shipdate AS VARCHAR) AS l_shipdate,
               CAST(l_commitdate AS VARCHAR) AS l_commitdate,
               CAST(l_receiptdate AS VARCHAR) AS l_receiptdate,
               l_shipinstruct, l_shipmode, l_comment
        FROM lineitem
    """,
}

for t, query in selects.items():
    path = f"{OUT_DIR}/{t}.parquet"
    con.execute(f"COPY ({query}) TO '{path}' (FORMAT PARQUET)")
    n = con.execute(f"SELECT count(*) FROM ({query})").fetchone()[0]
    print(f"{t}: {n} rows -> {path}")

print("done.")
```