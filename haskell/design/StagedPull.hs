{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. A pull stream, staged. This is design/PullStreams.hs
-- with every runtime component moved to generation time, which is what
-- strymonas does and why strymonas needs a staged host at all.
--
-- The representation is the same two pieces as the unstaged version. A producer
-- still has a state and a step; the stream is still either linear or nested.
-- What changes is that the state is now a `CodeQ`, so it exists only as a
-- variable in the program being generated, and the step is now a function from
-- code to code, so calling it emits an expression instead of returning a `Step`
-- value.
--
-- Two encodings do the work.
--
-- The step is in CONTINUATION-PASSING form with THREE continuations, one per
-- constructor of the unstaged `Step`: done, skip and yield. That is the same
-- information, expressed as which continuation gets emitted rather than as a
-- constructor that gets built and matched.
--
-- The existential over the state type is at GENERATION time. In the unstaged
-- version that existential survived into the compiled program and was exactly
-- what stopped GHC specialising the loop. Here it is gone before there is a
-- program: by the time anything runs, the state is an `Int`, or a pair of them,
-- named by a loop binder.
--
-- `Skip` is still here, unlike in design/Staged.hs, and it has to be: a pull
-- stream's whole point is that a consumer can advance one element at a time, so
-- a filter that rejects a row must be able to hand back control having produced
-- nothing. Generating the loop does not remove that obligation, it only removes
-- the cost of expressing it.
--
-- IT FUSES. Six queries — compose with a filter, a foreign key with holes,
-- membership, a three-deep `prod` tower, a zip of two filtered runs, and a
-- LIMIT — all compile to a worker over unboxed arguments with `joinrec` inner
-- levels, and all allocate ZERO bytes per row over one to two million rows,
-- matching the hand-written loop exactly. The measurements are at the bottom of
-- design/StagedPullMain.hs, which is the use site, along with the two bangs the
-- `prod` tower needed and what happens without them.
module StagedPull
  ( -- * Representation
    Producer (..)
  , SStream (..)
  , Cursor (..)
    -- * Leaves
  , universeProd
  , columnProd
  , universeP
  , columnP
  , colCursor
  , csrCursor
  , linear
  , filtProd
    -- * Operators
  , compose
  , prod
  , restrict
  , diff
  , filtS
  , mapvS
    -- * Consumers
  , sfoldWhile
  , sfold
  , count
  , anyOf
  , collect
    -- * What pull buys
  , limit
  , zipWithP
  ) where

import qualified Data.Vector.Unboxed as U
import Language.Haskell.TH (CodeQ)

--------------------------------------------------------------------------------
-- Representation
--------------------------------------------------------------------------------

-- | A producer that advances at most one element per step.
--
-- `pStep` is given the current state and three continuations. `yield` receives
-- the key, the value and the next state; `skip` receives only the next state;
-- `done` receives nothing. A step must emit exactly one of them.
--
-- Three continuations rather than a separate "is there more" predicate on the
-- state. A predicate reads more naturally but costs: the state gets decomposed
-- twice per iteration, once to test and once to step, which for `csrCursor`'s
-- @(offset, end)@ pair means unpacking a pair twice. It also cannot express a
-- producer that only discovers it is finished by trying to advance.
--
-- Note what `pStep` is NOT: it does not return a `Step d r s`. Nothing of this
-- type exists when the generated program runs.
data Producer d r = forall s. Producer
  { pInit :: CodeQ s
  , pStep :: forall w. CodeQ s
          -> (CodeQ d -> CodeQ r -> CodeQ s -> CodeQ w)   -- ^ yield
          -> (CodeQ s -> CodeQ w)                         -- ^ skip
          -> CodeQ w                                      -- ^ done
          -> CodeQ w
  }

-- | Linear, or an outer stream each of whose rows opens an inner stream.
-- Composition is flat-map and flat-map does not fit in a single producer, which
-- is the same reason this split exists in design/PullStreams.hs.
--
-- The inner side is an `SStream`, not a `Producer`, because it has to be closed
-- under nesting: a PROBED composition is itself nested iteration, which is what
-- @probe a x (\\y -> probe b y k)@ in "Prela.Ops" says.
--
-- The continuation takes the outer KEY as well as the outer value. `compose`
-- ignores the key and `prod` ignores the value, but both are needed — `prod`
-- probes its right-hand side at the key, not at the value. The inner stream is
-- keyed by @()@ because its keys are never looked at; the pair the consumer
-- eventually sees is the outer key with the inner value.
data SStream d r
  = Lin (Producer d r)
  | forall e. Bind (SStream d e) (CodeQ d -> CodeQ e -> SStream () r)

-- | A relation in probed position: given a key, the values at that key.
newtype Cursor d r = Cursor { at :: CodeQ d -> SStream () r }

--------------------------------------------------------------------------------
-- Leaves
--------------------------------------------------------------------------------

-- | The entity universe. State is the current id; it never skips.
--
-- The leaves come in a `Producer` form as well as an `SStream` one. Only a
-- producer can be zipped — `zipWithP` needs to own both states — and `Lin`
-- forgets which streams are still flat, so the distinction has to be visible in
-- the type or the restriction is unenforceable.
universeProd :: CodeQ Int -> Producer Int Int
universeProd n = Producer
  { pInit = [|| 0 ||]
  , pStep = \s yield _skip done ->
      [|| if $$s >= $$n then $$done else $$(yield s s [|| $$s + 1 ||]) ||]
  }

universeP :: CodeQ Int -> SStream Int Int
universeP = Lin . universeProd

-- | A dense 1:1 column, driven.
columnProd :: CodeQ (U.Vector Int) -> Producer Int Int
columnProd v = Producer
  { pInit = [|| 0 ||]
  , pStep = \s yield _skip done ->
      [|| if $$s >= U.length $$v
            then $$done
            else $$(yield s [|| $$v U.! $$s ||] [|| $$s + 1 ||]) ||]
  }

columnP :: CodeQ (U.Vector Int) -> SStream Int Int
columnP = Lin . columnProd

-- | Rejoin the ordinary pipeline, giving up the ability to zip.
linear :: Producer d r -> SStream d r
linear = Lin

-- | A dense 1:1 column in probed position: at most one value, and none at all
-- for a key outside the column. Same guard, same reason, as `csrCursor`.
colCursor :: CodeQ (U.Vector Int) -> Cursor Int Int
colCursor v = Cursor $ \key -> Lin Producer
  { pInit = [|| $$key ||]
  , pStep = \s yield _skip done ->
      [|| if $$s < 0 || $$s >= U.length $$v
            then $$done
            else $$(yield [|| () ||] [|| $$v U.! $$s ||] [|| -1 ||]) ||]
  }

-- | A CSR column in probed position: key i owns values[offs[i] .. offs[i+1]-1].
--
-- The state is the current offset paired with the row's end, so the end is read
-- once in `pInit` rather than on every step — a producer cannot hold a
-- loop-invariant anywhere else, since its two fields are spliced into two
-- different places.
--
-- The range guard is not defensive, it is the semantics. A probed key is
-- untrusted: a foreign key column has holes, and "Prela.Cache" spells a hole
-- @noId = -1@. The answer for a hole is NO VALUES, not a crash and not a wild
-- read, which is why "Prela.Ops" writes @when (0 <= i && i < n)@ around every
-- probed leaf. Bounds-checked indexing alone would not give that — it would
-- throw where the engine yields nothing. An out-of-range key gets the empty
-- range @(0, 0)@ here, which is that answer.
csrCursor :: CodeQ (U.Vector Int) -> CodeQ (U.Vector Int) -> Cursor Int Int
csrCursor offs vals = Cursor $ \key -> Lin Producer
  { pInit = [|| let os = $$offs
                    k  = $$key
                in if k < 0 || k + 1 >= U.length os
                     then (0, 0)
                     else (os U.! k, os U.! (k + 1)) ||]
  , pStep = \s yield _skip done ->
      [|| case $$s of
            (j, e) | j >= e    -> $$done
                   | otherwise -> $$(yield [|| () ||] [|| $$vals U.! j ||]
                                           [|| (j + 1, e) ||]) ||]
  }

--------------------------------------------------------------------------------
-- Operators
--------------------------------------------------------------------------------

-- | Chain through a shared middle value: the outer's value keys the inner.
compose :: SStream d e -> Cursor e r -> SStream d r
compose a b = Bind a (\_ e -> at b e)

-- | Both legs at the same key, paired. The right-hand side is probed at the
-- KEY, which is why `Bind`'s continuation is handed the key at all.
-- The two bangs are load bearing, and they were measured. The pair itself always
-- cancels — it is built and immediately taken apart, so GHC's
-- case-of-constructor rule removes it even four deep. What does not cancel are
-- its COMPONENTS. Each is an array read, and a checked array read has a
-- bottoming branch, so GHC will not speculate it; left lazy it becomes a boxed
-- `Int` thunk built once per outer row and forced somewhere inside the inner
-- loop. Two of those is 64 bytes a row on a three-deep tower, which is Q1's
-- shape. The bangs turn each one into a `case` on an unboxed `Int#` at the point
-- the pair is built, which a join point can carry in a register.
--
-- Forcing is not a semantic change for anything Prela can build: a relation's
-- values are array reads, and every consumer is a strict fold.
prod :: SStream d a -> Cursor d b -> SStream d (a, b)
prod a b = Bind a (\d u -> mapvS (\v -> [|| let !x = $$u
                                                !y = $$v
                                            in (x, y) ||]) (at b d))

-- | Keep rows whose VALUE the cursor has an entry for. This is where early
-- termination lands: `anyOf` stops the inner loop at the first yield.
restrict :: SStream d r -> Cursor r x -> SStream d r
restrict a b = filtKV (\_ r -> anyOf (at b r)) a

-- | Keep rows whose KEY the cursor has no entry for. SQL's @IS NULL@.
diff :: SStream d r -> Cursor d x -> SStream d r
diff a b = filtKV (\d _ -> [|| not $$(anyOf (at b d)) ||]) a

-- | A filter on the value alone.
filtS :: (CodeQ r -> CodeQ Bool) -> SStream d r -> SStream d r
filtS t = filtKV (\_ r -> t r)

-- | A filter with the key in scope too.
--
-- The nested case is where the key comes from: an inner stream is keyed by
-- @()@, so a predicate that mentions the key has to capture the OUTER key at
-- generation time, which the `Bind` continuation makes available.
filtKV :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> SStream d r -> SStream d r
filtKV t (Lin p)    = Lin (filtP t p)
filtKV t (Bind o g) = Bind o (\d e -> filtKV (\_ r -> t d r) (g d e))

-- | The Skip tax, and the reason it is affordable here. A rejected row emits
-- `skip`, which in a fold is a jump back to the loop head, so it costs a branch
-- rather than the `Step` allocation and unknown call it costs unstaged.
--
-- The `let !x` is not decoration. `r` is code, and code used twice is emitted
-- twice: without the binding, @filtS (> 1980) (columnP v)@ would emit the array
-- read once in the test and again in the yield. GHC's common-subexpression pass
-- would probably clean that up, but "probably" is not what this file is for.
-- strymonas inserts these bindings automatically with `genlet`; here every
-- operator that uses its value twice binds it by hand.
filtP :: (CodeQ d -> CodeQ r -> CodeQ Bool) -> Producer d r -> Producer d r
filtP t (Producer i step) = Producer
  { pInit = i
  , pStep = \s yield skip done ->
      step s
        (\d r s' -> [|| let !x = $$r
                        in if $$(t d [|| x ||])
                             then $$(yield d [|| x ||] s')
                             else $$(skip s') ||])
        skip done
  }

-- | A filter that keeps the stream flat, so the result can still be zipped.
filtProd :: (CodeQ r -> CodeQ Bool) -> Producer d r -> Producer d r
filtProd t = filtP (\_ r -> t r)

mapvS :: (CodeQ r -> CodeQ r') -> SStream d r -> SStream d r'
mapvS h (Lin p)    = Lin (mapvP h p)
mapvS h (Bind o g) = Bind o (\d e -> mapvS h (g d e))

mapvP :: (CodeQ r -> CodeQ r') -> Producer d r -> Producer d r'
mapvP h (Producer i step) = Producer
  { pInit = i
  , pStep = \s yield skip done -> step s (\d r s' -> yield d (h r) s') skip done
  }

--------------------------------------------------------------------------------
-- Consumers
--------------------------------------------------------------------------------

-- | Fold while the accumulator says to keep going. Every other consumer is this
-- one, and so is early termination: the guard is tested at each level of the
-- loop nest, so a nested stream stops as promptly as a flat one.
--
-- `go` is where the three continuations land, so each splices into a single
-- call rather than into a copy of the loop body. Without that, every operator
-- in the chain would duplicate its continuation once per branch and the emitted
-- code would grow exponentially in the length of the chain. This is the
-- let-insertion problem again; here the consumer's own loop binder solves it.
foldWhileP :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> Producer d r -> CodeQ acc
foldWhileP p f z (Producer i step) =
  [|| let go !s !acc
            | not $$(p [|| acc ||]) = acc
            | otherwise =
                $$(step [|| s ||]
                     (\d r s' -> [|| go $$s' $$(f [|| acc ||] d r) ||])
                     (\s'     -> [|| go $$s' acc ||])
                     [|| acc ||])
      in go $$i $$z ||]

-- | The loop nest. The inner fold carries the same accumulator and the same
-- guard as the outer, which is what makes an early exit propagate outwards
-- rather than only ending the innermost loop.
--
-- The @let !v@ is load bearing, and it was measured. The outer row's value has
-- to stay live across the whole inner loop. Left to itself GHC keeps it as a
-- LAZY binding, because a checked array read has a bottoming branch and GHC will
-- not speculate one, so what crosses the loop boundary is a boxed `Int` thunk
-- built once per outer row. Forcing it turns the binding into a `case` on an
-- unboxed `Int#`, which a join point can carry in a register.
--
-- On its own this bang changes nothing. It only pays off together with the
-- matching pair of bangs in `prod`, and only from three levels of nesting up.
-- See the measurements at the bottom of design/StagedPullMain.hs.
--
-- Forcing is not a semantic change for anything Prela can build: a relation's
-- values are array reads, and every consumer is a strict fold.
--
-- The KEY is spliced rather than bound, in two places. That is safe only because
-- every leaf here hands back a key that is already a variable — a loop counter,
-- or @()@ — so emitting it twice emits two variable references and costs
-- nothing. A leaf whose key were a real expression would compute it twice, and
-- the general answer to that is strymonas's `genlet`, which inserts a binding
-- only where the code is actually reached. Binding it unconditionally is not the
-- answer: a consumer that ignores the key (`count` does) then gets a dead
-- binding and a -Wunused-local-binds warning in every generated query.
sfoldWhile :: (CodeQ acc -> CodeQ Bool)
           -> (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
           -> CodeQ acc -> SStream d r -> CodeQ acc
sfoldWhile p f z (Lin q)    = foldWhileP p f z q
sfoldWhile p f z (Bind o g) =
  sfoldWhile p
    (\acc d e ->
       [|| let !v = $$e
           in $$(sfoldWhile p (\acc' _ r -> f acc' d r) acc (g d [|| v ||])) ||])
    z o

-- | Fold to exhaustion. The guard is the constant `True`, which the simplifier
-- deletes before it reaches the generated loop.
sfold :: (CodeQ acc -> CodeQ d -> CodeQ r -> CodeQ acc)
      -> CodeQ acc -> SStream d r -> CodeQ acc
sfold = sfoldWhile (\_ -> [|| True ||])

count :: SStream d r -> CodeQ Int
count = sfold (\acc _ _ -> [|| $$acc + 1 ||]) [|| 0 ||]

-- | Membership, which is early termination in its smallest form: stop at the
-- first row that arrives.
--
-- This is the whole of `probeAny`. The push engine writes it as a second copy
-- of every operator in the `Prb` instance, because a push consumer has no way
-- to stop and the short-circuit has to be built into each operator by hand.
anyOf :: SStream d r -> CodeQ Bool
anyOf = sfoldWhile (\a -> [|| not $$a ||]) (\_ _ _ -> [|| True ||]) [|| False ||]

-- | The first n rows, and then stop. Not expressible against `Drv` at all.
limit :: CodeQ Int -> SStream d r -> CodeQ [(d, r)]
limit n s = [|| reverse (fst $$(collectN n s)) ||]

collectN :: CodeQ Int -> SStream d r -> CodeQ ([(d, r)], Int)
collectN n = sfoldWhile (\a -> [|| snd $$a < $$n ||])
                        (\a d r -> [|| case $$a of (xs, k) -> (($$d, $$r) : xs, k + 1) ||])
                        [|| ([], 0 :: Int) ||]

collect :: SStream d r -> CodeQ [(d, r)]
collect s = [|| reverse $$(sfold (\a d r -> [|| ($$d, $$r) : $$a ||]) [|| [] ||] s) ||]

--------------------------------------------------------------------------------
-- Lockstep
--------------------------------------------------------------------------------

-- | Advance two producers in lockstep. This is the operator the push `Drv`
-- cannot express at any cost, and it works on FILTERED streams here, which is
-- what `SProd` in design/Staged.hs cannot do — filtering destroys
-- addressability, and an addressable range was all `SProd` had.
--
-- No element is buffered. When the left yields and the right skips, the left's
-- OLD state is kept, so next time round the left re-steps and produces the same
-- element again. That is sound because a step is a pure function of its state,
-- and it trades the `Maybe` that design/PullStreams.hs carried for re-running
-- the left's step — for a filter, re-testing one predicate.
--
-- `done` is spliced three times. That is free in practice because every
-- consumer here passes a variable for it (`foldWhileP` passes @acc@), but a
-- consumer passing an expression would triple it.
--
-- Both keys cannot be kept without building a pair per row, so the left's key
-- wins. That is the right default for Prela, where the left is the driving
-- relation and owns the key. Two NESTED streams cannot be zipped, and that is
-- not typeable rather than silently slow: strymonas's Theorem 1, same carve-out.
zipWithP :: (CodeQ a -> CodeQ b -> CodeQ c)
         -> Producer d a -> Producer e b -> Producer d c
zipWithP h (Producer i1 st1) (Producer i2 st2) = Producer
  { pInit = [|| ($$i1, $$i2) ||]
  , pStep = \s yield skip done ->
      [|| case $$s of
            (a, b) ->
              $$(st1 [|| a ||]
                   (\d x a' ->
                      st2 [|| b ||]
                        (\_ y b' -> yield d (h x y) [|| ($$a', $$b') ||])
                        (\b'     -> skip [|| (a, $$b') ||])
                        done)
                   (\a' -> skip [|| ($$a', b) ||])
                   done) ||]
  }
