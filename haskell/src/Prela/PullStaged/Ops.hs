{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Leaves and operators that work in either query position.
--
-- `SMode` has two instances: `Drive` for enumeration and `Probe` for keyed
-- access. A query remains polymorphic until its result type selects an instance.
-- Binary operators take a concrete `Probe` on the right because that side is
-- always accessed by key.
--
-- Pull consumers handle early termination, so these instances do not need a
-- second implementation corresponding to the push engine's `probeAny`.
module Prela.PullStaged.Ops
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

import Prela.PullStaged.Stream
import Prela.Storage

-- Storage fields use leading underscores because consumers such as `count` may
-- discard values. In that case staging removes the read and leaves the field
-- binder unused in generated code.
class SMode q where
  -- Leaves.
  universe       :: CodeQ Int -> q (Id e) (Id e)
  -- Driving skips holes. Probing only checks the id range because probed ids
  -- come from live rows.
  sparseUniverse :: CodeQ (Bits e) -> CodeQ Int -> q (Id e) (Id e)
  column         :: Elem r => CodeQ (Col e r) -> q (Id e) r
  sparseColumn   :: Elem r => CodeQ (SparseCol e r) -> q (Id e) r
  multiColumn    :: Elem r => CodeQ (MultiCol e r) -> q (Id e) r
  fromIndex      :: Ord d => CodeQ (Map d [r]) -> q d r
  fromCache      :: Ord d => CodeQ (Map d s) -> q d s
  fromDense      :: UV.Unbox t => CodeQ (Dense e t) -> q (Id e) t
  -- Drive order is hash-table slot order and is therefore unspecified.
  fromTable      :: (Hashable d, Key d, UV.Unbox t) => CodeQ (Table d t) -> q d t
  fromBits       :: CodeQ (Bits e) -> q (Id e) (Id e)

  -- Chain two relations through a shared middle value: `r : d -> e` and
  -- `s : e -> f` give `d -> f`. Also field navigation.
  compose  :: q d e -> Probe e f -> q d f

  -- Pair two relations sharing a DOMAIN: for each key, take both values.
  prod     :: q d u -> Probe d v -> q d (u, v)

  -- Keep rows whose value has at least one match in the second relation.
  restrict :: q d r -> Probe r e -> q d r

  -- Keep rows whose key has no match in the second relation.
  diff     :: q d r -> Probe d e -> q d r

  filt     :: (CodeQ r -> CodeQ Bool) -> q d r -> q d r
  mapv     :: (CodeQ r -> CodeQ s) -> q d r -> q d s

--------------------------------------------------------------------------------
-- Driven
--------------------------------------------------------------------------------

instance SMode Drive where
  universe n       = Lin (universeProd n)
  column c         = Lin (columnProd c)
  -- The mask determines the iteration extent.
  sparseUniverse b _ = Lin Producer
    { source       = b
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
        [|| case $$e of
              Bits bs ->
                if $$s >= bitsLen bs then $$done
                else if atBit bs $$s
                       then $$(yield [|| Id $$s ||] [|| Id $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  sparseColumn c = Lin Producer
    { source       = c
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
        [|| case $$e of
              SparseCol n _st pres ->
                if $$s >= n then $$done
                else if atBit pres $$s
                       then $$(yield [|| Id $$s ||] [|| atStore _st $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  -- Keep this flat so it remains zippable. State is @(key, cursor, rowEnd)@;
  -- `skip` advances to the next row.
  multiColumn c = Lin Producer
    { source       = c
    , initialState = \_ -> [|| (-1 :: Int, 0 :: Int, 0 :: Int) ||]
    , next         = \e s yield skip done ->
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
    { source       = c
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
        [|| case $$e of
              Dense n _vals seen ->
                if $$s >= n then $$done
                else if atBit seen $$s
                       then $$(yield [|| Id $$s ||] [|| _vals UV.! $$s ||] [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromTable t = Lin Producer
    { source       = t
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
        [|| case $$e of
              Table mask hs _ks _vs ->
                if $$s > mask then $$done
                else if hs UV.! $$s /= 0
                       then $$(yield [|| indexKey _ks $$s ||] [|| _vs UV.! $$s ||]
                                     [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromBits b = Lin Producer
    { source       = b
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
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
  -- Strict component bindings keep column reads from crossing the inner loop as
  -- boxed thunks. They work with the strict binding in `sfoldWhile`; see
  -- `design/StagedPullMain.hs`.
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

-- Probed leaves range-check keys because foreign-key holes use @noId = -1@.
-- Invalid keys produce an empty stream. Storage access itself remains unchecked.
instance SMode Probe where
  universe n = Probe $ \x -> mapvS (\_ -> x) (guardS [|| idInRange $$n $$x ||])
  sparseUniverse _ n = Probe $ \x -> mapvS (\_ -> x) (guardS [|| idInRange $$n $$x ||])
  column c = Probe $ \x -> Lin Producer
    { source       = c
    , initialState = \_ -> [|| True ||]
    , next         = \e s yield _skip done ->
        [|| case $$e of
              Col n _st -> case $$x of
                Id i | $$s && 0 <= i && i < n ->
                         $$(yield [|| () ||] [|| atStore _st i ||] [|| False ||])
                     | otherwise -> $$done ||]
    }
  sparseColumn c = Probe $ \x -> Lin Producer
    { source       = c
    , initialState = \_ -> [|| True ||]
    , next         = \e s yield _skip done ->
        [|| case $$e of
              SparseCol n _st pres -> case $$x of
                Id i | $$s && 0 <= i && i < n && atBit pres i ->
                         $$(yield [|| () ||] [|| atStore _st i ||] [|| False ||])
                     | otherwise -> $$done ||]
    }
  -- The row's extent is read once, in `initialState`, rather than on every step. An
  -- out-of-range key gets the empty range (0, 0), which is the no-values answer.
  multiColumn c = Probe $ \x -> Lin Producer
    { source       = c
    , initialState = \e -> [|| case $$e of
                          MultiCol n offs _ -> case $$x of
                            Id i | 0 <= i && i < n -> (off32 offs i, off32 offs (i + 1))
                                 | otherwise       -> (0, 0) ||]
    , next = \e s yield _skip done ->
        [|| case $$e of
              MultiCol _ _ _st -> case $$s of
                (j, end) | j >= end  -> $$done
                         | otherwise -> $$(yield [|| () ||] [|| atStore _st j ||]
                                                 [|| (j + 1, end) ||]) ||]
    }
  fromDense c = Probe $ \x -> Lin Producer
    { source       = c
    , initialState = \_ -> [|| True ||]
    , next         = \e s yield _skip done ->
        [|| case $$e of
              Dense n _vals seen -> case $$x of
                Id i | $$s && 0 <= i && i < n && atBit seen i ->
                         $$(yield [|| () ||] [|| _vals UV.! i ||] [|| False ||])
                     | otherwise -> $$done ||]
    }
  fromTable t = Probe $ \x -> Lin Producer
    { source       = t
    , initialState = \e -> [|| tableSlot $$e $$x ||]
    , next         = \e s yield _skip done ->
        [|| case $$e of
              Table _ _ _ _vs
                | $$s >= 0  -> $$(yield [|| () ||] [|| _vs UV.! $$s ||] [|| -1 ||])
                | otherwise -> $$done ||]
    }
  fromBits b = Probe $ \x -> mapvS (\_ -> x) (guardS [|| bitsMember $$b $$x ||])
  fromIndex m = Probe $ \x -> Lin (listProd [|| Map.findWithDefault [] $$x $$m ||])
  fromCache m = Probe $ \x -> Lin (maybeProd [|| Map.lookup $$x $$m ||])

  compose  a b = Probe $ \x -> Bind (at a x) (\_ e -> at b e)
  prod     a b = Probe $ \x ->
                   Bind (at a x)
                        (\_ u -> mapvS (\v -> [|| let !p = $$u
                                                      !q = $$v
                                                  in (p, q) ||]) (at b x))
  restrict a b = Probe $ \x -> filtS (\r -> anyOf (at b r)) (at a x)
  -- The test is on the KEY, so it is decided once for the whole probe rather
  -- than per row, which is what `whenS` is for.
  diff     a b = Probe $ \x -> whenS [|| not $$(anyOf (at b x)) ||] (at a x)
  filt     t a = Probe $ \x -> filtS t (at a x)
  mapv     f a = Probe $ \x -> mapvS f (at a x)

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
groupBy :: Drive d r -> Probe r k -> Drive k r
groupBy s key = Bind s (\_ x -> mapvS (\_ -> x) (byValue (at key x)))

-- | Left-compose, defined as @r <- s@ ≡ @r' -> s@: rekey the first relation by
-- its own value, then chain the second onto it. Driven only, since `invStream`
-- is.
leftCompose :: Drive d e -> Probe d f -> Drive e f
leftCompose a b = compose (invStream a) b

-- | One relation and then the other. Driven only: probing a union would have to
-- probe both sides and de-duplicate, and what queries actually want from OR is
-- membership, which is `disj`.
union :: Drive d r -> Drive d r -> Drive d r
union = catS

-- | Membership union, the OR of two filters. Never enumerated: only whether a
-- key is present in either side is defined, so the value type is @()@. That is
-- not a placeholder, it is the constraint made visible — you cannot navigate
-- through an OR or read a value out of one, and the type says so.
--
-- Under push this needed its own `Prb` record with both fields written out. Here
-- it is two `anyOf`s and a `guardS`, and the short-circuit is `anyOf`'s.
disj :: Probe d u -> Probe d v -> Probe d ()
disj a b = Probe $ \x -> guardS [|| $$(anyOf (at a x)) || $$(anyOf (at b x)) ||]

--------------------------------------------------------------------------------
-- Producer-level leaves
--------------------------------------------------------------------------------

-- The two leaves worth zipping. Everything else reaches lockstep by being
-- materialized first, which is the honest cost of the linearity restriction.

universeProd :: CodeQ Int -> Producer (Id e) (Id e)
universeProd n = indexedProd n
  (\size -> size)
  (\_ i -> [|| Id $$i ||])
  (\_ i -> [|| Id $$i ||])

columnProd :: Elem r => CodeQ (Col e r) -> Producer (Id e) r
columnProd c = Producer
  { source       = c
  , initialState = \_ -> [|| 0 :: Int ||]
  , next         = \sourceColumn index yield _skip done ->
      [|| case $$sourceColumn of
            Col size store
              | $$index >= size -> $$done
              | otherwise ->
                  $$(yield [|| Id $$index ||] [|| atStore store $$index ||]
                           [|| $$index + 1 ||]) ||]
  }

-- A flat producer whose state is an index in @[0, size)@.
indexedProd :: CodeQ source
            -> (CodeQ source -> CodeQ Int)
            -> (CodeQ source -> CodeQ Int -> CodeQ key)
            -> (CodeQ source -> CodeQ Int -> CodeQ value)
            -> Producer key value
indexedProd src size keyAt valueAt = Producer
  { source       = src
  , initialState = \_ -> [|| 0 :: Int ||]
  , next         = \bound index yield _skip done ->
      [|| if $$index >= $$(size bound)
            then $$done
            else $$(yield (keyAt bound index) (valueAt bound index)
                          [|| $$index + 1 ||]) ||]
  }

--------------------------------------------------------------------------------
-- List-backed producers
--------------------------------------------------------------------------------

-- The `Map`-backed leaves are the one place the engine is not flat, and they are
-- the reason "Prela.PullStaged.Materialize" prefers `withFold` over `withIndex`.
-- Nothing here can fuse into an array read, because there is no array.

listProd :: CodeQ [a] -> Producer () a
listProd xs = Producer
  { source       = xs
  , initialState = \e -> e
  , next         = \_ s yield _skip done ->
      [|| case $$s of
            []       -> $$done
            (y : ys) -> $$(yield [|| () ||] [|| y ||] [|| ys ||]) ||]
  }

pairProd :: CodeQ [(d, r)] -> Producer d r
pairProd xs = Producer
  { source       = xs
  , initialState = \e -> e
  , next         = \_ s yield _skip done ->
      [|| case $$s of
            []            -> $$done
            ((k, y) : ys) -> $$(yield [|| k ||] [|| y ||] [|| ys ||]) ||]
  }

maybeProd :: CodeQ (Maybe a) -> Producer () a
maybeProd m = Producer
  { source       = m
  , initialState = \e -> e
  , next         = \_ s yield _skip done ->
      [|| case $$s of
            Nothing -> $$done
            Just y  -> $$(yield [|| () ||] [|| y ||] [|| Nothing ||]) ||]
  }
