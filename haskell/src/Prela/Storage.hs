{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE TypeFamilies #-}

-- | What a relation is made of: entity ids, the physical layout of each element
-- type, and the handful of column shapes those layouts are arranged into.
--
-- Nothing here executes anything. Storage is built once, and the leaves in
-- "Prela.Ops" are VIEWS of it. That separation matters for a reason peculiar to
-- this design: a leaf is mode-polymorphic, and a polymorphic binding is
-- re-elaborated at each instantiation, so a column built inside a leaf would be
-- built twice for a column used in both modes.
module Prela.Storage where

import Control.Monad (when)
import Control.Monad.ST (ST)
import Data.Array (accumArray, elems)
import Data.Bits (shiftR, xor, (.&.))
import Data.Hashable (Hashable, hash, hashWithSalt)
import Data.Array.ST (STUArray, newArray, writeArray, runSTUArray)
import Data.Array.Unboxed (UArray)
import qualified Data.Array.Unboxed as U
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Unsafe as BSU
import Data.Maybe (isJust)
import qualified Data.Vector as BV
import qualified Data.Vector.Storable as V
import qualified Data.Vector.Unboxed as UV
import qualified Data.Vector.Unboxed.Mutable as UMV
import Data.Word (Word32)

--------------------------------------------------------------------------------
-- Entity ids
--------------------------------------------------------------------------------

-- An entity id. Under the hood a 0-based Int, but the phantom tag e (Movie,
-- Keyword, …) rides along in the type and never appears in a value, so
-- `Id Movie` and `Id Keyword` are the same bits and do not unify: composing a
-- Movie-keyed relation onto a Person-keyed one is a compile error, not a
-- silent wrong answer. Erased at runtime, so the safety is free.
newtype Id e = Id Int deriving (Eq, Ord, Show)

-- Ids are the commonest fold key, so hashing one has to cost nothing. The
-- phantom tag plays no part: two ids of different entities that hash alike can
-- never meet, since they cannot inhabit the same relation to begin with.
instance Hashable (Id e) where
  hashWithSalt s (Id i) = hashWithSalt s i
  {-# INLINE hashWithSalt #-}

-- | The missing-id sentinel — a foreign key that points at nothing. The value is
-- not arbitrary: it fails every `0 <= i && i < n` range check, so a hole probes
-- to nothing without any branch of its own, which is why Prela can claim there
-- are no NULLs and still store fixed-width id columns.
--
-- It is @-1@ rather than `maxBound` so that it is bit-for-bit the all-ones hole
-- word the cache writes (see "Prela.Cache"), read back as a signed machine word.
-- A loaded column and a hand-built one then hold the same bits, which they must,
-- since nothing downstream knows where a column came from. Same trick as the
-- Rust port's `NO_ID`, which is `usize::MAX` for the same reason read unsigned.
noId :: Id e
noId = Id (-1)

--------------------------------------------------------------------------------
-- How element types are physically stored
--------------------------------------------------------------------------------

-- Rust gets flat storage for free: `Vec<i64>` is already a contiguous array of
-- machine words. Haskell does not — `Array Int Int` is an array of POINTERS to
-- boxed Ints, which is a cache miss per row and defeats the point of a column
-- store. An unboxed array is flat, but only accepts primitive element types, so
-- one uniform column type cannot serve both an Int column and a string column.
--
-- This class is the way out: an associated DATA family, so each element type
-- names its own physical layout while the leaves in "Prela.Ops" see one
-- interface.
--
-- The layouts chosen here are deliberately the ON-DISK layouts of the cache
-- format (see "Prela.Cache"): 8-byte words for numbers and ids, 4-byte offsets
-- plus one packed buffer for strings. That is what lets loading be a VIEW of a
-- memory-mapped file rather than a conversion of it — `Storable` vectors are a
-- pointer, an offset and a length, so a column can point straight into the
-- mapping the way the Rust port's `&'static str` does. The difference is that
-- Rust has to leak the mmap to get a `'static` lifetime, whereas here the
-- vectors hold the mapping's finalizer and it is released when the last column
-- referring to it is collected.
--
-- `atStore` is UNCHECKED. Every caller is a leaf that has already done its own
-- range test, and doing it twice is exactly the kind of per-row cost this whole
-- design exists to avoid.
class Elem r where
  data Store r
  packStore :: [r] -> Store r
  storeLen  :: Store r -> Int
  atStore   :: Store r -> Int -> r

packU :: U.IArray UArray r => [r] -> UArray Int r
packU vs = U.listArray (0, length vs - 1) vs

packV :: V.Storable r => [r] -> V.Vector r
packV = V.fromList

-- Offsets are 32-bit, as on disk. Widening to an index is a zero-extend.
off32 :: V.Vector Word32 -> Int -> Int
off32 v i = fromIntegral (V.unsafeIndex v i)
{-# INLINE off32 #-}

instance Elem Int where
  newtype Store Int = IntStore (V.Vector Int)
  packStore = IntStore . packV
  storeLen (IntStore a) = V.length a
  atStore (IntStore a) i = V.unsafeIndex a i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

instance Elem Double where
  newtype Store Double = DblStore (V.Vector Double)
  packStore = DblStore . packV
  storeLen (DblStore a) = V.length a
  atStore (DblStore a) i = V.unsafeIndex a i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

-- An id column is a word array reinterpreted, which is what makes a foreign key
-- as cheap to store as a number. The Rust port gets this from
-- `#[repr(transparent)]`; here the newtype is erased, so it is the same bits.
instance Elem (Id e) where
  newtype Store (Id e) = IdStore (V.Vector Int)
  packStore = IdStore . packV . map (\(Id i) -> i)
  storeLen (IdStore a) = V.length a
  atStore (IdStore a) i = Id (V.unsafeIndex a i)
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

-- One concatenated buffer plus n+1 offsets. `unsafeTake`/`unsafeDrop` are
-- pointer arithmetic on the shared buffer, so reading a string copies nothing.
instance Elem ByteString where
  data Store ByteString = BsStore !(V.Vector Word32) !ByteString
  packStore vs = BsStore (packV (scanl (+) 0 (map (fromIntegral . BS.length) vs)))
                         (BS.concat vs)
  storeLen (BsStore offs _) = V.length offs - 1
  atStore (BsStore offs buf) i =
    case (off32 offs i, off32 offs (i + 1)) of
      (o, o') -> BSU.unsafeTake (o' - o) (BSU.unsafeDrop o buf)
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

--------------------------------------------------------------------------------
-- The three column shapes
--------------------------------------------------------------------------------

-- These are three separate types rather than one type with a shape field, for
-- the same reason the sparse universe is a separate leaf in "Prela.Ops":
-- dispatch happens once at compile time, so a dense column's loop carries no
-- branch testing whether it might have been sparse.

-- | Total 1:1: every key in `0 .. n-1` has exactly one value.
data Col e r = Col !Int !(Store r)

mkCol :: Elem r => [r] -> Col e r
mkCol vs = Col (storeLen s) s where s = packStore vs

-- | How many keys the column covers, which is also the size of its entity's id
-- space. `Prela.Schema` uses this to size a universe from a loaded column.
colLen :: Col e r -> Int
colLen (Col n _) = n

-- | 1:1 with holes: the values array stays full width, and a presence bit per
-- key says which slots are real. For an id-valued column `noId` in the holes
-- would do the same job for free, but a scalar column has no spare value — a
-- missing year is not year 0 — so the mask is what keeps "absent" from becoming
-- a wrong answer. Julia carries the same presence bitvector for this reason.
data SparseCol e r = SparseCol !Int !(Store r) !(UArray Int Bool)

-- | `fill` occupies the holes and is never read; only its width matters.
mkSparseCol :: Elem r => r -> [Maybe r] -> SparseCol e r
mkSparseCol fill ms =
  SparseCol (length ms) (packStore (map (maybe fill id) ms)) (packU (map isJust ms))

-- | Multi-valued, stored CSR: key i owns `values[offsets[i] .. offsets[i+1]-1]`,
-- and a key with no values is simply an empty range, so partial and multi-valued
-- are the same representation.
data MultiCol e r = MultiCol !Int !(V.Vector Word32) !(Store r)

-- | Pairs with a key outside `0 .. n-1` are dropped. Per-key value order follows
-- the input.
multiColLen :: MultiCol e r -> Int
multiColLen (MultiCol n _ _) = n

mkMultiCol :: Elem r => Int -> [(Int, r)] -> MultiCol e r
mkMultiCol n prs = MultiCol n (packV (scanl (+) 0 (map (fromIntegral . length) buckets)))
                             (packStore (concat buckets))
  where
    buckets = map reverse (elems (accumArray (flip (:)) [] (0, n - 1)
                                             [p | p@(k, _) <- prs, 0 <= k, k < n]))

--------------------------------------------------------------------------------
-- Caches an operator builds
--------------------------------------------------------------------------------

-- The two shapes above are loaded from data; these two are produced by the
-- materializing operators in "Prela.Materialize" and then read back as leaves,
-- which is why they live here with the rest of the storage rather than there.

-- One reduced value per key over a dense key space `0 .. n-1`: what a grouped
-- fold produces when the keys are entity ids rather than arbitrary values. The
-- presence array is what distinguishes "this key folded to init" from "this key
-- was never seen", and it is also how the outer variant is expressed — seeding
-- presence to all-True makes every key emit, so there is no separate flag.
--
-- The slots are an UNBOXED vector, which is the whole reason a fold over entity
-- ids is worth having. A boxed array holds a pointer per key, so folding a step
-- has to allocate a fresh accumulator on the heap and store a pointer to it —
-- once per input row, not once per key. `Data.Vector.Unboxed` instead stores a
-- product componentwise, so an accumulator of `(Double, Int)` is one `Double`
-- array beside one `Int` array and a step writes two machine words in place.
-- That is what makes the reduce loop allocation-free, matching what the Rust
-- port gets from a `Vec<S>` of `Copy` values.
--
-- The price is the `Unbox` constraint, which every reader of a `Dense` carries:
-- the accumulator has to be built out of primitives. In practice a fold state is
-- a number or a tuple of them, and anything that is not keeps to `fold`.
data Dense e t = Dense !Int !(UV.Vector t) !(UArray Int Bool)

-- | One reduced value per key when the keys are NOT a dense id space: a group
-- key of `(returnflag, linestatus)` or `(nation, year)` has no array index to
-- be. Same contents as `Dense`, addressed by hashing instead.
--
-- Open addressing with linear probing, over three parallel stores rather than
-- one array of entries: slot hashes, keys, accumulators. Splitting them is what
-- lets the accumulators be UNBOXED, for the reason spelled out on `Dense` — the
-- reduce step then writes machine words in place instead of allocating a fresh
-- accumulator per input row. It is also why the probe is cheap: scanning for a
-- slot touches only the hash vector, which is contiguous machine words, and the
-- boxed keys are read at most once, on the slot that actually matched.
--
-- Capacity is a power of two, so wrapping is a mask rather than a division.
-- Hash 0 marks an empty slot, which is why `slotHash` never returns it.
data Table d t = Table !Int              -- capacity - 1, used as the wrap mask
                       !(UV.Vector Word) -- slot hashes, 0 where empty
                       !(BV.Vector d)    -- key, meaningful where hash /= 0
                       !(UV.Vector t)    -- accumulator, likewise

-- | A key's slot hash: never 0, since 0 is the empty marker.
--
-- The extra mixing is not paranoia. `Hashable` combines a tuple's fields with a
-- multiply-add, which leaves the low bits of a key like `(Id, year)` poorly
-- distributed, and the low bits are exactly what the mask keeps. Linear probing
-- is unforgiving about that: keys that agree in the low bits land in one run and
-- probing degrades to a scan. This is the standard 64-bit avalanche step, which
-- spreads every input bit across the whole word.
slotHash :: Hashable d => d -> Word
slotHash d = if h == 0 then 1 else h
  where
    h  = z2 `xor` (z2 `shiftR` 33)
    z2 = (z1 `xor` (z1 `shiftR` 33)) * 0xc4ceb9fe1a85ec53
    z1 = (z0 `xor` (z0 `shiftR` 33)) * 0xff51afd7ed558ccd
    z0 = fromIntegral (hash d) :: Word
{-# INLINE slotHash #-}

-- | The slot holding `d`, or -1 if there is none.
--
-- An index rather than a `Maybe t`, because the accumulator is unboxed: handing
-- back a `Maybe` would have to build the `Just` and box the value inside it,
-- once per probe, which is the cost this whole representation exists to avoid.
tableSlot :: Hashable d => Table d t -> d -> Int
tableSlot (Table mask hs ks _) d = go (fromIntegral h .&. mask)
  where
    h = slotHash d
    go i = case hs UV.! i of
             0                                -> -1
             h' | h' == h && ks BV.! i == d   -> i
                | otherwise                   -> go ((i + 1) .&. mask)
{-# INLINE tableSlot #-}

-- A dense membership set: the identity relation on whichever ids are present.
-- `UArray Int Bool` is bit-packed by GHC, so a test is a word load and a shift,
-- which is the whole point of it over a `Map`. Driving it scans every bit rather
-- than using Rust's trailing-zeros skip, so it is best used probed.
newtype Bits e = Bits (UArray Int Bool)

-- | A membership set over `0 .. n-1` from the ids that are present. Ids outside
-- the range are dropped, `noId` among them.
mkBits :: Int -> [Id e] -> Bits e
mkBits n ids = Bits (runSTUArray (do
  bs <- newBits n False
  mapM_ (\(Id i) -> when (0 <= i && i < n) (writeArray bs i True)) ids
  return bs))

-- Typed wrappers so the two stores above are built in one pass over the input.
-- `freeze` alone leaves the mutable type ambiguous; these pin it.
newSlots :: UV.Unbox t => Int -> t -> ST s (UMV.MVector s t)
newSlots = UMV.replicate

newBits :: Int -> Bool -> ST s (STUArray s Int Bool)
newBits n v = newArray (0, n - 1) v
