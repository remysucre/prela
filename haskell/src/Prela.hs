{-# LANGUAGE RankNTypes #-}

-- | Prela in Haskell — the core execution protocol.
--
-- Prela is an embedded query language built on Tarski's algebra of relations.
-- There is exactly one kind of thing: a binary relation — a set of (x, y)
-- pairs — read as a multi-valued function `d -> r` (give it an x, it hands
-- back the y's it relates to: zero, one, or many). Everything else is built by
-- combining relations with operators.
--
-- This file is the miniature of the whole engine: the relation type and the
-- three functions every relation must answer, plus three operators to show the
-- shape. We build outward from here.

module Prela where

import Control.Monad (when)
import Control.Monad.ST (runST)
import Data.STRef (newSTRef, modifySTRef', readSTRef)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

--------------------------------------------------------------------------------
-- The two access modes, and the type that captures them
--------------------------------------------------------------------------------

-- A relation is accessed in exactly two directions, and which one decides how
-- it executes:
--
--   drive  — enumerate the whole thing: "walk every (x, y) pair." This is the
--            outer-loop role. The left of `→` gets driven.
--   probe  — random access: "here is a key x I already hold; give me its y's."
--            This is the inner role. A filter side gets probed.
--
-- The role is not a property of the relation, it's a property of the *position*
-- the relation sits in. In `movie : (keyword == "alien") → title`, `movie` is
-- driven while the filter and `title` are probed — same query, both modes.
--
-- We represent a relation directly as the bundle of functions that serve these
-- modes (a shallow / tagless embedding: a query *is* its continuations, there
-- is no separate plan data structure to interpret). Composition of operators is
-- composition of these functions, so a multi-operator query fuses into one
-- streaming pass with no materialized intermediates.
data Rel d r = Rel
  { -- | Run the action on every (x, y) pair. Enumerate.
    drive     :: forall m. Monad m => (d -> r -> m ()) -> m ()

    -- | Given a key x, run the action on every related y. Look up.
  , probe     :: forall m. Monad m => d -> (r -> m ()) -> m ()

    -- | Given a key x, is there a related y satisfying the predicate?
    --   Short-circuits at the first hit. This one is PURE and returns Bool:
    --   a membership test needs no effect, so it stays out of the monad. That
    --   keeps it usable inside `when`/`&&` in the streaming operators below.
  , probeAny  :: d -> (r -> Bool) -> Bool
  }

-- Only three functions — drive, probe, probeAny — matching the Julia and Rust
-- engines (which carry no key-enumeration primitive). We never need to
-- enumerate a domain separately: the thing you drive at the top of a query is
-- always a unary relation (a universe or a filtered universe), and driving that
-- already yields each key once because the value IS the key. Set difference and
-- dedup go through `member` checks over a driven universe, not key extraction.

-- Why the continuations are monadic actions (`m ()`) and not pure
-- accumulator-transformers (`a -> a`): the action shape keeps the operator
-- bodies free of accumulator plumbing (compare `probe s y (\z -> k x z)` with
-- having to thread an `acc` through both loops), and it lets the caching
-- operators pick `m = ST s` to write a mutable array at full speed, while the
-- top level picks `m = IO` to print results as they stream. The query code
-- never mentions `m`; it only shows up in these types.
--
-- The `forall m.` INSIDE each field (higher-rank, hence RankNTypes) is load-
-- bearing: one relation must be drivable in different monads — built in `ST`,
-- later printed in `IO`.

-- | "Is x in this relation at all?" — what a filter actually calls.
member :: Rel d r -> d -> Bool
member r x = probeAny r x (const True)

--------------------------------------------------------------------------------
-- Composition  (r → s)
--------------------------------------------------------------------------------

-- Chain two relations through a shared middle value: `r : d -> e` and
-- `s : e -> f` give `d -> f`, dropping the `e`. `x` reaches `z` when some `y`
-- has `x → y` in r and `y → z` in s. This is also field navigation —
-- `movie → Movie.info → Info.note` is three relations composed.
compose :: Rel d e -> Rel e f -> Rel d f
compose r s = Rel
  { drive     = \k   -> drive r     (\x y -> probe s y (\z -> k x z))
  , probe     = \x k -> probe r x   (\y   -> probe s y k)
  , probeAny  = \x p -> probeAny r x (\y  -> probeAny s y p)
  }

--------------------------------------------------------------------------------
-- Product  (a × b)  — also conjunction (a ∧ b)
--------------------------------------------------------------------------------

-- Pair two relations that share a DOMAIN. `r : d -> a` and `s : d -> b` give
-- `d -> (a, b)`: for each key x, take r's a's and s's b's and emit the pairs.
-- `year × title` on the movie universe turns each movie into `(year, title)`.
--
-- Contrast with compose (`→`). Both combine two relations, but they join on
-- different things:
--   compose r s : the value of r IS the key of s. We hop r → y → s. Types
--                 chain: `d -> e` then `e -> f`. The middle e is consumed.
--   prod    r s : r and s share the SAME key x. We look BOTH up by x and keep
--                 both values. Types stay parallel: both `d -> _`, one x in.
-- So `drive` here drives r (outer loop over keys) but *probes* s by that same
-- key x — it does NOT compose through r's value. That single choice — probe s
-- by x, not by a — is the whole difference between × and →.
prod :: Rel d a -> Rel d b -> Rel d (a, b)
prod r s = Rel
  { drive    = \k   -> drive r   (\x a -> probe s x (\b -> k x (a, b)))
  , probe    = \x k -> probe r x (\a   -> probe s x (\b -> k (a, b)))

    -- probeAny is where "product" and "conjunction" turn out to be one node.
    -- Ask "does key x have any (a, b) pair passing p?" We hunt for an a in r,
    -- then a b in s, then test p. Now specialise to `member` (p = const True):
    -- it collapses to `probeAny r x (\_ -> member s x)` = member r x && member
    -- s x, evaluated lazily — if r has nothing for x we never touch s. That is
    -- exactly the flat short-circuit AND that `∧` wants, and we got it for
    -- free. `∧` is not a separate operator; it is `prod` used in member
    -- position (as the argument of `restrict`/`filt`), where only the values'
    -- existence matters and the pair is never built.
  , probeAny = \x p -> probeAny r x (\a -> probeAny s x (\b -> p (a, b)))
  }

--------------------------------------------------------------------------------
-- Restriction  (a : b)
--------------------------------------------------------------------------------

-- Drive `a` and keep each row whose value is a member of `b`. The classic use
-- is filtering a universe: `movie : (year > 1980)` keeps the movies whose year
-- passes, and hands back movies (not years).
--
-- This is where `member` being a pure Bool pays off: it drops straight into
-- `when` and `&&`. `when (member b y) (k x y)` reads as "if y is in b, emit."
restrict :: Rel d r -> Rel r e -> Rel d r
restrict a b = Rel
  { drive     = \k   -> drive a     (\x y -> when (member b y) (k x y))
  , probe     = \x k -> probe a x   (\y   -> when (member b y) (k y))
  , probeAny  = \x p -> probeAny a x (\y  -> member b y && p y)
  }

--------------------------------------------------------------------------------
-- Predicate  (year > 1980, title == "Up", ...)
--------------------------------------------------------------------------------

-- Keep each row whose VALUE passes a test. `filt (> 1980) year` turns
-- {0->1979, 1->1986, 2->2009} into {1->1986, 2->2009}. Same gating as
-- `restrict`, but the test is any function instead of membership in another
-- relation. The comparison operators (==, <, >, ...) are just `filt` with a
-- comparison function. (Named `filt`, not `filter`, to avoid the Prelude clash.)
filt :: (r -> Bool) -> Rel d r -> Rel d r
filt t a = Rel
  { drive     = \k   -> drive a     (\x y -> when (t y) (k x y))
  , probe     = \x k -> probe a x   (\y   -> when (t y) (k y))
  , probeAny  = \x p -> probeAny a x (\y  -> t y && p y)
  }

--------------------------------------------------------------------------------
-- Predicate sugar  (== < > in ...)
--------------------------------------------------------------------------------

-- The comparison operators are not new nodes: each is filt with a fixed
-- closure. `eq v` keeps rows whose value equals v, `gt v` whose value exceeds
-- v, and so on — exactly Rust, where `.eq(v)` builds `Filter { p: |x| x == v }`.
-- The value is on the left of the comparison and v on the right, matching the
-- Prela surface `year > 1980`. (Rust also auto-navigates an entity value to its
-- primary field here; that elision belongs to the schema layer we don't have
-- yet, so these compare the value directly.)
eq, ne :: Eq r => r -> Rel d r -> Rel d r
eq v = filt (== v)
ne v = filt (/= v)

gt, lt, ge, le :: Ord r => r -> Rel d r -> Rel d r
gt v = filt (> v)
lt v = filt (< v)
ge v = filt (>= v)
le v = filt (<= v)

-- `a in (v1, …)` — keep rows whose value is one of vs.
isIn :: Eq r => [r] -> Rel d r -> Rel d r
isIn vs = filt (`elem` vs)

--------------------------------------------------------------------------------
-- Set difference  (a - b)
--------------------------------------------------------------------------------

-- Keep a's pairs whose KEY is not a member of b. a and b share the domain d;
-- b's value type is irrelevant because b is only ever asked `member` — does
-- this key exist in you at all. a's values pass through unchanged.
--
-- This is how SQL's `IS NULL` works here. A missing value is an absent pair, so
-- "movies with no keyword" is `movie - keyword` (the keyed keyword relation) —
-- drive the movie universe, drop every movie the keyword relation has a row
-- for, keep the rest.
--
-- Note the difference from `restrict`: restrict tested the VALUE against b,
-- diff tests the KEY. That is why b sits on the domain side (`Rel d e`, shared
-- d) rather than the value side.
diff :: Rel d r -> Rel d e -> Rel d r
diff a b = Rel
  { drive    = \k   -> drive a     (\x v -> when (not (member b x)) (k x v))
  , probe    = \x k -> when (not (member b x)) (probe a x k)
  , probeAny = \x p -> not (member b x) && probeAny a x p
  }

--------------------------------------------------------------------------------
-- Inverse  (r')
--------------------------------------------------------------------------------

-- Flip a relation's direction: `keyword : Movie -> Keyword` inverts to
-- `keyword' : Keyword -> Movie` ("which movies have this keyword"). Same pairs,
-- read backwards.
--
-- Driven, that's free: stream the forward pairs, swap each one. Probing it
-- ("which x relate to y?") would need a reverse index built in memory — we
-- leave that form out for now and add it later via a shared index builder
-- (the same one materialize will use).
invStream :: Rel d r -> Rel r d
invStream r = Rel
  { drive     = \k -> drive r (\x y -> k y x)     -- just flip each pair
  , probe     = error "invStream: driven-only for now (probed inverse needs an index)"
  , probeAny  = error "invStream: driven-only for now (probed inverse needs an index)"
  }

--------------------------------------------------------------------------------
-- Materialization and indexing
--------------------------------------------------------------------------------

-- Everything above streams: a subexpression used twice is recomputed twice, and
-- a driven-only relation (like invStream) cannot be probed. Both problems have
-- the same fix — drive the relation ONCE into an in-memory index, then serve
-- everything from that. This is the only place a query stops being pure
-- closures and holds real data.

-- The index: a Map from key to the list of values that key related to. `index`
-- drives q once and buckets each pair by its key.
--
-- Here is why `drive` is `forall m. Monad m` and not a pure fold. We need to
-- accumulate while driving, so we instantiate m = ST: allocate a mutable ref
-- holding a Map, drive the relation modifying the ref on each pair, then freeze
-- the ref back to an immutable Map with runST. The same q could be driven in IO
-- at the top level to print — one relation, different monads, which is exactly
-- what the higher-rank field type buys us.
--
-- insertWith prepends, so values within a key come out reversed. For an index
-- consulted by membership and re-driven as a bag that never matters.
index :: Ord d => Rel d r -> Map d [r]
index q = runST $ do
  ref <- newSTRef Map.empty
  drive q (\d r -> modifySTRef' ref (Map.insertWith (++) d [r]))
  readSTRef ref

-- Turn a built index back into a relation. It can now be driven AND probed, both
-- cheap, because the work already happened.
fromIndex :: Ord d => Map d [r] -> Rel d r
fromIndex m = Rel
  { drive    = \k   -> mapM_ (\(d, rs) -> mapM_ (k d) rs) (Map.toList m)
  , probe    = \x k -> maybe (return ()) (mapM_ k) (Map.lookup x m)
  , probeAny = \x p -> maybe False (any p) (Map.lookup x m)
  }

-- `materialize q` / `!q`: force a leg once so reuse is free. Same pairs as q,
-- but now backed by the index instead of recomputed on each pass.
materialize :: Ord d => Rel d r -> Rel d r
materialize = fromIndex . index

-- The probed inverse. invStream could only flip-and-drive; to probe an inverse
-- ("which x related to this y?") we index the flipped pairs. invStream q streams
-- (r, d); indexing that buckets each d under its r, giving a relation we can
-- probe by r. This is the InvIndexed form the invStream comment promised.
inv :: Ord r => Rel d r -> Rel r d
inv = fromIndex . index . invStream

--------------------------------------------------------------------------------
-- Fold  (q ▷ (op, init))  — group by key, reduce each group
--------------------------------------------------------------------------------

-- Every operator so far streamed one row at a time. A fold cannot: to reduce a
-- group it must see the whole group first. So like the index, it drives once
-- into a cache — but the cache holds ONE accumulated value per key, not a list.
-- For each pair (d, v) it folds v into key d's running accumulator with op,
-- seeded at init. `count = fold (\n _ -> n + 1) 0`; `sum = fold (+) 0`.
--
-- The result is an ordinary relation keyed by the group key, valued by the
-- reduction, so a fold composes and probes like anything else — a JOB
-- `MIN(x)` is a fold whose value then flows onward.
fold :: Ord d => (s -> r -> s) -> s -> Rel d r -> Rel d s
fold op ini q = Rel
  { -- one value per key: drive emits one pair per group, probe returns at most
    -- one value. All three read the finished cache directly (as Rust's Fold does).
    drive    = \k   -> mapM_ (\(d, s) -> k d s) (Map.toList cache)
  , probe    = \x k -> maybe (return ()) k (Map.lookup x cache)
  , probeAny = \x p -> maybe False p (Map.lookup x cache)
  }
  where
    cache = runST $ do
      ref <- newSTRef Map.empty
      -- alter looks up key d, folds v into whatever is there (init if absent).
      drive q (\d v -> modifySTRef' ref (Map.alter (\mb -> Just (op (maybe ini id mb) v)) d))
      readSTRef ref

-- No-group fold (q ⊵ (op, init), already unwrapped). Same as fold but with no
-- grouping: drive the whole relation, ignore the key, fold every value into one
-- accumulator, and return the bare scalar — not a relation. This is Prela's ⊵
-- with unwrap folded in, matching Rust's unwrap_fold. `foldAll min maxBound
-- year` is the whole-column MIN.
foldAll :: (s -> r -> s) -> s -> Rel d r -> s
foldAll op ini q = runST $ do
  ref <- newSTRef ini
  drive q (\_ v -> modifySTRef' ref (\acc -> op acc v))
  readSTRef ref

--------------------------------------------------------------------------------
-- Map  (q ↦ f)  — transform each row's value
--------------------------------------------------------------------------------

-- Run f on every value, leaving the key alone. `year ↦ (\y -> y `div` 10 * 10)`
-- turns years into decades. Same wiring as filt, but where filt gated the value
-- through unchanged, mapv replaces it with f applied. Streams like everything
-- else — no cache — because a per-row transform needs no group. (Named mapv;
-- Prelude owns `map`.)
mapv :: (r -> s) -> Rel d r -> Rel d s
mapv f q = Rel
  { drive    = \k   -> drive q   (\x v -> k x (f v))
  , probe    = \x k -> probe q x (\v   -> k (f v))
  , probeAny = \x p -> probeAny q x (\v -> p (f v))
  }

--------------------------------------------------------------------------------
-- Leaves — relations that actually hold data
--------------------------------------------------------------------------------

-- Everything above combines relations. These two are the base relations with
-- real data in them. Their KEYS carry an entity tag.

-- An entity id. Under the hood it is just a 0-based Int, but the phantom tag e
-- (Movie, Keyword, Person, …) rides along in the type and never appears in a
-- value. So `Id Movie` and `Id Keyword` are the same bits but different types,
-- and they do not unify: composing a Movie-keyed relation onto a Person-keyed
-- one is a COMPILE error, not a silent wrong answer. The tag is erased at
-- runtime (newtype = no wrapper), so the safety is free. This is Rust's
-- repr(transparent) `Id<E>`. Entity tags are declared as empty types with no
-- constructors — `data Movie` — since they exist only to label ids.
newtype Id e = Id Int deriving (Eq, Ord, Show)

-- The universe: "there are n entities of type e, numbered 0 .. n-1." It's the
-- identity relation (each id maps to itself), so it's what you drive to loop
-- over every entity — the start of a query. Stored as just the count n.
universe :: Int -> Rel (Id e) (Id e)
universe n = Rel
  { drive     = \k   -> mapM_ (\i -> k (Id i) (Id i)) [0 .. n - 1]  -- (0,0),(1,1),…
  , probe     = \x k -> when (inRange x) (k x)                      -- id maps to itself
  , probeAny  = \x p -> inRange x && p x
  }
  where inRange (Id i) = 0 <= i && i < n

-- A stored column keyed by an `Id e`: a lookup table `id -> value`, e.g.
-- year = [1979,1986,2009]. The value r is anything — a scalar (String, Int) for
-- an attribute, or another `Id other` for a foreign key. Probe id i, get
-- vals[i]. "Dense" = every id 0..n-1 has a value. (A real engine uses a packed
-- vector; a list is fine for a small demo.)
column :: [r] -> Rel (Id e) r
column vals = Rel
  { drive     = \k        -> mapM_ (\(i, v) -> k (Id i) v) (zip [0 ..] vals)
  , probe     = \(Id i) k -> when (inRange i) (k (vals !! i))
  , probeAny  = \(Id i) p -> inRange i && p (vals !! i)
  }
  where n = length vals
        inRange i = 0 <= i && i < n
