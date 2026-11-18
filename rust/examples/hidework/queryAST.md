Previously, we showed that a Prela query syntax can be represented as an explicit join tree which could be traversed with semi-reductions and loaded into memory as we did so less data than without the semireductions. But, building this tree explicitly and converting/compiling a query into this form is expensive. So the goal is to instead traverse the Query AST itself in a way that yields the same result -- we traverse it with semijoins and bloom filters, loading data into memory, and then re-use the query AST/actual syntax itself to execute it in-memory. So in this way, the query AST takes on two meanings: the "join"/loading level meaning and the meaning it has during the actual execution. 

The goal is therefore to define a framework, set of conistent rules that allow us to perform the joins. My initial thoughts so far are

Start with example query:

let q8a = move || {
        db.movie.with(company.with(country.eq("[jp]")
                                   .and(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)"))))
                .select(cast.with(cast_note.eq("(voice: English version)")
                                  .and(role.eq("actress")))
                            .select(person.with(person_name.rx(r"Yo").nrx(r"Yu"))).select(alias)
                        .and(title))
    };

Firstly, observe that the use of the "and" operator inside of a .with() clause is not useful because we aren't materializing the 'anded" columns in our output, just using it as a condition to semijoin reduce. We therefore could say that the only time "and" matters is that its outside of any .with() clause and that's where it acts more like a projection cos we're just picking out more columns to produce in our output. Also a .with() clause that doesnt hop is the same as a selection+predicate, i.e. w ith on the column in the table. so i argue this query should be equivalent

    movie.with(company.with(country.eq("[jp]"))
                 .with(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)")))
    .select(cast.with(cast_note.eq("(voice: English version)"))
                .with(role.eq("actress"))
                .select(person.with(person_name.rx(r"Yo").nrx(r"Yu")))
                .select(alias))
    .and(title)

So with this idea in mind, we should be thinking about queries as some AST consisting mostly of some with and select clauses and potentially an and clause which will query like a column in the table i.e. no hopping needed so it sjust a simple additional projection.

For now consider the following query

movie.with(company.with(country.eq("[jp]"))
                 .with(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)")))
    .select(cast.with(cast_note.eq("(voice: English version)"))
                .with(role.eq("actress"))
                .select(person.with(person_name.rx(r"Yo").nrx(r"Yu")))
                .select(alias))

We basically want to overload this syntax and at the loading level we are working with a different kind of objec that itsnt a vecrel. We define them below BUT the catch is that we can't do them as simple fnctions because the syntax adds additional branches we would like to know about before we execute. For example,

movie.with(company.select(company_ty.eq("B")).select(movie_name.eq("AX")))

If with did the semijoin immediately then we wouldn't really know the select clause exists so instead, I think that the return value of with should itself be the object so you can keep chaining stuff onto it and these chains just add additional info so that it effectively "compiles" it, and there should be a final .load() method which assumes compiled status and goes from there to actually traverse the structure created as we appended and build it.

class ChainObject:
        - we might have a subclass for like an id_column or a derived Chain object, or also a predicate which references a column of a table but a base table should have something that knows holds information about the schema of the file, including knowledge of foreign key relations and knowing which table they could map with. i.e. it can see that if we tell it to join with "Person" it will know that there is person_id foreign key it can use; if no foreign key is being mapped in/explicitly set then will default to using its primary key when asked what to join on for a given table
        - it should definitely know the name of its tables and others, have its file path and stuff
        - children of ChainObjects = List(ChainObject)
        - predicates list of things applied to the table: List(PredExprs)


    def with(self, chain:ChainObject): -> ChainObject
        self.children.add(chain) 
        return self
    
    def select(self, ChainObject) ->ChainObject:
        # there should be some sort of distinction between the special "base table" which are the ids columns and regular column which should be some sort of subclass in which case it should have just the predicate itself as a PredExpr and when added to Predicates -- it should just know what the column name is in that file, which is what it's named anyway 


    # we need to make this some way of formatting interpreting predicates which return chains but added in a nice way to Predicates
    def eq(self, )

    def gr()





    def load(self, acc, key=None):
        - this method should exploit the structure of the chainObject to have everything it needs to be able to evaluate it 
            filters = self.predicates.compile()
            for chain in ChildrenChains:
                probe_key, child_key = self.get_keys(chain.TableName) # the chain object should know sufficienly about its schema to be able to know that if you had to join it with something then which keys should it use
                filters.add(chain.load(key=child_key)) # this returns a bloom filter and when we register a bloom filter we'll register it to probe it with probe_key in the parquet file format
            
            reader = buildReaderFromFilters(filters)
            new_bloom = Bloom()
            for batch in reader:
                acc.put(batch) # some sort of accumulation actualy loading into data
                if key is not None:
                    new_bloom.put(batch.key)
            
            return new_bloom



company.with(company_ty.eq("B")).with(company_name.eq("AX"))

 - at the end of the day, we're working on binary relations so everything we need is already there, it's just translating it to a file at runti,e
    
class Chain:
    - FileObject - this is the file and should be kinda central cos .load() will work off of this -- it should be populated with any predicates that relate to the file and children should be foreign table columns/relations as
    - key
    - value

    
def with(self, chain):
    if chain not part of self's file:
        FileObject.children.add(chain)
        return Chain(self.FileObject, self.key, chain.value) # because (r1,r2).with((t1,t2)) produces (r1,t2)right 


______________

The key idea is that the objects we will be working with can come from the schema, and the syntax of prela in the loading meaning will be a way to populate the additional FileNode info that we need. 

class File:
    - Column object attributes # ideally make some sort of nice syntax so that for a file like movie you can just do movie.movie_id and it will be a Column or some hashmap or soething so that every file objects has access to Column objects, all the columns in its file (this is fine and easily understood from the schema)
    - file path
    - empty list of predicates 
    - empty list of File children (i.e. who it will be connected to)
    - it should have some infomation about which keys are foreign keys for which other tables

    def add_predicate(self,predicateExpr):
        this will literally be the very similar syntax to the previous join_tree -- we have all the info we need because we know which columns to use the file has all the info and the predicateExpr alwready is copied in
        // later on we can just call .compile on the predicate slist like in join_tree 
    
    def add_join_child(self, file:File):
        - we unambiguously know how to because it knows whetehr to use our own primary-key on foreign key by asking file or checking if we have a foreign key and doing so will basically create the Edge object like in join_tree which will know lower level once we traverse exactly what to do-- we're basically building the join tree cos its the info we need
    
    ```
def traverse(self, parent_key=Optional)->Bloom:
    """Loads all needed data into the sink. Returns a Bloom filter
    over parent_key for its parent to use. Returns None for the root or if useless."""

    filters = compile(self.predicates)       

    for edge in self.join_childrenchildren:
        bloom = traverse(edge.fileChild, edge.child_key)
        if bloom is not None:
            filters.append(ColumnFilter.bloom(edge.parent_key, bloom))

    # all selections are now known, so read this node as selectively as possible
    cols = node.out_columns + [parent_key] + [keys of its own edges]
    reader = scan(node.path, filters, cols)  # builds the RowFilter and the output mask

    bloom = Bloom.with_capacity(row_count) if parent_key else None
    for batch in reader:                        # only rows passing all filters arrive here
        if node.materialize:
            # the idea is that it would be super nice to process a batch into the individual columnar VecRel format here already and set it as file.materializedObject like the movie column would be the literal movie_id vecrelation that you can call movie.select(movie_title).drive() i.e. actually call the Prela engine on it. so sort of have that attached so that afterwards you can literally just do it -- i think in rust you will have issues with naming stuff the same bu this is more design, if we've got to this part it means we've succesfully loaded it.
        if bloom:
            bloom.insert_all(batch[parent_key])

    return bloom.finish() if bloom else None
```




At the end of the day Prela's framework is that every input and output is a binary relation which will have a key column and a value columnn like examples of data_ty would be (data_id, data_ty) (cos everything knows its mapping to id). We might need some special distinguishment about the id column but we'll see.

class Column:
    - attribute file: File object: is attached to some sort of reference to a single File object, which can access/mutate File and its the same shared object among all the columns in the file (kind of like FIle is the parent and all COlumns know which file they belong to so we can do comparisons like checking column1.file == column2.file)
    - attribute  primary key: the name of the id_column in the file
    - attribute value: the actual column name of the value. For example,  
    - potentially stores metadata from header file like the min max of the value of each column, number of distinct values when applicable etc.
    - columns to materialize should be a list of cols we know we need to materialize (Prela for now will rly want all of them but later on we don't need them in .with clauses)



movie.with(company)
Suppose the relation is movie column (the id column )
company column (the company id column)

movie.with(runtime.ge(10))
movie.select(runtime.ge(10))
movie.s(company).s(country)

    def with(self, column: Column) ->Column:
        - the goal of this method is to let the File object of self know that it will need to join with the column table 
        if self.File != column.File # as in, this is a foreign table
            self.File.add_join_child( column.File)
            - the schema of this relation still hasn't changed, because we're still reading from movies just effectively semijoin reducing in the process so we should just return ourselves with same schema (but should we copy orr acc return ourselves?)
    
        else: 
        # we don't need to do anything otherwise, because .with clauses on same files are equivalent to predicates which will be handled by itself
        return ourselves
    

    def select(self, column:Column)-> Column:
        - this is where if we're foreign it matters or not 
        - the difference is if we'r ehopping, then we need to make it look like the hopped schema rather than our own, but we still need to let our table know that we should semijoin reduce at loading so 
        if self.File != column.File:
            self.File.add_join_child(column.File)
            return column # because going forward what's next is the hop so we're carrying on as if from the child's schema like in this exaple movie.s(company).s(country)
        else:
            - we need to let the file object know that we need to materialize this column 
            self.File.add_materialize(column)
    
    def ge(self,usize)->Column:
        - some general size becaus ein regular syntax it's just some number 
        self.file.add_predicate(PredicateExpr(the predicate)) // reuse the same PredicateExpr as before; it knows which column its on because it's exactly what the self column is on thats the mask/column we're operating on -- see the join_tree syntax
        return self # the schema hasnt changed
    
    def eq(self, String | Literal)->Column:
        likewise add the predicateexpr to the file object and return our original file and schema column

    def .nrx():
        just return self for now, it's not supported so it's fine if we just leave it cos query execuetion will handleit anways
    
So once we've populated our file node... how do we traverse? Can we always assume it'll be the root corresponding the the start of the thing as well? like if its movie.(....), then if we call movie.File.traverse() it should just go from there?