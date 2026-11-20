This plan will make the loader (hopefully) much more efficient, because we will be constructing Bloom filetrs on physicsal row IDs rather than the table IDs, allowing us to avoid reading two columns for each ColumnRef (the table IDs + the value column) and instead just the value column (applying value predicates as well as row selections based on Bloom filters). To enact this change, there are several structural differences we need to create:

1. Scan
- in addition to taking a vector of filters (which are predicates on the value), it will also take an explicit RowSelection parquet object which will specify to the Parquet Batch builder the rows it needs to load. In addition, the returned reader will be constructed with 'virtual columns' enabled so that our ColumnRef loader will be able to insert those physical rows that passed the test into our KeyFilter. Note that the population/insertion of keys into the KeyFilter and the high level passing around of it can stay -- the differences is what we populate it with and how we iteract with it at loading.

2. Modifying the ColRef Struct:
    - We no longer need a self.key_col, just the value_col it corresponds to, because we're never reading table Ids with values anymore


3. A general load for a general ColumnRef
We know that at the end of the load, we are expected to return a MultiRel object. Previously, that MultiRel was constructed from the keys column because those keys are also 1..n. However, now they will always be constructed oevr the 1..n_rows in the physical table using the virtual columns of the loader. The procedure for loading is then:
- Receive the value and keyfilters 
    - Construct the ColumnFilters from the valuefilters; construct the RowSelection from keyfilters
    - receive batches from scan 
    - populate the relation into a MultiRel (virtual row number, value)
    - return a Bloom filter over the values and the keys that survived 
    - return it 

4. Introducing PrimaryColRef 
ideally, this should inherit everything from ColumnRef, except its Load implementation is different: For a PrimaryCOlRef, we will be returning a HashMap relation over (table_ids, physical_rows); this should work because physical keys should still be hashable types. The scan procedure is then:
    - (same steps up to the point of receiving batch from scan -- we can also get rid of key_column in the COlumnRef Struct )
    - Construct a HashMap(TableId : PhysicalRowId); recall that this wil make all the matches drive ecause we always map value to key 
    - an important fact to maintain is that key Bloom filters will always pass info about the surviving physical row IDs so this is how the keyFIlter is populated for the PrimaryColRef as well.

