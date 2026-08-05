{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The staged engine: combinators that GENERATE the
-- loop rather than hoping GHC recovers one. Used from design/StagedMain.hs,
-- which must be a separate module — TH's stage restriction means a splice
-- cannot see generators defined in its own file.
--
-- Two things change relative to design/PullStreams.hs, and both are
-- simplifications.
--
-- There is no `Skip`. `Skip` exists in an unstaged pull stream because a filter
-- has no loop of its own to continue. Here the filter emits an `if` into the
-- loop body the producer is generating, so a rejected row is a branch, not a
-- step.
--
-- There is no existential state. That was the thing GHC could not see through
-- in the unstaged version — a step function hidden behind `forall s.` is an
-- unknown call on every row. Here the state is whatever local variable the
-- generated loop happens to bind, and it never exists as a type at all.
--
-- The representation is the fold: a producer is given code for what to do with
-- the accumulator and each pair, and returns code for the whole loop. The
-- accumulator being a loop ARGUMENT rather than a mutable cell is what lets GHC
-- unbox it, which is the same lesson FUSION.md records for `Acc`, except here
-- it is unavoidable by construction rather than a trap to fall into.
--
-- IT FUSES. Measured the way FUSION.md measures the push engine, on the query in
-- design/StagedMain.hs: a million movies with two keywords each, filtered and
-- counted. The generated Core is
--
--   $wgo :: Int# -> Int# -> Int#
--
-- with the inner loop a `joinrec $wgo2 :: Int# -> Int# -> Int#` reached by
-- `jump`, reading through `indexIntArray#`, and one `I#` at the very end to box
-- the answer. No `Step`, no unknown calls, no per-row boxing — the three things
-- design/PullStreams.hs could not get rid of. It is the same code the
-- hand-written loop in `byHand` compiles to, and slightly tighter, since the
-- inner loop closes over the row's end offset instead of taking it as an
-- argument. Total heap allocation for the three queries is 40,586,480 bytes,
-- which is exactly the seven input vectors: the queries allocate nothing.
--
-- Indexing is CHECKED throughout, deliberately, and it is not what costs. Timed
-- against a variant with every `U.!` replaced by `U.unsafeIndex`, twenty rounds
-- each: 0.667 against 0.667 ms for the count over two million rows, 0.099
-- against 0.101 ms for the merge join. The check compiles to a single `ltWord#`
-- against a length that is already loaded, with the failure branch a cold call
-- that bottoms, so it predicts perfectly and never leaves the loop.
-- "Prela.Storage" leaves `atStore` unchecked for a different reason — it is one
-- accessor shared by leaves that each know their own bound — and pairs it with
-- an explicit range test at every probed leaf. Either discipline is sound. What
-- is not sound is the unchecked read WITHOUT that test, which is what this file
-- did at first; see `csrCursor`.
--
-- The bottom half of this file is lockstep, which staging alone does not buy —
-- see `SProd`.
module Staged
  ( SDrv (..)
  , universe
  , column
  , csrCursor
  , compose
  , filt
  , mapv
  , foldAll
  , count
  , SProd (..)
  , sortedRun
  , runProd
  , mergeJoinWith
  ) where

import qualified Data.Vector.Unboxed as U
import Language.Haskell.TH (CodeQ)

-- | A relation, as a generator of loops. Give it code for the step and code for
-- the initial accumulator, and it hands back code for the entire loop nest.
newtype SDrv d r = SDrv
  { sfold :: forall s. (CodeQ s -> CodeQ d -> CodeQ r -> CodeQ s) -> CodeQ s -> CodeQ s }

--------------------------------------------------------------------------------
-- Leaves
--------------------------------------------------------------------------------

-- | The entity universe: a counted loop from 0.
universe :: CodeQ Int -> SDrv Int Int
universe n = SDrv $ \f z ->
  [|| let go !i !acc = if i >= $$n then acc else go (i + 1) $$(f [||acc||] [||i||] [||i||])
      in go 0 $$z ||]

-- | A dense 1:1 column, driven.
column :: CodeQ (U.Vector Int) -> SDrv Int Int
column v = SDrv $ \f z ->
  [|| let vec = $$v
          n   = U.length vec
          go !i !acc
            | i >= n    = acc
            | otherwise = go (i + 1) $$(f [||acc||] [||i||] [||vec U.! i||])
      in go 0 $$z ||]

-- | A CSR column in probed position: key i owns values[offs[i] .. offs[i+1]-1].
-- The bound is read once, before the loop — the thing the unstaged pull version
-- had to re-derive on every row because it could not hold it anywhere.
--
-- The range guard is not defensive, it is the semantics. A probed key is
-- untrusted: a foreign key column has holes, and `Prela.Cache` spells a hole
-- @noId = -1@. The answer for a hole is NO VALUES, not a crash and not a wild
-- read, which is why "Prela.Push.Ops" writes @when (0 <= i && i < n)@ around every
-- probed leaf. Bounds-checked indexing alone would not give that — it would
-- throw where the engine yields nothing.
--
-- Two things here are about staging rather than about CSR.
--
-- `z0` exists because splicing `$$z` under both branches would duplicate
-- whatever code the consumer passed for the initial accumulator. A `CodeQ` used
-- twice is code emitted twice, so anything non-trivial has to be let-bound
-- exactly once — the staging equivalent of not evaluating an argument twice.
--
-- `$$vals` is spliced at the point of use for the opposite reason. Binding it up
-- front reads better, but a consumer that ignores the value — `count` does —
-- then never mentions the array, and the generated program has a dead binding
-- GHC warns about. A generator cannot know whether its consumer will use what it
-- produces, so it can neither always bind nor always inline; strymonas resolves
-- this with a `genlet` that inserts the binding only if the code is reached. The
-- expedient here is that `$$vals` is a variable reference, so duplicating it is
-- free. A caller passing an expression should bind it first.
csrCursor :: CodeQ (U.Vector Int) -> CodeQ (U.Vector Int) -> CodeQ Int -> SDrv () Int
csrCursor offs vals i = SDrv $ \f z ->
  [|| let os  = $$offs
          key = $$i
          z0  = $$z
      in if key < 0 || key + 1 >= U.length os
           then z0
           else let end = os U.! (key + 1)
                    go !j !acc
                      | j >= end  = acc
                      | otherwise = go (j + 1) $$(f [||acc||] [||()||] [||$$vals U.! j||])
                in go (os U.! key) z0 ||]

--------------------------------------------------------------------------------
-- Operators
--------------------------------------------------------------------------------

-- | Chain through a shared middle value. The inner generator is invoked while
-- generating the outer loop's body, so what comes out is a literal loop nest.
compose :: SDrv d e -> (CodeQ e -> SDrv () f) -> SDrv d f
compose a g = SDrv $ \f z ->
  sfold a (\acc x y -> sfold (g y) (\acc' _ v -> f acc' x v) acc) z

-- | A filter is an `if` in the loop body. No Skip, no extra state.
filt :: (CodeQ r -> CodeQ Bool) -> SDrv d r -> SDrv d r
filt t a = SDrv $ \f z ->
  sfold a (\acc x y -> [|| if $$(t y) then $$(f acc x y) else $$acc ||]) z

mapv :: (CodeQ r -> CodeQ s) -> SDrv d r -> SDrv d s
mapv h a = SDrv $ \f z -> sfold a (\acc x y -> f acc x (h y)) z

--------------------------------------------------------------------------------
-- Consumers
--------------------------------------------------------------------------------

foldAll :: (CodeQ s -> CodeQ r -> CodeQ s) -> CodeQ s -> SDrv d r -> CodeQ s
foldAll op z a = sfold a (\acc _ v -> op acc v) z

count :: SDrv d r -> CodeQ Int
count = foldAll (\acc _ -> [|| $$acc + 1 ||]) [|| 0 ||]

--------------------------------------------------------------------------------
-- Lockstep
--------------------------------------------------------------------------------

-- | An `SDrv` owns its loop, so two of them cannot be advanced against each
-- other — that was the original complaint about push, and generating the loop
-- does not fix it. What fixes it is saying which relations are ADDRESSABLE: a
-- half-open index range plus a way to read the key and the value at an index.
-- Something in that shape can be advanced one element at a time by whoever is
-- generating the loop, because "where am I" is a plain Int the generator can put
-- in a loop argument.
--
-- Every leaf Prela actually has fits: a dense column is 0..n, a CSR row is
-- offs[i]..offs[i+1], a sorted run is its own extent. What does NOT fit is
-- anything downstream of a compose or a filter, which is the same restriction
-- strymonas calls linearity — and the same reason its Theorem 1 says zipping two
-- flat-mapped streams has to allocate. Here that case is simply not typeable, so
-- it is a compile error rather than a silent deoptimisation.
data SProd d r = SProd
  { pStart :: CodeQ Int
  , pEnd   :: CodeQ Int
  , pKey   :: CodeQ Int -> CodeQ d
  , pVal   :: CodeQ Int -> CodeQ r
  }

-- | A run of (key, value) pairs held in parallel arrays, sorted by key. The
-- extent is the shorter of the two, so a mismatched pair of arrays truncates
-- rather than reading past the end of the values.
sortedRun :: CodeQ (U.Vector Int) -> CodeQ (U.Vector Int) -> SProd Int Int
sortedRun ks vs = SProd
  { pStart = [|| 0 ||]
  , pEnd   = [|| min (U.length $$ks) (U.length $$vs) ||]
  , pKey   = \i -> [|| $$ks U.! $$i ||]
  , pVal   = \i -> [|| $$vs U.! $$i ||]
  }

-- | Forget addressability and rejoin the ordinary pipeline.
runProd :: SProd d r -> SDrv d r
runProd p = SDrv $ \f z ->
  [|| let end = $$(pEnd p)
          go !i !acc
            | i >= end  = acc
            | otherwise = go (i + 1) $$(f [||acc||] (pKey p [||i||]) (pVal p [||i||]))
      in go $$(pStart p) $$z ||]

-- | A merge join: one loop, two cursors, neither side buffered. This is the
-- thing push cannot express at all and unstaged pull expresses but does not
-- compile well. The combining function is applied inside the loop so no pair is
-- ever built.
--
-- Equal keys are consumed one-for-one rather than crossed, so it is a join only
-- when at least one side is unique in the key. Duplicates on both sides need the
-- usual inner rescan, which this prototype does not do.
mergeJoinWith
  :: (CodeQ a -> CodeQ b -> CodeQ c) -> SProd Int a -> SProd Int b -> SDrv Int c
mergeJoinWith h p q = SDrv $ \f z ->
  [|| let pe = $$(pEnd p)
          qe = $$(pEnd q)
          go !i !j !acc
            | i >= pe || j >= qe = acc
            | otherwise =
                let ki = $$(pKey p [||i||])
                    kj = $$(pKey q [||j||])
                in case compare ki kj of
                     LT -> go (i + 1) j acc
                     GT -> go i (j + 1) acc
                     EQ -> go (i + 1) (j + 1)
                             $$(f [||acc||] [||ki||] (h (pVal p [||i||]) (pVal q [||j||])))
      in go $$(pStart p) $$(pStart q) $$z ||]
