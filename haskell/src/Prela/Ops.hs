{-# LANGUAGE RankNTypes #-}

-- | The mode class: every leaf, and every operator whose mode is free.
--
-- This is the heart of the port. `Mode` has exactly two instances — the two
-- records from "Prela.Mode" — and everything in the class is written once per
-- mode and then never mentions a mode again. A query built from these names is
-- polymorphic in `q`, so the signature at the top of a query picks the mode and
-- it flows down through the whole expression.
--
-- The class holds what it does for one reason, visible in every signature below:
-- the RIGHT-hand argument of a binary operator is concretely `Prb`, because that
-- side is probed whatever mode the result is in. Only the result mode varies, so
-- only the result mode is the class parameter. The operators that cannot obey
-- that — the ones demanding a driven input, or producing a fixed mode — are
-- deliberately outside the class, in "Prela.Stream" and "Prela.Materialize".
--
-- Both instances are here rather than in modules of their own, because splitting
-- them out would make them orphan instances, and an orphan's unfoldings are not
-- reliably available at the use site. Everything in this module depends on being
-- inlined into the caller's loop, so that is not a trade worth making.
module Prela.Ops where

import Control.Monad (when)
import Data.Array.Base (unsafeAt)
import Data.Array.Unboxed (UArray)
import qualified Data.Array.Unboxed as U
import Data.Hashable (Hashable)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Vector as BV
import qualified Data.Vector.Unboxed as UV

import Prela.Mode
import Prela.Storage

class Mode q where
  -- Leaves.
  universe  :: Int -> q (Id e) (Id e)
  -- A dense id space that carries holes. Its DRIVE walks only the live ids, but
  -- its PROBE checks the range and not the mask, which is not an oversight: an
  -- id reaching a universe in probe position was obtained by navigating from
  -- real data, so it is live by construction, and re-testing the bit would be
  -- work with no possible effect. The Rust port takes the same shortcut.
  sparseUniverse :: Bits e -> Int -> q (Id e) (Id e)
  column    :: Elem r => Col e r -> q (Id e) r
  sparseColumn :: Elem r => SparseCol e r -> q (Id e) r
  multiColumn  :: Elem r => MultiCol e r -> q (Id e) r
  fromIndex :: Ord d => Map d [r] -> q d r
  fromCache :: Ord d => Map d s -> q d s
  -- `Unbox` because a `Dense`'s slots are stored componentwise; see the note on
  -- the type in "Prela.Storage".
  fromDense :: UV.Unbox t => Dense e t -> q (Id e) t
  -- The same, for keys that are not entity ids. Note the DRIVE order is slot
  -- order, which is to say arbitrary — a fold has never promised one, but the
  -- `Map` this replaced happened to give sorted keys, so anything that was
  -- quietly relying on that has to sort for itself now.
  fromTable :: (Hashable d, UV.Unbox t) => Table d t -> q d t
  fromBits  :: Bits e -> q (Id e) (Id e)

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
  sparseUniverse b _ = Drv (\k -> case b of
                                    Bits bs -> mapM_ (\i -> when (unsafeAt bs i) (k (Id i) (Id i)))
                                                     (U.indices bs))
  column c = Drv (\k -> case c of
                          Col n s -> mapM_ (\i -> k (Id i) (atStore s i)) [0 .. n - 1])
  sparseColumn c =
    Drv (\k -> case c of
                 SparseCol n s pres ->
                   mapM_ (\i -> when (unsafeAt pres i) (k (Id i) (atStore s i))) [0 .. n - 1])
  multiColumn c =
    Drv (\k -> case c of
                 MultiCol n offs s ->
                   mapM_ (\i -> mapM_ (\j -> k (Id i) (atStore s j))
                                      [off32 offs i .. off32 offs (i + 1) - 1])
                         [0 .. n - 1])
  fromIndex m = Drv (\k -> mapM_ (\(d, rs) -> mapM_ (k d) rs) (Map.toList m))
  fromCache m = Drv (\k -> mapM_ (uncurry k) (Map.toList m))
  fromDense c = Drv (\k -> case c of
                             Dense n vals seen ->
                               mapM_ (\i -> when (seen U.! i) (k (Id i) (vals UV.! i)))
                                     [0 .. n - 1])
  fromTable t = Drv (\k -> case t of
                             Table mask hs ks vs ->
                               mapM_ (\i -> when (hs UV.! i /= 0)
                                                (k (ks BV.! i) (vs UV.! i)))
                                     [0 .. mask])
  fromBits b = Drv (\k -> case b of
                            Bits bs -> mapM_ (\i -> when (bs U.! i) (k (Id i) (Id i)))
                                             (U.indices bs))

  compose  a b = Drv (\k -> drive a (\x y -> probe b y (\z -> k x z)))
  prod     a b = Drv (\k -> drive a (\x u -> probe b x (\v -> k x (u, v))))
  restrict a b = Drv (\k -> drive a (\x y -> when (member b y) (k x y)))
  diff     a b = Drv (\k -> drive a (\x v -> when (not (member b x)) (k x v)))
  filt   t a   = Drv (\k -> drive a (\x y -> when (t y) (k x y)))
  mapv   f a   = Drv (\k -> drive a (\x v -> k x (f v)))
  {-# INLINE universe #-}
  {-# INLINE sparseUniverse #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromTable #-}
  {-# INLINE fromBits #-}
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
  -- Core; see design/CoreProbe.hs and FUSION.md.
  sparseUniverse _ n = Prb { probe    = \x k -> when (inUniverse n x) (k x)
                           , probeAny = \x p -> inUniverse n x && p x }
  column c =
    Prb { probe    = \(Id i) k -> case c of
                                    Col n s -> when (0 <= i && i < n) (k (atStore s i))
        , probeAny = \(Id i) p -> case c of
                                    Col n s -> 0 <= i && i < n && p (atStore s i) }
  sparseColumn c =
    Prb { probe    = \(Id i) k -> case c of
            SparseCol n s pres -> when (0 <= i && i < n && unsafeAt pres i) (k (atStore s i))
        , probeAny = \(Id i) p -> case c of
            SparseCol n s pres -> 0 <= i && i < n && unsafeAt pres i && p (atStore s i) }
  multiColumn c =
    Prb { probe    = \(Id i) k -> case c of
            MultiCol n offs s ->
              when (0 <= i && i < n)
                   (mapM_ (\j -> k (atStore s j))
                          [off32 offs i .. off32 offs (i + 1) - 1])
        , probeAny = \(Id i) p -> case c of
            MultiCol n offs s ->
              0 <= i && i < n
                && any (\j -> p (atStore s j))
                       [off32 offs i .. off32 offs (i + 1) - 1] }
  fromIndex m = Prb { probe    = \x k -> maybe (return ()) (mapM_ k) (Map.lookup x m)
                    , probeAny = \x p -> maybe False (any p) (Map.lookup x m) }
  fromCache m = Prb { probe    = \x k -> maybe (return ()) k (Map.lookup x m)
                    , probeAny = \x p -> maybe False p (Map.lookup x m) }
  fromDense c =
    Prb { probe    = \(Id i) k -> case c of
                                    Dense n vals seen ->
                                      when (0 <= i && i < n && seen U.! i)
                                           (k (vals UV.! i))
        , probeAny = \(Id i) p -> case c of
                                    Dense n vals seen ->
                                      0 <= i && i < n && seen U.! i
                                        && p (vals UV.! i) }
  fromTable t =
    Prb { probe    = \x k -> case t of
                               tb@(Table _ _ _ vs) ->
                                 case tableSlot tb x of
                                   i | i >= 0    -> k (vs UV.! i)
                                     | otherwise -> return ()
        , probeAny = \x p -> case t of
                               tb@(Table _ _ _ vs) ->
                                 case tableSlot tb x of
                                   i | i >= 0    -> p (vs UV.! i)
                                     | otherwise -> False }
  fromBits b =
    Prb { probe    = \x k -> case b of Bits bs -> when (hasBit bs x) (k x)
        , probeAny = \x p -> case b of Bits bs -> hasBit bs x && p x }

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
  {-# INLINE sparseUniverse #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromTable #-}
  {-# INLINE fromBits #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

inUniverse :: Int -> Id e -> Bool
inUniverse n (Id i) = 0 <= i && i < n
{-# INLINE inUniverse #-}

hasBit :: UArray Int Bool -> Id e -> Bool
hasBit bs (Id i) = case U.bounds bs of
                     (lo, hi) -> lo <= i && i <= hi && bs U.! i
{-# INLINE hasBit #-}
