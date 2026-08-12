{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Leaves and operators that work in either query position.
--
-- `SMode` has two instances: `Stream` for enumeration and `Lookup` for keyed
-- access. A query remains polymorphic until its result type selects an instance.
-- Binary operators take a concrete `Lookup` on the right because that side is
-- always accessed by key.
--
-- Keyed access carries a fused early-exit operation. This is what keeps a
-- membership test inside a scan as a straight-line predicate rather than a
-- one-row pull loop.
module Prela.PullStaged.Ops
  ( SMode (..)
    -- * Fixed-mode operators
  , groupBy
  , leftCompose
  , union
  , disj
  , resolveId
  , mapLookupKey
  ) where

import Data.Hashable (Hashable)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Vector.Unboxed as UV
import Language.Haskell.TH (CodeQ)

import Prela.Id
import qualified Prela.Id.Internal as IdInternal
import Prela.PullStaged.Stream.Internal
import Prela.Storage.Internal

-- Storage fields use leading underscores because consumers such as `count` may
-- discard values. In that case staging removes the read and leaves the field
-- binder unused in generated code.
class SMode q where
  -- Leaves.
  universe       :: CodeQ (Universe e) -> q (Id e) (Id e)
  column         :: Elem r => CodeQ (Col e r) -> q (Id e) r
  sparseColumn   :: CodeQ (SparseCol e r) -> q (Id e) r
  -- A declared one-valued reference is one physical leaf. Keeping its raw
  -- nullable index and target validation in the same generated branch avoids
  -- allocating @Maybe Int@ and @Maybe (Id target)@ for driven reads.
  referenceColumn
    :: CodeQ (Universe source)
    -> CodeQ (SparseCol source Int)
    -> CodeQ (Universe target)
    -> q (Id source) (Id target)
  referenceColumn sourceDomain raw targetDomain =
    compose (compose (universe sourceDomain) (sparseColumn raw))
            (resolveId targetDomain)
  multiColumn    :: Elem r => CodeQ (MultiCol e r) -> q (Id e) r
  fromIndex      :: Ord d => CodeQ (Map d [r]) -> q d r
  fromCache      :: Ord d => CodeQ (Map d s) -> q d s
  fromDense      :: UV.Unbox t => CodeQ (Dense e t) -> q (Id e) t
  fromDenseInt   :: UV.Unbox t => CodeQ (DenseInt t) -> q Int t
  -- Enumeration order is hash-table slot order and is therefore unspecified.
  fromTable      :: (Hashable d, Key d, UV.Unbox t) => CodeQ (Table d t) -> q d t
  fromBits       :: CodeQ (Bits e) -> q (Id e) (Id e)

  -- Chain two relations through a shared middle value: `r : d -> e` and
  -- `s : e -> f` give `d -> f`. Also field navigation.
  compose  :: q d e -> Lookup e f -> q d f

  -- Pair two relations sharing a DOMAIN: for each key, take both values.
  prod     :: q d u -> Lookup d v -> q d (u, v)

  -- Keep rows whose value has at least one match in the second relation.
  restrict :: q d r -> Lookup r e -> q d r

  -- Keep rows whose key has no match in the second relation.
  diff     :: q d r -> Lookup d e -> q d r

  filt     :: (CodeQ r -> CodeQ Bool) -> q d r -> q d r
  mapv     :: (CodeQ r -> CodeQ s) -> q d r -> q d s

-- | Does a keyed relation contain any value? The accepting predicate is kept
-- explicit in 'probeAny' so filters and compositions can fuse into the same
-- generated test instead of allocating an intermediate stream value.
memberAt :: Lookup d r -> CodeQ d -> CodeQ Bool
memberAt relation key = probeAny relation key (\_ -> [|| True ||])

-- | Compatibility construction for inherently list-backed lookups. Loaded
-- columns and executor caches provide direct probes below; a materialized list
-- still needs to walk its values, but does so only for operators that chose that
-- representation explicitly.
streamLookup :: (CodeQ d -> Stream () r) -> Lookup d r
streamLookup values = Lookup
  { at = values
  , probeAny = \key accept -> anyOf (filtS accept (values key))
  }

-- | Read and validate one stored reference without constructing an optional
-- intermediate. The boxed storage branch consumes its already-stored 'Maybe';
-- the trusted word-backed branch is a raw load followed by sentinel and
-- universe checks. 'Id' is a newtype, so the candidates erase after inlining.
withReference
  :: CodeQ (Universe source)
  -> CodeQ (SparseCol source Int)
  -> CodeQ (Universe target)
  -> CodeQ (Id source)
  -> (CodeQ (Id target) -> CodeQ result)
  -> CodeQ result
  -> CodeQ result
withReference sourceDomain rawColumn targetDomain sourceId found missing =
  [|| if containsId $$sourceDomain $$sourceId
      then
        let !sourceIndex = idIndex $$sourceId
        in withSparseIntAt $$rawColumn sourceIndex $$missing (\targetIndex ->
             let !targetId = IdInternal.Id targetIndex
             in if containsId $$targetDomain targetId
                  then $$(found [|| targetId ||])
                  else $$missing)
      else $$missing ||]

--------------------------------------------------------------------------------
-- Enumeration
--------------------------------------------------------------------------------

instance SMode Stream where
  universe u       = Lin (universeProd u)
  column c         = Lin (columnProd c)
  sparseColumn c = Lin Producer
    { source       = c
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
        [|| if $$s >= sparseColLen $$e then $$done
            else case sparseAt $$e $$s of
                   Nothing -> $$(skip [|| $$s + 1 ||])
                   Just value ->
                     case boundedId (sparseColLen $$e) $$s of
                       Just d  -> $$(yield [|| d ||] [|| value ||] [|| $$s + 1 ||])
                       Nothing -> $$(skip [|| $$s + 1 ||]) ||]
    }
  referenceColumn sourceDomain rawColumn targetDomain = Lin Producer
    { source = [|| ($$sourceDomain, $$rawColumn, $$targetDomain) ||]
    , initialState = \_ -> [|| 0 :: Int ||]
    , next = \environment state yield skip done ->
        [|| case $$environment of
              (sourceUniverse, references, targetUniverse)
                | $$state >= universeSize sourceUniverse
                    || $$state >= sparseColLen references -> $$done
                | otherwise ->
                    let !sourceId = IdInternal.Id $$state
                    in $$(withReference
                            [|| sourceUniverse ||]
                            [|| references ||]
                            [|| targetUniverse ||]
                            [|| sourceId ||]
                            (\targetId ->
                               yield [|| sourceId ||] targetId [|| $$state + 1 ||])
                            (skip [|| $$state + 1 ||])) ||]
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
                  | j < end   -> case boundedId n i of
                                   Just d  -> $$(yield [|| d ||] [|| atStore _st j ||]
                                                       [|| (i, j + 1, end) ||])
                                   Nothing -> $$(skip [|| (i, j + 1, end) ||])
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
                       then case boundedId n $$s of
                              Just d  -> $$(yield [|| d ||] [|| _vals UV.! $$s ||]
                                                  [|| $$s + 1 ||])
                              Nothing -> $$(skip [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromDenseInt c = Lin Producer
    { source       = c
    , initialState = \_ -> [|| 0 :: Int ||]
    , next         = \e s yield skip done ->
        [|| case $$e of
              DenseInt n _vals seen ->
                if $$s >= n then $$done
                else if atBit seen $$s
                       then $$(yield [|| $$s ||] [|| _vals UV.! $$s ||]
                                     [|| $$s + 1 ||])
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
                       then case boundedId (bitsLen bs) $$s of
                              Just d  -> $$(yield [|| d ||] [|| d ||] [|| $$s + 1 ||])
                              Nothing -> $$(skip [|| $$s + 1 ||])
                       else $$(skip [|| $$s + 1 ||]) ||]
    }
  fromIndex m =
    Bind (Lin (pairProd [|| Map.toList $$m ||]))
         (\d vs -> mapkS (\_ -> d) (Lin (listProd vs)))
  fromCache m = Lin (pairProd [|| Map.toList $$m ||])

  compose  a b = Bind a (\d e -> mapkS (\_ -> d) (at b e))
  -- Strict component bindings keep column reads from crossing the inner loop as
  -- boxed thunks. They work with the strict binding in `sfoldWhile`.
  prod     a b = Bind a (\d u -> mapkS (\_ -> d)
                                   (mapvS (\v -> [|| let !x = $$u
                                                         !y = $$v
                                                     in (x, y) ||]) (at b d)))
  restrict a b = filtKV (\_ r -> memberAt b r) a
  diff     a b = filtKV (\d _ -> [|| not $$(memberAt b d) ||]) a
  filt     t a = filtS t a
  mapv     f a = mapvS f a

--------------------------------------------------------------------------------
-- Keyed access
--------------------------------------------------------------------------------

-- Lookup leaves validate keys before reading storage. Invalid keys produce an
-- empty stream. Each leaf supplies both full enumeration and a fused
-- predicate probe; the latter is the hot path for restrict/difference.
instance SMode Lookup where
  universe u = Lookup
    { at = \x -> mapvS (\_ -> x) (guardS [|| containsId $$u $$x ||])
    , probeAny = \x accept ->
        [|| containsId $$u $$x && $$(accept x) ||]
    }
  column c = Lookup
    { at = \x -> Lin Producer
        { source       = c
        , initialState = \_ -> [|| True ||]
        , next         = \e s yield _skip done ->
            [|| case $$e of
                  Col n _st ->
                    let i = idIndex $$x
                    in if $$s && i < n
                         then $$(yield [|| () ||] [|| atStore _st i ||] [|| False ||])
                         else $$done ||]
        }
    , probeAny = \x accept ->
        [|| case $$c of
              Col n _st ->
                let i = idIndex $$x
                in i < n && $$(accept [|| atStore _st i ||]) ||]
    }
  sparseColumn c = Lookup
    { at = \x -> Lin Producer
        { source       = c
        , initialState = \_ -> [|| True ||]
        , next         = \e s yield _skip done ->
            [|| let i = idIndex $$x
                in if $$s && i < sparseColLen $$e
                     then case sparseAt $$e i of
                            Just value -> $$(yield [|| () ||] [|| value ||] [|| False ||])
                            Nothing    -> $$done
                     else $$done ||]
        }
    , probeAny = \x accept ->
        [|| let i = idIndex $$x
            in if i < sparseColLen $$c
                 then case sparseAt $$c i of
                        Just value -> $$(accept [|| value ||])
                        Nothing    -> False
                 else False ||]
    }
  referenceColumn sourceDomain rawColumn targetDomain = Lookup
    { at = \sourceId -> Lin Producer
        { source = [|| ($$sourceDomain, $$rawColumn, $$targetDomain) ||]
        , initialState = \_ -> [|| True ||]
        , next = \environment active yield _skip done ->
            [|| case $$environment of
                  (sourceUniverse, references, targetUniverse) ->
                    if $$active
                      then $$(withReference
                               [|| sourceUniverse ||]
                               [|| references ||]
                               [|| targetUniverse ||]
                               sourceId
                               (\targetId ->
                                  yield [|| () ||] targetId [|| False ||])
                               done)
                      else $$done ||]
        }
    , probeAny = \sourceId accept ->
        withReference sourceDomain rawColumn targetDomain sourceId accept [|| False ||]
    }
  -- The row's extent is read once, in `initialState`, rather than on every step. An
  -- out-of-range key gets the empty range (0, 0), which is the no-values answer.
  multiColumn c = Lookup
    { at = \x -> Lin Producer
        { source       = c
        , initialState = \e -> [|| case $$e of
                              MultiCol n offs _ ->
                                let i = idIndex $$x
                                in if i < n then (off32 offs i, off32 offs (i + 1))
                                            else (0, 0) ||]
        , next = \e s yield _skip done ->
            [|| case $$e of
                  MultiCol _ _ _st -> case $$s of
                    (j, end) | j >= end  -> $$done
                             | otherwise -> $$(yield [|| () ||] [|| atStore _st j ||]
                                                     [|| (j + 1, end) ||]) ||]
        }
    , probeAny = \x accept ->
        [|| case $$c of
              MultiCol n offs _st ->
                let i = idIndex $$x
                    go j end
                      | j >= end = False
                      | $$(accept [|| atStore _st j ||]) = True
                      | otherwise = go (j + 1) end
                in i < n && go (off32 offs i) (off32 offs (i + 1)) ||]
    }
  fromDense c = Lookup
    { at = \x -> Lin Producer
        { source       = c
        , initialState = \_ -> [|| True ||]
        , next         = \e s yield _skip done ->
            [|| case $$e of
                  Dense n _vals seen ->
                    let i = idIndex $$x
                    in if $$s && i < n && atBit seen i
                         then $$(yield [|| () ||] [|| _vals UV.! i ||] [|| False ||])
                         else $$done ||]
        }
    , probeAny = \x accept ->
        [|| case $$c of
              Dense n _vals seen ->
                let i = idIndex $$x
                in i < n && atBit seen i && $$(accept [|| _vals UV.! i ||]) ||]
    }
  fromDenseInt c = Lookup
    { at = \x -> Lin Producer
        { source       = c
        , initialState = \_ -> [|| True ||]
        , next         = \e s yield _skip done ->
            [|| case $$e of
                  DenseInt n _vals seen ->
                    if $$s && 0 <= $$x && $$x < n && atBit seen $$x
                      then $$(yield [|| () ||] [|| _vals UV.! $$x ||] [|| False ||])
                      else $$done ||]
        }
    , probeAny = \x accept ->
        [|| case $$c of
              DenseInt n _vals seen ->
                0 <= $$x && $$x < n && atBit seen $$x
                  && $$(accept [|| _vals UV.! $$x ||]) ||]
    }
  fromTable t = Lookup
    { at = \x -> Lin Producer
        { source       = t
        , initialState = \e -> [|| tableSlot $$e $$x ||]
        , next         = \e s yield _skip done ->
            [|| case $$e of
                  Table _ _ _ _vs
                    | $$s >= 0  -> $$(yield [|| () ||] [|| _vs UV.! $$s ||] [|| -1 ||])
                    | otherwise -> $$done ||]
        }
    , probeAny = \x accept ->
        [|| case $$t of
              Table _ _ _ _vs ->
                let slot = tableSlot $$t $$x
                in slot >= 0 && $$(accept [|| _vs UV.! slot ||]) ||]
    }
  fromBits b = Lookup
    { at = \x -> mapvS (\_ -> x) (guardS [|| bitsMember $$b $$x ||])
    , probeAny = \x accept -> [|| bitsMember $$b $$x && $$(accept x) ||]
    }
  fromIndex m = streamLookup (\x -> Lin (listProd [|| Map.findWithDefault [] $$x $$m ||]))
  fromCache m = Lookup
    { at = \x -> Lin (maybeProd [|| Map.lookup $$x $$m ||])
    , probeAny = \x accept ->
        [|| case Map.lookup $$x $$m of
              Nothing -> False
              Just value -> $$(accept [|| value ||]) ||]
    }

  compose a b = Lookup
    { at = \x -> Bind (at a x) (\_ e -> at b e)
    , probeAny = \x accept -> probeAny a x (\e -> probeAny b e accept)
    }
  prod a b = Lookup
    { at = \x -> Bind (at a x)
                  (\_ u -> mapvS (\v -> [|| let { !p = $$u; !q = $$v }
                                              in (p, q) ||]) (at b x))
    , probeAny = \x accept ->
        probeAny a x (\u -> probeAny b x (\v -> accept [|| ($$u, $$v) ||]))
    }
  restrict a b = Lookup
    { at = \x -> filtS (memberAt b) (at a x)
    , probeAny = \x accept ->
        probeAny a x (\r -> [|| $$(memberAt b r) && $$(accept r) ||])
    }
  -- The test is on the KEY, so it is decided once for the whole lookup rather
  -- than per returned row, which is what `whenS` is for.
  diff a b = Lookup
    { at = \x -> whenS [|| not $$(memberAt b x) ||] (at a x)
    , probeAny = \x accept ->
        [|| not $$(memberAt b x) && $$(probeAny a x accept) ||]
    }
  filt t a = Lookup
    { at = \x -> filtS t (at a x)
    , probeAny = \x accept ->
        probeAny a x (\r -> [|| $$(t r) && $$(accept r) ||])
    }
  mapv f a = Lookup
    { at = \x -> mapvS f (at a x)
    , probeAny = \x accept -> probeAny a x (accept . f)
    }

--------------------------------------------------------------------------------
-- Fixed-mode operators
--------------------------------------------------------------------------------

-- | Re-key a stream by something computed from each VALUE. What comes out is
-- (group key, value), which is exactly what a grouped fold wants, so
-- @withFold op ini (groupBy q key)@ is Prela's GROUP BY over a non-key column.
--
-- Enumeration only, and free: one extra keyed lookup per row with nothing
-- stored. Its second argument is looked up at the VALUE rather than the key,
-- which is what makes it different from `compose`.
--
groupBy :: Stream d r -> Lookup r k -> Stream k r
groupBy s key = Bind s (\_ x -> mapvS (\_ -> x) (byValue (at key x)))

-- | Left-compose, defined as @r <- s@ ≡ @r' -> s@: rekey the first relation by
-- its own value, then chain the second onto it. Enumeration only, since
-- `invStream` is.
leftCompose :: Stream d e -> Lookup d f -> Stream e f
leftCompose a b = compose (invStream a) b

-- | One relation and then the other. Enumeration only: keyed access to a union
-- would have to search both sides and de-duplicate, and what queries actually
-- want from OR is membership, which is `disj`.
union :: Stream d r -> Stream d r -> Stream d r
union = catS

-- | Membership union, the OR of two filters. Never enumerated: only whether a
-- key is present in either side is defined, so the value type is @()@. That is
-- not a placeholder, it is the constraint made visible — you cannot navigate
-- through an OR or read a value out of one, and the type says so.
--
-- The probe path is a direct short-circuiting OR. Full enumeration remains a
-- one-row unit stream because this relation deliberately exposes membership
-- only.
disj :: Lookup d u -> Lookup d v -> Lookup d ()
disj a b = Lookup
  { at = \x -> guardS [|| $$(memberAt a x) || $$(memberAt b x) ||]
  , probeAny = \x accept ->
      [|| ($$(memberAt a x) || $$(memberAt b x)) && $$(accept [|| () ||]) ||]
  }

-- | Resolve a stored non-negative index through its target entity universe.
-- Missing, out-of-range, and dead identifiers all produce no value.
resolveId :: CodeQ (Universe e) -> Lookup Int (Id e)
resolveId domain = Lookup
  { at = \index -> Lin (maybeProd [|| lookupId $$domain $$index ||])
  , probeAny = \index accept ->
      [|| case lookupId $$domain $$index of
            Nothing -> False
            Just identifier -> $$(accept [|| identifier ||]) ||]
  }

-- | Adapt a keyed relation by transforming the key supplied to it. This is
-- contravariant in the lookup key and does not enumerate or allocate anything.
mapLookupKey :: (CodeQ a -> CodeQ b) -> Lookup b r -> Lookup a r
mapLookupKey transform relation = Lookup
  { at = at relation . transform
  , probeAny = \key accept -> probeAny relation (transform key) accept
  }

--------------------------------------------------------------------------------
-- Producer-level leaves
--------------------------------------------------------------------------------

-- The two leaves worth zipping. Everything else reaches lockstep by being
-- materialized first, which is the honest cost of the linearity restriction.

universeProd :: CodeQ (Universe e) -> Producer (Id e) (Id e)
universeProd u = Producer
  { source       = u
  , initialState = \_ -> [|| 0 :: Int ||]
  , next         = \domain index yield skip done ->
      [|| if $$index >= universeSize $$domain
            then $$done
            else case lookupId $$domain $$index of
                   Just d  -> $$(yield [|| d ||] [|| d ||] [|| $$index + 1 ||])
                   Nothing -> $$(skip [|| $$index + 1 ||]) ||]
  }

columnProd :: Elem r => CodeQ (Col e r) -> Producer (Id e) r
columnProd c = Producer
  { source       = c
  , initialState = \_ -> [|| 0 :: Int ||]
  , next         = \sourceColumn index yield _skip done ->
      [|| case $$sourceColumn of
            Col size store
              | $$index >= size -> $$done
              | otherwise ->
                  case boundedId size $$index of
                    Just d  -> $$(yield [|| d ||] [|| atStore store $$index ||]
                                        [|| $$index + 1 ||])
                    Nothing -> $$done ||]
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
