{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Staged pull streams.
--
-- A stream advances one step at a time. Its consumer generates the loop, so it
-- can stop early or advance several streams together. This supports `limit`,
-- `anyOf`, and lockstep without separate stream operations.
--
-- These types exist only while generating code. Their existential state is
-- replaced by ordinary loop variables in the generated program.
--
-- Reusing a `CodeQ` emits its code again. Bind shared runtime expressions before
-- inserting them in more than one generated location.
module Prela.PullStaged.Stream.Internal
  ( -- * Representation
    Stream (..)
  , Drive (..)
  , Probe (..)
    -- * Mode-free operators
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
    -- * Consumers
  , dfoldWhile
  , dfold
  , dfoldST
  , foldAll
  , count
  , anyOf
  , collect
  , limit
  , fromList
    -- * Lockstep
  , zipWithS
  , mapvS
  , filtS
    -- * Staging utilities
  , lam1
  ) where

import Control.Monad.ST (ST)
import Language.Haskell.TH (CodeQ)

--------------------------------------------------------------------------------
-- Staging utilities
--------------------------------------------------------------------------------

-- | Generate a unary function.
--
-- A top-level splice cannot refer to a runtime argument introduced outside its
-- quote. Therefore this is invalid:
--
-- > q6 :: TPCH -> Double            -- WRONG
-- > q6 s = $$(revenue [|| s ||])    -- s is a runtime binder; the splice runs earlier
--
-- `lam1` introduces the argument inside the generated quote:
--
-- > q6 :: TPCH -> Double
-- > q6 = $$(lam1 (\s -> revenue s))
--
lam1 :: (CodeQ a -> CodeQ b) -> CodeQ (a -> b)
lam1 f = [|| \_x -> $$(f [|| _x ||]) ||]

--------------------------------------------------------------------------------
-- Representation
--------------------------------------------------------------------------------

-- | A flat pull stream that advances by at most one element.
--
-- `next` receives the source and state, then emits one continuation:
-- @yield@, @skip@, or @done@. A filtered row uses @skip@, returning control
-- without producing a value.
--
-- `source` holds loop-invariant code such as a schema field selection. Consumers
-- bind it once outside the loop, preventing that expression from being emitted
-- in both `initialState` and `next` or repeated per row.
data Stream k v = forall st src. Stream
  { source       :: CodeQ src                  -- ^ Data shared by every call to `next`, e.g. a column.
  , initialState :: CodeQ src -> CodeQ st      -- ^ State passed to the first call to `next`.
  , next         :: forall a. CodeQ src
          -> CodeQ st
          -> (CodeQ k -> CodeQ v -> CodeQ st -> CodeQ a) -- yield continuation
          -> (CodeQ st -> CodeQ a)                       -- skip continuation
          -> CodeQ a                                     -- done result
          -> CodeQ a                                     -- ^ Emit one step.
  }

-- | A compositional drive that enumerates relation pairs.
--
-- `Lin` is one stream, `Bind` is flat-map, and `Cat` concatenates drives.
-- `Bind` passes both the outer key and value because different operators use
-- different sides. The consumer, not `Drive`, runs the generated loop.
data Drive k v
  = Lin (Stream k v) -- ^ One flat stream.
  | forall k' v'. Bind
      (Drive k' v')
      (CodeQ k' -> CodeQ v' -> Drive k v) -- ^ Run an inner drive for each outer row.
  | Cat (Drive k v) (Drive k v) -- ^ Run two drives in sequence.

-- | Generated keyed access to a relation.
--
-- @at@ preserves the full drive of values for consumers that genuinely need
-- to enumerate a multi-valued row. @exists@ is the allocation-free membership
-- path: it asks whether any value at a key satisfies a generated predicate.
-- Keeping that operation explicit lets functional columns, compositions and
-- products emit straight-line tests instead of constructing a tiny drive
-- and immediately consuming it with 'anyOf' for every outer row.
data Probe k v = Probe
  { at     :: CodeQ k -> Drive () v
    -- ^ Enumerate every value stored at the supplied key.
  , exists :: CodeQ k -> (CodeQ v -> CodeQ Bool) -> CodeQ Bool
    -- ^ Test values at the key with a fused, short-circuiting predicate.
  }

--------------------------------------------------------------------------------
-- Mode-free operators
--------------------------------------------------------------------------------

-- | Turn a flat stream into a compositional drive.
--
-- Only flat t'Stream' values can be zipped because lockstep needs direct access to both
-- states. After `linear`, that capability is intentionally unavailable.
linear :: Stream d r -> Drive d r
linear = Lin
{-# INLINE linear #-}

-- | Map the values of a drive without changing its keys or shape.
mapvD :: (CodeQ r -> CodeQ r') -> Drive d r -> Drive d r'
mapvD h (Lin p)    = Lin (mapvS h p)
mapvD h (Bind o g) = Bind o (\x e -> mapvD h (g x e))
mapvD h (Cat a b)  = Cat (mapvD h a) (mapvD h b)

-- | Map the keys of a drive without changing its values or shape.
mapkD :: (CodeQ d -> CodeQ d') -> Drive d r -> Drive d' r
mapkD h (Lin p)    = Lin (mapkS h p)
mapkD h (Bind o g) = Bind o (\x e -> mapkD h (g x e))
mapkD h (Cat a b)  = Cat (mapkD h a) (mapkD h b)

-- | A key-map with the value in scope as well. `byValue` is the special case
-- that throws the old key away; this is the general one, and it is what builds
-- the @(key, value)@ pairs that @withCountDistinct@ deduplicates.
mapkVD :: (CodeQ d -> CodeQ r -> CodeQ d') -> Drive d r -> Drive d' r
mapkVD h (Lin p)    = Lin (mapkVS h p)
mapkVD h (Bind o g) = Bind o (\x e -> mapkVD h (g x e))
mapkVD h (Cat a b)  = Cat (mapkVD h a) (mapkVD h b)

-- | Set each row's key to its own value. @groupBy@ adds a keyed probe.
byValue :: Drive d r -> Drive r r
byValue (Lin p)    = Lin (byValueS p)
byValue (Bind o g) = Bind o (\x e -> byValue (g x e))
byValue (Cat a b)  = Cat (byValue a) (byValue b)

-- | Swap key and value. Enumeration only, and free. There is no t'Probe' form:
-- keyed access to an inverse requires building a reverse index, which is
-- "Prela.PullStaged.Materialize"'s @invert@.
invDrive :: Drive d r -> Drive r d
invDrive (Lin p)    = Lin (swapS p)
invDrive (Bind o g) = Bind o (\x e -> invDrive (g x e))
invDrive (Cat a b)  = Cat (invDrive a) (invDrive b)

-- | Keep the drive rows whose values satisfy the staged predicate.
filtD :: (CodeQ r -> CodeQ Bool) -> Drive d r -> Drive d r
filtD t = filtDKV (\_ r -> t r)

-- | A filter with the key in scope as well as the value.
filtDKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> Drive d r -> Drive d r
filtDKV t (Lin p)    = Lin (filtSKV t p)
filtDKV t (Bind o g) = Bind o (\x e -> filtDKV t (g x e))
filtDKV t (Cat a b)  = Cat (filtDKV t a) (filtDKV t b)

-- | One row of @()@ if the test passes, none if it does not.
guardD :: CodeQ Bool -> Drive () ()
guardD c = Lin Stream
  { source       = [|| () ||]
  , initialState = \_ -> [|| True ||]
  , next         = \_ s yield _skip done ->
      [|| if $$s && $$c then $$(yield [|| () ||] [|| () ||] [|| False ||]) else $$done ||]
  }

-- | Run a drive only if a generation-time-emitted test passes at runtime. Used
-- by the t'Probe' implementation of @diff@, where the test is on the supplied
-- key and so is decided once for the whole drive rather than per row.
whenD :: CodeQ Bool -> Drive d r -> Drive d r
whenD c s = Bind (guardD c) (\_ _ -> s)

-- | One drive and then the other. Prela's union.
catD :: Drive d r -> Drive d r -> Drive d r
catD = Cat

--------------------------------------------------------------------------------
-- Stream-level operators
--------------------------------------------------------------------------------

-- | Map the values yielded by a flat stream.
mapvS :: (CodeQ r -> CodeQ r') -> Stream d r -> Stream d r'
mapvS h (Stream env i step) = Stream
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield d (h r) s') skip done
  }

-- | Map the keys yielded by a flat stream.
mapkS :: (CodeQ d -> CodeQ d') -> Stream d r -> Stream d' r
mapkS h (Stream env i step) = Stream
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield (h d) r s') skip done
  }

-- | Map each stream key with both the old key and value in scope.
mapkVS :: (CodeQ d -> CodeQ r -> CodeQ d') -> Stream d r -> Stream d' r
mapkVS h (Stream env i step) = Stream
  { source = env, initialState = i
  , next = \e s yield skip done ->
      step e s (\d r s' -> [|| let !_x = $$r in $$(yield (h d [|| _x ||]) [|| _x ||] s') ||])
        skip done
  }

-- | Replace each stream key with the corresponding value.
byValueS :: Stream d r -> Stream r r
byValueS (Stream env i step) = Stream
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\_ r s' -> yield r r s') skip done
  }

-- | Exchange the keys and values yielded by a stream.
swapS :: Stream d r -> Stream r d
swapS (Stream env i step) = Stream
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield r d s') skip done
  }

-- | Keep stream rows whose values satisfy the staged predicate.
filtS :: (CodeQ r -> CodeQ Bool) -> Stream d r -> Stream d r
filtS t = filtSKV (\_ r -> t r)

-- | Keep stream rows satisfying a predicate over both key and value.
-- Bind the value once because both the test and yield use it.
filtSKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> Stream d r -> Stream d r
filtSKV t (Stream env i step) = Stream
  { source = env, initialState = i
  , next = \e s yield skip done ->
      step e s
        (\d r s' -> [|| let !_x = $$r
                        in if $$(t d [|| _x ||])
                             then $$(yield d [|| _x ||] s')
                             else $$(skip s') ||])
        skip done
  }

--------------------------------------------------------------------------------
-- Consumers
--------------------------------------------------------------------------------

-- | Fold while the accumulator satisfies a predicate.
--
-- The predicate is checked at every loop level, allowing an inner result to end
-- the complete nested traversal. `anyOf` and `limit` are special cases.
--
-- In the `Bind` case, the strict value binding prevents an outer column read
-- from surviving across the inner loop as a boxed thunk. The matching strict
-- bindings in `Prela.PullStaged.Ops.prod` are also required. Measurements are in
-- `design/StagedPullMain.hs`.
dfoldWhile :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> Drive d r -> CodeQ acc
dfoldWhile p f z (Lin q) = foldWhileS p f z q
dfoldWhile p f z (Bind o g) =
  dfoldWhile p
    (\acc x e -> [|| let !_v = $$e
                     in $$(dfoldWhile p f acc (g x [|| _v ||])) ||])
    z o
dfoldWhile p f z (Cat a b) = dfoldWhile p f (dfoldWhile p f z a) b

-- | Implement a stopping fold over one flat stream.
-- Bind the stream environment once outside the loop.
foldWhileS :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> Stream d r -> CodeQ acc
foldWhileS p f z (Stream env i step) =
  [|| let !_e = $$env
          go !s !acc
            | not $$(p [|| acc ||]) = acc
            | otherwise =
                $$(step [|| _e ||] [|| s ||]
                     (\d r s' -> [|| go $$s' $$(f [|| acc ||] d r) ||])
                     (\s'     -> [|| go $$s' acc ||])
                     [|| acc ||])
      in go $$(i [|| _e ||]) $$z ||]

-- | Fold to exhaustion.
dfold :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
      -> CodeQ acc -> Drive d r -> CodeQ acc
dfold = dfoldWhile (\_ -> [|| True ||])

-- | Fold with an effectful `ST` step.
--
-- Materializers use this to thread mutable construction state directly through
-- the generated loop. The strict bind prevents a chain of deferred updates.
dfoldST :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ (ST s acc))
        -> CodeQ acc -> Drive d r -> CodeQ (ST s acc)
dfoldST f z (Lin q) = foldSTS f z q
dfoldST f z (Bind o g) =
  dfoldST
    (\acc x e -> [|| let !_v = $$e
                     in $$(dfoldST f acc (g x [|| _v ||])) ||])
    z o
dfoldST f z (Cat a b) =
  [|| $$(dfoldST f z a) >>= \ !z' -> $$(dfoldST f [|| z' ||] b) ||]

