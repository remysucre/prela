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
import Data.Array (Array, accumArray, elems)
import Data.Array.ST (STArray, STUArray, newArray, writeArray, runSTUArray)
import Data.Array.Unboxed (UArray)
import qualified Data.Array.Unboxed as U
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Unsafe as BSU
import Data.Maybe (isJust)
import qualified Data.Vector.Storable as V
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
data Dense e t = Dense !Int !(Array Int t) !(UArray Int Bool)

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

-- Typed wrappers so the two arrays above are built in one pass over the input.
-- `freeze` alone leaves the mutable array type ambiguous; these pin it.
newBoxed :: Int -> t -> ST s (STArray s Int t)
newBoxed n v = newArray (0, n - 1) v

newBits :: Int -> Bool -> ST s (STUArray s Int Bool)
newBits n v = newArray (0, n - 1) v
