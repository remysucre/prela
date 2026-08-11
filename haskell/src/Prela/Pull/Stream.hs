{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE RankNTypes #-}

-- | Pull streams, without staging.
--
-- This is "Prela.PullStaged.Stream" with every `CodeQ` erased back to the plain
-- value it stood for. The representation survives unchanged: a stream
-- describes one step, and its CONSUMER generates the loop by supplying
-- `yield`/`skip`/`done` continuations — that is what lets `anyOf` stop at the
-- first matching row instead of walking the whole relation, with no separate
-- short-circuiting operator needed. Staging only ever decided WHEN that loop
-- was built (compile time, as generated code, vs. here, as an ordinary
-- recursive function called at run time); it was never what the loop does.
-- Removing it is why this module has no leaves of its own — those lived in
-- "Prela.PullStaged.Ops" reading `Col`/`SparseCol`/`Table`/etc., and there is no
-- physical storage here for a leaf to read.
module Prela.Pull.Stream
  ( -- * Representation
    Producer (..)
  , Stream (..)
  , Lookup (..)
    -- * List-backed leaves
  , listProd
  , pairProd
    -- * Mode-free operators
  , mapvS
  , mapkS
  , mapkVS
  , filtS
  , filtKV
  , invStream
  , byValue
  , whenS
  , guardS
  , catS
  , linear
    -- * Consumers
  , sfoldWhile
  , sfold
  , foldAll
  , count
  , anyOf
  , collect
  , limit
  ) where

--------------------------------------------------------------------------------
-- Representation
--------------------------------------------------------------------------------

-- | A producer that advances by at most one element.
--
-- `next` receives the source and state, then calls exactly one of `yield`,
-- `skip`, or `done`. A filtered-out row calls `skip`, returning control
-- without producing a value — this is what lets `filtS` stay a `Producer`
-- instead of degrading to a `Stream` that has to buffer.
data Producer k v = forall st src. Producer
  { source       :: src                  -- ^ Data shared by every call to `next`.
  , initialState :: src -> st            -- ^ State passed to the first call to `next`.
  , next         :: forall a. src
          -> st
          -> (k -> v -> st -> a) -- ^ yield
          -> (st -> a)           -- ^ skip
          -> a                   -- ^ done
          -> a                   -- ^ Take one step.
  }

-- | A stream that enumerates relation pairs.
--
-- `Lin` is one producer, `Bind` is flat-map, `Cat` concatenates two streams.
-- `Bind` passes both the outer key and value because different operators use
-- different sides.
data Stream k v
  = Lin (Producer k v)
  | forall k' v'. Bind (Stream k' v') (k' -> v' -> Stream k v)
  | Cat (Stream k v) (Stream k v)

-- | Keyed access to a relation. Applying a key returns a pull stream of
-- matching values; the stream's own key is `()` because the supplied key is
-- already known.
newtype Lookup k v = Lookup { at :: k -> Stream () v }

--------------------------------------------------------------------------------
-- List-backed leaves
--------------------------------------------------------------------------------

-- | The only two producers this module needs, because a leaf here is always
-- "some values I was handed", never a column to read by index.
listProd :: [a] -> Producer () a
listProd xs = Producer
  { source       = xs
  , initialState = id
  , next         = \_ s yield _skip done ->
      case s of
        []       -> done
        (y : ys) -> yield () y ys
  }

pairProd :: [(d, r)] -> Producer d r
pairProd xs = Producer
  { source       = xs
  , initialState = id
  , next         = \_ s yield _skip done ->
      case s of
        []            -> done
        ((k, y) : ys) -> yield k y ys
  }

--------------------------------------------------------------------------------
-- Mode-free operators
--------------------------------------------------------------------------------

linear :: Producer d r -> Stream d r
linear = Lin

mapvS :: (r -> r') -> Stream d r -> Stream d r'
mapvS h (Lin p)    = Lin (mapvP h p)
mapvS h (Bind o g) = Bind o (\x e -> mapvS h (g x e))
mapvS h (Cat a b)  = Cat (mapvS h a) (mapvS h b)

mapkS :: (d -> d') -> Stream d r -> Stream d' r
mapkS h (Lin p)    = Lin (mapkP h p)
mapkS h (Bind o g) = Bind o (\x e -> mapkS h (g x e))
mapkS h (Cat a b)  = Cat (mapkS h a) (mapkS h b)

-- | A key-map with the value in scope as well.
mapkVS :: (d -> r -> d') -> Stream d r -> Stream d' r
mapkVS h (Lin p)    = Lin (mapkVP h p)
mapkVS h (Bind o g) = Bind o (\x e -> mapkVS h (g x e))
mapkVS h (Cat a b)  = Cat (mapkVS h a) (mapkVS h b)

-- | Set each row's key to its own value. `groupBy` adds a keyed lookup.
byValue :: Stream d r -> Stream r r
byValue (Lin p)    = Lin (byValueP p)
byValue (Bind o g) = Bind o (\x e -> byValue (g x e))
byValue (Cat a b)  = Cat (byValue a) (byValue b)

-- | Swap key and value. Enumeration only, and free. Looking up an inverse by
-- key requires a reverse index, which is "Prela.Pull.Materialize"'s `inv`.
invStream :: Stream d r -> Stream r d
invStream (Lin p)    = Lin (swapP p)
invStream (Bind o g) = Bind o (\x e -> invStream (g x e))
invStream (Cat a b)  = Cat (invStream a) (invStream b)

filtS :: (r -> Bool) -> Stream d r -> Stream d r
filtS t = filtKV (\_ r -> t r)

-- | A filter with the key in scope as well as the value.
filtKV :: (d -> r -> Bool) -> Stream d r -> Stream d r
filtKV t (Lin p)    = Lin (filtPKV t p)
filtKV t (Bind o g) = Bind o (\x e -> filtKV t (g x e))
filtKV t (Cat a b)  = Cat (filtKV t a) (filtKV t b)

-- | One row of `()` if the test passes, none if it does not.
guardS :: Bool -> Stream () ()
guardS c = Lin Producer
  { source       = ()
  , initialState = \_ -> True
  , next         = \_ s yield _skip done ->
      if s && c then yield () () False else done
  }

-- | Run a stream only if a test decided once (not per row) passes. Used by the
-- `Lookup` implementation of `diff`, where the test is on the supplied key.
whenS :: Bool -> Stream d r -> Stream d r
whenS c s = Bind (guardS c) (\_ _ -> s)

-- | One stream and then the other. Prela's union.
catS :: Stream d r -> Stream d r -> Stream d r
catS = Cat

--------------------------------------------------------------------------------
-- Producer-level operators
--------------------------------------------------------------------------------

mapvP :: (r -> r') -> Producer d r -> Producer d r'
mapvP h (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield d (h r) s') skip done
  }

mapkP :: (d -> d') -> Producer d r -> Producer d' r
mapkP h (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield (h d) r s') skip done
  }

mapkVP :: (d -> r -> d') -> Producer d r -> Producer d' r
mapkVP h (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield (h d r) r s') skip done
  }

byValueP :: Producer d r -> Producer r r
byValueP (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\_ r s' -> yield r r s') skip done
  }

swapP :: Producer d r -> Producer r d
swapP (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield r d s') skip done
  }

filtPKV :: (d -> r -> Bool) -> Producer d r -> Producer d r
filtPKV t (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done ->
      step e s
        (\d r s' -> if t d r then yield d r s' else skip s')
        skip done
  }

--------------------------------------------------------------------------------
-- Consumers
--------------------------------------------------------------------------------

-- | Fold while the accumulator satisfies a predicate. `anyOf` and `limit` are
-- special cases; checking at every nesting level is what lets an inner result
-- end the whole traversal early.
sfoldWhile :: (acc -> Bool) -> (acc -> d -> r -> acc) -> acc -> Stream d r -> acc
sfoldWhile p f z (Lin q)    = foldWhileP p f z q
sfoldWhile p f z (Bind o g) = sfoldWhile p (\acc x e -> sfoldWhile p f acc (g x e)) z o
sfoldWhile p f z (Cat a b)  = sfoldWhile p f (sfoldWhile p f z a) b

foldWhileP :: (acc -> Bool) -> (acc -> d -> r -> acc) -> acc -> Producer d r -> acc
foldWhileP p f z (Producer env i step) = go (i env) z
  where
    go s acc
      | not (p acc) = acc
      | otherwise   = step env s (\d r s' -> go s' (f acc d r)) (`go` acc) acc

-- | Fold to exhaustion.
sfold :: (acc -> d -> r -> acc) -> acc -> Stream d r -> acc
sfold = sfoldWhile (const True)

-- | The no-group fold: ignore the keys and reduce every value into one scalar.
foldAll :: (acc -> r -> acc) -> acc -> Stream d r -> acc
foldAll op = sfold (\acc _ v -> op acc v)

count :: Stream d r -> Int
count = sfold (\acc _ _ -> acc + 1) 0

-- | Test whether a stream produces a row, stopping at the first one.
anyOf :: Stream d r -> Bool
anyOf = sfoldWhile not (\_ _ _ -> True) False

-- | Materialize a stream as a list.
collect :: Stream d r -> [(d, r)]
collect s = reverse (sfold (\a d r -> (d, r) : a) [] s)

-- | Collect at most the first @n@ rows.
limit :: Int -> Stream d r -> [(d, r)]
limit n s =
  reverse (fst (sfoldWhile (\a -> snd a < n)
                            (\a d r -> case a of (xs, k) -> ((d, r) : xs, k + 1))
                            ([], 0 :: Int) s))
