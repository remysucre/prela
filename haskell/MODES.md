# Modes

Start with what a relation has to be able to do. In Prela everything is a set of
pairs, and there are exactly two ways any piece of code ever asks a relation a
question. The first is "walk everything you have," which hands back every pair
one at a time. The second is "here is a key I already have in hand; give me the
values you associate with it." We call the first one driving and the second one
probing. Probing has a short-circuiting cousin, `probeAny`, which stops at the
first value that satisfies a test, and membership is just that with a test that
always says yes.

The important thing is that these are not two kinds of relation. They are two
ways of using the same relation, and which one gets used depends entirely on
where the relation sits in the query. Take `movie : (year > 1980) → title`. The
filtered movie universe on the left is driven, because something has to be the
outer loop that produces keys. The year predicate is probed, because for each
movie we already have the key and we want to ask whether that movie's year
passes. Then `title` is probed too, because we have the movie and we want its
title. If you rewrote the query so that some other relation were the outer loop,
the roles would shift. So the mode is a property of the position, not of the
thing.

That would be a mere labelling exercise if driving and probing were implemented
the same way, but they usually aren't, and sometimes one of them is impossible.
The inverse operator is the clearest case. Inverting a relation means reading its
pairs backwards, so that a relation from movies to keywords becomes one from
keywords to movies. If you only ever drive that inverse, the implementation is
nearly free: stream the original pairs and swap each one as it goes past. But if
you want to probe it, that is, hand it a keyword and ask which movies have it,
streaming is useless, because you would have to scan the entire original relation
for every single probe. The only sane implementation is to walk the original
once, bucket every movie under its keyword, and keep that index in memory. Two
completely different implementations with completely different costs, and which
one you need is decided by where the inverse appears in the query. The same is
true of materialization and, in a different way, of folds, which cannot stream at
all because reducing a group means seeing the whole group first.

So every implementation of Prela has to answer the question of how a node finds
out which mode it is in. Julia answers it with a compilation pass. You build a
query tree where nothing has been decided yet, and then `prepare` walks it from
the top down, telling each node which mode it is in and rewriting mode-agnostic
nodes into concrete single-mode ones. An inverse node becomes either `InvStream`
or `InvIndexed` depending on what the pass found, and a materialization becomes
either a streaming or a probed form. By the time anything runs, every node has
been replaced by a version that knows exactly what it is.

Rust answers it without a pass, by putting the modes in the type system. `Drive`
and `Probe` are separate traits, and a node type implements whichever ones it can
honestly support. The rules of the algebra then appear as constraints on those
implementations. Driving a composition, for example, requires that the left side
be drivable and the right side be probable, so the impl at engine.rs:608 reads
`impl<A: Drive, B: Probe<D = A::R>> Drive for Compose<A, B>`. There is a separate
impl saying that a composition can be probed when both sides can be probed.
Because these constraints chain, asking for the whole query to be drivable at the
top induces a requirement on every node beneath it, and if any node has ended up
in a position it cannot serve, the program does not compile. There is no runtime
error to hit, and no preparation pass to run.

Our Haskell version does neither. `Rel d r` is a single record with all three
functions in it, so every relation claims to support every mode, and every
operator is obliged to produce all three functions whether or not it can. That
leaves nowhere to put the truth. For the inverse, we ended up with two separate
functions the query author picks between by hand: `invStream`, whose probe fields
are literally `error` calls that will crash if anything probes it, and `inv`,
which always builds the index whether or not anyone needed it. So the choice that
Julia makes with a compiler pass and Rust makes with trait resolution, we
currently make by asking the person writing the query to remember which function
to type, with a crash as the penalty for getting it wrong and wasted work as the
penalty for playing it safe.

There are two ways out, and picking between them is the decision that should come
first. Both give the guarantee we want, and both let the query author stop
thinking about modes entirely, which is the real goal: whether a node is driven
or probed should be decided by where it sits, not typed out by hand at every use.
Working prototypes of both are in `design/ModesDeep.hs` and
`design/ModesFinal.hs`, which are not part of the build; both typecheck clean
under `-Wall` on GHC 9.10, and the illegal cases below really are the errors GHC
produces.

The first way is to mirror Rust: give every operator its own data type, make the
modes classes, and let the algebra's rules live in the instance contexts. The
domain and value types become associated type families, standing in for Rust's
associated types on its `Query` trait:

```haskell
class Query q where
  type D q
  type R q

class Query q => Drive q where
  drive :: Monad m => q -> (D q -> R q -> m ()) -> m ()

class Query q => Probe q where
  probe    :: Monad m => q -> D q -> (R q -> m ()) -> m ()
  probeAny :: q -> D q -> (R q -> Bool) -> Bool
```

Then each operator is a data type carrying its arguments, and the rules about
which mode demands what appear as instance contexts. These three instances are a
direct transcription of the three Rust impls for `Compose`:

