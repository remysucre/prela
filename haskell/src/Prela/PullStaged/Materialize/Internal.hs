{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The operators that stop the pipeline.
--
-- Everything else in the port emits a loop. These are the exceptions: each one
-- consumes its input stream once into real storage and hands back something that
-- reads the storage. A query that names something from here is a query that
-- allocates, and Prela is non-materialized by default. The public operations
-- keep those boundaries explicit in query source.
--
-- Two details follow directly from the pull representation.
--
-- THE ACCUMULATOR IS LOOP STATE. The consumer generates the loop, so a table
-- under construction is a loop argument rather than a mutable reference — see
-- @sfoldST@ in "Prela.PullStaged.Stream". Carrying it directly avoids a
-- per-row `STRef` allocation.
--
-- THE SHAPE IS A CONTINUATION, not a return. This is the one rule of staging:
-- a `CodeQ` used twice is code emitted twice. @fold q@ returning a relation
-- would emit the whole table build at every use site, which is precisely what a
-- materializer exists to prevent. So each one here takes the rest of the query as
-- a function and binds the storage once:
--
-- > withFold step [|| 0 ||] revenue $ \total ->
-- >   [|| ($$(collect (compose (universe n) total)), $$(count total)) ||]
--
-- @total@ arrives polymorphic in the mode, so the body may enumerate it, look up
-- values by key, or both, and every use reads the same runtime binding. That is
-- strymonas's @genlet@ cut down to the one shape Prela needs. The build cannot
-- be duplicated because what is polymorphic is the leaf wrapper and what is
-- shared is the storage underneath it.
module Prela.PullStaged.Materialize.Internal where

import Control.Monad (forM_, when)
import Control.Monad.ST (ST, runST)
import Data.Array.ST (freeze, runSTUArray, writeArray)
import Data.Bits ((.&.))
import Data.Hashable (Hashable)
import Data.List (sortBy)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Vector as BV
import qualified Data.Vector.Mutable as BMV
import qualified Data.Vector.Unboxed as UV
import qualified Data.Vector.Unboxed.Mutable as UMV
import Language.Haskell.TH (CodeQ)

import Prela.Id
import Prela.PullStaged.Ops
import Prela.PullStaged.Stream.Internal
import Prela.Storage.Internal

--------------------------------------------------------------------------------
-- Bounded order
--------------------------------------------------------------------------------

-- | Retain only the best @n@ rows according to a generated comparator, then
-- enumerate those rows in comparator order. The mutable buffer is bounded by
-- @n@; candidates which cannot enter it never acquire a retained tuple. A
-- linear worst-row search is deliberate: TPC-H limits are 10, 20, or 100, so it
-- is cheaper and simpler than allocating a general-purpose heap structure.
withTopK
  :: CodeQ Int
  -> (CodeQ (d, r) -> CodeQ (d, r) -> CodeQ Ordering)
  -> Stream d r
  -> (Stream d r -> CodeQ w)
  -> CodeQ w
withTopK requested compareRows rows continue =
  [|| let !topRows = runST (do
            let !capacity = max 0 $$requested
                comparePair leftRow rightRow =
                  $$(compareRows [|| leftRow ||] [|| rightRow ||])
                findWorst buffer !rowCount =
                  let go !cursor !worst
                        | cursor >= rowCount = return worst
                        | otherwise = do
                            candidate <- BMV.unsafeRead buffer cursor
                            incumbent <- BMV.unsafeRead buffer worst
                            go (cursor + 1)
                              (if comparePair candidate incumbent == GT
                                 then cursor else worst)
                  in go 1 0
            buffer <- BMV.new capacity
            kept <- $$(sfoldST
              (\rowCount key value ->
                [|| if $$rowCount < capacity
                      then BMV.unsafeWrite buffer $$rowCount ($$key, $$value)
                           >> return ($$rowCount + 1)
                      else if capacity == 0
                        then return $$rowCount
                        else do
                          worst <- findWorst buffer $$rowCount
                          incumbent <- BMV.unsafeRead buffer worst
                          if $$(compareRows [|| ($$key, $$value) ||]
                                            [|| incumbent ||]) == LT
                            then BMV.unsafeWrite buffer worst ($$key, $$value)
                                   >> return $$rowCount
                            else return $$rowCount ||])
              [|| 0 :: Int ||] rows)
            frozen <- BV.freeze (BMV.slice 0 kept buffer)
            return (sortBy comparePair (BV.toList frozen)))
      in $$(continue (fromList [|| topRows ||])) ||]

--------------------------------------------------------------------------------
-- Map-backed
--------------------------------------------------------------------------------

-- | Consume a stream once and bucket its pairs by key.
--
-- Each group comes out in reverse stream order, because @insertWith (++)@ conses.
-- `withBufFold` is the only thing that cares and it reverses; nothing else has
-- ever promised an order.
index :: Ord d => Stream d r -> CodeQ (Map d [r])
index = sfold (\m d r -> [|| Map.insertWith (++) $$d [$$r] $$m ||]) [|| Map.empty ||]

-- | Force a leg once so reuse is free. Same pairs, now backed by the index.
withMaterialize :: Ord d
                => Stream d r
                -> ((forall q. Mode q => q d r) -> CodeQ w) -> CodeQ w
withMaterialize s k =
  [|| let !m = $$(index s) in $$(k (fromIndex [|| m ||])) ||]

-- | A materialized inverse: index the flipped pairs so the result supports both
-- enumeration and keyed access. `invStream` alone is free but only supports
-- enumeration.
withInv :: Ord r
        => Stream d r
        -> ((forall q. Mode q => q r d) -> CodeQ w) -> CodeQ w
withInv s k = withMaterialize (invStream s) k

-- | The whole-group fold: instead of reducing pairwise, hand the reducer the
-- entire group at once. For anything that does not fit `withFold`'s shape, which
-- is to say anything needing to see the group twice, or in order: count-distinct,
-- median, a range between the extremes.
--
-- It costs strictly more than `withFold`, since every value is retained until the
-- group is complete rather than collapsing into an accumulator, so reach for
-- `withFold` unless the reducer genuinely cannot be written that way. The list
-- each group is handed is in stream order.
withBufFold :: Ord d
            => (CodeQ [r] -> CodeQ t) -> Stream d r
            -> ((forall q. Mode q => q d t) -> CodeQ w) -> CodeQ w
withBufFold f s k =
  [|| let !m = Map.map (\xs -> $$(f [|| reverse xs ||])) $$(index s)
      in $$(k (fromCache [|| m ||])) ||]

--------------------------------------------------------------------------------
-- The open-addressed table
--------------------------------------------------------------------------------

-- | Group by key and reduce each group. A fold cannot emit groups as it reads
-- them because reducing a group means seeing the whole group, so it consumes
-- the input once into a cache. The cache holds ONE value per key.
-- @withFold (\n _ -> [|| $$n + 1 ||]) [|| 0 ||]@ is
-- COUNT. What comes back is an ordinary relation and composes like one.
--
-- @op@ is a generation-time function, so the reducer is emitted into the insert
-- rather than called through a closure.
withFold :: (Hashable d, Key d, UV.Unbox t)
         => (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t -> Stream d r
         -> ((forall q. Mode q => q d t) -> CodeQ w) -> CodeQ w
withFold op ini s k =
  [|| let !tbl = $$(buildTable op ini s) in $$(k (fromTable [|| tbl ||])) ||]

-- | Consume the stream once, reducing into an open-addressed table.
--
-- Kept private because its result is the executor's hash-table representation;
-- `withFold` and `withCountDistinct` are the query-level construction boundary.
buildTable :: (Hashable d, Key d, UV.Unbox t)
           => (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t -> Stream d r
           -> CodeQ (Table d t)
buildTable op ini s =
  [|| runST (do
        let !z = $$ini
        cnt <- UMV.replicate 1 (0 :: Int)
        vsc <- BMV.new 1
        mt0 <- newMTable cnt vsc 64 z
        done <- $$(sfoldST
                     (\acc d r ->
                        [|| insertTable
                              (\_current -> $$(op [|| _current ||] r))
                              z $$d $$acc ||])
                     [|| mt0 ||] s)
        case done of
          MTable mask _ hs ks _ -> do
            vs <- BMV.read vsc 0
            Table mask <$> UV.freeze hs <*> freezeKeys ks <*> UV.freeze vs) ||]

-- The mutable table under construction: the same three stores as `Table`, plus
-- the mask and a one-slot count. Passed and returned by value — growth replaces
-- all of it at once, and under pull there is somewhere to put the replacement.
--
-- Two of the five fields are one-element mutable vectors rather than plain
-- values, and both are there for the same reason. This record is the accumulator
-- `sfoldST` threads through the loop, so GHC unboxes it into loop arguments; the
-- moment the loop WRITES through one of those arguments it has to rebuild the
-- record to pass it on, and a rebuild is an allocation on every row.
--
-- The COUNT would be written on every fresh key. The VALUE STORE is written on
-- every row, so it is the expensive one: threading it directly cost 32 bytes a
-- row, measured at 193 MB for one grouped fold over the 6M lineitems. Behind a
-- cell the loop only READS the field, the record it was projected from is passed
-- along untouched, and nothing is allocated.
--
-- Both cells have to survive growth, which is why `newMTable` is handed them
-- rather than making them: growth writes the new value store into the same cell,
-- so that field never changes and only the mask, hashes and keys are replaced.
--
-- EVERY STORE FIELD IS LAZY, and those missing bangs are the same problem seen
-- from the other side. The record is what gets unboxed, so the fix above stops
-- the loop rebuilding it; the fields are the pieces that unboxing hands out, and
-- a strict one gets taken apart too, then rebuilt wherever a branch wants it
-- whole again.
--
-- The key store showed it worst, because a compound key stores its columns side
-- by side: `MKeys` for @((Id Order, Int), Int)@ is @MPairKeys (MPairKeys _ _) _@,
-- a record holding a record, which GHC unboxed two levels deep and then rebuilt
-- to reach the insert. The rebuild was hoisted above the branch, so it ran on
-- every input row whether or not that row survived the filter: 24 bytes for each
-- of the 6M lineitems Q3 scans, 148 MB, to fill a table of 11,620 groups. The
-- hash vector was the same thing one size down, an @MVector@ rebuilt on every
-- fresh key, which is Q18's 1.5M orders at 32 bytes each.
--
-- Lazy, each field is one pointer that is passed along and never taken apart.
-- Q3 went from 148 MB to 4.7 MB, the suite from 9.16 GB to 8.67 GB, and the wall
-- time did not move. Nothing is left unevaluated by this: `newMTable` binds every
-- store with @<-@ before the record is built.
data MTable s d t =
  MTable !Int (UMV.MVector s Int) (UMV.MVector s Word) (MKeys s d)
         (BMV.MVector s (UMV.MVector s t))

newMTable :: (Key d, UV.Unbox t)
          => UMV.MVector s Int -> BMV.MVector s (UMV.MVector s t) -> Int -> t
          -> ST s (MTable s d t)
newMTable cnt vsc cap ini = do
  hs <- UMV.replicate cap 0
  ks <- newKeys cap
  vs <- UMV.replicate cap ini
  BMV.write vsc 0 vs
  return (MTable (cap - 1) cnt hs ks vsc)
{-# INLINE newMTable #-}

-- | Reduce one pair into the table, growing it if that pushed it past three
-- quarters full. Linear probing degrades badly past that, and every rehash is
-- paid back over the rows that follow it, so the doubling costs O(n) in total
-- across the whole build.
--
-- @f@ is the reducer already applied to the incoming value, so a fresh slot gets
-- @f ini@ and an occupied one gets @f old@. It is a runtime function only in this
-- signature: the caller passes a literal lambda and `INLINE` puts it back where
-- it came from.
--
-- The body is in three parts, and the split is load bearing rather than stylistic.
-- `probe` walks the collision chain and returns nothing but a slot index; then
-- the old value is fetched; then the reducer runs, ONCE. Written the obvious way
-- — one loop that reduces in whichever branch it lands in — the reducer appears
-- at two sites, both inside a recursive function. That has two costs, and Q1 paid
-- both. GHC has to keep the incoming value alive across the loop, so the pair
-- `prod` built cannot cancel against the pattern match that takes it apart; and
-- with two use sites it will not duplicate the work that produced it, so every
-- column read the value came from is left behind as a per-row thunk. Q1's four
-- doubles, their four bounds checks and the group key all landed there — about
-- 800 bytes a row.
--
-- With one use site outside the loop, the reducer's argument is produced and
-- consumed in straight line code, so case-of-constructor cancels the pair and the
-- reads inline into the loop as raw `Double#`.
insertTable :: (Hashable d, Key d, UV.Unbox t)
            => (t -> t) -> t -> d -> MTable s d t -> ST s (MTable s d t)
insertTable f ini d mt@(MTable mask cnt hs ks vsc) = do
  i <- probe (fromIntegral h .&. mask)
  h' <- UMV.read hs i
  let !fresh = h' == 0
  vs <- BMV.read vsc 0
  old <- if fresh then return ini else UMV.read vs i
  UMV.write vs i (f old)
  if fresh
    then do
      UMV.write hs i h
      writeKey ks i d
      n <- (+ 1) <$> UMV.read cnt 0
      UMV.write cnt 0 n
      if 4 * n > 3 * (mask + 1) then growTable mt ini else return mt
    else return mt
  where
    h = slotHash d
    -- Check the compact hash store before reading a potentially compound key.
    probe i = do
      h' <- UMV.read hs i
      if h' == 0
        then return i
        else if h' == h
          then do
            matches <- matchesKey ks i d
            if matches then return i else advance i
          else advance i
    advance i = probe ((i + 1) .&. mask)
{-# INLINE insertTable #-}

-- | Rehash into a table of twice the capacity. Only occupied slots move, and
-- each lands on the first free slot of its new probe sequence, so no key
-- comparison is needed here: every key present is already distinct.
growTable :: (Key d, UV.Unbox t) => MTable s d t -> t -> ST s (MTable s d t)
growTable (MTable mask cnt hs ks vsc) ini = do
  vs <- BMV.read vsc 0
  -- `newMTable` overwrites the cell, so the old store has to be read out first.
  new@(MTable mask' _ hs' ks' _) <- newMTable cnt vsc (2 * (mask + 1)) ini
  vs' <- BMV.read vsc 0
  forM_ [0 .. mask] $ \i -> do
    h <- UMV.read hs i
    when (h /= 0) $ do
      v <- UMV.read vs i
      let place j = do
            h' <- UMV.read hs' j
            if h' == 0
              then UMV.write hs' j h >> copyKey ks i ks' j >> UMV.write vs' j v
              else place ((j + 1) .&. mask')
      place (fromIntegral h .&. mask')
  return new

-- | Distinct values per key (SQL's @COUNT(DISTINCT x)@).
--
-- Two tables rather than a buffer per group. The first is a SET: key it on the
-- whole pair and reduce with @()@, so a repeated @(d, r)@ finds its slot already
-- taken and nothing happens. @Unbox ()@ stores no bytes, so that accumulator is
-- free. Then re-key what survived to @d@ alone and count with an ordinary fold.
--
-- The two builds are nested rather than sequenced, because the second enumerates
-- the first table — @dd@ is in scope for the inner splice, and the inner
-- `buildTable` generates a loop over its slots.
withCountDistinct :: forall d r w. (Hashable d, Key d, Hashable r, Key r)
                  => Stream d r
                  -> ((forall q. Mode q => q d Int) -> CodeQ w) -> CodeQ w
withCountDistinct s k =
  [|| let !dd = $$(buildTable (\_ _ -> [|| () ||]) [|| () ||] pairs)
      in $$(withFold (\n _ -> [|| $$n + (1 :: Int) ||]) [|| 0 ||]
                     (distinct [|| dd ||]) k) ||]
  where
    pairs :: Stream (d, r) ()
    pairs = mapvS (\_ -> [|| () ||]) (mapkVS (\d r -> [|| ($$d, $$r) ||]) s)

    distinct :: CodeQ (Table (d, r) ()) -> Stream d ()
    distinct t = mapkS (\p -> [|| fst $$p ||]) (fromTable t)

-- | Count distinct entity ids per bounded integer group. On normal extents the
-- pair is represented by one integer, and the final counts use a dense array;
-- invalid or overflowing bounds retain the generic, fully checked semantics.
withDenseDistinctCount :: CodeQ Int -> CodeQ Int -> Stream Int (Id e)
                       -> ((forall q. Mode q => q Int Int) -> CodeQ w)
                       -> CodeQ w
withDenseDistinctCount groupCount memberCount rows continue =
  [|| let !groups = $$groupCount
          !members = $$memberCount
      in if 0 <= groups && 0 < members
            && groups <= (maxBound :: Int) `div` members
         then
           let !distinctPairs =
                 $$(buildTable (\_ _ -> [|| () ||]) [|| () ||]
                      (packed [|| groups ||] [|| members ||]))
           in $$(withDenseInt [|| groups ||]
                  (\total _ -> [|| $$total + (1 :: Int) ||]) [|| 0 ||]
                  (distinct [|| members ||] [|| distinctPairs ||]) continue)
         else $$(withCountDistinct rows continue) ||]
  where
    packed groups members =
      mapvS (\_ -> [|| () ||])
      . mapkVS
          (\group member ->
            [|| $$group * $$members + idIndex $$member ||])
      $ filtKV
          (\group member ->
            [|| let !g = $$group
                    !m = idIndex $$member
                in 0 <= g && g < $$groups && 0 <= m && m < $$members ||])
          rows

    distinct members table =
      mapkS (\pair -> [|| $$pair `div` $$members ||]) (fromTable table)

--------------------------------------------------------------------------------
-- Dense
--------------------------------------------------------------------------------

-- | The dense-array grouped fold: the same thing as `withFold`, opted into a
-- different physical cache. When the keys are entity ids over a known @0 .. n-1@
-- the per-key slot can be an array index instead of a table entry, which removes
-- a hash and collision-chain search from every reduce step. Keys outside the
-- range are dropped rather than growing the store.
withDense :: UV.Unbox t
          => CodeQ Int -> (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t
          -> Stream (Id e) r
          -> ((forall q. Mode q => q (Id e) t) -> CodeQ w) -> CodeQ w
withDense n op ini s k =
  [|| let !dn = $$(buildDense n op ini [|| False ||] s)
      in $$(k (fromDense [|| dn ||])) ||]

-- | Like `withDense`, but every key in @0 .. n-1@ emits, seeded with @init@ where
-- nothing matched — the left-outer-join aggregate. Correct ONLY when @0 .. n-1@
-- is exactly the key universe; over a sparse key space it invents rows for keys
-- that do not exist.
withDenseOuter :: UV.Unbox t
               => CodeQ Int -> (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t
               -> Stream (Id e) r
               -> ((forall q. Mode q => q (Id e) t) -> CodeQ w) -> CodeQ w
withDenseOuter n op ini s k =
  [|| let !dn = $$(buildDense n op ini [|| True ||] s)
      in $$(k (fromDense [|| dn ||])) ||]

-- | Dense grouped fold for an explicitly bounded integer key. Unlike the
-- entity-id form, both bounds are checked because an ordinary 'Int' can be
-- negative. Only seen slots are emitted, so unused positions in a packed key
-- space do not fabricate groups.
withDenseInt :: UV.Unbox t
             => CodeQ Int -> (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t
             -> Stream Int r
             -> ((forall q. Mode q => q Int t) -> CodeQ w) -> CodeQ w
withDenseInt n op ini s k =
  [|| let !dn = $$(buildDenseInt n op ini s)
      in $$(k (fromDenseInt [|| dn ||])) ||]

-- | Replace repeated values with compact integer codes while building a dense
-- entity-to-code relation. The boxed vector contains one copy of each distinct
-- value in code order, so reporting code can recover the original value after
-- the hot part of the query has finished.
withDictionary :: Ord value
               => CodeQ Int -> Stream (Id e) value
               -> ((forall q. Mode q => q (Id e) Int)
                   -> CodeQ (BV.Vector value) -> CodeQ w)
               -> CodeQ w
withDictionary n rows continue =
  [|| case $$(buildDictionary n rows) of
        (!codes, !labels) ->
          $$(continue (fromDense [|| codes ||]) [|| labels ||]) ||]

-- The reduce step is `modify` rather than a read, an apply and a write, because
-- that is the form that stays allocation-free: @op@ is applied to the slot's
-- components and the result written straight back, with no accumulator built on
-- the heap in between. The vector library's checked update remains
-- allocation-free on this guarded path.
--
-- The accumulator threaded through `sfoldST` is @()@: everything this loop
-- changes is in the two arrays, which are bound once, outside it.
buildDense :: UV.Unbox t
           => CodeQ Int -> (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t -> CodeQ Bool
           -> Stream (Id e) r -> CodeQ (Dense e t)
buildDense n op ini pre s =
  [|| runST (do
        let !cap = $$n
        vals <- newSlots cap $$ini
        seen <- newBits cap $$pre
        () <- $$(sfoldST
                   (\acc d v ->
                      [|| let i = idIndex $$d
                          in if i < cap
                               then do
                                 UMV.modify vals
                                   (\_current -> $$(op [|| _current ||] v)) i
                                 writeArray seen i True
                                 return $$acc
                               else return $$acc ||])
                   [|| () ||] s)
        Dense cap <$> UV.freeze vals <*> freeze seen) ||]

buildDenseInt :: UV.Unbox t
              => CodeQ Int -> (CodeQ t -> CodeQ r -> CodeQ t) -> CodeQ t
              -> Stream Int r -> CodeQ (DenseInt t)
buildDenseInt n op ini s =
  [|| runST (do
        let !cap = $$n
        vals <- newSlots cap $$ini
        seen <- newBits cap False
        () <- $$(sfoldST
                   (\acc d v ->
                      [|| let i = $$d
                          in if 0 <= i && i < cap
                               then do
                                 UMV.modify vals
                                   (\_current -> $$(op [|| _current ||] v)) i
                                 writeArray seen i True
                                 return $$acc
                               else return $$acc ||])
                   [|| () ||] s)
        DenseInt cap <$> UV.freeze vals <*> freeze seen) ||]

-- Build the dictionary and dense code column in one pass. The ordered map is
-- construction-only: later scans see only integer array reads.
buildDictionary :: Ord value
                => CodeQ Int -> Stream (Id e) value
                -> CodeQ (Dense e Int, BV.Vector value)
buildDictionary n rows =
  [|| runST (do
        let !cap = $$n
        codes <- newSlots cap (-1 :: Int)
        seen <- newBits cap False
        (_, labels, _) <- $$(sfoldST
          (\state entity value ->
            [|| case $$state of
                  (!dictionary, !items, !nextCode) ->
                    let !i = idIndex $$entity
                        !item = $$value
                    in if 0 <= i && i < cap
                       then case Map.lookup item dictionary of
                         Just code -> do
                           UMV.write codes i code
                           writeArray seen i True
                           return (dictionary, items, nextCode)
                         Nothing -> do
                           UMV.write codes i nextCode
                           writeArray seen i True
                           return (Map.insert item nextCode dictionary,
                                   item : items, nextCode + 1)
                       else return (dictionary, items, nextCode) ||])
          [|| (Map.empty, [], 0 :: Int) ||] rows)
        dense <- Dense cap <$> UV.freeze codes <*> freeze seen
        return (dense, BV.fromList (reverse labels))) ||]

-- | Precompute dense membership: consume a stream, set a bit at each VALUE it
-- emits, and hand back the identity relation on those ids. This is the one
-- materializer that exists purely for speed — it is semantically the same as
-- restricting against the relation's values, but a bit test beats re-running a
-- subquery, so it pays off when the same filter is checked many times.
--
-- Under pull it is also the one with a rival. `anyOf` short-circuits, so an
-- EXISTS that used to need a precomputed bitset can now inspect the keyed stream
-- directly. Keep this for the case where the same membership is looked up
-- millions of times; use `anyOf` where it is asked once per outer row and the
-- inner relation is small.
withBits :: CodeQ Int -> Stream d (Id e)
         -> ((forall q. Mode q => q (Id e) (Id e)) -> CodeQ w) -> CodeQ w
withBits n s k =
  [|| let !bs = Bits (runSTUArray (do
                        let !cap = $$n
                        arr <- newBits cap False
                        () <- $$(sfoldST
                                   (\acc _ v ->
                                      [|| let i = idIndex $$v
                                          in if i < cap
                                               then do
                                                 writeArray arr i True
                                                 return $$acc
                                               else return $$acc ||])
                                   [|| () ||] s)
                        return arr))
      in $$(k (fromBits [|| bs ||])) ||]