-- | Implement an effectful fold over one flat stream.
foldSTS :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ (ST s acc))
        -> CodeQ acc -> Stream d r -> CodeQ (ST s acc)
foldSTS f z (Stream env i step) =
  [|| let !_e = $$env
          go !s !acc =
            $$(step [|| _e ||] [|| s ||]
                 (\d r s' -> [|| $$(f [|| acc ||] d r) >>= \ !acc' -> go $$s' acc' ||])
                 (\s'     -> [|| go $$s' acc ||])
                 [|| return acc ||])
      in go $$(i [|| _e ||]) $$z ||]

-- | The no-group fold: ignore the keys and reduce every value into one scalar.
foldAll :: (CodeQ acc -> CodeQ r -> CodeQ acc)
        -> CodeQ acc -> Drive d r -> CodeQ acc
foldAll op = dfold (\acc _ v -> op acc v)

-- | Count all rows in a drive.
count :: Drive d r -> CodeQ Int
count = dfold (\acc _ _ -> [|| $$acc + 1 ||]) [|| 0 ||]

-- | Test whether a drive yields a row, stopping at the first one.
anyOf :: Drive d r -> CodeQ Bool
anyOf = dfoldWhile (\a -> [|| not $$a ||]) (\_ _ _ -> [|| True ||]) [|| False ||]

