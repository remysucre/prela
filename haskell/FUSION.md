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
ordinary loop argument and unbox it. That monad is `Acc` in `Prela.Stream`. The
grouped `fold` still uses `ST`, which is fine because its cache is built once
rather than touched per row.

The storage layer added a third thing that could have gone wrong and did not.
Element access goes through a class method, `atStore`, so that each element type
gets its own flat layout, and a class method in a hot loop is exactly the kind of
thing that stays an unknown call. It does not here: the dictionary is resolved at
the use site, and `atStore` on an `Int` column becomes a bare `readIntOffAddr#`
inline in the loop body, plus a `touch#` that keeps the backing buffer live and
costs nothing at the machine level. The only mention of `Elem` left in the dump is the
mode-polymorphic `year` binding itself, which nothing calls at runtime because
both of its uses specialize. So the class is free, but it is free for the same
reason as everything else here, and it earns the same warning: the `Store` must
be matched inside the lambda, not before the record is built.

Splitting the engine into per-phase modules changed none of this, which is worth
saying because it easily could have. Nothing here survives without inlining, and
inlining across a module boundary needs the unfolding to be in the interface
file — which it is, because every method of `Mode` and every streaming operator
carries an `INLINE` pragma. That is also why both `Mode` instances have to stay in
`Prela.Ops` rather than getting a module each: an orphan instance's unfoldings are
not reliably available at the use site, and one unspecialized dictionary in the
inner loop would undo the whole thing. The loop after the split is
instruction-for-instruction the one before it.

The schema layer had to be measured too, and there the first design was the
broken one. A schema is a record of loaded columns, and the obvious thing is to
store the relations themselves in it, as fields of type `forall q. Mode q => q
(Id Movie) Int`, so that unpacking the record puts every leaf name in scope at
once. That allocates about 250 bytes per row and leaves `Prb` and a `Monad`
dictionary in the loop. The reason is the same one as everywhere else on this
page, seen from a new angle: what arrives at the use site is a field selection
rather than a definition, so the compiler never learns which instance to
specialize to. Keeping plain columns in the record and reading them through
top-level functions of it, which is what `Prela.Schema` generates, is back to 57
KB and a clean dump; so is binding the names locally in a `where` with
signatures, which is how a query module gets the bare spelling back. The three
variants and their numbers are in `design/SchemaProbe.hs`.

All of these regress quietly, so re-run `CoreProbe` after adding operators or
storage kinds. Grep the dump for `Prb`, `$fMonad`, `MutVar#` and `((), I#`; all
four should be absent. Note also that this checks one query shape. The other
streaming operators are built the same way and should behave the same, but that
has not been read out of Core. `index`, `materialize`, `inv`, `fold`,
`foldDense` and `bitset` deliberately do not fuse: each drives its input once
into real storage and stops the pipeline there, which is what they are for.
