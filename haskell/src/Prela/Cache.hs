{-# LANGUAGE ScopedTypeVariables #-}

-- | Reading the on-disk cache — cache format v2, the same files the Rust port
-- reads (`rust/src/format.rs` is the spec, `regen` the writer).
--
-- The format exists so that loading is not a conversion. Every load-time
-- decision has already been made by the writer: ids are 0-based with the hole
-- word baked in, dates are pre-parsed to yyyymmdd integers, strings are laid out
-- as offsets plus one packed buffer, multi-valued columns are CSR. So the reader
-- here memory-maps the file, checks the header, and hands back a column that
-- POINTS INTO the mapping. Nothing is copied and nothing is walked — a
-- gigabyte-scale table costs a page-table entry, not a load loop.
--
-- What makes that possible is that the storage types in "Prela" were chosen to
-- BE these on-disk layouts, so a `Storable` vector is just a pointer, an offset
-- and a length into the mapped bytes. Where the Rust port has to leak the mmap
-- to hand out `&'static` slices, the vectors here retain the mapping's
-- finalizer, so it is unmapped once the last column over it is collected.
--
-- One file per column, named `<dir>/<Entity>_<field>.bin` — the field name is
-- the filename, verbatim, which is what lets a schema layer generate these calls
-- mechanically.
module Prela.Cache
  ( -- * Loading
    loadInts
  , loadDoubles
  , loadIds
  , loadStrs
  , loadMultiInts
  , loadMultiIds
  , loadMultiStrs
  , validityBits
    -- * Writing (fixtures, and building a cache without the Rust regen)
  , writeInts
  , writeDoubles
  , writeIds
  , writeStrs
  , writeMultiInts
  , writeMultiIds
  , writeMultiStrs
    -- * The format itself
  , Kind (..)
  , magic
  , headerLen
  , holeWord
  , align8
  ) where

import Control.Monad (when)
import Data.Array.ST (writeArray, runSTUArray)
import Data.Bits (shiftL, (.|.))
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Builder as B
import qualified Data.ByteString.Internal as BSI
import qualified Data.ByteString.Lazy as BL
import qualified Data.ByteString.Unsafe as BSU
import Data.Int (Int64)
import qualified Data.Vector.Storable as V
import Data.Word (Word32, Word64, Word8)
import Foreign.ForeignPtr (castForeignPtr)
import Foreign.ForeignPtr.Unsafe (unsafeForeignPtrToPtr)
import Foreign.Ptr (Ptr, ptrToIntPtr)
import Foreign.Storable (Storable, alignment, sizeOf)
import System.IO.MMap (mmapFileByteString)

import Prela

--------------------------------------------------------------------------------
-- The format
--------------------------------------------------------------------------------

-- | Every file is a 32-byte header followed by a payload:
--
-- >  [ 0..8)   magic  "prela2\0\0"
-- >  [ 8..12)  u32    kind
-- >  [12..16)  u32    reserved, 0
-- >  [16..24)  u64    n — number of key slots (the universe size)
-- >  [24..32)  u64    m — kind-specific second count
--
-- All little-endian. The payload starts 8-byte aligned, and since a mapping
-- starts page-aligned, so does every payload — which is what lets the numeric
-- payloads be viewed in place rather than copied.
data Kind
  = DenseI64  -- ^ @n@ 8-byte words: scalars, dates, and id/FK columns. @m = 0@.
  | DenseF64  -- ^ @n@ doubles. @m = 0@.
  | DenseStr  -- ^ @n+1@ u32 offsets then @m@ bytes. Holes are empty strings.
  | CsrWords  -- ^ @n+1@ u32 offsets, pad to 8, then @m@ 8-byte words.
  | CsrStr    -- ^ @n+1@ u32 row offsets, @m+1@ u32 byte offsets, then bytes.
  deriving (Eq, Show)

kindCode :: Kind -> Word32
kindCode DenseI64 = 0
kindCode DenseF64 = 1
kindCode DenseStr = 2
kindCode CsrWords = 3
kindCode CsrStr   = 4

magic :: ByteString
magic = BS.pack [0x70, 0x72, 0x65, 0x6c, 0x61, 0x32, 0x00, 0x00] -- "prela2\0\0"

headerLen :: Int
headerLen = 32

-- | The hole word in a dense id column, as written to disk: all ones. Read back
-- as a signed machine word that is @-1@, which is out of range for every table,
-- so a hole needs no presence bit and no branch of its own — it simply fails the
-- range check each probe already performs. This is 'noId'.
holeWord :: Word64
holeWord = maxBound

align8 :: Int -> Int
align8 x = (x + 7) `div` 8 * 8

--------------------------------------------------------------------------------
-- Opening a file
--------------------------------------------------------------------------------

-- | Map @<dir>/<name>.bin@, validate the header, return @(n, m, bytes)@.
--
-- Failures are loud on purpose. A v1 cache — the retired pair-stream format,
-- whose files begin with a count rather than the magic — read as v2 would be an
-- off-by-one disaster rather than an error, so a bad magic aborts, and so does a
-- kind the caller did not ask for.
open :: FilePath -> String -> Kind -> IO (Int, Int, ByteString)
open dir name kind = do
  let path = dir ++ "/" ++ name ++ ".bin"
  bytes <- mmapFileByteString path Nothing
  when (BS.length bytes < headerLen || BS.take 8 bytes /= magic) $
    error (path ++ ": not a cache-format-v2 file (stale v1 cache?) — rerun regen")
  let k = leWord bytes 8 4
  when (k /= fromIntegral (kindCode kind)) $
    error (path ++ ": cache kind " ++ show k ++ " but the loader expects "
                ++ show kind ++ " (" ++ show (kindCode kind) ++ ")"
                ++ " — cache/loader mismatch, rerun regen")
  return (fromIntegral (leWord bytes 16 8), fromIntegral (leWord bytes 24 8), bytes)

-- Little-endian integer of `k` bytes. Only ever used on the header, so the
-- byte-at-a-time loop costs nothing.
leWord :: ByteString -> Int -> Int -> Word64
leWord bs off k =
  foldr (\i acc -> acc `shiftL` 8 .|. fromIntegral (BSU.unsafeIndex bs (off + i)))
        0 [0 .. k - 1]

--------------------------------------------------------------------------------
-- Viewing bytes as storage, without copying
--------------------------------------------------------------------------------

-- | Reinterpret @n@ elements at a byte offset as a vector, sharing the mapping.
--
-- The alignment check is not paranoia about this format, which aligns every
-- payload by construction; it is there because an unaligned load is undefined
-- rather than merely slow, and a truncated or hand-written file is exactly the
-- kind of thing a reader meets.
viewVec :: forall a. Storable a => String -> ByteString -> Int -> Int -> V.Vector a
viewVec what bs off n
  | off + n * sizeOf (undefined :: a) > BS.length bs =
      error (what ++ ": cache file truncated")
  | ptrToIntPtr p `mod` fromIntegral (alignment (undefined :: a)) /= 0 =
      error (what ++ ": payload is not aligned for its element type")
  | otherwise = V.unsafeFromForeignPtr0 (castForeignPtr fp') n
  where
    -- `toForeignPtr0` hands back the start of the bytes and their length; since
    -- bytestring 0.11 a ByteString carries no separate offset.
    (fp, _len) = BSI.toForeignPtr0 bs
    fp' = fp `BSI.plusForeignPtr` off
    p = unsafeForeignPtrToPtr fp' :: Ptr Word8

-- A byte range of the mapping as a ByteString. O(1) — the slice shares the
-- mapping's finalizer, so the strings a column hands out keep it alive.
viewBytes :: String -> ByteString -> Int -> Int -> ByteString
viewBytes what bs off n
  | off + n > BS.length bs = error (what ++ ": cache file truncated")
  | otherwise = BSU.unsafeTake n (BSU.unsafeDrop off bs)

--------------------------------------------------------------------------------
-- Dense columns
--------------------------------------------------------------------------------

-- | A dense integer column: scalars, and dates as pre-parsed yyyymmdd.
loadInts :: FilePath -> String -> IO (Col e Int)
loadInts dir name = do
  (n, _, bs) <- open dir name DenseI64
  return (Col n (IntStore (viewVec name bs headerLen n)))

-- | A dense double column.
loadDoubles :: FilePath -> String -> IO (Col e Double)
loadDoubles dir name = do
  (n, _, bs) <- open dir name DenseF64
  return (Col n (DblStore (viewVec name bs headerLen n)))

-- | A dense id column — a foreign key. Same bytes as 'loadInts': the words are
-- 0-based ids with 'holeWord' in the gaps, and since `Id` is a newtype over a
-- machine word the tagging is a type-level relabel with no per-element work.
-- Which entity the ids point at comes from the caller's type annotation, which
-- is the whole of the FK type checking.
loadIds :: FilePath -> String -> IO (Col e (Id t))
loadIds dir name = do
  (n, _, bs) <- open dir name DenseI64
  return (Col n (IdStore (viewVec name bs headerLen n)))

-- | A dense string column. Offsets and bytes are both views; holes are empty
-- strings, matching the writer.
loadStrs :: FilePath -> String -> IO (Col e ByteString)
loadStrs dir name = do
  (n, m, bs) <- open dir name DenseStr
  let offs = viewVec name bs headerLen (n + 1)
      dat  = viewBytes name bs (headerLen + (n + 1) * 4) m
  return (Col n (BsStore offs dat))

--------------------------------------------------------------------------------
-- CSR multi-valued columns
--------------------------------------------------------------------------------

-- | A multi-valued integer column.
loadMultiInts :: FilePath -> String -> IO (MultiCol e Int)
loadMultiInts dir name = do
  (n, offs, vals) <- csrWords dir name
  return (MultiCol n offs (IntStore vals))

-- | A multi-valued id column — the many side of a foreign key.
loadMultiIds :: FilePath -> String -> IO (MultiCol e (Id t))
loadMultiIds dir name = do
  (n, offs, vals) <- csrWords dir name
  return (MultiCol n offs (IdStore vals))

csrWords :: Storable a
         => FilePath -> String -> IO (Int, V.Vector Word32, V.Vector a)
csrWords dir name = do
  (n, m, bs) <- open dir name CsrWords
  let offs = viewVec name bs headerLen (n + 1)
      vals = viewVec name bs (align8 (headerLen + (n + 1) * 4)) m
  return (n, offs, vals)

-- | A multi-valued string column: row offsets over string offsets over bytes,
-- which is exactly a 'MultiCol' whose element storage is the string layout, so
-- this too is three views and no copying.
loadMultiStrs :: FilePath -> String -> IO (MultiCol e ByteString)
loadMultiStrs dir name = do
  (n, m, bs) <- open dir name CsrStr
  let strOffAt = headerLen + (n + 1) * 4
      dataAt   = strOffAt + (m + 1) * 4
      rowOff = viewVec name bs headerLen (n + 1)
      strOff = viewVec name bs strOffAt (m + 1)
      dat    = viewBytes name bs dataAt (BS.length bs - dataAt)
  return (MultiCol n rowOff (BsStore strOff dat))

--------------------------------------------------------------------------------
-- Deriving a sparse universe
--------------------------------------------------------------------------------

-- | The live slots of an entity whose id space has holes, taken from one of its
-- foreign-key columns: a slot holding 'holeWord' is a gap in the id space rather
-- than a row with a missing value.
--
-- The cache stores no validity mask, because it does not need to — the holes are
-- already visible in any FK column of the entity, so the mask is derived at load
-- time in one pass. This is what feeds 'sparseUniverse'; the Rust port builds
-- the same bitset the same way.
validityBits :: Col e (Id t) -> Bits e
validityBits (Col n (IdStore vs)) = Bits (runSTUArray (do
  bs <- newBits n False
  mapM_ (\i -> when (V.unsafeIndex vs i >= 0) (writeArray bs i True)) [0 .. n - 1]
  return bs))

--------------------------------------------------------------------------------
-- Writing
--------------------------------------------------------------------------------

-- These are the writer half of the spec. The Rust `regen` binary is what
-- actually builds a JOB or TPC-H cache, from parquet; these exist so this port
-- can produce and round-trip its own files, which is how the reader above is
-- tested, and so a small cache can be built without leaving Haskell.

header :: Kind -> Int -> Int -> B.Builder
header kind n m =
  B.byteString magic <> B.word32LE (kindCode kind) <> B.word32LE 0
                     <> B.word64LE (fromIntegral n) <> B.word64LE (fromIntegral m)

write :: FilePath -> String -> Kind -> Int -> Int -> B.Builder -> IO ()
write dir name kind n m payload =
  BL.writeFile (dir ++ "/" ++ name ++ ".bin")
               (B.toLazyByteString (header kind n m <> payload))

-- | A dense integer column, one value per id.
writeInts :: FilePath -> String -> [Int] -> IO ()
writeInts dir name vs =
  write dir name DenseI64 (length vs) 0 (foldMap (B.int64LE . fromIntegral) vs)

-- | A dense double column.
writeDoubles :: FilePath -> String -> [Double] -> IO ()
writeDoubles dir name vs =
  write dir name DenseF64 (length vs) 0 (foldMap B.doubleLE vs)

-- | A dense id column. `Nothing` writes 'holeWord'.
writeIds :: FilePath -> String -> [Maybe (Id t)] -> IO ()
writeIds dir name vs =
  write dir name DenseI64 (length vs) 0 (foldMap word vs)
  where word = B.word64LE . maybe holeWord (\(Id i) -> fromIntegral i)

-- | A dense string column. A hole is the empty string.
writeStrs :: FilePath -> String -> [ByteString] -> IO ()
writeStrs dir name vs =
  write dir name DenseStr (length vs) (BS.length dat)
        (foldMap B.word32LE offs <> B.byteString dat)
  where
    offs = scanl (+) 0 (map (fromIntegral . BS.length) vs)
    dat  = BS.concat vs

-- | A CSR integer column, given one value list per id.
writeMultiInts :: FilePath -> String -> [[Int]] -> IO ()
writeMultiInts dir name = writeCsrWords dir name . map (map fromIntegral)

-- | A CSR id column, given one id list per id. An empty list is a key with no
-- values, which is how the format expresses both "missing" and "multi-valued".
writeMultiIds :: FilePath -> String -> [[Id t]] -> IO ()
writeMultiIds dir name = writeCsrWords dir name . map (map (\(Id i) -> fromIntegral i))

writeCsrWords :: FilePath -> String -> [[Int64]] -> IO ()
writeCsrWords dir name rows =
  write dir name CsrWords n m
        (foldMap B.word32LE offs <> pad <> foldMap B.int64LE (concat rows))
  where
    n    = length rows
    m    = sum (map length rows)
    offs = scanl (+) 0 (map (fromIntegral . length) rows) :: [Word32]
    -- The values are 8-byte, so the offsets array is padded up rather than
    -- letting an odd n push them onto a 4-byte boundary.
    pad  = foldMap B.word8 (replicate (align8 endOffs - endOffs) 0)
    endOffs = headerLen + (n + 1) * 4

-- | A CSR string column.
writeMultiStrs :: FilePath -> String -> [[ByteString]] -> IO ()
writeMultiStrs dir name rows =
  write dir name CsrStr n m
        (foldMap B.word32LE rowOff <> foldMap B.word32LE strOff <> B.byteString dat)
  where
    n      = length rows
    strs   = concat rows
    m      = length strs
    rowOff = scanl (+) 0 (map (fromIntegral . length) rows) :: [Word32]
    strOff = scanl (+) 0 (map (fromIntegral . BS.length) strs) :: [Word32]
    dat    = BS.concat strs