```haskell
data Compose a b = Compose a b

instance (Query a, Query b, R a ~ D b) => Query (Compose a b) where
  type D (Compose a b) = D a
  type R (Compose a b) = R b

instance (Drive a, Probe b, R a ~ D b) => Drive (Compose a b) where
  drive (Compose a b) k = drive a (\x y -> probe b y (\z -> k x z))

instance (Probe a, Probe b, R a ~ D b) => Probe (Compose a b) where
  probe    (Compose a b) x k = probe    a x (\y -> probe    b y k)
  probeAny (Compose a b) x p = probeAny a x (\y -> probeAny b y p)
```

The streamed inverse is then a data type with a `Drive` instance and no `Probe`
instance at all, so a query containing one in a probed position has no `Probe`
instance either, and asking for one is rejected:

```haskell
probe (compose (InvStream keywordOf) title) (Id 0) putStrLn
-- error: [GHC-39999]
--     • No instance for ‘Probe (InvStream (Column Movie (Id Keyword)))’
--         arising from a use of ‘probe’
```

That is exactly Rust's guarantee, arrived at exactly Rust's way. The costs are
two. It gives up the representation where a query simply is the functions that
run it, since a query is now a tree of constructors and the class instances are
an interpreter over that tree. And the plan ends up written out in the type, so a
three-operator query has a signature like

```haskell
recentTitles :: Compose (Restrict (Universe Movie) (Filt (Column Movie Int)))
                        (Column Movie String)
```

which is fine when inferred and unpleasant when it appears in an error message or
when a query needs a signature. Smart constructors keep the use sites readable,
but the types stay this shape.

The second way keeps the shallow representation and gets the same guarantee for
less. Split the record in two, one per mode, holding exactly the functions that
mode needs:

```haskell
newtype Drv d r = Drv
  { drive :: forall m. Monad m => (d -> r -> m ()) -> m () }

data Prb d r = Prb
  { probe    :: forall m. Monad m => d -> (r -> m ()) -> m ()
  , probeAny :: d -> (r -> Bool) -> Bool
  }
```

Then make the mode a class with those two records as its only instances, and put
the leaves in the class alongside the operators. Because the leaves are methods,
a leaf is polymorphic in the mode and instantiates at whatever its position
demands. The operators take `q` and return `q`, so the mode flows through them,
while the right-hand argument of every binary operator is concretely `Prb`,
because that side is probed no matter what mode the result is in:

```haskell
class Mode q where
  universe  :: Int -> q (Id e) (Id e)
  column    :: [r] -> q (Id e) r
  fromIndex :: Ord d => Map d [r] -> q d r

  compose   :: q d e -> Prb e f -> q d f
  prod      :: q d u -> Prb d v -> q d (u, v)
  restrict  :: q d r -> Prb r e -> q d r
  diff      :: q d r -> Prb d e -> q d r
  filt      :: (r -> Bool) -> q d r -> q d r
  mapv      :: (r -> s) -> q d r -> q d s
```

The two instances are the streaming implementations we already have, sorted by
mode. `Drv`'s `compose` drives its left and probes its right; `Prb`'s probes
both. Nothing else changes about how the operators are written.

The result is that a query names no mode anywhere. The same expression is a
driven query or a probed one depending only on the signature it is given:

```haskell
recentTitles :: Drv (Id Movie) String
recentTitles = compose (restrict movie (gt 1980 year)) title

recentTitleOf :: Prb (Id Movie) String
recentTitleOf = compose (restrict movie (gt 1980 year)) title
```

and a named subquery left mode-polymorphic is usable on either side of anything:

```haskell
recent :: Mode q => q (Id Movie) (Id Movie)
recent = restrict movie (gt 1980 year)
```

The operators whose mode genuinely is not free stay outside the class, which is
the whole point of having it. `invStream` takes and returns a `Drv`. `index`
consumes a `Drv`. `materialize` and `inv` consume a `Drv` and hand back something
mode-polymorphic, since once the index has been paid for it can serve either
mode. Asking to probe a streamed inverse is rejected at the offending
subexpression:

```haskell
bad :: Prb (Id Keyword) String
bad = compose (invStream keywordOf) title
-- error: [GHC-83865]
--     • Couldn't match type ‘Drv’ with ‘Prb’
--       Expected: Prb (Id Keyword) (Id Movie)
--         Actual: Drv (Id Keyword) (Id Movie)
```

One design that does not work is worth recording so nobody tries it again. If you
keep the two records but write the operators as named pairs, `composeD` and
`composeP`, the query author has to choose the variant at every use, which is the
thing we were trying to get rid of. Trying to recover the abstraction by adding a
coercion class, so that a both-modes relation can be passed where either is
wanted, fails as soon as operators nest: the intermediate result of an inner call
is no longer pinned to a type and GHC reports an ambiguous type variable at every
nested call. Making the leaves polymorphic instead of coercing at the arguments
is what avoids this, and it is why the class above includes `universe` and
`column`.

