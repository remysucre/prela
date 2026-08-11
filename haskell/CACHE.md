# Reading the cache

Queries use cache format v2: one binary file per column, named
`<Entity>_<field>.bin`. The format is shared with the Rust implementation. It
stores zero-based foreign indices, dates pre-parsed as integers, strings as
offsets over a packed byte buffer, and multi-valued columns in CSR form.

`Prela.Cache` treats every file as untrusted input. Loading performs these checks
before constructing a column:

- magic, version kind, and reserved header fields;
- `Word64` counts fitting in the host `Int` range;
- exact payload length with overflow-safe bounds arithmetic;
- string and CSR offsets beginning at zero and remaining monotonic;
- final offsets agreeing with their payload counts;
- foreign indices fitting in `Int`.

The reader uses `ByteString.readFile` and copies numeric payloads into managed
vectors. Strings retain checked slices of a managed byte buffer. This costs a
load-time pass, but it keeps the implementation within ordinary safe Haskell:
there are no mapped-pointer reinterpretations or unchecked indexing operations.

## Foreign keys and holes

The all-ones hole word on disk is decoded immediately to `Nothing`. A loaded
one-valued foreign-key column therefore has the form `SparseCol source Int`.
Generated schema accessors resolve each present integer through the target
entity's `Universe`; missing, out-of-range, and dead target IDs produce no pair.
The storage sentinel never appears as an `Id`.

For a sparse entity, the generated loader derives its live-row universe from the
presence mask of the first declared one-valued reference. It also verifies that
every column belonging to that entity has the same extent.

## Writers

The writer functions in `Prela.Cache` exist for tests and small local datasets.
They emit the same v2 layout and reject counts that exceed its 32-bit offset
fields. Real benchmark caches are still produced by the Rust regeneration tool.
