{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The mode class: every leaf, and every operator whose mode is free.
--
-- This is the same shape MODES.md argues for and the push engine's "Prela.Ops"
-- has. `SMode` has exactly two instances — `SStream` for driven position and
-- `Cursor` for probed — and everything in the class is written once per mode and
-- then never mentions a mode again. A query built from these names is
-- polymorphic in @q@, so the signature at the top picks the mode and it flows
-- down through the whole expression.
--
-- The class holds what it does for one reason, visible in every signature below:
-- the RIGHT-hand argument of a binary operator is concretely `Cursor`, because
-- that side is probed whatever mode the result is in. Only the result mode
-- varies, so only the result mode is the class parameter.
--
-- What changed from push. There is no @probeAny@, so there is no second copy of
-- every operator — the `Prb` instance in the push engine writes each one twice,
-- once for values and once for short-circuit, and here membership is `anyOf`, a
-- consumer. That is roughly half the module gone.
--
-- Both instances are here rather than in modules of their own. Under push that
-- was about orphan unfoldings; here it is only about keeping the two readable
-- side by side, since nothing about a staged instance depends on being inlined.
module Prela.Staged.Ops
  ( SMode (..)
    -- * Fixed-mode operators
  , groupBy
  , leftCompose
  , union
  , disj
    -- * Producer-level leaves, for lockstep
  , universeProd
  , columnProd
  ) where

import Data.Hashable (Hashable)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Vector.Unboxed as UV
import Language.Haskell.TH (CodeQ)

import Prela.Staged.Stream
import Prela.Storage

-- A note on the leading underscores below. A leaf takes its storage apart with a
-- `case`, but whether it then READS the value store depends on the consumer:
-- `anyOf` and `count` want only that a row exists, so the yield continuation
-- discards the value and the array read is never emitted. That leaves a bound
-- variable with no uses, and `-Wall` on the SPLICING module reports it — a
-- warning about a name the query author never wrote. Prefixing the field with an
-- underscore silences it without changing anything else, since the prefix
-- suppresses the warning but the name is still usable.
class SMode q where
  -- Leaves.
  universe       :: CodeQ Int -> q (Id e) (Id e)
  -- A dense id space that carries holes. Its DRIVE walks only the live ids, but
  -- its PROBE checks the range and not the mask, which is not an oversight: an
  -- id reaching a universe in probe position was obtained by navigating from
  -- real data, so it is live by construction, and re-testing the bit would be
  -- work with no possible effect. The Rust port takes the same shortcut.
  sparseUniverse :: CodeQ (Bits e) -> CodeQ Int -> q (Id e) (Id e)
  column         :: Elem r => CodeQ (Col e r) -> q (Id e) r
  sparseColumn   :: Elem r => CodeQ (SparseCol e r) -> q (Id e) r
  multiColumn    :: Elem r => CodeQ (MultiCol e r) -> q (Id e) r
  fromIndex      :: Ord d => CodeQ (Map d [r]) -> q d r
  fromCache      :: Ord d => CodeQ (Map d s) -> q d s
  fromDense      :: UV.Unbox t => CodeQ (Dense e t) -> q (Id e) t
  -- The DRIVE order is slot order, which is to say arbitrary. A fold has never
  -- promised one, so anything that wants sorted keys sorts after `collect`.
  fromTable      :: (Hashable d, Key d, UV.Unbox t) => CodeQ (Table d t) -> q d t
  fromBits       :: CodeQ (Bits e) -> q (Id e) (Id e)

  -- Chain two relations through a shared middle value: `r : d -> e` and
  -- `s : e -> f` give `d -> f`. Also field navigation.
  compose  :: q d e -> Cursor e f -> q d f

  -- Pair two relations sharing a DOMAIN: for each key, take both values.
  prod     :: q d u -> Cursor d v -> q d (u, v)

  -- Keep each row whose VALUE is a member of the second relation. This is where
  -- early termination lands: `anyOf` stops at the first inner row.
  restrict :: q d r -> Cursor r e -> q d r

  -- Keep each row whose KEY is absent from the second relation. SQL's IS NULL:
  -- a missing value is an absent pair.
  diff     :: q d r -> Cursor d e -> q d r

  filt     :: (CodeQ r -> CodeQ Bool) -> q d r -> q d r
  mapv     :: (CodeQ r -> CodeQ s) -> q d r -> q d s

--------------------------------------------------------------------------------
-- Driven
--------------------------------------------------------------------------------

instance SMode SStream where
  universe n       = Lin (universeProd n)
  column c         = Lin (columnProd c)
  -- The drive walks the MASK's extent, not the id space's. Those agree in
  -- practice, but the mask is the thing being indexed, so it is the one that
  -- decides where to stop. The push engine's `U.indices` said the same.
  sparseUniverse b _ = Lin Producer
    { pEnv  = b
    , pInit = \_ -> [|| 0 :: Int ||]
    , pStep = \e s yield skip done ->
        [|| case $$e of
              Bits bs ->
                if $$s >= bitsLen bs then $$done
                else if atBit bs $$s
                       then $$(yield [|| Id $$s ||] [|| Id $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  sparseColumn c = Lin Producer
    { pEnv  = c
    , pInit = \_ -> [|| 0 :: Int ||]
    , pStep = \e s yield skip done ->
        [|| case $$e of
              SparseCol n _st pres ->
                if $$s >= n then $$done
                else if atBit pres $$s
                       then $$(yield [|| Id $$s ||] [|| atStore _st $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  -- Flat rather than a `Bind` of two loops, so a multi-valued column stays
  -- zippable. The state is (key, cursor, row end); a `skip` opens the next row.
  -- Starting at key -1 with an empty range makes the empty column fall out
  -- without a special case.
  multiColumn c = Lin Producer
    { pEnv  = c
    , pInit = \_ -> [|| (-1 :: Int, 0 :: Int, 0 :: Int) ||]
    , pStep = \e s yield skip done ->
        [|| case $$e of
              MultiCol n offs _st -> case $$s of
                (i, j, end)
                  | j < end   -> $$(yield [|| Id i ||] [|| atStore _st j ||]
                                          [|| (i, j + 1, end) ||])
                  | i + 1 >= n -> $$done
                  | otherwise ->
                      let !i' = i + 1
                      in $$(skip [|| (i', off32 offs i', off32 offs (i' + 1)) ||]) ||]
    }
  fromDense c = Lin Producer
    { pEnv  = c
    , pInit = \_ -> [|| 0 :: Int ||]
    , pStep = \e s yield skip done ->
        [|| case $$e of
              Dense n _vals seen ->
                if $$s >= n then $$done
                else if atBit seen $$s
                       then $$(yield [|| Id $$s ||] [|| _vals UV.! $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromTable t = Lin Producer
    { pEnv  = t
    , pInit = \_ -> [|| 0 :: Int ||]
    , pStep = \e s yield skip done ->
        [|| case $$e of
              Table mask hs _ks _vs ->
                if $$s > mask then $$done
                else if hs UV.! $$s /= 0
                       then $$(yield [|| indexKey _ks $$s ||] [|| _vs UV.! $$s ||]
                                     [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromBits b = Lin Producer
    { pEnv  = b
    , pInit = \_ -> [|| 0 :: Int ||]
    , pStep = \e s yield skip done ->
        [|| case $$e of
              Bits bs ->
                if $$s >= bitsLen bs then $$done
                else if atBit bs $$s
                       then $$(yield [|| Id $$s ||] [|| Id $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromIndex m =
    Bind (Lin (pairProd [|| Map.toList $$m ||]))
         (\d vs -> mapkS (\_ -> d) (Lin (listProd vs)))
  fromCache m = Lin (pairProd [|| Map.toList $$m ||])

  compose  a b = Bind a (\d e -> mapkS (\_ -> d) (at b e))
  -- The two bangs are load bearing and they were measured. The PAIR always
  -- cancels — it is built and immediately taken apart, so case-of-constructor
  -- removes it even four deep, which is why the value type can stay a code-level
  -- tuple. Its COMPONENTS do not cancel on their own: each is a checked array
  -- read, which has a bottoming branch GHC will not speculate, so left lazy it
  -- crosses the inner loop as a boxed thunk. That is 64 bytes a row on a
  -- three-deep tower and Q1's payload is four deep. This bang and the matching
  -- one in `sfoldWhile` only work together; see design/StagedPullMain.hs.
  prod     a b = Bind a (\d u -> mapkS (\_ -> d)
                                   (mapvS (\v -> [|| let !x = $$u
                                                         !y = $$v
                                                     in (x, y) ||]) (at b d)))
  restrict a b = filtKV (\_ r -> anyOf (at b r)) a
  diff     a b = filtKV (\d _ -> [|| not $$(anyOf (at b d)) ||]) a
  filt     t a = filtS t a
  mapv     f a = mapvS f a

--------------------------------------------------------------------------------
-- Probed
--------------------------------------------------------------------------------

-- Every probed leaf range-checks its key, and that is the semantics rather than
-- defensiveness. A probed key is untrusted: a foreign key column has holes, and
-- "Prela.Cache" spells a hole @noId = -1@. The answer for a hole is NO VALUES,
-- not a crash and not a wild read. Bounds-checked indexing would not give that —
-- it would throw where the engine must yield nothing — which is also why
-- `atStore` in "Prela.Storage" stays unchecked and each leaf tests for itself.
instance SMode Cursor where
  universe n = Cursor $ \x -> mapvS (\_ -> x) (guardS [|| idInRange $$n $$x ||])
  sparseUniverse _ n = Cursor $ \x -> mapvS (\_ -> x) (guardS [|| idInRange $$n $$x ||])
  column c = Cursor $ \x -> Lin Producer
    { pEnv  = c
    , pInit = \_ -> [|| True ||]
    , pStep = \e s yield _skip done ->
        [|| case $$e of
              Col n _st -> case $$x of
                Id i | $$s && 0 <= i && i < n ->
                         $$(yield [|| () ||] [|| atStore _st i ||] [|| False ||])
                     | otherwise -> $$done ||]
    }
  sparseColumn c = Cursor $ \x -> Lin Producer
    { pEnv  = c
    , pInit = \_ -> [|| True ||]
    , pStep = \e s yield _skip done ->
        [|| case $$e of
              SparseCol n _st pres -> case $$x of
                Id i | $$s && 0 <= i && i < n && atBit pres i ->
                         $$(yield [|| () ||] [|| atStore _st i ||] [|| False ||])
                     | otherwise -> $$done ||]
    }
  -- The row's extent is read once, in `pInit`, rather than on every step. An
  -- out-of-range key gets the empty range (0, 0), which is the no-values answer.
  multiColumn c = Cursor $ \x -> Lin Producer
    { pEnv  = c
    , pInit = \e -> [|| case $$e of
                          MultiCol n offs _ -> case $$x of
                            Id i | 0 <= i && i < n -> (off32 offs i, off32 offs (i + 1))
                                 | otherwise       -> (0, 0) ||]
    , pStep = \e s yield _skip done ->
        [|| case $$e of
              MultiCol _ _ _st -> case $$s of
                (j, end) | j >= end  -> $$done
                         | otherwise -> $$(yield [|| () ||] [|| atStore _st j ||]
                                                 [|| (j + 1, end) ||]) ||]
    }
  fromDense c = Cursor $ \x -> Lin Producer
    { pEnv  = c
    , pInit = \_ -> [|| True ||]
    , pStep = \e s yield _skip done ->
        [|| case $$e of
              Dense n _vals seen -> case $$x of
                Id i | $$s && 0 <= i && i < n && atBit seen i ->
                         $$(yield [|| () ||] [|| _vals UV.! i ||] [|| False ||])
                     | otherwise -> $$done ||]
    }
  fromTable t = Cursor $ \x -> Lin Producer
    { pEnv  = t
    , pInit = \e -> [|| tableSlot $$e $$x ||]
    , pStep = \e s yield _skip done ->
        [|| case $$e of
              Table _ _ _ _vs
                | $$s >= 0  -> $$(yield [|| () ||] [|| _vs UV.! $$s ||] [|| -1 ||])
                | otherwise -> $$done ||]
    }
  fromBits b = Cursor $ \x -> mapvS (\_ -> x) (guardS [|| bitsMember $$b $$x ||])
  fromIndex m = Cursor $ \x -> Lin (listProd [|| Map.findWithDefault [] $$x $$m ||])
  fromCache m = Cursor $ \x -> Lin (maybeProd [|| Map.lookup $$x $$m ||])

  compose  a b = Cursor $ \x -> Bind (at a x) (\_ e -> at b e)
  prod     a b = Cursor $ \x ->
                   Bind (at a x)
                        (\_ u -> mapvS (\v -> [|| let !p = $$u
                                                      !q = $$v
                                                  in (p, q) ||]) (at b x))
  restrict a b = Cursor $ \x -> filtS (\r -> anyOf (at b r)) (at a x)
  -- The test is on the KEY, so it is decided once for the whole probe rather
  -- than per row, which is what `whenS` is for.
  diff     a b = Cursor $ \x -> whenS [|| not $$(anyOf (at b x)) ||] (at a x)
  filt     t a = Cursor $ \x -> filtS t (at a x)
  mapv     f a = Cursor $ \x -> mapvS f (at a x)

--------------------------------------------------------------------------------
-- Fixed-mode operators
--------------------------------------------------------------------------------

-- | Re-key a stream by something computed from each VALUE. What comes out is
-- (group key, value), which is exactly what a grouped fold wants, so
-- @withFold op ini (groupBy q key)@ is Prela's GROUP BY over a non-key column.
--
-- Driven only, and free: one extra probe per row with nothing stored. Its second
-- argument is probed at the VALUE rather than the key, which is what makes it
-- different from `compose`.
--
-- One sharp edge, shared with the Rust port: grouping by a foreign key with
-- holes puts every hole in a group of its own, keyed by `noId`, rather than
-- dropping those rows. Restrict first if that group is unwanted.
groupBy :: SStream d r -> Cursor r k -> SStream k r
groupBy s key = Bind s (\_ x -> mapvS (\_ -> x) (byValue (at key x)))

-- | Left-compose, defined as @r <- s@ ≡ @r' -> s@: rekey the first relation by
-- its own value, then chain the second onto it. Driven only, since `invStream`
-- is.
leftCompose :: SStream d e -> Cursor d f -> SStream e f
leftCompose a b = compose (invStream a) b

-- | One relation and then the other. Driven only: probing a union would have to
-- probe both sides and de-duplicate, and what queries actually want from OR is
-- membership, which is `disj`.
union :: SStream d r -> SStream d r -> SStream d r
union = catS

-- | Membership union, the OR of two filters. Never enumerated: only whether a
-- key is present in either side is defined, so the value type is @()@. That is
-- not a placeholder, it is the constraint made visible — you cannot navigate
-- through an OR or read a value out of one, and the type says so.
--
-- Under push this needed its own `Prb` record with both fields written out. Here
-- it is two `anyOf`s and a `guardS`, and the short-circuit is `anyOf`'s.
disj :: Cursor d u -> Cursor d v -> Cursor d ()
disj a b = Cursor $ \x -> guardS [|| $$(anyOf (at a x)) || $$(anyOf (at b x)) ||]

--------------------------------------------------------------------------------
-- Producer-level leaves
--------------------------------------------------------------------------------

-- The two leaves worth zipping. Everything else reaches lockstep by being
-- materialized first, which is the honest cost of the linearity restriction.

universeProd :: CodeQ Int -> Producer (Id e) (Id e)
universeProd n = Producer
  { pEnv  = n
  , pInit = \_ -> [|| 0 :: Int ||]
  , pStep = \e s yield _skip done ->
      [|| if $$s >= $$e then $$done
            else $$(yield [|| Id $$s ||] [|| Id $$s ||] [|| $$s + 1 ||]) ||]
  }

columnProd :: Elem r => CodeQ (Col e r) -> Producer (Id e) r
columnProd c = Producer
  { pEnv  = c
  , pInit = \_ -> [|| 0 :: Int ||]
  , pStep = \e s yield _skip done ->
      [|| case $$e of
            Col n _st | $$s >= n  -> $$done
                     | otherwise -> $$(yield [|| Id $$s ||] [|| atStore _st $$s ||]
                                              [|| $$s + 1 ||]) ||]
  }

--------------------------------------------------------------------------------
-- List-backed producers
--------------------------------------------------------------------------------

-- The `Map`-backed leaves are the one place the engine is not flat, and they are
-- the reason "Prela.Staged.Materialize" prefers `withFold` over `withIndex`.
-- Nothing here can fuse into an array read, because there is no array.

listProd :: CodeQ [a] -> Producer () a
listProd xs = Producer
  { pEnv  = xs
  , pInit = \e -> e
  , pStep = \_ s yield _skip done ->
      [|| case $$s of
            []       -> $$done
            (y : ys) -> $$(yield [|| () ||] [|| y ||] [|| ys ||]) ||]
  }

pairProd :: CodeQ [(d, r)] -> Producer d r
pairProd xs = Producer
  { pEnv  = xs
  , pInit = \e -> e
  , pStep = \_ s yield _skip done ->
      [|| case $$s of
            []            -> $$done
            ((k, y) : ys) -> $$(yield [|| k ||] [|| y ||] [|| ys ||]) ||]
  }

maybeProd :: CodeQ (Maybe a) -> Producer () a
maybeProd m = Producer
  { pEnv  = m
  , pInit = \e -> e
  , pStep = \_ s yield _skip done ->
      [|| case $$s of
            Nothing -> $$done
            Just y  -> $$(yield [|| () ||] [|| y ||] [|| Nothing ||]) ||]
  }
