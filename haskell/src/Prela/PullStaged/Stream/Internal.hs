{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Staged pull streams.
--
-- A stream describes one step. Its consumer generates the loop, so it can stop
-- early or advance several streams together. This supports `limit`, `anyOf`,
-- and lockstep without separate producer operations.
--
-- These types exist only while generating code. Their existential state is
-- replaced by ordinary loop variables in the generated program.
--
-- Reusing a `CodeQ` emits its code again. Bind shared runtime expressions before
-- inserting them in more than one generated location.
module Prela.PullStaged.Stream.Internal
  ( -- * Representation
    Producer (..)
  , Stream (..)
  , Lookup (..)
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
  , sfoldST
  , foldAll
  , count
  , anyOf
  , collect
  , limit
  , fromList
    -- * Lockstep
  , zipWithP
  , mapvP
  , filtP
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

-- | A producer that advances by at most one element.
--
-- `next` receives the source and state, then emits one continuation:
-- @yield@, @skip@, or @done@. A filtered row uses @skip@, returning control
-- without producing a value.
--
-- `source` holds loop-invariant code such as a schema field selection. Consumers
-- bind it once outside the loop, preventing that expression from being emitted
-- in both `initialState` and `next` or repeated per row.
data Producer k v = forall st src. Producer
  { source       :: CodeQ src                  -- ^ Data shared by every call to `next`, e.g. a column.
  , initialState :: CodeQ src -> CodeQ st      -- ^ State passed to the first call to `next`.
  , next         :: forall a. CodeQ src
          -> CodeQ st
          -> (CodeQ k -> CodeQ v -> CodeQ st -> CodeQ a) -- yield continuation
          -> (CodeQ st -> CodeQ a)                       -- skip continuation
          -> CodeQ a                                     -- done result
          -> CodeQ a                                     -- ^ Emit one step.
  }

-- | A generated stream that enumerates relation pairs.
--
-- `Lin` is one producer, `Bind` is flat-map, and `Cat` concatenates streams.
-- `Bind` passes both the outer key and value because different operators use
-- different sides. The consumer, not `Stream`, runs the generated loop.
data Stream k v
  = Lin (Producer k v) -- ^ One (flat) producer.
  | forall k' v'. Bind
      (Stream k' v')
      (CodeQ k' -> CodeQ v' -> Stream k v) -- ^ Run an inner stream for each outer row.
  | Cat (Stream k v) (Stream k v) -- ^ Run two streams in sequence.

-- | Generated keyed access to a relation.
--
-- @at@ preserves the full stream of values for consumers that genuinely need
-- to enumerate a multi-valued row. @probeAny@ is the allocation-free membership
-- path: it asks whether any value at a key satisfies a generated predicate.
-- Keeping that operation explicit lets functional columns, compositions and
-- products emit straight-line tests instead of constructing a tiny pull stream
-- and immediately consuming it with 'anyOf' for every outer row.
data Lookup k v = Lookup
  { at       :: CodeQ k -> Stream () v
    -- ^ Enumerate every value stored at the supplied key.
  , probeAny :: CodeQ k -> (CodeQ v -> CodeQ Bool) -> CodeQ Bool
    -- ^ Test values at the key with a fused, short-circuiting predicate.
  }

--------------------------------------------------------------------------------
-- Mode-free operators
--------------------------------------------------------------------------------

-- | Turn a flat producer into a general stream.
--
-- Only t'Producer's can be zipped because lockstep needs direct access to both
-- states. After `linear`, that capability is intentionally unavailable.
linear :: Producer d r -> Stream d r
linear = Lin
{-# INLINE linear #-}

-- | Map the values of a stream without changing its keys or shape.
mapvS :: (CodeQ r -> CodeQ r') -> Stream d r -> Stream d r'
mapvS h (Lin p)    = Lin (mapvP h p)
mapvS h (Bind o g) = Bind o (\x e -> mapvS h (g x e))
mapvS h (Cat a b)  = Cat (mapvS h a) (mapvS h b)

-- | Map the keys of a stream without changing its values or shape.
mapkS :: (CodeQ d -> CodeQ d') -> Stream d r -> Stream d' r
mapkS h (Lin p)    = Lin (mapkP h p)
mapkS h (Bind o g) = Bind o (\x e -> mapkS h (g x e))
mapkS h (Cat a b)  = Cat (mapkS h a) (mapkS h b)

-- | A key-map with the value in scope as well. `byValue` is the special case
-- that throws the old key away; this is the general one, and it is what builds
-- the @(key, value)@ pairs that @withCountDistinct@ deduplicates.
mapkVS :: (CodeQ d -> CodeQ r -> CodeQ d') -> Stream d r -> Stream d' r
mapkVS h (Lin p)    = Lin (mapkVP h p)
mapkVS h (Bind o g) = Bind o (\x e -> mapkVS h (g x e))
mapkVS h (Cat a b)  = Cat (mapkVS h a) (mapkVS h b)

-- | Set each row's key to its own value. @groupBy@ adds a keyed lookup.
byValue :: Stream d r -> Stream r r
byValue (Lin p)    = Lin (byValueP p)
byValue (Bind o g) = Bind o (\x e -> byValue (g x e))
byValue (Cat a b)  = Cat (byValue a) (byValue b)

-- | Swap key and value. Enumeration only, and free. There is no t'Lookup' form:
-- keyed access to an inverse requires building a reverse index, which is
-- "Prela.PullStaged.Materialize"'s @invert@.
invStream :: Stream d r -> Stream r d
invStream (Lin p)    = Lin (swapP p)
invStream (Bind o g) = Bind o (\x e -> invStream (g x e))
invStream (Cat a b)  = Cat (invStream a) (invStream b)

-- | Keep the stream rows whose values satisfy the staged predicate.
filtS :: (CodeQ r -> CodeQ Bool) -> Stream d r -> Stream d r
filtS t = filtKV (\_ r -> t r)

-- | A filter with the key in scope as well as the value.
filtKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> Stream d r -> Stream d r
filtKV t (Lin p)    = Lin (filtPKV t p)
filtKV t (Bind o g) = Bind o (\x e -> filtKV t (g x e))
filtKV t (Cat a b)  = Cat (filtKV t a) (filtKV t b)

-- | One row of @()@ if the test passes, none if it does not.
guardS :: CodeQ Bool -> Stream () ()
guardS c = Lin Producer
  { source       = [|| () ||]
  , initialState = \_ -> [|| True ||]
  , next         = \_ s yield _skip done ->
      [|| if $$s && $$c then $$(yield [|| () ||] [|| () ||] [|| False ||]) else $$done ||]
  }

-- | Run a stream only if a generation-time-emitted test passes at runtime. Used
-- by the t'Lookup' implementation of @diff@, where the test is on the supplied
-- key and so is decided once for the whole stream rather than per row.
whenS :: CodeQ Bool -> Stream d r -> Stream d r
whenS c s = Bind (guardS c) (\_ _ -> s)

-- | One stream and then the other. Prela's union.
catS :: Stream d r -> Stream d r -> Stream d r
catS = Cat

--------------------------------------------------------------------------------
-- Producer-level operators
--------------------------------------------------------------------------------

-- | Map the values yielded by a flat producer.
mapvP :: (CodeQ r -> CodeQ r') -> Producer d r -> Producer d r'
mapvP h (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield d (h r) s') skip done
  }

-- | Map the keys yielded by a flat producer.
mapkP :: (CodeQ d -> CodeQ d') -> Producer d r -> Producer d' r
mapkP h (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield (h d) r s') skip done
  }

-- | Map each producer key with both the old key and value in scope.
mapkVP :: (CodeQ d -> CodeQ r -> CodeQ d') -> Producer d r -> Producer d' r
mapkVP h (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done ->
      step e s (\d r s' -> [|| let !_x = $$r in $$(yield (h d [|| _x ||]) [|| _x ||] s') ||])
        skip done
  }

-- | Replace each producer key with the corresponding value.
byValueP :: Producer d r -> Producer r r
byValueP (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\_ r s' -> yield r r s') skip done
  }

-- | Exchange the keys and values yielded by a producer.
swapP :: Producer d r -> Producer r d
swapP (Producer env i step) = Producer
  { source = env, initialState = i
  , next = \e s yield skip done -> step e s (\d r s' -> yield r d s') skip done
  }

-- | Keep producer rows whose values satisfy the staged predicate.
filtP :: (CodeQ r -> CodeQ Bool) -> Producer d r -> Producer d r
filtP t = filtPKV (\_ r -> t r)

-- | Keep producer rows satisfying a predicate over both key and value.
-- Bind the value once because both the test and yield use it.
filtPKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> Producer d r -> Producer d r
filtPKV t (Producer env i step) = Producer
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
sfoldWhile :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> Stream d r -> CodeQ acc
sfoldWhile p f z (Lin q) = foldWhileP p f z q
sfoldWhile p f z (Bind o g) =
  sfoldWhile p
    (\acc x e -> [|| let !_v = $$e
                     in $$(sfoldWhile p f acc (g x [|| _v ||])) ||])
    z o
sfoldWhile p f z (Cat a b) = sfoldWhile p f (sfoldWhile p f z a) b

-- | Implement a stopping fold over one flat producer.
-- Bind the producer environment once outside the loop.
foldWhileP :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> Producer d r -> CodeQ acc
foldWhileP p f z (Producer env i step) =
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
sfold :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
      -> CodeQ acc -> Stream d r -> CodeQ acc
sfold = sfoldWhile (\_ -> [|| True ||])

-- | Fold with an effectful `ST` step.
--
-- Materializers use this to thread mutable construction state directly through
-- the generated loop. The strict bind prevents a chain of deferred updates.
sfoldST :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ (ST s acc))
        -> CodeQ acc -> Stream d r -> CodeQ (ST s acc)
sfoldST f z (Lin q) = foldSTP f z q
sfoldST f z (Bind o g) =
  sfoldST
    (\acc x e -> [|| let !_v = $$e
                     in $$(sfoldST f acc (g x [|| _v ||])) ||])
    z o
sfoldST f z (Cat a b) =
  [|| $$(sfoldST f z a) >>= \ !z' -> $$(sfoldST f [|| z' ||] b) ||]

-- | Implement an effectful fold over one flat producer.
foldSTP :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ (ST s acc))
        -> CodeQ acc -> Producer d r -> CodeQ (ST s acc)
foldSTP f z (Producer env i step) =
  [|| let !_e = $$env
          go !s !acc =
            $$(step [|| _e ||] [|| s ||]
                 (\d r s' -> [|| $$(f [|| acc ||] d r) >>= \ !acc' -> go $$s' acc' ||])
                 (\s'     -> [|| go $$s' acc ||])
                 [|| return acc ||])
      in go $$(i [|| _e ||]) $$z ||]

