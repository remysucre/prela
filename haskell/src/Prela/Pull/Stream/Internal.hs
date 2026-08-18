{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE RankNTypes #-}

-- | Unstaged pull streams.
--
-- This is deliberately the runtime analogue of
-- "Prela.PullStaged.Stream.Internal".  The representation and interpreters
-- have the same shape; ordinary values replace 'CodeQ' expressions.
module Prela.Pull.Stream.Internal
  ( Stream (..)
  , Drive (..)
  , Probe (..)
  , mapvD
  , mapkD
  , mapkVD
  , filtD
  , filtDKV
  , invDrive
  , byValue
  , whenD
  , guardD
  , catD
  , linear
  , bindD
  , dfoldWhile
  , dfold
  , foldAll
  , count
  , anyOf
  , collect
  , limit
  , fromList
  , mapvS
  , filtS
  ) where

-- | A flat pull stream which advances by at most one row.
--
-- As in the staged engine, the three outcomes are represented by
-- continuations instead of allocating a @Step@ value.
data Stream k v = forall st src. Stream
  { source       :: !src
  , initialState :: src -> st
  , next         :: forall a. src
          -> st
          -> (k -> v -> st -> a)
          -> (st -> a)
          -> a
          -> a
  }

-- | A compositional traversal interpreted at run time.
data Drive k v
  = Lin (Stream k v)
  | forall outerKey outerValue. Bind
      (Drive outerKey outerValue)
      (outerKey -> outerValue -> Drive k v)
  | Cat (Drive k v) (Drive k v)

-- | Keyed access to a relation.
data Probe k v = Probe
  { at     :: k -> Drive () v
  , exists :: k -> (v -> Bool) -> Bool
  }

linear :: Stream k v -> Drive k v
linear = Lin
{-# INLINE [0] linear #-}

bindD :: Drive outerKey outerValue
      -> (outerKey -> outerValue -> Drive key value)
      -> Drive key value
bindD = Bind
{-# INLINE [0] bindD #-}

mapvD :: (v -> w) -> Drive k v -> Drive k w
mapvD transform (Lin stream) = linear (mapvS transform stream)
mapvD transform (Bind outer inner) =
  bindD outer (\key value -> mapvD transform (inner key value))
mapvD transform (Cat left right) =
  catD (mapvD transform left) (mapvD transform right)
{-# INLINE [0] mapvD #-}

mapkD :: (k -> j) -> Drive k v -> Drive j v
mapkD transform (Lin stream) = linear (mapkS transform stream)
mapkD transform (Bind outer inner) =
  bindD outer (\key value -> mapkD transform (inner key value))
mapkD transform (Cat left right) =
  catD (mapkD transform left) (mapkD transform right)
{-# INLINE [0] mapkD #-}

mapkVD :: (k -> v -> j) -> Drive k v -> Drive j v
mapkVD transform (Lin stream) = linear (mapkVS transform stream)
mapkVD transform (Bind outer inner) =
  bindD outer (\key value -> mapkVD transform (inner key value))
mapkVD transform (Cat left right) =
  catD (mapkVD transform left) (mapkVD transform right)
{-# INLINE [0] mapkVD #-}

byValue :: Drive k v -> Drive v v
byValue (Lin stream) = linear (byValueS stream)
byValue (Bind outer inner) =
  bindD outer (\key value -> byValue (inner key value))
byValue (Cat left right) = catD (byValue left) (byValue right)
{-# INLINE [0] byValue #-}

invDrive :: Drive k v -> Drive v k
invDrive (Lin stream) = linear (swapS stream)
invDrive (Bind outer inner) =
  bindD outer (\key value -> invDrive (inner key value))
invDrive (Cat left right) = catD (invDrive left) (invDrive right)
{-# INLINE [0] invDrive #-}

filtD :: (v -> Bool) -> Drive k v -> Drive k v
filtD predicate = filtDKV (\_ value -> predicate value)
{-# INLINE filtD #-}

filtDKV :: (k -> v -> Bool) -> Drive k v -> Drive k v
filtDKV predicate (Lin stream) = linear (filtSKV predicate stream)
filtDKV predicate (Bind outer inner) =
  bindD outer (\key value -> filtDKV predicate (inner key value))
filtDKV predicate (Cat left right) =
  catD (filtDKV predicate left) (filtDKV predicate right)
{-# INLINE [0] filtDKV #-}

guardD :: Bool -> Drive () ()
guardD condition = linear Stream
  { source = ()
  , initialState = \_ -> True
  , next = \_ active yield _skip done ->
      if active && condition then yield () () False else done
  }
{-# INLINE [0] guardD #-}

whenD :: Bool -> Drive k v -> Drive k v
whenD condition rows = bindD (guardD condition) (\_ _ -> rows)
{-# INLINE [0] whenD #-}

catD :: Drive k v -> Drive k v -> Drive k v
catD = Cat
{-# INLINE [0] catD #-}

mapvS :: (v -> w) -> Stream k v -> Stream k w
mapvS f (Stream environment initial step) = Stream
  { source = environment
  , initialState = initial
  , next = \env state yield skip done ->
      step env state (\key value state' -> yield key (f value) state') skip done
  }
{-# INLINE mapvS #-}

mapkS :: (k -> j) -> Stream k v -> Stream j v
mapkS f (Stream environment initial step) = Stream
  { source = environment
  , initialState = initial
  , next = \env state yield skip done ->
      step env state (\key value state' -> yield (f key) value state') skip done
  }
{-# INLINE mapkS #-}

mapkVS :: (k -> v -> j) -> Stream k v -> Stream j v
mapkVS f (Stream environment initial step) = Stream
  { source = environment
  , initialState = initial
  , next = \env state yield skip done ->
      step env state
        (\key value state' ->
          let !value' = value
          in yield (f key value') value' state')
        skip done
  }
{-# INLINE mapkVS #-}

byValueS :: Stream k v -> Stream v v
byValueS (Stream environment initial step) = Stream
  { source = environment
  , initialState = initial
  , next = \env state yield skip done ->
      step env state
        (\_ value state' ->
          let !value' = value
          in yield value' value' state')
        skip done
  }
{-# INLINE byValueS #-}

swapS :: Stream k v -> Stream v k
swapS (Stream environment initial step) = Stream
  { source = environment
  , initialState = initial
  , next = \env state yield skip done ->
      step env state (\key value state' -> yield value key state') skip done
  }
{-# INLINE swapS #-}

filtS :: (v -> Bool) -> Stream k v -> Stream k v
filtS predicate = filtSKV (\_ value -> predicate value)
{-# INLINE filtS #-}

filtSKV :: (k -> v -> Bool) -> Stream k v -> Stream k v
filtSKV predicate (Stream environment initial step) = Stream
  { source = environment
  , initialState = initial
  , next = \env state yield skip done ->
      step env state
        (\key value state' ->
          let !value' = value
          in if predicate key value'
               then yield key value' state'
               else skip state')
        skip done
  }
{-# INLINE filtSKV #-}

-- | Interpret a drive while the accumulator satisfies a condition.
dfoldWhile :: (acc -> Bool)
           -> (acc -> k -> v -> acc)
           -> acc
           -> Drive k v
           -> acc
dfoldWhile predicate consume initial (Lin stream) =
  foldWhileS predicate consume initial stream
dfoldWhile predicate consume initial (Bind outer inner) =
  dfoldWhile predicate (foldBind predicate consume inner) initial outer
dfoldWhile predicate consume initial (Cat left right) =
  dfoldWhile predicate consume
    (dfoldWhile predicate consume initial left)
    right
{-# INLINE [0] dfoldWhile #-}

foldBind
  :: (acc -> Bool)
  -> (acc -> k -> v -> acc)
  -> (outerKey -> outerValue -> Drive k v)
  -> acc
  -> outerKey
  -> outerValue
  -> acc
foldBind predicate consume inner acc key value =
  let !value' = value
  in dfoldWhile predicate consume acc (inner key value')
-- Unfold one phase before the constructors so fusion can continue inside the
-- dynamically selected inner drive of a 'Bind'.
{-# INLINE [1] foldBind #-}

foldWhileS :: (acc -> Bool)
           -> (acc -> k -> v -> acc)
           -> acc
           -> Stream k v
           -> acc
foldWhileS predicate consume initial (Stream environment start advance) =
  go (start environment) initial
  where
    go !state !acc
      | not (predicate acc) = acc
      | otherwise =
          advance environment state
            (\key value state' -> go state' (consume acc key value))
            (\state' -> go state' acc)
            acc
{-# INLINE [0] foldWhileS #-}

mapvFold :: (v -> w) -> (acc -> k -> w -> acc) -> acc -> k -> v -> acc
mapvFold transform consume acc key value = consume acc key (transform value)
{-# INLINE [0] mapvFold #-}

mapkFold :: (k -> j) -> (acc -> j -> v -> acc) -> acc -> k -> v -> acc
mapkFold transform consume acc key value = consume acc (transform key) value
{-# INLINE [0] mapkFold #-}

mapkVFold
  :: (k -> v -> j)
  -> (acc -> j -> v -> acc)
  -> acc
  -> k
  -> v
  -> acc
mapkVFold transform consume acc key value =
  let !value' = value
  in consume acc (transform key value') value'
{-# INLINE [0] mapkVFold #-}

byValueFold :: (acc -> v -> v -> acc) -> acc -> k -> v -> acc
byValueFold consume acc _ value =
  let !value' = value
  in consume acc value' value'
{-# INLINE [0] byValueFold #-}

invFold :: (acc -> v -> k -> acc) -> acc -> k -> v -> acc
invFold consume acc key value = consume acc value key
{-# INLINE [0] invFold #-}

filterFold
  :: (k -> v -> Bool)
  -> (acc -> k -> v -> acc)
  -> acc
  -> k
  -> v
  -> acc
filterFold keep consume acc key value =
  let !value' = value
  in if keep key value'
       then consume acc key value'
       else acc
{-# INLINE [0] filterFold #-}

-- These rules are the unstaged analogue of stream fusion.  Constructors and
-- transformations are kept opaque until phase 0, giving a consumer the chance
-- to rewrite them directly into one fold first.
{-# RULES
"dfoldWhile/linear" [~0]
  forall predicate consume initial stream.
    dfoldWhile predicate consume initial (linear stream) =
      foldWhileS predicate consume initial stream
"dfoldWhile/bindD" [~0]
  forall predicate consume initial outer inner.
    dfoldWhile predicate consume initial (bindD outer inner) =
      dfoldWhile predicate (foldBind predicate consume inner) initial outer
"dfoldWhile/catD" [~0]
  forall predicate consume initial left right.
    dfoldWhile predicate consume initial (catD left right) =
      dfoldWhile predicate consume
        (dfoldWhile predicate consume initial left)
        right
"dfoldWhile/mapvD" [~0]
  forall predicate consume initial transform rows.
    dfoldWhile predicate consume initial (mapvD transform rows) =
      dfoldWhile predicate (mapvFold transform consume) initial rows
"dfoldWhile/mapkD" [~0]
  forall predicate consume initial transform rows.
    dfoldWhile predicate consume initial (mapkD transform rows) =
      dfoldWhile predicate (mapkFold transform consume) initial rows
"dfoldWhile/mapkVD" [~0]
  forall predicate consume initial transform rows.
    dfoldWhile predicate consume initial (mapkVD transform rows) =
      dfoldWhile predicate (mapkVFold transform consume) initial rows
"dfoldWhile/byValue" [~0]
  forall predicate consume initial rows.
    dfoldWhile predicate consume initial (byValue rows) =
      dfoldWhile predicate (byValueFold consume) initial rows
"dfoldWhile/invDrive" [~0]
  forall predicate consume initial rows.
    dfoldWhile predicate consume initial (invDrive rows) =
      dfoldWhile predicate (invFold consume) initial rows
"dfoldWhile/filtDKV" [~0]
  forall predicate consume initial keep rows.
    dfoldWhile predicate consume initial (filtDKV keep rows) =
      dfoldWhile predicate (filterFold keep consume) initial rows
  #-}

dfold :: (acc -> k -> v -> acc) -> acc -> Drive k v -> acc
dfold = dfoldWhile (const True)
{-# INLINE dfold #-}

foldAll :: (acc -> v -> acc) -> acc -> Drive k v -> acc
foldAll step = dfold (\acc _ value -> step acc value)
{-# INLINE foldAll #-}

count :: Drive k v -> Int
count = dfold (\acc _ _ -> acc + 1) 0
{-# INLINE count #-}

anyOf :: Drive k v -> Bool
anyOf = dfoldWhile not (\_ _ _ -> True) False
{-# INLINE anyOf #-}

collect :: Drive k v -> [(k, v)]
collect rows = reverse (dfold (\acc key value -> (key, value) : acc) [] rows)

limit :: Int -> Drive k v -> [(k, v)]
limit size rows = reverse (fst result)
  where
    result = dfoldWhile (\(_, n) -> n < size)
      (\(values, n) key value -> ((key, value) : values, n + 1))
      ([], 0 :: Int) rows

fromList :: [(k, v)] -> Drive k v
fromList rows = linear Stream
  { source = rows
  , initialState = id
  , next = \_ remaining yield _skip done -> case remaining of
      [] -> done
      ((key, value) : rest) -> yield key value rest
  }