5. Introducing ForeignColRef
The struct for this should extend ColRef with an extra field: it should know the foreign path to the file it links to (as well as its own), and the name of the ID columns in that table, like knowing to compare itself (say, the id column in movies) to the movie_id column in the Cast table. Otherwise, all other implementations should be the same for this object as ColumnRef, except for its Load: at the end of the day, the foreign key column should facilitate a join across tables, which is like movie.select(cast).select(person_name) does (movie_id, movie_id) . select (movie_id, cast_id), . select(cast_id, person_name). In the sense, we should be able to continue the chain. Hence, we need something that will be the (physical_ids of table 1/original table, corresponding physical_ids of table 2 foreign table). The loading procedure is then:
    - (same up to receiving a batch with the row selections and filters applied)
    - as we read in this data, we record the physical row IDs that survived (this is in table 1)
    - Next, we make ANOTHER read of the table ID column in the table 2 file by applying a **predicate** over the column id values (i.e. we know which foreign keys correspond to the surviving row so we only want to load those; there's no RowSelection here, we dont have any other info to go off of )
    - so as we receive our foreign_batch and read it, we will make note of the *foreign* physical row numbers that Parquet is giving to us. Then we will construct a MultiRel on (Table1 physical row ids, Table 2 Physical row Ids) which is what we return 
    - the Bloom filter constructed should just be over these physical Ids as keys for table 1 and values for table 2

6. Changes to JobFileSchema 
    - we will just have to be more careful in constructing our schema to use the Foreign and Primay Col structs for the deisgnated foreign and primary keys rather than just making a COlumnref for all of them. There is already some distinction in specifying foreign/primary key columns so this just needs to be modified.

7. KeyLoad implementations for primary and foreign keys can be the same as the loads, except that it should only return a bloom filter on its survived physical IDs rather than its values



## Explanations of Formalised Plan

#### 1. Adding funcionality to KeyFilter to produce a RowSelection/ COlumnFIlter using the physical row IDs
- Recall how we have a way of converting a KeyFilter (independent of rows) into a COlumnFIlter object which is attached to the idea of rows and how we pushed them into our vector of filters which would eventuallyget proceessed by scan. Well since our not all our filters will be bloom filters on VALUES of columns but rather as physical row filtering, we can extend this same idea by introducing the to_selection method which basically is given an input of all the physical rows we want to keep and translates that into a RowSelection object which Parquet knows how to read (it needs it as the contiguous runs of skip or keep )
- So the above works in the situation when we have like movie.with(something).select(title) where we've reduced movie so now only want the title IDs which are surviving that. So we just tell Parquet to keep thos ephysical rows. 
- But the thing about the in-memory JOB schema is that not all the tables have 1-1 correspondence with files, and there's actually more separate files than the single table. For example, a Movie table comes from being made up of like title.parquet and cast_info.parquet. So that when we do movie.cast.select(c.person).select(p.name), then actually movie.cast needs to be (movie_id, file_id) so that cast.person is (file_id, person_id) to get a (movie_id, person_id) kind of relation. So to reflect this fact that not all the ID columns are the *file's* own ID column (because of this not file-to-table correspondence), we indicate this with a Via(file) type, which tells us the mapping of the columns we actually need to use as the IDs. So in the case that when we're trying to load a Via(foreign_file) column, given a KeyFilter on those native rowIds, then we proceed by: creating a ColumnFilter on *our* column in this file, but we test membership within the KeyFIlter by using the foreign_file's index to map our table IDs into the rows we need to keep. This is what push_via does.

### 2. Building an Index storing table_id-> physical row_id
This will be something that we effectively cache after loading it once, using the OnceLock, and it will belong to a file, using the path attribute.
To get the row mapping, we have the rows method which eitehr just fetches it from cache or loads it using the scan_rows method. The row_in function just lets u get the membership. This is very useful because now we don't need to repeatdely recheck the physical to table id mappings everytime, it's just cached.

### 3. Distinguishing between regular ColumnRefs, Primary Keys Columns and ForeignKey COlumns
Recall that at the end of the day, a ColumnRef needs to produce a MultiRel(D, R). Hence, it needs to know where to retrieve its D and R from. 
In some cases, like in the most vanilla cases, D is just the physicl row_id and R is the value of the column, e.g. movie.select(title), where here if we wanted to load title, we just load the single title column with row selections on physical rows. 
On the other hand, something like movie is a Primary Key and this requires us the use of our index, it's a mapping table_id->physical_id, so whenever loading PrimaryKeys it's actually using our index. 
For loading ForeignKeys, when D or R holds foriegn keys, rather than reloading the ID column we should jus use the index but of the foreign file, which is why we have Via(file).
Here's some examples:
title: D = Row, R = Value
movie: D= Row, R=Row
c.person: D = Row, R=Via(name)
m.cast: D= Via(title), R= Row
-- these are basically enums and they tell us how to handle the loading of data for a given ColRef. More details/clarity will be seen in context.

### 4. How filter construction works 
We can introduce a helper function on a ColumnRef to help us process in all that we need o get our row selections + filters given the KeyFilters on the keys and values, and predicates. The approach is:
- treat each side of the column separately and act as it instructs. 
- If it's a Row: then this does not require us to read in any id columns but instead just the value column, but with the appropriate row selections, specified by our KeyFilter
- If it's a Via(file), then we need to convert it as a predicate over what we're reading to test membership with our filters -- this could happen in our val_filter which came from filtering b from a 

- there's another helper/method that helps us construct the KeyFilter, since its no longer our table ID column -- it's either the physical row IDs of our actual file, or if it's a Via(f_file) then it should be done on the single source/origin aka that f_file's physical rows as we pass it on. So in the Row case, it just retrieves the batches returned physical row IDs. In the case of the Via, then it looks at the columns we loaded as the foreign keys and tells us the corresponding physical row positions in that native file. You wouldn't construct a KeyFiler from a plain value/ they dont map to anything.


### 5. Loading for a Leaf
It's now very unified because our helepr functions handle each case/"side" separately.The load procedure is:
- given our keyfilters, valfilters, and preds, process them using our helepr functions into things that scan needs: the row selector and columnfilter vectors
- specify the columns we need to read (if its just physical row ids we dont need to read it)
- actually scan and extract the key and value filters, convert them to Blooms. The MultiRel is constructed now with keys being 0..n on the physical rows