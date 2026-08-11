# Reading the cache

Queries use cache format v2: one binary file per column, named
`<Entity>_<field>.bin`. The format is shared with the Rust implementation. It
stores zero-based foreign indices, dates pre-parsed as integers, strings as
offsets over a packed byte buffer, and multi-valued columns in CSR form.

Generated schemas provide two loaders with the same result type. For a schema
named `TPCHS`, they are:

```haskell
schema <- loadTPCHS cacheDir          -- trusted mmap; the default
schema <- loadTPCHSChecked cacheDir   -- fully checked input
schema <- loadTPCHSFast cacheDir      -- compatibility alias for loadTPCHS
```

The choice affects loading only; both values are used by exactly the same query
code.

## Checked loader

The `*Checked` readers in `Prela.Cache` treat every file as untrusted input.
Loading performs these checks before constructing a column:

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

## Trusted fast loader

The unsuffixed readers in `Prela.Cache` memory-map each file and make zero-copy
storable-vector views of its numeric payloads and offsets. Unlike the Rust
implementation, the mapping is retained by ordinary `ForeignPtr` ownership and
is unmapped when the schema becomes unreachable; it is not leaked for the
lifetime of the process.

The fast loader retains constant-time checks for the v2 magic, column kind,
reserved header, host representation, dimensions, and exact payload length. It
skips the linear passes that validate every string/CSR offset and every foreign
identifier. Use it only for immutable cache files produced by Prela's `regen`
tool. A mapped file must not be replaced or modified while its schema is alive.

The TPC-H runner selects the fast loader by default. Set
`PRELA_LOADER=checked` to validate untrusted input instead.

## Foreign keys and holes

The checked loader decodes the all-ones hole word immediately to `Nothing`. The
trusted loader retains the word-backed array, but its `SparseCol` view translates
the sentinel to `Nothing` before it can escape. Generated schema accessors resolve
each present integer through the target entity's `Universe`; missing,
out-of-range, and dead target IDs produce no pair. The storage sentinel never
appears as an `Id`.

For a sparse entity, the generated loader derives its live-row universe from the
presence mask of the first declared one-valued reference. It also verifies that
every column belonging to that entity has the same extent.

## Writers

The writer functions in `Prela.Cache` exist for tests and small local datasets.
They emit the same v2 layout and reject counts that exceed its 32-bit offset
fields. Real benchmark caches are still produced by the Rust regeneration tool.
