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