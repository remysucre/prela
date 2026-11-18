How to use the loading engine -- understanding the IO objects. 

Catalog -- this is the object that interacts with the headers of parquet files and creates a column reference object for it (validating it against the actual schema of the file)

Then we define structs corresponding to individual files with their fields as just the actual columns (they're columnref objects)

There's also an analogous TPCH schema which keeps track of the files themselves -- the file objects. the tpch function just blindly stupidly writes and generates all the columnref objects -- this is all just hardcoded. 

There is an analogous/symmetric structs corresponding to the in-memory file objects which we'll set after running load and their types are the usual MultiRel stuff -- whatever the prela engine typically can handle. 

The to_memory method of the TPchfiles object just stupidly hardcodes all the loaded data (note that ids are concated across all the key columns of the loaded ColumnRefs) - the fileSchema file is therefor ejust a lot of hard-coded structure to help us work easily.

So the outline would be 



1. read from the directory and gather schema info 
let mut catalog = Catalog::open("../data/tpch/parquet");
2. load in the lightweight db with the ColumnRef objects
let db: &'static TpchFiles = Box::leak(Box::new(tpch(&mut catalog)));
3. do a lot of renaming of local variables as we like

4. define your query loading plan and .load() it

5. convert the loaded database into -in memory
let mem: &'static MemTpch = Box::leak(Box::new(db.to_memory()));
6. .drive() your query execution plan 

7. to rerun a diff query you need to go to memdb.clear() and start from step 4.






