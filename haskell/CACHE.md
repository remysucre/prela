Reading the cache

Queries need real tables, and the tables already exist in a form both other ports
read: cache format v2, one binary file per column, named `<Entity>_<field>.bin`.
The spec is `rust/src/format.rs` and the writer is the Rust `regen` binary, which
absorbs every load-time decision — ids come out 0-based with the hole word baked
in, dates pre-parsed to yyyymmdd integers, strings as offsets plus one packed
buffer, multi-valued columns as CSR. Nothing is left for the reader to compute.

That is the point, and it decided the design of `Prela.Cache`. If the writer has
already chosen the physical layout, the reader should not convert anything: it
should memory-map the file, check the header, and hand back a column that points
into the mapping. So the storage types in `Prela.hs` were made to *be* the on-disk
layouts — eight-byte words for numbers and ids, four-byte offsets over one packed
buffer for strings — and a column is then a `Storable` vector, which is a pointer,
an offset and a length. Loading a gigabyte table costs page-table entries rather
than a pass over the bytes.

The measurement is in `design/CacheProbe.hs`: two full passes over a million-row
column, loaded from a file, allocate 58 KB in total and peak at 43 KB of
residency, against an 8 MB file. That is startup and nothing per row, and the
answer agrees exactly with the same query over a hand-built column. The Core is
still one self-jumping loop with an unboxed index and an unboxed accumulator; the
only change from the array-backed version is that the read is now a
`readIntOffAddr#` from the mapped pointer, plus a `touch#` to keep the mapping
live, which costs nothing at the machine level.

There is one thing Haskell gets right here that Rust could not. The Rust reader
leaks the mmap, because the slices it hands out are `&'static` and there is no
other way to give them that lifetime. Here the vectors and byte slices all retain
the mapping's finalizer, so the file is unmapped when the last column over it is
collected, and no leak is needed to make the lifetimes work out.

Two details are worth knowing because they look like bugs and are not. The first
is that `noId` is `-1` rather than `maxBound`: the hole word on disk is all ones,
read back as a signed machine word that is `-1`, and a loaded column and a
hand-built one have to hold the same bits since nothing downstream knows where a
column came from. Either value fails the `0 <= i && i < n` check every probe
already performs, which is what makes a hole cost nothing, so matching the disk
won. The second is that the cache stores no validity mask for an entity whose id
space has gaps, because it does not need one — the gaps are already visible as
holes in any foreign-key column of that entity, so `validityBits` derives the mask
in one pass at load time, and that is what feeds `sparseUniverse`. The Rust port
builds the same bitset the same way.

The writers in the same module are the other half of the spec. `regen` is what
builds a real JOB or TPC-H cache from parquet, and these are not a replacement for
it; they exist so this port can produce its own files and read them back, which is
what `test/Spec.hs` does for every kind, including the cases most likely to be got
wrong: a hole in the middle of a string column, an empty row in a CSR column, and
an odd key count, which pushes the CSR values off a four-byte boundary and so
tests that the writer's padding and the reader's `align8` agree. The two loud
failures are tested too, since a v1 cache read as v2 would be an off-by-one
disaster rather than an error.
