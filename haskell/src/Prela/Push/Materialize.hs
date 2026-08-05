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
module Prela.Push.Materialize where

import Control.Monad (forM_, when)
import Control.Monad.ST (ST, runST)
import Data.Array.ST (freeze, runSTUArray, writeArray)
import Data.Bits ((.&.))
import Data.Hashable (Hashable)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.STRef (STRef, modifySTRef', newSTRef, readSTRef, writeSTRef)
import qualified Data.Vector.Unboxed as UV
import qualified Data.Vector.Unboxed.Mutable as UMV

import Prela.Id
import Prela.Push.Mode
import Prela.Push.Ops
import Prela.Push.Stream (invStream)
import Prela.Storage

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
fold :: (Mode q, Hashable d, Key d, UV.Unbox s)
     => (s -> r -> s) -> s -> Drv d r -> q d s
fold op ini q = fromTable (buildTable op ini q)
{-# INLINE fold #-}

-- The mutable table under construction: the same three stores as `Table`, plus
-- the mask. Held in an `STRef` because growth replaces all of it at once and the
-- driving continuation has nowhere else to keep state — `drive`'s callback
-- returns `m ()`, so nothing can be threaded through it.
--
-- That costs one pointer read per input row, which is the price of amortized
-- growth and is what Rust pays too, its closure reaching the map through a
-- borrow. Everything downstream of the read is flat.
data MTable s d t = MTable !Int !(UMV.MVector s Word) !(MKeys s d) !(UMV.MVector s t)

newMTable :: (Key d, UV.Unbox t) => Int -> t -> ST s (MTable s d t)
newMTable cap ini =
  MTable (cap - 1) <$> UMV.replicate cap 0 <*> newKeys cap <*> UMV.replicate cap ini

-- | Drive once, reducing into an open-addressed table.
--
-- Growth is at three quarters full. Linear probing degrades badly past that,
-- and every rehash is paid back over the rows that follow it, so the doubling
-- costs O(n) in total across the whole build.
buildTable :: (Hashable d, Key d, UV.Unbox t)
           => (t -> r -> t) -> t -> Drv d r -> Table d t
buildTable op ini q = runST $ do
  ref  <- newSTRef =<< newMTable 64 ini
  nref <- newSTRef (0 :: Int)
  drive q $ \d v -> do
    MTable mask hs ks vs <- readSTRef ref
    let h = slotHash d
        go i = do
          h' <- UMV.read hs i
          if h' == 0
            then do
              UMV.write hs i h
              writeKey ks i d
              UMV.write vs i (op ini v)
              n <- (+ 1) <$> readSTRef nref
              writeSTRef nref n
              when (4 * n > 3 * (mask + 1)) (growTable ref ini)
            else do
              k <- readKey ks i
              if h' == h && k == d
                then UMV.modify vs (`op` v) i
                else go ((i + 1) .&. mask)
    go (fromIntegral h .&. mask)
  MTable mask hs ks vs <- readSTRef ref
  Table mask <$> UV.freeze hs <*> freezeKeys ks <*> UV.freeze vs
-- INLINE, not INLINABLE: the point is for the `drive` inside to fuse with the
-- query it is given, the same way every other operator does. Left to itself GHC
-- will not export an unfolding for something this size, and the fold degenerates
-- into a generic call that re-enters the query through a closure on every row.
-- Measured: without this the streaming queries run ~10x slower.
{-# INLINE buildTable #-}

-- Rehash into a table of twice the capacity. Only occupied slots move, and each
-- one lands on the first free slot of its new probe sequence, so no key
-- comparison is needed here: every key present is already distinct.
growTable :: (Key d, UV.Unbox t) => STRef s (MTable s d t) -> t -> ST s ()
growTable ref ini = do
  MTable mask hs ks vs <- readSTRef ref
  MTable mask' hs' ks' vs' <- newMTable (2 * (mask + 1)) ini
  forM_ [0 .. mask] $ \i -> do
    h <- UMV.read hs i
    when (h /= 0) $ do
      k <- readKey ks i
      v <- UMV.read vs i
      let place j = do
            h' <- UMV.read hs' j
            if h' == 0
              then UMV.write hs' j h >> writeKey ks' j k >> UMV.write vs' j v
              else place ((j + 1) .&. mask')
      place (fromIntegral h .&. mask')
  writeSTRef ref (MTable mask' hs' ks' vs')

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

-- | Distinct values per key (SQL's `COUNT(DISTINCT x)`).
--
-- Two tables rather than a buffer per group. The first is a SET: key it on the
-- whole pair and reduce with `()`, so a repeated `(d, r)` finds its slot already
-- taken and nothing happens. `Unbox ()` stores no bytes, so that accumulator is
-- free. Then re-key what survived to `d` alone and count with an ordinary
-- `fold`.
--
-- This used to go through `bufFold`, which meant a `Map` of cons-lists and a
-- sort per group. Dropping it removes the sort as well as the lists, so the
-- change is algorithmic and not only a matter of layout.
countDistinct :: (Mode q, Hashable d, Key d, Hashable r, Key r)
              => Drv d r -> q d Int
countDistinct q = fold (\n _ -> n + (1 :: Int)) 0 distinct
  where
    pairs    = Drv (\k -> drive q (\d r -> k (d, r) ()))
    distinct = Drv (\k -> drive (dedup pairs) (\(d, _) u -> k d u))
{-# INLINE countDistinct #-}

-- Drop duplicate keys by building a table and reading it back. The `Drv`
-- annotation is on the result type here rather than inline at the use site so
-- that the mode is pinned without needing a scoped type variable.
dedup :: (Hashable a, Key a) => Drv a b -> Drv a ()
dedup s = fromTable (buildTable (\_ _ -> ()) () s)
{-# INLINE dedup #-}

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