-- | The no-group fold: ignore the keys and reduce every value into one scalar.
foldAll :: (CodeQ acc -> CodeQ r -> CodeQ acc)
        -> CodeQ acc -> Stream d r -> CodeQ acc
foldAll op = sfold (\acc _ v -> op acc v)

-- | Count all rows in a stream.
count :: Stream d r -> CodeQ Int
count = sfold (\acc _ _ -> [|| $$acc + 1 ||]) [|| 0 ||]

-- | Test whether a stream produces a row, stopping at the first one.
anyOf :: Stream d r -> CodeQ Bool
anyOf = sfoldWhile (\a -> [|| not $$a ||]) (\_ _ _ -> [|| True ||]) [|| False ||]

-- | Materialize a stream as a list.
collect :: Stream d r -> CodeQ [(d, r)]
collect s = [|| reverse $$(sfold (\a d r -> [|| ($$d, $$r) : $$a ||]) [|| [] ||] s) ||]

-- | Collect at most the first @n@ rows.
limit :: CodeQ Int -> Stream d r -> CodeQ [(d, r)]
limit n s =
  [|| reverse (fst $$(sfoldWhile (\a -> [|| snd $$a < $$n ||])
                                 (\a d r -> [|| case $$a of
                                                  (xs, k) -> (($$d, $$r) : xs, k + 1) ||])
                                 [|| ([], 0 :: Int) ||] s)) ||]

-- | Re-enter the staged stream world from a list bound by a materializer.
-- This is intentionally internal: ordinary plans should not turn pipelines
-- into lists, but bounded operators such as top-k need to hand their small,
-- already-materialized result to the rest of a query.
fromList :: CodeQ [(d, r)] -> Stream d r
fromList rows = Lin Producer
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

-- | Advance two flat producers together without buffering.
--
-- If one side skips, the other retains its old state and repeats that pure step.
-- Keys come from the left. Nested `Stream`s cannot be zipped because this
-- function requires direct access to both producer states.
zipWithP :: (CodeQ a -> CodeQ b -> CodeQ c)
         -> Producer d a -> Producer e b -> Producer d c
zipWithP h (Producer env1 i1 st1) (Producer env2 i2 st2) = Producer
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
