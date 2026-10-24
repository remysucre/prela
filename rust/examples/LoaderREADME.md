
## Running a query with the loader

See README.md for how to download the dataset of (Customer, Order, Region, Nation, etc.)

Then do 
```cargo run --release --features regen --example runner```


## Loading Engine

trialLoader.rs implements a loading strategy which works off of a post-order traversal of the query's AST. To achieve this, objects which represent files and columns are created, which are endowed with information obtained from the data files' schemas, and which can support the same Prela syntax as the in-memory version.


For example, the code below can be interpreted as both a loading plan (triggered by .load()) and an execution plan (triggered by .drive()), depending on whether db is the "in-memory" or the "loading" struct.

```
let q = db.lineitem
        .with(shipdate.during("1994-01-01", "1995-01-01")
         .and(discount.between(0.05, 0.07))
         .and(quantity.lt(24.0)))
        .select(extendedprice.and(discount));
```

### Loading Structures
In fileSchema.rs, there are several objects which build up this file abstraction which enables us to support the same syntax. 

- Catalog: this is a class which will read in a directory and all the files wihin it and will let you build correct `ColumnRef` objects. 

- there is a hard-coded class for each file, whose attributes are ColumnRefs

- The tpchFiles class holds the file objects (created by using Catalog). The `db` is therefore a tpchFile object. We can give aliases to the underlying structure which is why we can have very analogous syntax.

- there is an analogous in-memory version of the files and tpchFiles classes, which can be created from the to_memory method, where it effectively collects all the data from the sinks of all the columns and returns this structure which now has the actual data as its fields. You can also .clear() any data in the in-memory buffers. Note also that if a column is mentioned twice in a query then the rows loaded for each one are unioned.



## Loading different nodes
The `ColumnRef` loading is the leaf of the AST and therefore acts like the base case for loading. For all other nodes, their operation is quite similar in the sense that they just make recursive calls to .load() to load their children and the only thing they need to do is figure out how to combine the results of the loaded calls, based on the behaviour we need. 

### ColumnRef
The most important object which is at the heart of this loading engine is the `ColumnRef` object. It acts as a kind of "placeholders" in place of the actual materialized in-memory relations which Prela previously works with, but can support much of the same syntax so that our query loading and execution plans look identical in syntax. The idea of the `ColumnRef` is that it stores all the information we need in order to actually load in the data to eventually represent this column with real materalized data. This is why `ColumnRef` has attributes like its file path, its key and value columns, as well as a `sink` which is the buffer where the actual column's data will be loaded in to. 

The procedure of .load()ing a `ColumnRef` looks like this:
- as additional input we are given a Bloom filter on the keys of the column, the values of the column (derived from previous semi-join reductions), and a list of predicates we'd like to apply to this column
- since the `ColumnRef` knows its column names and filepath insid ethe schema, it has all the information it needs to then communicate to Parquet to only load the columns which pass these filters
- by reading over the batches returned by Parquet, the `ColumnRef` populates its sink with the actual data. 
- in addition, it returns this data/in-memory relation object which Prela works with (a `MultiRel`) because this .load() method actually builds up and returns the in-memory query plan that corresponds to the query join plan (in case we want to execute the query in the same way we loaded it)

### R.select(T) // Compose
For a Compose, we need to match R.value=T.key. Therefore, a Compose node will first load R, and then load T using the selectivity we learned from loading R (i.e. passing in the values that survived when we loaded R). In addition, the output column of this expression is (R.key, T.value). Therefore, if we call this compose node with key and value filters, then we should apply the key filter to R and the value filter to T. Furthermore, if there are any predicates, then this will be acting on T's values, so we may push down the predicates to T's column. 

### R.with(T) // Restrict
In this case, we require that we only keep the rows of R which satisfy R.key=T.key. We therefore should first load T and then use the selectivity/Bloom filter we learned from doing so and apply it when we load R. Therefore, unlike the Compose clause, we load the right-hand side first, and then the left-hand side.


### R.and(T) // Prod 
It is safe to only load the keys which are in both R and T. Therefore, we can start by loading a and then apply our Bloom filter when we load b, in addition to any selectivity we got given for our keys. We can returned an ANDed bloom filter between a and b because only rows which are present in both will survive later on.


### Filter
In Prela's engine, a predicate is just a lambda function which gets evaluated on a particular row. To achieve predicate pushdown as we load, the `to_predicate` function is used which converts this callable into a different callable function which takes as input a parquet batch and returns a boolean array by evaluating it on every entry in the batch. At the leaf level, Blooms and predicates essentially turn into `ColumnFilter` objects which are then converted by `scan` into objects which Parquet can work with.


## Questions/ Further caveats/ Comments

- The current method of running a query looks like this. It's a bit clunky

    1. read from the directory and gather schema info 
    ```let mut catalog = Catalog::open("../data/tpch/parquet");```
    2. load in the lightweight db with the ColumnRef objects
    ```let db: &'static TpchFiles = Box::leak(Box::new(tpch(&mut catalog)));```
    3. do a lot of renaming of local variables as we like

    4. define your query loading plan and .load() it

    5. convert the loaded database into -in memory:
    
    ```let mem: &'static MemTpch = Box::leak(Box::new(db.to_memory()));```

    6. .drive() your query execution plan 

    7. to rerun a diff query you need to go to memdb.clear() and start from step 4.

- What can we do about .groupby nodes, and how should we convert queries like in common.rs into equivalent join-plans?
- We can make the loading more specialised with load_right and load_left (whether we need the key or the value column), but for now I just have .load()

