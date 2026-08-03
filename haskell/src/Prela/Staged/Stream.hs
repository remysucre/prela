{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The execution protocol: a pull stream, staged.
--
-- This replaces the two records in "Prela.Mode". The difference is who owns the
-- loop. `Drv`'s @drive@ took a callback and ran to completion, so a consumer
-- could not stop it, could not advance it one element at a time, and could not
-- run two of them side by side. Here the CONSUMER generates the loop and the
-- stream only says how to take one step, which is what makes LIMIT, membership
-- with early exit, and lockstep all expressible, and expressible once.
--
-- Everything is at GENERATION time. A `CodeQ a` is a fragment of Haskell that
-- will have type @a@ once it is spliced; nothing in this module exists when the
-- generated program runs. In particular the state type below is existential, and
-- that existential is gone before there is a program to run — by the time
-- anything executes, a state is an `Int`, or a pair of them, named by a loop
-- binder. design/PullStreams.hs is the same design UNstaged and records what
-- that costs: the existential survives into the compiled code, so every row is
-- an unknown call plus a `Step` constructor built and matched.
--
-- THE ONE RULE. A `CodeQ` used twice is code emitted twice. Everything the old
-- FUSION.md warned about was a way of asking an optimizer to recover a loop it
-- could not see; there is no optimizer to ask any more, because the loop is
-- written out. What replaces those rules is this one, and it is why the
-- materializers in "Prela.Staged.Materialize" take a continuation instead of
-- returning a relation.
module Prela.Staged.Stream
  ( -- * Representation
    Producer (..)
  , SStream (..)
  , Cursor (..)
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

-- | Turn a generator into a generated FUNCTION.
--
-- This is how a query takes its schema, and without it there is no way to write
-- one. The stage restriction says a splice cannot name a variable bound in the
-- module it appears in, so this does not compile:
--
-- > q6 :: TPCH -> Double            -- WRONG
-- > q6 s = $$(revenue [|| s ||])    -- s is a runtime binder; the splice runs earlier
--
-- The fix is to let the GENERATOR introduce the binder, inside the quote, so the
-- splice sits within its scope:
--
-- > q6 :: TPCH -> Double
-- > q6 = $$(lam1 (\s -> revenue s))
--
-- Now @s@ is bound by the emitted lambda and @[|| _x ||]@ below is a reference to
-- a name the quote itself introduced, which is allowed. It is the same move
-- `foldWhileP` makes with its environment, and the same one `withFold` makes
-- with its table.
lam1 :: (CodeQ a -> CodeQ b) -> CodeQ (a -> b)
lam1 f = [|| \_x -> $$(f [|| _x ||]) ||]

--------------------------------------------------------------------------------
-- Representation
--------------------------------------------------------------------------------

-- | A producer that advances at most one element per step.
--
-- `pStep` is handed the environment, the current state and three continuations,
-- and must emit exactly one of them: `yield` with the key, the value and the
-- next state; `skip` with only the next state; or `done`.
--
-- `skip` is what lets a consumer advance one element at a time even through a
-- filter — a rejected row hands back control having produced nothing. Staging
-- does not remove that obligation, it removes its cost: a skip is a branch in
-- the generated loop rather than a constructor returned and matched.
--
-- Three continuations rather than a separate "is there more" predicate over the
-- state. A predicate reads better but makes the state get decomposed twice per
-- iteration, and it cannot express a producer that only discovers it is finished
-- by trying to advance.
--
-- `pEnv` is the leaf's loop invariant, and it exists because of this module's
-- one rule. A column reaches a leaf as CODE, and at the use site that code is
-- something like @[|| lineitem_quantity s ||]@ — a record selector, not a
-- variable. `pInit` and `pStep` are separate quotes spliced in different places,
-- so without somewhere to put it, that selector would be emitted into the loop
-- body and run per row. The consumer binds `pEnv` ONCE, outside the loop it
-- generates, and hands `pInit` and `pStep` a plain variable referring to it.
--
-- This is strymonas's @genlet@ cut down to the one shape Prela needs. GHC's full
-- laziness would probably float the selector out on its own, but "probably
-- floats" is what the push engine's FUSION.md spent five invariants failing to
-- guarantee, and a leaf with nothing to bind just uses @()@.
data Producer d r = forall s e. Producer
  { pEnv  :: CodeQ e
  , pInit :: CodeQ e -> CodeQ s
  , pStep :: forall w. CodeQ e
          -> CodeQ s
          -> (CodeQ d -> CodeQ r -> CodeQ s -> CodeQ w)   -- ^ yield
          -> (CodeQ s -> CodeQ w)                         -- ^ skip
          -> CodeQ w                                      -- ^ done
          -> CodeQ w
  }

-- | A stream is flat, or a loop nest, or two streams end to end.
--
-- Composition is flat-map, and flat-map does not fit in a single producer, which
-- is why `Bind` exists. Its outer stream's key and value types are both hidden:
-- the result's key comes from the CONTINUATION, not from the outer stream, which
-- is what lets `groupBy` and `invStream` change the key without any of the
-- operators having to reach inside a nest.
--
-- The continuation is handed the outer key AND the outer value. `compose`
-- ignores the key and `prod` ignores the value, but both are needed, because
-- `prod` probes its right-hand side at the key rather than at the value.
data SStream d r
  = Lin (Producer d r)
  | forall x e. Bind (SStream x e) (CodeQ x -> CodeQ e -> SStream d r)
  | Cat (SStream d r) (SStream d r)

-- | A relation in probed position: given a key, the values at that key.
--
-- This is what `Prb` was, minus @probeAny@. The push engine needed a second
-- field because a push consumer cannot stop, so short-circuiting had to be built
-- into every operator by hand — which is why the `Prb` instance in
-- "Prela.Ops" writes each operator twice. Here membership is `anyOf`, a
-- consumer, and there is one copy of everything.
newtype Cursor d r = Cursor { at :: CodeQ d -> SStream () r }

--------------------------------------------------------------------------------
-- Mode-free operators
--------------------------------------------------------------------------------

-- | Rejoin the ordinary pipeline, giving up the ability to zip.
--
-- Only a `Producer` can be zipped, because `zipWithP` has to own both states.
-- `Lin` forgets which streams are still flat, so the distinction has to be
-- visible in the types or the restriction is unenforceable. That is the same
-- carve-out strymonas calls linearity, and the reason its Theorem 1 says zipping
-- two flat-mapped streams must allocate. Here it is a type error instead.
linear :: Producer d r -> SStream d r
linear = Lin
{-# INLINE linear #-}

mapvS :: (CodeQ r -> CodeQ r') -> SStream d r -> SStream d r'
mapvS h (Lin p)    = Lin (mapvP h p)
mapvS h (Bind o g) = Bind o (\x e -> mapvS h (g x e))
mapvS h (Cat a b)  = Cat (mapvS h a) (mapvS h b)

mapkS :: (CodeQ d -> CodeQ d') -> SStream d r -> SStream d' r
mapkS h (Lin p)    = Lin (mapkP h p)
mapkS h (Bind o g) = Bind o (\x e -> mapkS h (g x e))
mapkS h (Cat a b)  = Cat (mapkS h a) (mapkS h b)

-- | A key-map with the value in scope as well. `byValue` is the special case
-- that throws the old key away; this is the general one, and it is what builds
-- the @(key, value)@ pairs that @withCountDistinct@ deduplicates.
mapkVS :: (CodeQ d -> CodeQ r -> CodeQ d') -> SStream d r -> SStream d' r
mapkVS h (Lin p)    = Lin (mapkVP h p)
mapkVS h (Bind o g) = Bind o (\x e -> mapkVS h (g x e))
mapkVS h (Cat a b)  = Cat (mapkVS h a) (mapkVS h b)

-- | Set each row's key to its own value. `groupBy` is this plus a probe.
byValue :: SStream d r -> SStream r r
byValue (Lin p)    = Lin (byValueP p)
byValue (Bind o g) = Bind o (\x e -> byValue (g x e))
byValue (Cat a b)  = Cat (byValue a) (byValue b)

-- | Swap key and value. Driven only, and free. There is no probed form: probing
-- an inverse means building a reverse index, which is
-- "Prela.Staged.Materialize"'s @withInv@.
invStream :: SStream d r -> SStream r d
invStream (Lin p)    = Lin (swapP p)
invStream (Bind o g) = Bind o (\x e -> invStream (g x e))
invStream (Cat a b)  = Cat (invStream a) (invStream b)

filtS :: (CodeQ r -> CodeQ Bool) -> SStream d r -> SStream d r
filtS t = filtKV (\_ r -> t r)

-- | A filter with the key in scope as well as the value.
filtKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> SStream d r -> SStream d r
filtKV t (Lin p)    = Lin (filtPKV t p)
filtKV t (Bind o g) = Bind o (\x e -> filtKV t (g x e))
filtKV t (Cat a b)  = Cat (filtKV t a) (filtKV t b)

-- | One row of `()` if the test passes, none if it does not.
guardS :: CodeQ Bool -> SStream () ()
guardS c = Lin Producer
  { pEnv  = [|| () ||]
  , pInit = \_ -> [|| True ||]
  , pStep = \_ s yield _skip done ->
      [|| if $$s && $$c then $$(yield [|| () ||] [|| () ||] [|| False ||]) else $$done ||]
  }

-- | Run a stream only if a generation-time-emitted test passes at runtime. Used
-- by `diff` in probed position, where the test is on the key and so is decided
-- once for the whole stream rather than per row.
whenS :: CodeQ Bool -> SStream d r -> SStream d r
whenS c s = Bind (guardS c) (\_ _ -> s)

-- | One stream and then the other. Prela's union.
catS :: SStream d r -> SStream d r -> SStream d r
catS = Cat

--------------------------------------------------------------------------------
-- Producer-level operators
--------------------------------------------------------------------------------

mapvP :: (CodeQ r -> CodeQ r') -> Producer d r -> Producer d r'
mapvP h (Producer env i step) = Producer
  { pEnv = env, pInit = i
  , pStep = \e s yield skip done -> step e s (\d r s' -> yield d (h r) s') skip done
  }

mapkP :: (CodeQ d -> CodeQ d') -> Producer d r -> Producer d' r
mapkP h (Producer env i step) = Producer
  { pEnv = env, pInit = i
  , pStep = \e s yield skip done -> step e s (\d r s' -> yield (h d) r s') skip done
  }

mapkVP :: (CodeQ d -> CodeQ r -> CodeQ d') -> Producer d r -> Producer d' r
mapkVP h (Producer env i step) = Producer
  { pEnv = env, pInit = i
  , pStep = \e s yield skip done ->
      step e s (\d r s' -> [|| let !_x = $$r in $$(yield (h d [|| _x ||]) [|| _x ||] s') ||])
        skip done
  }

byValueP :: Producer d r -> Producer r r
byValueP (Producer env i step) = Producer
  { pEnv = env, pInit = i
  , pStep = \e s yield skip done -> step e s (\_ r s' -> yield r r s') skip done
  }

swapP :: Producer d r -> Producer r d
swapP (Producer env i step) = Producer
  { pEnv = env, pInit = i
  , pStep = \e s yield skip done -> step e s (\d r s' -> yield r d s') skip done
  }

filtP :: (CodeQ r -> CodeQ Bool) -> Producer d r -> Producer d r
filtP t = filtPKV (\_ r -> t r)

-- The @let !x@ binds the value exactly once. Without it the value is spliced
-- into the test and again into the yield, and by the one rule of this module
-- that emits the array read twice.
filtPKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> Producer d r -> Producer d r
filtPKV t (Producer env i step) = Producer
  { pEnv = env, pInit = i
  , pStep = \e s yield skip done ->
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

-- | Fold while the accumulator says to keep going. Every other consumer here is
-- this one specialised, which is the whole early-termination payoff: membership,
-- LIMIT and EXISTS are the same function at three guards.
--
-- The guard is tested at EVERY loop level, which is what makes an early exit
-- propagate outwards instead of only ending the innermost loop.
--
-- The @let !v@ in the `Bind` case is load bearing and it was measured. The outer
-- row's value has to stay live across the whole inner loop, and left to itself
-- GHC keeps it lazy — a checked array read has a bottoming branch, which GHC
-- will not speculate, so what crosses the loop boundary is a boxed thunk built
-- once per outer row. Forcing it turns the binding into a `case` on an unboxed
-- value that a join point can carry in a register. It works only together with
-- the matching bangs in "Prela.Staged.Ops"'s @prod@; see the measurements at the
-- bottom of design/StagedPullMain.hs, where dropping either one costs 64 bytes
-- a row on a three-deep `prod`.
--
-- Forcing is not a semantic change for anything Prela can build: a relation's
-- values are array reads, and every consumer is a strict fold.
--
-- The KEY is spliced rather than bound, and may be emitted more than once. That
-- is safe only because every leaf hands back a key that is already a variable —
-- a loop counter, or @()@ — so emitting it twice emits two variable references.
-- Binding it unconditionally is not the answer: a consumer that ignores the key
-- would then get a dead binding and a warning in every generated query. The
-- general fix is strymonas's @genlet@, which inserts a binding only where the
-- code is reached, and nothing here needs it yet.
sfoldWhile :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> SStream d r -> CodeQ acc
sfoldWhile p f z (Lin q) = foldWhileP p f z q
sfoldWhile p f z (Bind o g) =
  sfoldWhile p
    (\acc x e -> [|| let !_v = $$e
                     in $$(sfoldWhile p f acc (g x [|| _v ||])) ||])
    z o
sfoldWhile p f z (Cat a b) = sfoldWhile p f (sfoldWhile p f z a) b

-- The environment is bound OUTSIDE the loop, which is the whole reason it is a
-- field. Everything the leaf then splices refers to `e`, a variable.
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

-- | Fold to exhaustion. The guard is the constant `True`, which the simplifier
-- deletes before it reaches the generated loop.
sfold :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
      -> CodeQ acc -> SStream d r -> CodeQ acc
sfold = sfoldWhile (\_ -> [|| True ||])

-- | Fold where the step may mutate. This is `sfold` with the accumulator
-- threaded through `ST` instead of returned directly, and it exists for exactly
-- one caller: "Prela.Staged.Materialize", whose caches are mutable arrays under
-- construction.
--
-- It is worth naming what this replaces. "Prela.Materialize" keeps its
-- open-addressed table in an `STRef` and says why at line 63: @drive@'s callback
-- returns @m ()@, so there is nowhere to thread state. That costs a pointer read
-- per input row. Here the accumulator IS the loop argument, so a table under
-- construction is passed and returned like any other value and the `STRef` is
-- gone. FUSION.md's rule about never carrying a per-row accumulator in an
-- `STRef` stops being a rule you can break.
--
-- Note the `!` on @acc'@: the bind is strict, so the accumulator is forced each
-- step rather than building a chain of suspended updates.
sfoldST :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ (ST s acc))
        -> CodeQ acc -> SStream d r -> CodeQ (ST s acc)
sfoldST f z (Lin q) = foldSTP f z q
sfoldST f z (Bind o g) =
  sfoldST
    (\acc x e -> [|| let !_v = $$e
                     in $$(sfoldST f acc (g x [|| _v ||])) ||])
    z o
sfoldST f z (Cat a b) =
  [|| $$(sfoldST f z a) >>= \ !z' -> $$(sfoldST f [|| z' ||] b) ||]

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
        -> CodeQ acc -> SStream d r -> CodeQ acc
foldAll op = sfold (\acc _ v -> op acc v)

count :: SStream d r -> CodeQ Int
count = sfold (\acc _ _ -> [|| $$acc + 1 ||]) [|| 0 ||]

-- | Membership, which is early termination in its smallest form: stop at the
-- first row that arrives. This one function is the whole of @probeAny@.
anyOf :: SStream d r -> CodeQ Bool
anyOf = sfoldWhile (\a -> [|| not $$a ||]) (\_ _ _ -> [|| True ||]) [|| False ||]

-- | The boundary: above it fused algebra, below it a plain list.
collect :: SStream d r -> CodeQ [(d, r)]
collect s = [|| reverse $$(sfold (\a d r -> [|| ($$d, $$r) : $$a ||]) [|| [] ||] s) ||]

-- | The first n rows, and then stop. Not expressible against `Drv` at all.
limit :: CodeQ Int -> SStream d r -> CodeQ [(d, r)]
limit n s =
  [|| reverse (fst $$(sfoldWhile (\a -> [|| snd $$a < $$n ||])
                                 (\a d r -> [|| case $$a of
                                                  (xs, k) -> (($$d, $$r) : xs, k + 1) ||])
                                 [|| ([], 0 :: Int) ||] s)) ||]

--------------------------------------------------------------------------------
-- Lockstep
--------------------------------------------------------------------------------

-- | Advance two producers together, combining their values. One loop, two
-- cursors, nothing buffered.
--
-- When the left yields and the right skips, the left's OLD state is kept and it
-- re-steps next time round. That is sound because a step is a pure function of
-- its state, which is exactly the property push does not have — this is the
-- thing `Drv` cannot express at all.
--
-- Keys come from the left. Two nested streams cannot be zipped, and that is a
-- type error rather than a silent deoptimisation, since this takes `Producer`s.
zipWithP :: (CodeQ a -> CodeQ b -> CodeQ c)
         -> Producer d a -> Producer e b -> Producer d c
zipWithP h (Producer env1 i1 st1) (Producer env2 i2 st2) = Producer
  { pEnv  = [|| ($$env1, $$env2) ||]
  , pInit = \e -> [|| case $$e of (e1, e2) -> ($$(i1 [|| e1 ||]), $$(i2 [|| e2 ||])) ||]
  , pStep = \e s yield skip done ->
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
