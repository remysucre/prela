{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE GADTs #-}
{-# LANGUAGE TypeFamilies #-}

-- | Physical element layouts and the column shapes built from them.
--
-- Nothing here executes anything. Storage is built once, and the staged leaves
-- are views of it. That separation matters because a leaf is mode-polymorphic,
-- and a polymorphic binding is
-- re-elaborated at each instantiation, so a column built inside a leaf would be
-- built twice for a column used in both modes.
module Prela.Storage.Internal where

import Control.Monad (when)
import Control.Monad.ST (ST)
import Data.Array (accumArray, elems)
import Data.Bits (shiftR, xor, (.&.))
import Data.Hashable (Hashable, hash)
import Data.Array.ST (STUArray, newArray, writeArray, runSTUArray)
import Data.Array.Unboxed (UArray)
import qualified Data.Array.Unboxed as U
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Maybe (isJust)
import qualified Data.Vector as BV
import qualified Data.Vector.Mutable as BMV
import qualified Data.Vector.Storable as V
import qualified Data.Vector.Unboxed as UV
import qualified Data.Vector.Unboxed.Mutable as UMV
import Data.Word (Word32, Word64)

import Prela.Id
import qualified Prela.Id.Internal as IdInternal

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
-- names its own physical layout while the staged leaves see one
-- interface.
--
-- The layouts closely follow the on-disk cache format (see "Prela.Cache"):
-- 8-byte words for numbers, and 4-byte offsets plus one packed buffer for
-- strings. The checked loader validates and copies those bytes; the trusted
-- loader can instead retain read-only mapped views in the same store types.
-- Element access uses the vector and bytestring libraries' checked operations
-- either way.
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
off32 v i = fromIntegral (v V.! i)
{-# INLINE off32 #-}

instance Elem Int where
  newtype Store Int = IntStore (V.Vector Int)
  packStore = IntStore . packV
  storeLen (IntStore a) = V.length a
  atStore (IntStore a) i = a V.! i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

instance Elem Double where
  newtype Store Double = DblStore (V.Vector Double)
  packStore = DblStore . packV
  storeLen (DblStore a) = V.length a
  atStore (DblStore a) i = a V.! i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

-- In-memory ids stay boxed so their private constructor is never bypassed by a
-- representation cast.
instance Elem (Id e) where
  newtype Store (Id e) = IdStore (BV.Vector (Id e))
  packStore = IdStore . BV.fromList
  storeLen (IdStore a) = BV.length a
  atStore (IdStore a) i = a BV.! i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

-- One concatenated buffer plus n+1 validated offsets. Slicing shares the
-- underlying buffer, so reading a string copies nothing.
instance Elem ByteString where
  data Store ByteString = BsStore !(V.Vector Word32) !ByteString
  packStore vs = BsStore (packV (scanl (+) 0 (map (fromIntegral . BS.length) vs)))
                         (BS.concat vs)
  storeLen (BsStore offs _) = V.length offs - 1
  atStore (BsStore offs buf) i =
    case (off32 offs i, off32 offs (i + 1)) of
      (o, o') -> BS.take (o' - o) (BS.drop o buf)
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

--------------------------------------------------------------------------------
-- How fold keys are physically stored
--------------------------------------------------------------------------------

-- `Elem` above says how a COLUMN's values are laid out. This says the same for a
-- hash table's KEYS, and it exists for the same reason. A boxed vector of
-- `Id Order` is a vector of pointers, so confirming that a slot really holds the
-- key being looked up chases into the heap for an object wrapping one machine
-- word. The hash is compared first, so that chase lands on the slot that
-- matched, which is to say once per input row.
--
-- Two associated data families rather than one, because a table is filled
-- mutably and read immutably and the two want different types. `Data.Vector`
-- splits `MVector`/`Vector` for the same reason.
--
-- The PAIR instance is what makes four instances cover the whole schema: a key
-- of `((Id Order, Int), Int)` becomes three flat arrays side by side and is
-- never a tuple in memory. Every compound key in TPC-H is built by pairing, so
-- nothing else needs an instance of its own. Same structure-of-arrays trick as
-- `Data.Vector.Unboxed`'s tuple instances, which is also why the accumulators
-- can use those directly.
--
-- INVARIANT: a slot is written before it is read. `newKeys` does not initialize,
-- so the caller has to have its own record of which slots are live — for the
-- table below that is the hash vector, where 0 marks an empty slot.
class Eq d => Key d where
  data MKeys s d
  data Keys d
  newKeys    :: Int -> ST s (MKeys s d)
  matchesKey :: MKeys s d -> Int -> d -> ST s Bool
  writeKey   :: MKeys s d -> Int -> d -> ST s ()
  copyKey    :: MKeys s d -> Int -> MKeys s d -> Int -> ST s ()
  freezeKeys :: MKeys s d -> ST s (Keys d)
  indexKey   :: Keys d -> Int -> d
  matchesFrozenKey :: Keys d -> Int -> d -> Bool

instance Key Int where
  newtype MKeys s Int = MIntKeys (UMV.MVector s Int)
  newtype Keys Int    = IntKeys (UV.Vector Int)
  newKeys n                 = MIntKeys <$> UMV.new n
  matchesKey (MIntKeys v) i x = (== x) <$> UMV.read v i
  writeKey (MIntKeys v) i x = UMV.write v i x
  copyKey (MIntKeys source) i (MIntKeys target) j =
    UMV.read source i >>= UMV.write target j
  freezeKeys (MIntKeys v)   = IntKeys <$> UV.freeze v
  indexKey (IntKeys v) i    = v UV.! i
  matchesFrozenKey (IntKeys v) i x = v UV.! i == x
  {-# INLINE newKeys #-}
  {-# INLINE matchesKey #-}
  {-# INLINE writeKey #-}
  {-# INLINE copyKey #-}
  {-# INLINE indexKey #-}
  {-# INLINE matchesFrozenKey #-}

-- The tag is erased, so an id table key is stored as an unboxed `Int`. Rebuilding
-- a key for enumeration is proof-preserving: every stored index came from an
-- already validated 'Id'. Ordinary loaded ID columns remain boxed so their
-- representation never escapes through the public storage API.
instance Key (Id e) where
  newtype MKeys s (Id e) = MIdKeys (UMV.MVector s Int)
  newtype Keys (Id e)    = IdKeys (UV.Vector Int)
  newKeys n                     = MIdKeys <$> UMV.new n
  matchesKey (MIdKeys v) i x    = (== idIndex x) <$> UMV.read v i
  writeKey (MIdKeys v) i x      = UMV.write v i (idIndex x)
  copyKey (MIdKeys source) i (MIdKeys target) j =
    UMV.read source i >>= UMV.write target j
  freezeKeys (MIdKeys v)        = IdKeys <$> UV.freeze v
  indexKey (IdKeys v) i         = IdInternal.Id (v UV.! i)
  matchesFrozenKey (IdKeys v) i x = v UV.! i == idIndex x
  {-# INLINE newKeys #-}
  {-# INLINE matchesKey #-}
  {-# INLINE writeKey #-}
  {-# INLINE copyKey #-}
  {-# INLINE indexKey #-}
  {-# INLINE matchesFrozenKey #-}

instance Key Double where
  newtype MKeys s Double = MDblKeys (UMV.MVector s Double)
  newtype Keys Double    = DblKeys (UV.Vector Double)
  newKeys n                 = MDblKeys <$> UMV.new n
  matchesKey (MDblKeys v) i x = (== x) <$> UMV.read v i
  writeKey (MDblKeys v) i x = UMV.write v i x
  copyKey (MDblKeys source) i (MDblKeys target) j =
    UMV.read source i >>= UMV.write target j
  freezeKeys (MDblKeys v)   = DblKeys <$> UV.freeze v
  indexKey (DblKeys v) i    = v UV.! i
  matchesFrozenKey (DblKeys v) i x = v UV.! i == x
  {-# INLINE newKeys #-}
  {-# INLINE matchesKey #-}
  {-# INLINE writeKey #-}
  {-# INLINE copyKey #-}
  {-# INLINE indexKey #-}
  {-# INLINE matchesFrozenKey #-}

-- The one key type that stays boxed: a ByteString is a pointer, a length and an
-- offset, so there is no flat form to put it in. Group keys that are strings are
-- common enough (Q1, Q4 and Q22 all group by one) that this is not a fallback.
instance Key ByteString where
  newtype MKeys s ByteString = MBsKeys (BMV.MVector s ByteString)
  newtype Keys ByteString    = BsKeys (BV.Vector ByteString)
  newKeys n                = MBsKeys <$> BMV.new n
  matchesKey (MBsKeys v) i x = (== x) <$> BMV.read v i
  writeKey (MBsKeys v) i x = BMV.write v i x
  copyKey (MBsKeys source) i (MBsKeys target) j =
    BMV.read source i >>= BMV.write target j
  freezeKeys (MBsKeys v)   = BsKeys <$> BV.freeze v
  indexKey (BsKeys v) i    = v BV.! i
  matchesFrozenKey (BsKeys v) i x = v BV.! i == x
  {-# INLINE newKeys #-}
  {-# INLINE matchesKey #-}
  {-# INLINE writeKey #-}
  {-# INLINE copyKey #-}
  {-# INLINE indexKey #-}
  {-# INLINE matchesFrozenKey #-}

instance (Key a, Key b) => Key (a, b) where
  data MKeys s (a, b) = MPairKeys !(MKeys s a) !(MKeys s b)
  data Keys (a, b)    = PairKeys !(Keys a) !(Keys b)
  newKeys n                         = MPairKeys <$> newKeys n <*> newKeys n
  matchesKey (MPairKeys u v) i (x, y) = do
    firstMatches <- matchesKey u i x
    if firstMatches then matchesKey v i y else return False
  writeKey (MPairKeys u v) i (x, y) = writeKey u i x >> writeKey v i y
  copyKey (MPairKeys su sv) i (MPairKeys tu tv) j =
    copyKey su i tu j >> copyKey sv i tv j
  freezeKeys (MPairKeys u v)        = PairKeys <$> freezeKeys u <*> freezeKeys v
  indexKey (PairKeys u v) i         = (indexKey u i, indexKey v i)
  matchesFrozenKey (PairKeys u v) i (x, y) =
    matchesFrozenKey u i x && matchesFrozenKey v i y
  {-# INLINE newKeys #-}
  {-# INLINE matchesKey #-}
  {-# INLINE writeKey #-}
  {-# INLINE copyKey #-}
  {-# INLINE indexKey #-}
  {-# INLINE matchesFrozenKey #-}

--------------------------------------------------------------------------------
-- The three column shapes
--------------------------------------------------------------------------------

-- These are three separate types rather than one type with a shape field, for
-- the same reason a sparse universe carries a separate live-row mask:
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

-- | Read one dense-column row.  Invalid row numbers are rejected rather than
-- delegated to the underlying vector's partial indexing operation.
colAt :: Elem r => Col e r -> Int -> Maybe r
colAt (Col n values) i
  | 0 <= i && i < n = Just (atStore values i)
  | otherwise       = Nothing

-- | Inspect a column without exposing its physical store.
colValues :: Elem r => Col e r -> [r]
colValues (Col n values) = map (atStore values) [0 .. n - 1]

-- | 1:1 with holes. The ordinary constructor represents absence explicitly as
-- 'Nothing'.  The word-backed constructor is used only by the trusted cache
-- reader: it keeps the cache's all-ones hole word in flat storage, but
-- 'sparseAt' translates that word to 'Nothing' before it can escape.  Thus even
-- the fast representation never manufactures an invalid 'Id'.
data SparseCol e r where
  SparseCol      :: !(BV.Vector (Maybe r)) -> SparseCol e r
  SparseWordCol  :: !(V.Vector Word64) -> SparseCol e Int

mkSparseCol :: [Maybe r] -> SparseCol e r
mkSparseCol = SparseCol . BV.fromList

sparseColLen :: SparseCol e r -> Int
sparseColLen (SparseCol values) = BV.length values
sparseColLen (SparseWordCol values) = V.length values

sparseAt :: SparseCol e r -> Int -> Maybe r
sparseAt (SparseCol values) i
  | 0 <= i && i < BV.length values = values BV.! i
  | otherwise                      = Nothing
sparseAt (SparseWordCol values) i
  | i < 0 || i >= V.length values = Nothing
  | value == maxBound             = Nothing
  | otherwise                     = Just (fromIntegral value)
  where
    value = values V.! i
{-# INLINE sparseAt #-}

sparseMask :: SparseCol e r -> [Bool]
sparseMask (SparseCol values) = map isJust (BV.toList values)
sparseMask (SparseWordCol values) = map (/= maxBound) (V.toList values)

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

-- | Read one CSR row.  Invalid row numbers are rejected.
multiAt :: Elem r => MultiCol e r -> Int -> Maybe [r]
multiAt (MultiCol n offsets values) i
  | 0 <= i && i < n =
      Just (map (atStore values) [off32 offsets i .. off32 offsets (i + 1) - 1])
  | otherwise = Nothing

-- | Inspect all CSR rows without exposing offsets or physical value storage.
multiValues :: Elem r => MultiCol e r -> [[r]]
multiValues column =
  [ values
  | i <- [0 .. multiColLen column - 1]
  , Just values <- [multiAt column i]
  ]

--------------------------------------------------------------------------------
-- Caches an operator builds
--------------------------------------------------------------------------------

-- The two shapes above are loaded from data; these two are produced by the
-- materializing operators in "Prela.PullStaged.Materialize" and read back as leaves,
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

-- | The same dense aggregate layout addressed by an explicitly bounded
-- integer key. Keeping it distinct from 'Dense' prevents an arbitrary 'Int'
-- from being confused with a validated entity identifier while still allowing
-- compact packed group keys (for example TPC-H Q1's two one-byte flags) to use
-- direct array indexing.
data DenseInt t = DenseInt !Int !(UV.Vector t) !(UArray Int Bool)

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
                       !(Keys d)         -- key, meaningful where hash /= 0
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
tableSlot :: (Hashable d, Key d) => Table d t -> d -> Int
tableSlot (Table mask hs ks _) d = go (fromIntegral h .&. mask)
  where
    h = slotHash d
    go i = case hs UV.! i of
             0                                -> -1
             h' | h' == h && matchesFrozenKey ks i d -> i
                | otherwise                   -> go ((i + 1) .&. mask)
{-# INLINE tableSlot #-}

-- A dense membership set: the identity relation on whichever ids are present.
-- `UArray Int Bool` is bit-packed by GHC, so a test is a word load and a shift,
-- which is the whole point of it over a `Map`. Driving it scans every bit rather
-- than using Rust's trailing-zeros skip, so it is best used probed.
newtype Bits e = Bits (UArray Int Bool)

-- | A membership set over `0 .. n-1` from the ids that are present. Ids outside
-- the range are dropped.
mkBits :: Int -> [Id e] -> Bits e
mkBits n ids = Bits (runSTUArray (do
  bs <- newBits n False
  mapM_ (\x -> let i = idIndex x
                in when (i < n) (writeArray bs i True)) ids
  return bs))

mkBitsFromMask :: [Bool] -> Bits e
mkBitsFromMask = Bits . packU

-- Typed wrappers so the two stores above are built in one pass over the input.
-- `freeze` alone leaves the mutable type ambiguous; these pin it.
newSlots :: UV.Unbox t => Int -> t -> ST s (UMV.MVector s t)
newSlots = UMV.replicate

newBits :: Int -> Bool -> ST s (STUArray s Int Bool)
newBits n v = newArray (0, n - 1) v

--------------------------------------------------------------------------------
-- Accessors the staged leaves splice
--------------------------------------------------------------------------------

-- The staged engine emits leaf bodies as CODE, so everything a leaf reads has to
-- be a top-level name it can mention by reference. These three are the bit-mask
-- reads that "Prela.PullStaged.Ops" needs, factored out of the leaves that used to
-- write them inline.

-- | Test a mask bit. Callers establish the range first; the array operation is
-- checked as a final guard.
atBit :: UArray Int Bool -> Int -> Bool
atBit bs i = bs U.! i
{-# INLINE atBit #-}

-- | How many bits a mask covers.
bitsLen :: UArray Int Bool -> Int
bitsLen bs = case U.bounds bs of (lo, hi) -> hi - lo + 1
{-# INLINE bitsLen #-}

-- | Is this id inside a universe of the given size?
idInRange :: Int -> Id e -> Bool
idInRange n x = idIndex x < n
{-# INLINE idInRange #-}

-- | Is this id's bit set? Range-checked, since a probed id is untrusted.
bitsMember :: Bits e -> Id e -> Bool
bitsMember (Bits bs) x = case U.bounds bs of
                          (lo, hi) -> lo <= i && i <= hi && bs U.! i
  where
    i = idIndex x
{-# INLINE bitsMember #-}
