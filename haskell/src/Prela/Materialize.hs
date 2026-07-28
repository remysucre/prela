-- | The operators that stop the pipeline.
--
-- Everything else in the port is a closure: build a query, and no work happens
-- until something drives it. These are the exceptions, and they are the reason
-- `drive`'s continuation is `forall m. Monad m` rather than a pure fold — each
-- one instantiates `m = ST`, mutates while driving, and freezes the result. That
-- is also why they are worth having in a module of their own: a query that names
-- something from here is a query that allocates, and Prela is non-materialized by
-- default, so making that visible in the import list is the point.
--
-- The caches they build (`Dense`, `Bits`, and a plain `Map`) live in
-- "Prela.Storage" beside the loaded columns, because the leaves that read them
-- back are ordinary leaves — a fold's result composes and probes like a column.
module Prela.Materialize where

import Control.Monad (when)
import Control.Monad.ST (runST)
import Data.Array.ST (freeze, runSTUArray, writeArray)
import qualified Data.List as List
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.STRef (modifySTRef', newSTRef, readSTRef)
import qualified Data.Vector.Unboxed as UV
import qualified Data.Vector.Unboxed.Mutable as UMV

import Prela.Mode
import Prela.Ops
import Prela.Storage
import Prela.Stream (invStream)

-- | Drive a relation once and bucket its pairs by key. This is where a query
-- stops being pure closures and holds real data.
index :: Ord d => Drv d r -> Map d [r]
index q = runST $ do
  ref <- newSTRef Map.empty
  drive q (\d r -> modifySTRef' ref (Map.insertWith (++) d [r]))
  readSTRef ref

-- | Force a leg once so reuse is free. Same pairs, now backed by the index.
-- Bind the RESULT monomorphically: it is mode-polymorphic, and a polymorphic
-- binding used at two modes builds its index twice, which is the exact cost
-- this exists to remove.
materialize :: (Mode q, Ord d) => Drv d r -> q d r
materialize = fromIndex . index

-- | The probed inverse: index the flipped pairs, and the result serves either
-- mode because the work has already been paid for.
inv :: (Mode q, Ord r) => Drv d r -> q r d
inv = fromIndex . index . invStream

-- | Group by key and reduce each group (`q ▷ (op, init)`). A fold cannot
-- stream — reducing a group means seeing the whole group — so like the index
-- it drives once into a cache, but the cache holds ONE value per key.
-- `count = fold (\n _ -> n + 1) 0`. The result is an ordinary relation, so a
-- fold composes and probes like anything else.
fold :: (Mode q, Ord d) => (s -> r -> s) -> s -> Drv d r -> q d s
fold op ini q = fromCache cache
  where
    cache = runST $ do
      ref <- newSTRef Map.empty
      drive q (\d v -> modifySTRef' ref (Map.alter (Just . flip op v . maybe ini id) d))
      readSTRef ref

-- | The whole-group fold: instead of reducing pairwise, hand the reducer the
-- entire group at once. For anything that does not fit `fold`'s
-- `(s -> r -> s)` shape, which is to say anything needing to see the group
-- twice, or in order: count-distinct, median, a range between the extremes.
-- The Julia port calls this a BufFold for the buffer it has to keep.
--
-- It costs strictly more than `fold`, since every value is retained until the
-- group is complete rather than collapsing into an accumulator, so reach for
-- `fold` unless the reducer genuinely cannot be written that way. The list each
-- group is handed is in drive order.
bufFold :: (Mode q, Ord d) => ([r] -> s) -> Drv d r -> q d s
bufFold f = fromCache . Map.map (f . reverse) . index

-- | Distinct values per key (SQL's `COUNT(DISTINCT x)`), the instance of
-- `bufFold` that motivates it. Sorting and then counting runs of equals beats a
-- set per group at the sizes that actually occur, which is the same choice the
-- Rust port makes for the same reason.
countDistinct :: (Mode q, Ord d, Ord r) => Drv d r -> q d Int
countDistinct = bufFold (length . List.group . List.sort)

-- | The dense-array grouped fold (`q ▷ (op, init, n)`): the same thing as
-- `fold`, opted into a different physical cache. When the keys are entity ids
-- over a known `0 .. n-1` the per-key slot can be an array index instead of a
-- map entry, which removes a hash and an allocation from every reduce step. Keys
-- outside the range are dropped rather than growing the store.
foldDense :: (Mode q, UV.Unbox t) => Int -> (t -> r -> t) -> t -> Drv (Id e) r -> q (Id e) t
foldDense n op ini = fromDense . buildDense n op ini False

-- | Like `foldDense`, but every key in `0 .. n-1` emits, seeded with `init`
-- where nothing matched — the left-outer-join aggregate. Correct ONLY when
-- `0 .. n-1` is exactly the key universe; over a sparse key space it invents
-- rows for keys that do not exist.
foldDenseOuter :: (Mode q, UV.Unbox t)
               => Int -> (t -> r -> t) -> t -> Drv (Id e) r -> q (Id e) t
foldDenseOuter n op ini = fromDense . buildDense n op ini True

-- The reduce step is `modify` rather than a read, an apply and a write, because
-- that is the form that stays allocation-free: `op` is applied to the slot's
-- components and the result written straight back, with no accumulator built on
-- the heap in between. The bounds check `modify` does on top of the guard above
-- it measures as free, so this is not the unchecked `unsafeModify`.
buildDense :: UV.Unbox t => Int -> (t -> r -> t) -> t -> Bool -> Drv (Id e) r -> Dense e t
buildDense n op ini pre q = runST $ do
  vals <- newSlots n ini
  seen <- newBits n pre
  drive q (\(Id i) v -> when (0 <= i && i < n) (do
             UMV.modify vals (`op` v) i
             writeArray seen i True))
  Dense n <$> UV.freeze vals <*> freeze seen

-- | Precompute dense membership (`bitset(q, n)`): drive a relation and set a bit
-- at each VALUE it emits, giving back the identity relation on those ids. This is
-- the one operator that exists purely for speed — it is semantically the same as
-- restricting against the relation's values, but a bit test beats re-running a
-- subquery or hashing into a set, so it pays off on a filter probed many times.
bitset :: Mode q => Int -> Drv d (Id e) -> q (Id e) (Id e)
bitset n q = fromBits (Bits (runSTUArray (do
  bs <- newBits n False
  drive q (\_ (Id i) -> when (0 <= i && i < n) (writeArray bs i True))
  return bs)))
