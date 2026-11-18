- fix predicate pushdown 
- change to a mutating design 
- remove weird restrictions 
- add a right and a left load (later)








movie.select(company.select(country).eq("USA))

movie.with(company.with(country.eq("USA")))
.select(title.and(cast.select(person).select(name)))



movie.select(company).select(country)       (company_id, country) 

movie.select(movie_company).select(company_id_to_company_row).select(company_country).select(country_id_to_country_row)


(movie_id, company_id)


                        Compose
                 Restrict           title.and(cast.select(person).select(name))
                                            

            movie       company.with(country.eq("USA"))  


cast.select(person).select(name)

                        compose
                  compose              name
            cast            person

movie.with(title.eq("H"))
        Eq
    

struct Col:
    - source: file and column name it comes from
    - sink: buffer for the col data


Load(Restrict, BloomFilter) ->BloomFilter:
    lhs, rhs = Restrict.left, Restrict.right
    rbf = Load(rhs, True)
    lbf = Load(lhs, rbf)

Load(Select, BloomFilter) ->BloomFilter:
    lhs, rhs = Select.left, Select.right
    lbf = Load(lhs, True)
    rbf = Load(rhs, lbf)

Load(Col, BloomFilter) -> BloomFilter:
    - loads in the column with BloomFIlter predicates added, populates sink 

Load(Eq, BloomFilter) ->BloomFilter:
        literal = Eq.literal
        col = Eq.col

        newbloom = create_bloom(eq literal)
        bloom += new-bloom

        load(column, bloom)














The goal is to create a File object with Columns from just reading the schema and traversing the query syntax should be sufficient to populate/construct the join tree. The slight issue is mappings/keys between two tables. Ideally we would like to join based on the values but this won't always work so for now we need to give the File object an explicit mapping, which doesn't explicitly come from reading the schema itself.

```
class Column:
    - headFile   # what we need to reduce with anything we are inside of an argument
    - tailFile   # what we need to reduce with anything inside our own argument
    - key        # the id column, in headFile
    - value      # usually just the column's name, in tailFile, e.g. title has key=id (movie_id) and value=title
```

    def touch(self):
        # just a way to let the File know which columns it needs to load
        self.headFile.add_materialize(self.key)
        self.tailFile.add_materialize(self.value)

    def select(self, column):
        self.touch()
        column.touch()
        # the column is inside our argument, so we should check its headFile
        if self.tailFile != column.headFile:
            self.tailFile.add_join_child(column.headFile)   # the files know which foreign keys to use
        # in both cases: keep our head, carry on from the argument's tail
        # (for movie.select(title) this is the same schema as title itself)
        return Column(headFile=self.headFile, tailFile=column.tailFile,
                      key=self.key, value=column.value)

    def with(self, column):
        self.touch()
        column.touch()
        if self.tailFile != column.headFile:
            self.tailFile.add_join_child(column.headFile)
        return self

    def eq(self, literal):
        self.touch()
        self.tailFile.add_predicate(PredicateExpr.Cmp(col=self.value, op=Eq, literal=literal))
        return self

    # similar for other predicates

```
class File:
    - filepath
    - filename
    - column objects
    - children[]
    - predicates[]
    - file_mappings{}     # filename -> (our key, their key), so the File owns the join keys
    - materialize{}       # a set, since touch adds the same columns many times
```

    def add_join_child(self, file):
        parent_key, child_key = self.file_mappings[file.filename]   # no entry: no foreign key between us, unsupported
        edge = Edge(child=file, parent_key=parent_key, child_key=child_key)
        if edge not in self.children:
            self.children.push(edge)

    def add_materialize(self, column_name):
        self.materialize.add(column_name)

    def add_predicate(self, predicate: PredicateExpr):
        self.predicates.push(predicate)

    def traverse(self, parent_key, sink) -> Bloom:
        filters = compile(self.predicates)
        for edge in self.children:
            child_bloom = edge.child.traverse(edge.child_key, sink)
            if child_bloom is not None:
                filters.append(ColumnFilter.bloom(edge.parent_key, child_bloom))

        # all selections are now known, so read this file as selectively as possible
        cols_to_read = self.materialize + [e.parent_key for e in self.children]
        if parent_key: cols_to_read += [parent_key]
        reader, row_count = scan(self.filepath, filters, cols_to_read)

        own_bloom = Bloom.with_capacity(row_count) if parent_key and filters else None
        for batch in reader:                     # only rows passing all filters arrive here
            sink.push(self, project(batch, self.materialize))
            if own_bloom:
                own_bloom.insert_all(batch[parent_key])

        return own_bloom.finish() if own_bloom else None