-- | Materialize a drive as a list.
collect :: Drive d r -> CodeQ [(d, r)]
collect s = [|| reverse $$(dfold (\a d r -> [|| ($$d, $$r) : $$a ||]) [|| [] ||] s) ||]

-- | Collect at most the first @n@ rows.
limit :: CodeQ Int -> Drive d r -> CodeQ [(d, r)]
limit n s =
  [|| reverse (fst $$(dfoldWhile (\a -> [|| snd $$a < $$n ||])
                                 (\a d r -> [|| case $$a of
                                                  (xs, k) -> (($$d, $$r) : xs, k + 1) ||])
                                 [|| ([], 0 :: Int) ||] s)) ||]

-- | Re-enter the staged drive world from a list bound by a materializer.
-- This is intentionally internal: ordinary plans should not turn pipelines
-- into lists, but bounded operators such as top-k need to hand their small,
-- already-materialized result to the rest of a query.
fromList :: CodeQ [(d, r)] -> Drive d r
fromList rows = Lin Stream
  { source = rows
  , initialState = \sourceRows -> sourceRows
  , next = \_ remaining yield _skip done ->
      [|| case $$remaining of
            [] -> $$done
            ((key, value) : rest) ->
              $$(yield [|| key ||] [|| value ||] [|| rest ||]) ||]
  }

