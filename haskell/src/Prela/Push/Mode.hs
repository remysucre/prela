{-# LANGUAGE RankNTypes #-}

-- | The execution protocol: the two ways a relation can be accessed.
--
-- A relation is reached in exactly two directions, and which one it is decides
-- how it executes:
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
-- This module is only the two records. What makes a query able to sit in either
-- of them without saying so is the `Mode` class in "Prela.Push.Ops", which has
-- these two types as its only instances; MODES.md explains why it is built
-- that way and what the two rejected alternatives were.
module Prela.Push.Mode where

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