I lean toward the second option. Both reject the same programs, but the first
gives up the thing that makes the Haskell port worth doing, which is that a query
is directly the functions that run it with no plan structure and no interpretation
step in between, and it charges plan-shaped types for the privilege. The second
keeps the shallow embedding, needs one class with two instances rather than a
type and three instances per operator, and reads the same at the use site.

Two things to watch if we take it. The first is that polymorphism defeats
sharing: a mode-polymorphic binding is re-elaborated at each instantiation, so a
materialized relation bound polymorphically and used at both modes will build its
index twice, which is precisely the cost `materialize` exists to eliminate.
Materialized relations must be bound monomorphically. The second is that
operators as class methods mean GHC is passing dictionaries where it previously
saw direct calls, and the entire bet of this port is that the operator chain
fuses into one loop nest, so specialization firing is something to confirm by
looking at the Core rather than something to assume.

The second of those has since been checked and it comes out well: at plain `-O2`
the query fuses to a single unboxed loop with no dictionary and no allocation
per row. Getting there needed two fixes that are easy to get wrong and produce
no error when you do, both written up in FUSION.md.

The reason not to defer any of this is that both options change the type every
single operator is written against, so any operator written before it is settled
has to be rewritten afterwards. It is much cheaper to decide now, with twelve
operators on the books, than after the schema layer and the storage layer have
been built on top of the current shape.

## Closed: splitting membership out of `Prb`

The second option is what got built, and it holds up, but it merged one
distinction that Rust keeps. Rust has three traits, not two. `Member` carries only
`member`, `Probe` carries `probe` and `probe_any` and requires `Member`, and a
type that implements `Probe` gets `Member` free from a default that calls
`probe_any` with a test that always says yes. So a node can honestly say it can
answer "is this key present" without claiming it can produce values, and `Disj`
does exactly that: it implements `Member` alone.

Our `Prb` is those two traits collapsed into one record, so `disj` is obliged to
supply a `probe` field it has no way to implement meaningfully. It fills it with
`()`, which is why `disj`'s value type is `()` rather than anything real. The
guarantee still lands, since a `()` carries no information and nothing can
navigate through it, but it lands by convention rather than by construction:
`probe` on a `disj` is a function you can call, and it hands you back nothing
useful instead of not existing.

The fix is to make membership a class rather than a record field, so that a weaker
record can satisfy it:

```haskell
newtype Mem d r = Mem (d -> Bool)          -- membership and nothing else

class Membership f where
  member :: f d r -> d -> Bool

instance Membership Mem where member (Mem f) x = f x
instance Membership Prb where member q x = probeAny q x (const True)
```

The operators that only ever need membership then stop demanding a `Prb` and take
anything in the class, and `disj` returns the weaker record:

```haskell
restrict :: Membership f => q d r -> f r e -> q d r
diff     :: Membership f => q d r -> f d e -> q d r
disj     :: Membership f => f d u -> f d v -> Mem d ()
```

`Mem` has no `probe` field, so reading a value out of an OR would become
impossible rather than merely pointless.

It does not work. `design/MemSplit.hs` is that sketch cut down to two modes, one
leaf and one operator, and it does not compile:

```
Ambiguous type variable 'f0' arising from a use of 'filt'
prevents the constraint '(Mode f0)' from being solved.
  Potentially matching instances: Mode Drv, Mode Prb
```

The reason is a property of the whole encoding rather than of these three
operators. A filter argument is itself mode-polymorphic: `gt 1980 year` has type
`Mode q => q d r`, because the leaf inside it does. What decides its mode today
is the concrete `Prb` sitting in `restrict`'s signature. Replace that with a
class variable and nothing decides it, since GHC has two instances that fit and
no reason to prefer either, so every filter in every query would need an
annotation.

The variants cost more than they buy. Making `Mem` a third instance of `Mode`
would resolve the mode, but membership cannot implement `compose` — deciding
whether some value of `a` at `x` is in `b` means enumerating `a`'s values — so
`restrict movie (compose keyword keywordText)`, the commonest filter shape there
is, would have no meaning. Keeping `restrict` on `Prb` and injecting `Mem` into
it only moves the same fudge into the injection. Letting `disj` return a fully
polymorphic `Prb d r` whose probe yields nothing is worse than the status quo,
because navigating through an OR would then typecheck and silently return no rows
instead of being rejected.

So `Prb d ()` stays, and it deserves a better description than the one this
section opened with. A membership set IS a relation into the unit type: the value
carries no information because there is none to carry, and a probe handing back
`()` is that set's characteristic function rather than a stub. What Rust has and
we do not is the ability to DECLINE to implement probing; what we have instead is
a value type that makes probing useless. Same guarantee, reached differently.

Reopen this only if the port ever moves to a deep embedding, where a query is
data and `drive`/`probe` are functions over it. That design, which this document
rejects above for other reasons, can say "this node implements membership only",
because a node's type no longer has to name its mode.
