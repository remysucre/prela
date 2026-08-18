{-# LANGUAGE BangPatterns #-}

-- | Unstaged relational leaves and operators.
--
-- The definitions mirror "Prela.PullStaged.Ops".  The difference is solely
-- that leaves read ordinary storage values and operators construct a runtime
-- 'Drive' or 'Probe'.
module Prela.Pull.Ops
  ( Mode (..)
  , groupBy
  , leftCompose
  , union
  , disj
  , resolveId
  , mapProbeKey
  ) where

import Data.Hashable (Hashable)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Vector.Unboxed as UV

import Prela.Id
import qualified Prela.Id.Internal as IdInternal
import Prela.Pull.Stream.Internal
import Prela.Storage.Internal

-- | Operations available in both whole-relation and keyed-access modes.
class Mode q where
  universe       :: Universe e -> q (Id e) (Id e)
  column         :: Elem r => Col e r -> q (Id e) r
  sparseColumn   :: SparseCol e r -> q (Id e) r
  referenceColumn
    :: Universe source
    -> SparseCol source Int
    -> Universe target
    -> q (Id source) (Id target)
  referenceColumn sourceDomain raw targetDomain =
    compose (compose (universe sourceDomain) (sparseColumn raw))
            (resolveId targetDomain)
  multiColumn    :: Elem r => MultiCol e r -> q (Id e) r
  fromIndex      :: Ord k => Map k [v] -> q k v
  fromCache      :: Ord k => Map k v -> q k v
  fromDense      :: UV.Unbox v => Dense e v -> q (Id e) v
  fromDenseInt   :: UV.Unbox v => DenseInt v -> q Int v
  fromTable      :: (Hashable k, Key k, UV.Unbox v) => Table k v -> q k v
  fromBits       :: Bits e -> q (Id e) (Id e)

  compose  :: q k middle -> Probe middle v -> q k v
  prod     :: q k left -> Probe k right -> q k (left, right)
  restrict :: q k v -> Probe v test -> q k v
  diff     :: q k v -> Probe k test -> q k v
  filt     :: (v -> Bool) -> q k v -> q k v
  mapv     :: (v -> w) -> q k v -> q k w

memberAt :: Probe k v -> k -> Bool
memberAt relation key = exists relation key (const True)
{-# INLINE memberAt #-}

driveProbe :: (k -> Drive () v) -> Probe k v
driveProbe values = Probe
  { at = values
  , exists = \key accept -> anyOf (filtD accept (values key))
  }
{-# INLINE driveProbe #-}

withReference
  :: Universe source
  -> SparseCol source Int
  -> Universe target
  -> Id source
  -> (Id target -> result)
  -> result
  -> result
withReference sourceDomain rawColumn targetDomain sourceId found missing =
  if containsId sourceDomain sourceId
    then
      let !sourceIndex = idIndex sourceId
      in withSparseIntAt rawColumn sourceIndex missing (\targetIndex ->
           let !targetId = IdInternal.Id targetIndex
           in if containsId targetDomain targetId
                then found targetId
                else missing)
    else missing
{-# INLINE withReference #-}

instance Mode Drive where
  universe domain = linear (universeStream domain)
  column values = linear (columnStream values)
  sparseColumn values = linear Stream
    { source = values
    , initialState = \_ -> 0 :: Int
    , next = \sourceColumn index yield skip done ->
        if index >= sparseColLen sourceColumn
          then done
          else case sparseAt sourceColumn index of
            Nothing -> skip (index + 1)
            Just value -> case boundedId (sparseColLen sourceColumn) index of
              Just key -> yield key value (index + 1)
              Nothing -> skip (index + 1)
    }
  referenceColumn sourceDomain rawColumn targetDomain = linear Stream
    { source = (sourceDomain, rawColumn, targetDomain)
    , initialState = \_ -> 0 :: Int
    , next = \(sourceUniverse, references, targetUniverse) index yield skip done ->
        if index >= universeSize sourceUniverse || index >= sparseColLen references
          then done
          else
            let !sourceId = IdInternal.Id index
            in withReference sourceUniverse references targetUniverse sourceId
                 (\targetId -> yield sourceId targetId (index + 1))
                 (skip (index + 1))
    }
  multiColumn values = linear Stream
    { source = values
    , initialState = \_ -> (-1 :: Int, 0 :: Int, 0 :: Int)
    , next = \sourceColumn state yield skip done -> case sourceColumn of
        MultiCol size offsets store -> case state of
          (key, cursor, rowEnd)
            | cursor < rowEnd -> case boundedId size key of
                Just domain -> yield domain (atStore store cursor)
                                     (key, cursor + 1, rowEnd)
                Nothing -> skip (key, cursor + 1, rowEnd)
            | key + 1 >= size -> done
            | otherwise ->
                let !key' = key + 1
                in skip (key', off32 offsets key', off32 offsets (key' + 1))
    }
  fromDense values = linear Stream
    { source = values
    , initialState = \_ -> 0 :: Int
    , next = \dense index yield skip done -> case dense of
        Dense size slots seen
          | index >= size -> done
          | atBit seen index -> case boundedId size index of
              Just key -> yield key (slots UV.! index) (index + 1)
              Nothing -> skip (index + 1)
          | otherwise -> skip (index + 1)
    }
  fromDenseInt values = linear Stream
    { source = values
    , initialState = \_ -> 0 :: Int
    , next = \dense index yield skip done -> case dense of
        DenseInt size slots seen
          | index >= size -> done
          | atBit seen index -> yield index (slots UV.! index) (index + 1)
          | otherwise -> skip (index + 1)
    }
  fromTable values = linear Stream
    { source = values
    , initialState = \_ -> 0 :: Int
    , next = \table index yield skip done -> case table of
        Table mask hashes keys slots
          | index > mask -> done
          | hashes UV.! index /= 0 ->
              yield (indexKey keys index) (slots UV.! index) (index + 1)
          | otherwise -> skip (index + 1)
    }
  fromBits values = linear Stream
    { source = values
    , initialState = \_ -> 0 :: Int
    , next = \bits index yield skip done -> case bits of
        Bits mask
          | index >= bitsLen mask -> done
          | atBit mask index -> case boundedId (bitsLen mask) index of
              Just key -> yield key key (index + 1)
              Nothing -> skip (index + 1)
          | otherwise -> skip (index + 1)
    }
  fromIndex values =
    bindD (linear (pairStream (Map.toList values)))
          (\key vs -> mapkD (const key) (linear (listStream vs)))
  fromCache values = linear (pairStream (Map.toList values))

  compose left right =
    bindD left (\key middle -> mapkD (const key) (at right middle))
  prod left right =
    bindD left (\key leftValue ->
      mapkD (const key) (mapvD (strictPair leftValue) (at right key)))
  restrict rows predicate = filtDKV (\_ value -> memberAt predicate value) rows
  diff rows excluded = filtDKV (\key _ -> not (memberAt excluded key)) rows
  filt = filtD
  mapv = mapvD
  {-# INLINE universe #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE referenceColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromDenseInt #-}
  {-# INLINE fromTable #-}
  {-# INLINE fromBits #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

strictPair :: left -> right -> (left, right)
strictPair left right =
  let !x = left
      !y = right
  in (x, y)
{-# INLINE strictPair #-}

instance Mode Probe where
  universe domain = Probe
    { at = \key -> mapvD (const key) (guardD (containsId domain key))
    , exists = \key accept -> containsId domain key && accept key
    }
  column values = Probe
    { at = \key -> linear Stream
        { source = values
        , initialState = \_ -> True
        , next = \sourceColumn active yield _skip done -> case sourceColumn of
            Col size store ->
              let index = idIndex key
              in if active && index < size
                   then yield () (atStore store index) False
                   else done
        }
    , exists = \key accept -> case values of
        Col size store ->
          let index = idIndex key
          in index < size && accept (atStore store index)
    }
  sparseColumn values = Probe
    { at = \key -> linear Stream
        { source = values
        , initialState = \_ -> True
        , next = \sourceColumn active yield _skip done ->
            let index = idIndex key
            in if active && index < sparseColLen sourceColumn
                 then case sparseAt sourceColumn index of
                   Just value -> yield () value False
                   Nothing -> done
                 else done
        }
    , exists = \key accept ->
        let index = idIndex key
        in if index < sparseColLen values
             then maybe False accept (sparseAt values index)
             else False
    }
  referenceColumn sourceDomain rawColumn targetDomain = Probe
    { at = \sourceId -> linear Stream
        { source = (sourceDomain, rawColumn, targetDomain)
        , initialState = \_ -> True
        , next = \environment active yield _skip done ->
            case environment of
              (sourceUniverse, references, targetUniverse) ->
                if active
                  then withReference
                         sourceUniverse references targetUniverse sourceId
                         (\targetId -> yield () targetId False)
                         done
                  else done
        }
    , exists = \sourceId accept ->
        withReference sourceDomain rawColumn targetDomain sourceId accept False
    }
  multiColumn values = Probe
    { at = \key -> linear Stream
        { source = values
        , initialState = \sourceColumn -> case sourceColumn of
            MultiCol size offsets _ ->
              let index = idIndex key
              in if index < size
                   then (off32 offsets index, off32 offsets (index + 1))
                   else (0, 0)
        , next = \sourceColumn (cursor, rowEnd) yield _skip done ->
            case sourceColumn of
              MultiCol _ _ store
                | cursor >= rowEnd -> done
                | otherwise ->
                    yield () (atStore store cursor) (cursor + 1, rowEnd)
        }
    , exists = \key accept -> case values of
        MultiCol size offsets store ->
          let index = idIndex key
              go cursor rowEnd
                | cursor >= rowEnd = False
                | accept (atStore store cursor) = True
                | otherwise = go (cursor + 1) rowEnd
          in index < size && go (off32 offsets index) (off32 offsets (index + 1))
    }
  fromDense values = Probe
    { at = \key -> linear Stream
        { source = values
        , initialState = \_ -> True
        , next = \dense active yield _skip done -> case dense of
            Dense size slots seen ->
              let index = idIndex key
              in if active && index < size && atBit seen index
                   then yield () (slots UV.! index) False
                   else done
        }
    , exists = \key accept -> case values of
        Dense size slots seen ->
          let index = idIndex key
          in index < size && atBit seen index && accept (slots UV.! index)
    }
  fromDenseInt values = Probe
    { at = \key -> linear Stream
        { source = values
        , initialState = \_ -> True
        , next = \dense active yield _skip done -> case dense of
            DenseInt size slots seen ->
              if active && 0 <= key && key < size && atBit seen key
                then yield () (slots UV.! key) False
                else done
        }
    , exists = \key accept -> case values of
        DenseInt size slots seen ->
          0 <= key && key < size && atBit seen key && accept (slots UV.! key)
    }
  fromTable values = Probe
    { at = \key -> linear Stream
        { source = values
        , initialState = \table -> tableSlot table key
        , next = \table slot yield _skip done -> case table of
            Table _ _ _ slots
              | slot >= 0 -> yield () (slots UV.! slot) (-1)
              | otherwise -> done
        }
    , exists = \key accept -> case values of
        Table _ _ _ slots ->
          let slot = tableSlot values key
          in slot >= 0 && accept (slots UV.! slot)
    }
  fromBits values = Probe
    { at = \key -> mapvD (const key) (guardD (bitsMember values key))
    , exists = \key accept -> bitsMember values key && accept key
    }
  fromIndex values =
    driveProbe (\key -> linear (listStream (Map.findWithDefault [] key values)))
  fromCache values = Probe
    { at = \key -> linear (maybeStream (Map.lookup key values))
    , exists = \key accept -> maybe False accept (Map.lookup key values)
    }

  compose left right = Probe
    { at = \key -> bindD (at left key) (\_ middle -> at right middle)
    , exists = \key accept -> exists left key (\middle -> exists right middle accept)
    }
  prod left right = Probe
    { at = \key -> bindD (at left key) (\_ leftValue ->
        mapvD (strictPair leftValue) (at right key))
    , exists = \key accept ->
        exists left key (\leftValue ->
          exists right key (\rightValue -> accept (leftValue, rightValue)))
    }
  restrict rows predicate = Probe
    { at = \key -> filtD (memberAt predicate) (at rows key)
    , exists = \key accept ->
        exists rows key (\value -> memberAt predicate value && accept value)
    }
  diff rows excluded = Probe
    { at = \key -> whenD (not (memberAt excluded key)) (at rows key)
    , exists = \key accept ->
        not (memberAt excluded key) && exists rows key accept
    }
  filt predicate rows = Probe
    { at = \key -> filtD predicate (at rows key)
    , exists = \key accept ->
        exists rows key (\value -> predicate value && accept value)
    }
  mapv transform rows = Probe
    { at = \key -> mapvD transform (at rows key)
    , exists = \key accept -> exists rows key (accept . transform)
    }
  {-# INLINE universe #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE referenceColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromDenseInt #-}
  {-# INLINE fromTable #-}
  {-# INLINE fromBits #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

groupBy :: Drive k v -> Probe v group -> Drive group v
groupBy rows key =
  bindD rows (\_ value ->
    mapvD (const value) (mapkVD (\_ group -> group) (at key value)))
{-# INLINE groupBy #-}

leftCompose :: Drive k middle -> Probe k v -> Drive middle v
leftCompose left right = compose (invDrive left) right

union :: Drive k v -> Drive k v -> Drive k v
union = catD

disj :: Probe k left -> Probe k right -> Probe k ()
disj left right = Probe
  { at = \key -> guardD (memberAt left key || memberAt right key)
  , exists = \key accept ->
      (memberAt left key || memberAt right key) && accept ()
  }
{-# INLINE disj #-}

resolveId :: Universe e -> Probe Int (Id e)
resolveId domain = Probe
  { at = \index -> linear (maybeStream (lookupId domain index))
  , exists = \index accept -> maybe False accept (lookupId domain index)
  }
{-# INLINE resolveId #-}

mapProbeKey :: (a -> b) -> Probe b v -> Probe a v
mapProbeKey transform relation = Probe
  { at = at relation . transform
  , exists = \key accept -> exists relation (transform key) accept
  }
{-# INLINE mapProbeKey #-}

maybeStream :: Maybe v -> Stream () v
maybeStream value = Stream
  { source = value
  , initialState = id
  , next = \_ remaining yield _skip done -> case remaining of
      Nothing -> done
      Just result -> yield () result Nothing
  }
{-# INLINE maybeStream #-}

universeStream :: Universe e -> Stream (Id e) (Id e)
universeStream domain = Stream
  { source = domain
  , initialState = \_ -> 0 :: Int
  , next = \sourceDomain index yield skip done ->
      if index >= universeSize sourceDomain
        then done
        else case lookupId sourceDomain index of
          Just key -> yield key key (index + 1)
          Nothing -> skip (index + 1)
  }

columnStream :: Elem v => Col e v -> Stream (Id e) v
columnStream values = Stream
  { source = values
  , initialState = \_ -> 0 :: Int
  , next = \sourceColumn index yield _skip done -> case sourceColumn of
      Col size store
        | index >= size -> done
        | otherwise -> case boundedId size index of
            Just key -> yield key (atStore store index) (index + 1)
            Nothing -> done
  }

listStream :: [v] -> Stream () v
listStream values = Stream
  { source = values
  , initialState = id
  , next = \_ remaining yield _skip done -> case remaining of
      [] -> done
      (value : rest) -> yield () value rest
  }

pairStream :: [(k, v)] -> Stream k v
pairStream values = Stream
  { source = values
  , initialState = id
  , next = \_ remaining yield _skip done -> case remaining of
      [] -> done
      ((key, value) : rest) -> yield key value rest
  }
