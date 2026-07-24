{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE RankNTypes #-}

-- | Prela in Haskell — the core execution protocol.
--
-- Prela is an embedded query language built on Tarski's algebra of relations.
-- There is exactly one kind of thing: a binary relation — a set of (x, y)
-- pairs — read as a multi-valued function `d -> r` (give it an x, it hands
-- back the y's it relates to: zero, one, or many). Everything else is built by
-- combining relations with operators.
--
-- A relation is accessed in exactly two directions, and which one decides how
-- it executes:
--
--   drive  — enumerate the whole thing: "walk every (x, y) pair." This is the
--            outer-loop role. The left of `→` gets driven.
--   probe  — random access: "here is a key x I already hold; give me its y's."
--            This is the inner role. A filter side gets probed.
--
-- The role is not a property of the relation, it is a property of the POSITION
-- the relation sits in. In `movie : (keyword == "alien") → title`, `movie` is
-- driven while the filter and `title` are probed — same query, both modes.
--
-- The two modes are two record types, and `Mode` is the class with those two
-- records as its instances. Leaves are class methods too, so a leaf is
-- polymorphic and instantiates at whichever mode its position demands, and a
-- query mentions no mode at all — the signature at the top picks one and it
-- flows down. See MODES.md for why it is built this way and what the two
-- rejected alternatives were.

module Prela where

import Control.Monad (when)
import Control.Monad.ST (runST)
import Data.STRef (newSTRef, modifySTRef', readSTRef)
import Data.Array (Array, listArray, (!))
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

--------------------------------------------------------------------------------
-- The two modes
--------------------------------------------------------------------------------

-- | Enumerate: run the action on every (x, y) pair.
--
-- The `forall m.` inside the field (higher-rank, hence RankNTypes) is load
-- bearing: one relation must be drivable in different monads — driven into an
-- index in `ST`, printed in `IO`. The continuations are monadic actions rather
-- than pure accumulator-transformers so the operator bodies stay free of
-- accumulator plumbing.
newtype Drv d r = Drv
  { drive :: forall m. Monad m => (d -> r -> m ()) -> m () }

-- | Look up: given a key x, run the action on every related y.
--
-- `probeAny` short-circuits at the first hit and is PURE: a membership test
-- needs no effect, so it stays out of the monad and drops straight into `when`
-- and `&&` in the streaming operators.
data Prb d r = Prb
  { probe    :: forall m. Monad m => d -> (r -> m ()) -> m ()
  , probeAny :: d -> (r -> Bool) -> Bool
  }

-- | "Is x in this relation at all?" — what a filter actually calls.
member :: Prb d r -> d -> Bool
member q x = probeAny q x (const True)
{-# INLINE member #-}

--------------------------------------------------------------------------------
-- Leaf storage
--------------------------------------------------------------------------------

-- An entity id. Under the hood a 0-based Int, but the phantom tag e (Movie,
-- Keyword, …) rides along in the type and never appears in a value, so
-- `Id Movie` and `Id Keyword` are the same bits and do not unify: composing a
-- Movie-keyed relation onto a Person-keyed one is a compile error, not a
-- silent wrong answer. Erased at runtime, so the safety is free.
newtype Id e = Id Int deriving (Eq, Ord, Show)

-- Storage is its own type, built once, and the leaf constructors below are
-- VIEWS of it. That matters: a leaf is mode-polymorphic, and a polymorphic
-- binding is re-elaborated per instantiation, so if the array were built inside
-- the class method a column used in both modes would be built twice.
data Col e r = Col !Int !(Array Int r)

mkCol :: [r] -> Col e r
mkCol vs = Col n (listArray (0, n - 1) vs)
  where n = length vs

--------------------------------------------------------------------------------
-- The mode class: leaves and every operator whose mode is free
--------------------------------------------------------------------------------

-- The result mode `q` flows through each operator. The right-hand argument of
-- a binary operator is concretely `Prb`, because that side is probed whatever
-- mode the result is in — that single fact is what the whole class turns on.
class Mode q where
  -- Leaves.
  universe  :: Int -> q (Id e) (Id e)
  column    :: Col e r -> q (Id e) r
  fromIndex :: Ord d => Map d [r] -> q d r
  fromCache :: Ord d => Map d s -> q d s

  -- Chain two relations through a shared middle value: `r : d -> e` and
  -- `s : e -> f` give `d -> f`. Also field navigation.
  compose   :: q d e -> Prb e f -> q d f

  -- Pair two relations sharing a DOMAIN: for each key, take both values.
  -- Used in member position this is conjunction — `probeAny` collapses to a
  -- short-circuiting AND and the pair is never built.
  prod      :: q d u -> Prb d v -> q d (u, v)

  -- Keep each row whose VALUE is a member of the second relation.
  restrict  :: q d r -> Prb r e -> q d r

  -- Keep each row whose KEY is absent from the second relation. This is SQL's
  -- IS NULL: a missing value is an absent pair.
  diff      :: q d r -> Prb d e -> q d r

  -- Keep each row whose value passes a test.
  filt      :: (r -> Bool) -> q d r -> q d r

  -- Replace each value, leaving the key alone.
  mapv      :: (r -> s) -> q d r -> q d s

instance Mode Drv where
  universe n = Drv (\k -> mapM_ (\i -> k (Id i) (Id i)) [0 .. n - 1])
  -- Same thunk hazard as the Prb instance below: match the Col inside the
  -- lambda, not on the left of the `=`.
  column c = Drv (\k -> case c of
                          Col n arr -> mapM_ (\i -> k (Id i) (arr ! i)) [0 .. n - 1])
  fromIndex m = Drv (\k -> mapM_ (\(d, rs) -> mapM_ (k d) rs) (Map.toList m))
  fromCache m = Drv (\k -> mapM_ (uncurry k) (Map.toList m))

  compose  a b = Drv (\k -> drive a (\x y -> probe b y (\z -> k x z)))
  prod     a b = Drv (\k -> drive a (\x u -> probe b x (\v -> k x (u, v))))
  restrict a b = Drv (\k -> drive a (\x y -> when (member b y) (k x y)))
  diff     a b = Drv (\k -> drive a (\x v -> when (not (member b x)) (k x v)))
  filt   t a   = Drv (\k -> drive a (\x y -> when (t y) (k x y)))
  mapv   f a   = Drv (\k -> drive a (\x v -> k x (f v)))
  {-# INLINE universe #-}
  {-# INLINE column #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

instance Mode Prb where
  universe n = Prb { probe    = \x k -> when (inUniverse n x) (k x)
                   , probeAny = \x p -> inUniverse n x && p x }
  -- NOTE: the `Col` is deliberately NOT matched on the left of the `=`. Doing
  -- that would make the record a thunk (it must force the column before it can
  -- return), and GHC will not duplicate a thunk into a loop — the probe would
  -- then be an unknown call through a shared record field on every row. Built
  -- this way the record is already a constructor application, so it inlines at
  -- each use site and the array read lands directly in the loop. Verified in
  -- Core; see design/CoreProbe.hs.
  column c =
    Prb { probe    = \(Id i) k -> case c of
                                    Col n arr -> when (0 <= i && i < n) (k (arr ! i))
        , probeAny = \(Id i) p -> case c of
                                    Col n arr -> 0 <= i && i < n && p (arr ! i) }
  fromIndex m = Prb { probe    = \x k -> maybe (return ()) (mapM_ k) (Map.lookup x m)
                    , probeAny = \x p -> maybe False (any p) (Map.lookup x m) }
  fromCache m = Prb { probe    = \x k -> maybe (return ()) k (Map.lookup x m)
                    , probeAny = \x p -> maybe False p (Map.lookup x m) }

  compose a b = Prb
    { probe    = \x k -> probe    a x (\y -> probe    b y k)
    , probeAny = \x p -> probeAny a x (\y -> probeAny b y p)
    }
  prod a b = Prb
    { probe    = \x k -> probe    a x (\u -> probe    b x (\v -> k (u, v)))
    , probeAny = \x p -> probeAny a x (\u -> probeAny b x (\v -> p (u, v)))
    }
  restrict a b = Prb
    { probe    = \x k -> probe    a x (\y -> when (member b y) (k y))
    , probeAny = \x p -> probeAny a x (\y -> member b y && p y)
    }
  diff a b = Prb
    { probe    = \x k -> when (not (member b x)) (probe a x k)
    , probeAny = \x p -> not (member b x) && probeAny a x p
    }
  filt t a = Prb
    { probe    = \x k -> probe    a x (\y -> when (t y) (k y))
    , probeAny = \x p -> probeAny a x (\y -> t y && p y)
    }
  mapv f a = Prb
    { probe    = \x k -> probe    a x (\v -> k (f v))
    , probeAny = \x p -> probeAny a x (\v -> p (f v))
    }
  {-# INLINE universe #-}
  {-# INLINE column #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

inUniverse :: Int -> Id e -> Bool
inUniverse n (Id i) = 0 <= i && i < n
{-# INLINE inUniverse #-}

--------------------------------------------------------------------------------
-- Predicate sugar
--------------------------------------------------------------------------------

-- The comparison operators are not new nodes: each is `filt` with a fixed
-- closure, matching Rust, where `.eq(v)` builds `Filter { p: |x| x == v }`.
-- The value is on the left and v on the right, as in the Prela surface
-- `year > 1980`.
eq, ne :: (Mode q, Eq r) => r -> q d r -> q d r
eq v = filt (== v)
ne v = filt (/= v)
{-# INLINE eq #-}
{-# INLINE ne #-}

gt, lt, ge, le :: (Mode q, Ord r) => r -> q d r -> q d r
gt v = filt (> v)
lt v = filt (< v)
ge v = filt (>= v)
le v = filt (<= v)
{-# INLINE gt #-}
{-# INLINE lt #-}
{-# INLINE ge #-}
{-# INLINE le #-}

isIn :: (Mode q, Eq r) => [r] -> q d r -> q d r
isIn vs = filt (`elem` vs)
{-# INLINE isIn #-}

--------------------------------------------------------------------------------
-- The operators whose mode is NOT free
--------------------------------------------------------------------------------

-- These are exactly the ones the class cannot hold, and keeping them out of it
-- is the point of the design. Each either demands a driven input, or produces
-- a fixed mode, or both.

-- | Flip a relation's direction. Driven, that is free: stream the pairs and
-- swap each one. It has NO probed form — probing an inverse means building a
-- reverse index, which is `inv` below. Since this returns `Drv`, a query that
-- tries to probe it does not compile.
invStream :: Drv d r -> Drv r d
invStream a = Drv (\k -> drive a (\x y -> k y x))
{-# INLINE invStream #-}

-- | Drive a relation once and bucket its pairs by key. This is where a query
-- stops being pure closures and holds real data, and it is why `drive` is
-- `forall m. Monad m` rather than a pure fold: we instantiate m = ST, mutate a
-- ref while driving, and freeze the result.
index :: Ord d => Drv d r -> Map d [r]
index q = runST $ do
  ref <- newSTRef Map.empty
  drive q (\d r -> modifySTRef' ref (Map.insertWith (++) d [r]))
  readSTRef ref

-- | Force a leg once so reuse is free. Same pairs, now backed by the index.
-- Bind the RESULT monomorphically: it is mode-polymorphic, and a polymorphic
-- binding used at two modes builds its index twice, which is the exact cost
-- this exists to remove.
materialize :: (Mode q, Ord d) => Drv d r -> q d r
materialize = fromIndex . index

-- | The probed inverse: index the flipped pairs, and the result serves either
-- mode because the work has already been paid for.
inv :: (Mode q, Ord r) => Drv d r -> q r d
inv = fromIndex . index . invStream

-- | Group by key and reduce each group (`q ▷ (op, init)`). A fold cannot
-- stream — reducing a group means seeing the whole group — so like the index
-- it drives once into a cache, but the cache holds ONE value per key.
-- `count = fold (\n _ -> n + 1) 0`. The result is an ordinary relation, so a
-- fold composes and probes like anything else.
fold :: (Mode q, Ord d) => (s -> r -> s) -> s -> Drv d r -> q d s
fold op ini q = fromCache cache
  where
    cache = runST $ do
      ref <- newSTRef Map.empty
      drive q (\d v -> modifySTRef' ref (Map.alter (Just . flip op v . maybe ini id) d))
      readSTRef ref

-- | A minimal strict state monad, used only by `foldAll`.
--
-- `drive` hands its continuation a monadic action, so a whole-relation fold has
-- to carry the accumulator in the monad. Doing that with an `STRef` costs a
-- heap cell and a boxed write per row; threading it as a state function instead
-- lets GHC turn the accumulator into an argument of the fused loop and unbox it.
newtype Acc s a = Acc { runAcc :: s -> (a, s) }

instance Functor (Acc s) where
  fmap f (Acc g) = Acc (\s -> case g s of (a, s') -> (f a, s'))
  {-# INLINE fmap #-}

instance Applicative (Acc s) where
  pure a = Acc (\s -> (a, s))
  {-# INLINE pure #-}
  Acc f <*> Acc g = Acc (\s -> case f s of
                                 (h, s')  -> case g s' of
                                   (a, s'') -> (h a, s''))
  {-# INLINE (<*>) #-}

instance Monad (Acc s) where
  Acc g >>= f = Acc (\s -> case g s of (a, s') -> runAcc (f a) s')
  {-# INLINE (>>=) #-}

-- | No-group fold (`q ⊵ (op, init)`, already unwrapped): ignore the keys, fold
-- every value into one accumulator, return the bare scalar.
foldAll :: (s -> r -> s) -> s -> Drv d r -> s
foldAll op ini q =
  snd (runAcc (drive q (\_ v -> Acc (\acc -> let !acc' = op acc v in ((), acc')))) ini)
{-# INLINE foldAll #-}
