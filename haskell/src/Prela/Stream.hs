{-# LANGUAGE BangPatterns #-}

-- | Operators whose mode is fixed, but which still stream.
--
-- These are the first half of what the `Mode` class in "Prela.Ops" cannot hold,
-- and keeping them out of it is the point of the design rather than a gap in it.
-- Each either demands a driven input or produces a fixed mode, so its type says
-- where in a query it may appear and a misuse is a type error rather than a
-- runtime surprise. What they all share is that nothing is stored: they are still
-- closures over their inputs, and they fuse. The ones that DO stop the pipeline
-- and hold data are in "Prela.Materialize".
module Prela.Stream where

import Control.Monad (when)

import Prela.Mode
import Prela.Ops

-- | Flip a relation's direction. Driven, that is free: stream the pairs and
-- swap each one. It has NO probed form — probing an inverse means building a
-- reverse index, which is `Prela.Materialize.inv`. Since this returns `Drv`, a
-- query that tries to probe it does not compile.
invStream :: Drv d r -> Drv r d
invStream a = Drv (\k -> drive a (\x y -> k y x))
{-# INLINE invStream #-}

-- | Left-compose (`←`), which is defined as `r ← s ≡ r' → s`: rekey the first
-- relation by its own value, then chain the second onto it. Both arguments share
-- a DOMAIN and the result is keyed by the first one's VALUE. Being built on
-- `invStream` it inherits its mode: driven only, no probed form.
leftCompose :: Drv d e -> Prb d f -> Drv e f
leftCompose a b = compose (invStream a) b
{-# INLINE leftCompose #-}

-- | Union of two relations over the same domain and value type (`Drive`-only in
-- the Rust port too): drive one and then the other. There is no probed form,
-- because probing a union would have to probe both sides and de-duplicate, and
-- nothing in Prela asks for that — what queries actually want from OR is
-- membership, which is `disj` below.
union :: Drv d r -> Drv d r -> Drv d r
union a b = Drv (\k -> drive a k >> drive b k)
{-# INLINE union #-}

-- | Membership union (`∨`), the OR of two filters. Never enumerated: only
-- whether a key is present in either side is defined, so the value type is `()`.
-- That is not a placeholder, it is the constraint made visible — you cannot
-- navigate through a `∨` or read a value out of one, and the type says so, which
-- is how the Rust port gets the same guarantee by implementing only `Member`.
-- The two sides may relate their keys to entirely different things.
--
-- The `probe` field below is the part that is not honest: Rust can decline to
-- implement `Probe` at all, whereas `Prb` is one record and forces us to supply
-- something. MODES.md, under "Still open", has the fix and its cost.
disj :: Prb d u -> Prb d v -> Prb d ()
disj a b = Prb
  { probe    = \x k -> when (member a x || member b x) (k ())
  , probeAny = \x p -> (member a x || member b x) && p ()
  }
{-# INLINE disj #-}

--------------------------------------------------------------------------------
-- The whole-relation fold
--------------------------------------------------------------------------------

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
-- every value into one accumulator, return the bare scalar. This one is in this
-- module rather than "Prela.Materialize" because it stores nothing: it is a
-- streaming pass whose only state is the accumulator, and the whole query fuses
-- into it.
foldAll :: (s -> r -> s) -> s -> Drv d r -> s
foldAll op ini q =
  snd (runAcc (drive q (\_ v -> Acc (\acc -> let !acc' = op acc v in ((), acc')))) ini)
{-# INLINE foldAll #-}
