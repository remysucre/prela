{-# LANGUAGE RankNTypes #-}

-- | Not part of the build, and EXPECTED TO FAIL. This is the record of why
-- membership is not split out of `Prb`; MODES.md, under "Closed", has the
-- discussion. Keep it failing.
--
--   cabal exec -- ghc -fno-code -fforce-recomp design/MemSplit.hs
--
--   Ambiguous type variable 'f0' arising from a use of 'filt'
--   prevents the constraint '(Mode f0)' from being solved.
--     Potentially matching instances: Mode Drv, Mode Prb
--
-- The question the sketch in MODES.md did not ask is whether a class-polymorphic
-- argument still lets GHC resolve the leaf's mode. It does not: the filter
-- argument is mode-polymorphic itself, and the concrete `Prb` in `restrict`'s
-- signature is the only thing that was pinning it down.
--
-- Cut down to the smallest thing that can fail: two modes, one leaf, one
-- operator, and one query written the way a real one is.
module MemSplit where

import Control.Monad (when)

newtype Drv d r = Drv { drive :: forall m. Monad m => (d -> r -> m ()) -> m () }
data    Prb d r = Prb { probeAny :: d -> (r -> Bool) -> Bool }
newtype Mem d r = Mem (d -> Bool)          -- membership and nothing else

class Membership f where
  member :: f d r -> d -> Bool
instance Membership Prb where member q x = probeAny q x (const True)
instance Membership Mem where member (Mem f) x = f x

-- The proposed signature: `restrict` no longer names `Prb`, it accepts anything
-- that can answer membership.
class Mode q where
  universe :: Int -> q Int Int
  filt     :: (r -> Bool) -> q d r -> q d r
  restrict :: Membership f => q d r -> f r e -> q d r

instance Mode Drv where
  universe n = Drv (\k -> mapM_ (\i -> k i i) [0 .. n - 1])
  filt t a = Drv (\k -> drive a (\x y -> when (t y) (k x y)))
  restrict a b = Drv (\k -> drive a (\x y -> when (member b y) (k x y)))

instance Mode Prb where
  universe n = Prb (\x p -> 0 <= x && x < n && p x)
  filt t a = Prb (\x p -> probeAny a x (\y -> t y && p y))
  restrict a b = Prb (\x p -> probeAny a x (\y -> member b y && p y))

-- And here is a query in the style every real one is written in: the filter
-- argument is built from a mode-polymorphic leaf, so its own mode is decided by
-- the position it lands in.
q :: Drv Int Int
q = restrict (universe 10) (filt (> 3) (universe 10))
