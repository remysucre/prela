Does it actually fuse?

The whole bet of this port is that a query, which is nothing but a chain of
composed continuations, compiles down to one loop with no intermediates. That is
worth checking rather than assuming, so `design/CoreProbe.hs` builds a
million-row column and runs `movie : (year > 1980) → year ⊵ count` through the
real operators. Its header has the invocation for dumping Core.

The answer is yes. At plain `-O2`, with no extra flags, the query becomes a
single join point of type `Int# -> Int# -> (# Int #)`: unboxed row index,
unboxed accumulator, the array read and the `> 1980` comparison inline in the
loop body, no `Mode` dictionary at runtime, no `Prb` record, no intermediate
structure, and no allocation per row.

It did not come out that way on the first build, and the two things that went
wrong are worth knowing, because neither is a type error or a warning. The first
is that `column` originally matched its `Col` on the left of the `=`. That makes
the `Prb` it returns a thunk, since it must force the column before it can
produce a record, and GHC will not duplicate a thunk into a loop — duplicating a
thunk can duplicate work. The probed side of the join was therefore an unknown
call through a record field on every row, with a `Monad` dictionary passed at
runtime. Matching inside the lambdas instead makes the record a constructor
application, which is cheap and duplicable, so each use site gets a copy the
simplifier can see through. No pragma or flag recovers this: `INLINE` on the
leaf does nothing, and `-fno-full-laziness`, `-fno-cse` and
`-fspecialise-aggressively` each fixed one variant of the program and not
another. The shape of the definition is the fix. The rule this generalizes to,
and it binds every leaf and storage kind still to be written, is that a leaf
constructor must reach weak head normal form without forcing its storage.

The second is that `foldAll` carried its accumulator in an `STRef`. That is the
natural reach, since `drive` hands its continuation a monadic action and the
accumulator has to live somewhere in the monad, but it cost a `readMutVar#`, a
`writeMutVar#` and a freshly boxed `Int` on every matching row. Threading the
accumulator through a small strict state monad instead lets GHC make it an
ordinary loop argument and unbox it. That monad is `Acc` in `Prela.hs`. The
grouped `fold` still uses `ST`, which is fine because its cache is built once
rather than touched per row.

Both of these regress quietly, so re-run `CoreProbe` after adding operators or
storage kinds. Grep the dump for `Prb`, `$fMonad`, `MutVar#` and `((), I#`; all
four should be absent.