--------------------------------------------------------------------------------
-- Lockstep
--------------------------------------------------------------------------------

-- | Advance two flat streams together without buffering.
--
-- If one side skips, the other retains its old state and repeats that pure step.
-- Keys come from the left. Nested `Drive`s cannot be zipped because this
-- function requires direct access to both stream states.
zipWithS :: (CodeQ a -> CodeQ b -> CodeQ c)
         -> Stream d a -> Stream e b -> Stream d c
zipWithS h (Stream env1 i1 st1) (Stream env2 i2 st2) = Stream
  { source       = [|| ($$env1, $$env2) ||]
  , initialState = \e -> [|| case $$e of (e1, e2) -> ($$(i1 [|| e1 ||]), $$(i2 [|| e2 ||])) ||]
  , next         = \e s yield skip done ->
      [|| case $$e of
            (e1, e2) -> case $$s of
              (a, b) ->
                $$(st1 [|| e1 ||] [|| a ||]
                     (\d x a' ->
                        st2 [|| e2 ||] [|| b ||]
                          (\_ y b' -> yield d (h x y) [|| ($$a', $$b') ||])
                          (\b'     -> skip [|| (a, $$b') ||])
                          done)
                     (\a' -> skip [|| ($$a', b) ||])
                     done) ||]
  }
