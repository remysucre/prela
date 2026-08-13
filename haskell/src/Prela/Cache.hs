{-# LANGUAGE ScopedTypeVariables #-}

-- | Fast and checked reading, plus writing, of cache-format-v2 column files.
--
-- Unsuffixed readers are the default trusted mmap path. The @Checked@ readers
-- validate every offset and identifier and copy numeric payloads into managed
-- storage. Both paths construct the same column types. Files begin with a
-- 32-byte little-endian header containing a kind tag and two shape counts;
-- numeric payloads are 64-bit, string and CSR offsets are 32-bit, and CSR word
-- payloads begin at an eight-byte boundary. Writers in this module are used by
-- tests; the Rust regeneration tool produces benchmark caches in the same
-- format.
module Prela.Cache
  ( loadInts
  , loadDoubles
  , loadIds
  , loadStrs
  , loadMultiInts
  , loadMultiIds
  , loadMultiStrs
  , loadIntsChecked
  , loadDoublesChecked
  , loadIdsChecked
  , loadStrsChecked
  , loadMultiIntsChecked
  , loadMultiIdsChecked
  , loadMultiStrsChecked
  , validityBits
  , validityUniverse
  , writeInts
  , writeDoubles
  , writeIds
  , writeStrs
  , writeMultiInts
  , writeMultiIds
  , writeMultiStrs
  ) where

import Control.Monad (unless, when)
import Data.Bits (shiftL, (.|.))
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Builder as B
import qualified Data.ByteString.Internal as BSI
import qualified Data.ByteString.Lazy as BL
import Data.Int (Int64)
import qualified Data.Vector.Storable as V
import Data.Word (Word32, Word64)
import Foreign.ForeignPtr (castForeignPtr)
import Foreign.Storable (Storable, alignment, sizeOf)
import GHC.ByteOrder (ByteOrder (LittleEndian), targetByteOrder)
import GHC.Float (castWord64ToDouble)
import System.FilePath ((</>))
import System.IO.MMap (mmapFileByteString)

import Prela.Id
import Prela.Storage.Internal

-- | Physical payload variants identified by the cache header.
data Kind
  = DenseI64
  | DenseF64
  | DenseStr
  | CsrWords
  | CsrStr
  deriving (Eq, Show)

-- | Encode a physical payload kind in the cache header.
kindCode :: Kind -> Word32
kindCode DenseI64 = 0
kindCode DenseF64 = 1
kindCode DenseStr = 2
kindCode CsrWords = 3
kindCode CsrStr   = 4

-- | Eight-byte signature identifying cache format version 2.
magic :: ByteString
magic = BS.pack [0x70, 0x72, 0x65, 0x6c, 0x61, 0x32, 0x00, 0x00]

-- | Fixed byte length of a format-v2 header.
headerLen :: Int
headerLen = 32

-- | On-disk sentinel representing a missing entity reference.
holeWord :: Word64
holeWord = maxBound

-- | Round a byte offset up to the next eight-byte boundary.
align8 :: Int -> Int
align8 x = (x + 7) `div` 8 * 8

-- | Raise a user-facing cache error prefixed by the affected path.
cacheError :: FilePath -> String -> IO a
cacheError path message = ioError (userError (path ++ ": " ++ message))

-- | Read and validate the common header using the supplied byte source.
openWith
  :: (FilePath -> IO ByteString)
  -> FilePath -> String -> Kind -> IO (FilePath, Int, Int, ByteString)
openWith readBytes dir name expected = do
  let path = dir </> name ++ ".bin"
  bytes <- readBytes path
  unless (BS.length bytes >= headerLen && BS.take 8 bytes == magic) $
    cacheError path "not a cache-format-v2 file"
  let actual = fromIntegral (wordLE bytes 8 4) :: Word32
      reserved = wordLE bytes 12 4
  unless (actual == kindCode expected) $
    cacheError path ("cache kind " ++ show actual ++ " but loader expects " ++ show expected)
  unless (reserved == 0) $
    cacheError path "reserved header word is nonzero"
  n <- countToInt path "primary count" (wordLE bytes 16 8)
  m <- countToInt path "secondary count" (wordLE bytes 24 8)
  pure (path, n, m, bytes)

-- | Open a cache file by copying it into a managed strict byte string.
openChecked :: FilePath -> String -> Kind -> IO (FilePath, Int, Int, ByteString)
openChecked = openWith BS.readFile

-- | Memory-map a cache file after checking that the host matches its physical
-- little-endian, 64-bit numeric representation.
openFast :: FilePath -> String -> Kind -> IO (FilePath, Int, Int, ByteString)
openFast dir name expected = do
  let path = dir </> name ++ ".bin"
  unless (targetByteOrder == LittleEndian && sizeOf (0 :: Int) == 8) $
    cacheError path "fast cache loading requires a little-endian 64-bit host"
  openWith (\file -> mmapFileByteString file Nothing) dir name expected

-- | Convert an unsigned header count to a host 'Int', rejecting overflow.
countToInt :: FilePath -> String -> Word64 -> IO Int
countToInt path label value
  | value > fromIntegral (maxBound :: Int) = cacheError path (label ++ " exceeds Int range")
  | otherwise = pure (fromIntegral value)

-- | Decode a little-endian unsigned word of the requested byte width.
wordLE :: ByteString -> Int -> Int -> Word64
wordLE bytes offset width =
  foldr step 0 [0 .. width - 1]
  where
    step i acc = acc `shiftL` 8 .|. fromIntegral (BS.index bytes (offset + i))

-- | Validate a fixed-width payload slice and return its exclusive end offset.
sliceEnd :: FilePath -> ByteString -> Int -> Int -> Int -> IO Int
sliceEnd path bytes offset count width
  | offset < 0 || count < 0 || width <= 0 = cacheError path "negative payload dimension"
  | offset > BS.length bytes = cacheError path "payload offset lies past end of file"
  | count > (BS.length bytes - offset) `div` width = cacheError path "cache file is truncated"
  | otherwise = pure (offset + count * width)

-- | Require a decoded payload to consume the entire cache file.
requireExactEnd :: FilePath -> ByteString -> Int -> IO ()
requireExactEnd path bytes end =
  unless (end == BS.length bytes) (cacheError path "payload length does not match header")

-- | Decode copied little-endian 64-bit words from a byte range.
words64 :: ByteString -> Int -> Int -> V.Vector Word64
words64 bytes offset count = V.generate count (\i -> wordLE bytes (offset + i * 8) 8)

-- | Decode copied little-endian 32-bit words from a byte range.
words32 :: ByteString -> Int -> Int -> V.Vector Word32
words32 bytes offset count =
  V.generate count (\i -> fromIntegral (wordLE bytes (offset + i * 4) 4))

-- | Decode copied signed 64-bit integers into host-sized integers.
ints64 :: ByteString -> Int -> Int -> V.Vector Int
ints64 bytes offset count =
  V.map (fromIntegral . (fromIntegral :: Word64 -> Int64)) (words64 bytes offset count)

-- | Validate a zero-based monotonic offset vector ending at a payload limit.
validateOffsets :: FilePath -> String -> Int -> V.Vector Word32 -> IO ()
validateOffsets path label limit offsets = do
  unless (not (V.null offsets) && V.head offsets == 0) $
    cacheError path (label ++ " offsets must start at zero")
  unless (and (V.toList (V.zipWith (<=) offsets (V.tail offsets)))) $
    cacheError path (label ++ " offsets are not monotonic")
  unless (fromIntegral (V.last offsets) == limit) $
    cacheError path (label ++ " final offset does not match its payload count")

--------------------------------------------------------------------------------
-- Trusted mmap readers (the default)
--------------------------------------------------------------------------------

-- | Add one to a shape count, rejecting host-integer overflow.
successor :: FilePath -> String -> Int -> IO Int
successor path label value
  | value == maxBound = cacheError path (label ++ " is too large")
  | otherwise = pure (value + 1)

-- The full-file mmap begins on a page boundary and every typed cache section is
-- 4- or 8-byte aligned. The vector retains the mapping's ForeignPtr, so it is
-- unmapped when the loaded schema becomes unreachable rather than leaked.
-- | View an aligned cache range as a storable vector without copying it.
-- The vector retains the original mapping's foreign pointer.
mappedVector
  :: forall a. Storable a
  => FilePath -> ByteString -> Int -> Int -> V.Vector a
mappedVector path bytes byteOffset count
  | totalOffset `mod` alignment (undefined :: a) /= 0 =
      error (path ++ ": mapped cache payload is misaligned")
  | otherwise =
      V.unsafeFromForeignPtr (castForeignPtr backing) elementOffset count
  where
    (backing, baseOffset, _) = BSI.toForeignPtr bytes
    totalOffset = baseOffset + byteOffset
    elementOffset = totalOffset `div` sizeOf (undefined :: a)

-- | Memory-map a dense signed-integer column.
loadInts :: FilePath -> String -> IO (Col e Int)
loadInts dir name = do
  (path, n, m, bytes) <- openFast dir name DenseI64
  unless (m == 0) (cacheError path "dense integer secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  pure (Col n (IntStore (mappedVector path bytes headerLen n)))

-- | Memory-map a dense double-precision column.
loadDoubles :: FilePath -> String -> IO (Col e Double)
loadDoubles dir name = do
  (path, n, m, bytes) <- openFast dir name DenseF64
  unless (m == 0) (cacheError path "dense double secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  pure (Col n (DblStore (mappedVector path bytes headerLen n)))

-- | Memory-map a nullable entity-reference index column. Hole sentinels remain
-- encoded until a row is read.
loadIds :: FilePath -> String -> IO (SparseCol e Int)
loadIds dir name = do
  (path, n, m, bytes) <- openFast dir name DenseI64
  unless (m == 0) (cacheError path "dense id secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  let values = mappedVector path bytes headerLen n :: V.Vector Word64
  pure (SparseWordCol values)

-- | Memory-map a dense byte-string column backed by offsets and shared bytes.
loadStrs :: FilePath -> String -> IO (Col e ByteString)
loadStrs dir name = do
  (path, n, byteCount, bytes) <- openFast dir name DenseStr
  offsetCount <- successor path "string count" n
  offsetEnd <- sliceEnd path bytes headerLen offsetCount 4
  dataEnd <- sliceEnd path bytes offsetEnd byteCount 1
  requireExactEnd path bytes dataEnd
  let offsets = mappedVector path bytes headerLen offsetCount :: V.Vector Word32
      payload = BS.take byteCount (BS.drop offsetEnd bytes)
  pure (Col n (BsStore offsets payload))

-- | Memory-map a CSR multi-valued signed-integer column.
loadMultiInts :: FilePath -> String -> IO (MultiCol e Int)
loadMultiInts = loadMultiWords

-- | Memory-map a CSR multi-valued entity-reference index column.
loadMultiIds :: FilePath -> String -> IO (MultiCol e Int)
loadMultiIds = loadMultiWords

-- | Shared mmap implementation for CSR 64-bit word payloads.
loadMultiWords :: FilePath -> String -> IO (MultiCol e Int)
loadMultiWords dir name = do
  (path, n, m, bytes) <- openFast dir name CsrWords
  offsetCount <- successor path "row count" n
  offsetsEnd <- sliceEnd path bytes headerLen offsetCount 4
  let valuesAt = align8 offsetsEnd
  when (valuesAt > BS.length bytes) $
    cacheError path "aligned values offset lies past end of file"
  valuesEnd <- sliceEnd path bytes valuesAt m 8
  requireExactEnd path bytes valuesEnd
  let offsets = mappedVector path bytes headerLen offsetCount :: V.Vector Word32
      values = mappedVector path bytes valuesAt m :: V.Vector Int
  pure (MultiCol n offsets (IntStore values))

-- | Memory-map a CSR multi-valued byte-string column.
loadMultiStrs :: FilePath -> String -> IO (MultiCol e ByteString)
loadMultiStrs dir name = do
  (path, n, m, bytes) <- openFast dir name CsrStr
  rowCount <- successor path "row count" n
  stringCount <- successor path "string count" m
  rowEnd <- sliceEnd path bytes headerLen rowCount 4
  stringEnd <- sliceEnd path bytes rowEnd stringCount 4
  let rowOffsets = mappedVector path bytes headerLen rowCount :: V.Vector Word32
      stringOffsets = mappedVector path bytes rowEnd stringCount :: V.Vector Word32
      payload = BS.drop stringEnd bytes
  pure (MultiCol n rowOffsets (BsStore stringOffsets payload))

--------------------------------------------------------------------------------
-- Fully checked readers
--------------------------------------------------------------------------------

-- | Validate and copy a dense signed-integer column.
loadIntsChecked :: FilePath -> String -> IO (Col e Int)
loadIntsChecked dir name = do
  (path, n, m, bytes) <- openChecked dir name DenseI64
  unless (m == 0) (cacheError path "dense integer secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  pure (Col n (IntStore (ints64 bytes headerLen n)))

-- | Validate and copy a dense double-precision column.
loadDoublesChecked :: FilePath -> String -> IO (Col e Double)
loadDoublesChecked dir name = do
  (path, n, m, bytes) <- openChecked dir name DenseF64
  unless (m == 0) (cacheError path "dense double secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  pure (Col n (DblStore (V.map castWord64ToDouble (words64 bytes headerLen n))))

-- | Load a nullable foreign-key column. The on-disk all-ones sentinel becomes
-- absence in a sparse relation and is never represented as an 'Id'.
loadIdsChecked :: FilePath -> String -> IO (SparseCol e Int)
loadIdsChecked dir name = do
  (path, n, m, bytes) <- openChecked dir name DenseI64
  unless (m == 0) (cacheError path "dense id secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  values <- traverse (decodeOptionalIndex path) (V.toList (words64 bytes headerLen n))
  pure (mkSparseCol values)

-- | Decode the hole sentinel or a range-checked host entity index.
decodeOptionalIndex :: FilePath -> Word64 -> IO (Maybe Int)
decodeOptionalIndex _ value | value == holeWord = pure Nothing
decodeOptionalIndex path value = Just <$> countToInt path "identifier" value

-- | Validate all offsets and copy a dense byte-string column's offset vector.
loadStrsChecked :: FilePath -> String -> IO (Col e ByteString)
loadStrsChecked dir name = do
  (path, n, byteCount, bytes) <- openChecked dir name DenseStr
  offsetEnd <- sliceEnd path bytes headerLen (n + 1) 4
  dataEnd <- sliceEnd path bytes offsetEnd byteCount 1
  requireExactEnd path bytes dataEnd
  let offsets = words32 bytes headerLen (n + 1)
      payload = BS.take byteCount (BS.drop offsetEnd bytes)
  validateOffsets path "string" byteCount offsets
  pure (Col n (BsStore offsets payload))

-- | Validate and copy a CSR signed-integer column.
loadMultiIntsChecked :: FilePath -> String -> IO (MultiCol e Int)
loadMultiIntsChecked dir name = do
  (n, offsets, values) <- csrWordsChecked dir name
  pure (MultiCol n offsets (IntStore (V.map fromIntegral values)))

-- | Validate and copy a CSR entity-reference index column.
loadMultiIdsChecked :: FilePath -> String -> IO (MultiCol e Int)
loadMultiIdsChecked dir name = do
  (path, n, offsets, values) <- csrWord64Checked dir name
  identifiers <- traverse (countToInt path "identifier") (V.toList values)
  pure (MultiCol n offsets (packStore identifiers))

-- | Decode validated CSR words as signed 64-bit values.
csrWordsChecked :: FilePath -> String -> IO (Int, V.Vector Word32, V.Vector Int64)
csrWordsChecked dir name = do
  (_path, n, offsets, values) <- csrWord64Checked dir name
  pure (n, offsets, V.map fromIntegral values)

-- | Validate a CSR word payload and return its row count, offsets, and words.
csrWord64Checked :: FilePath -> String -> IO (FilePath, Int, V.Vector Word32, V.Vector Word64)
csrWord64Checked dir name = do
  (path, n, m, bytes) <- openChecked dir name CsrWords
  offsetsEnd <- sliceEnd path bytes headerLen (n + 1) 4
  let valuesAt = align8 offsetsEnd
  when (valuesAt > BS.length bytes) (cacheError path "aligned values offset lies past end of file")
  valuesEnd <- sliceEnd path bytes valuesAt m 8
  requireExactEnd path bytes valuesEnd
  let offsets = words32 bytes headerLen (n + 1)
  validateOffsets path "row" m offsets
  pure (path, n, offsets, words64 bytes valuesAt m)

-- | Validate row and string offsets in a CSR byte-string column.
loadMultiStrsChecked :: FilePath -> String -> IO (MultiCol e ByteString)
loadMultiStrsChecked dir name = do
  (path, n, m, bytes) <- openChecked dir name CsrStr
  rowEnd <- sliceEnd path bytes headerLen (n + 1) 4
  stringEnd <- sliceEnd path bytes rowEnd (m + 1) 4
  let rowOffsets = words32 bytes headerLen (n + 1)
      stringOffsets = words32 bytes rowEnd (m + 1)
      payload = BS.drop stringEnd bytes
  validateOffsets path "row" m rowOffsets
  validateOffsets path "string" (BS.length payload) stringOffsets
  pure (MultiCol n rowOffsets (BsStore stringOffsets payload))

-- | Build dense membership bits from the live rows of a nullable reference
-- column.
validityBits :: SparseCol e Int -> Bits e
validityBits column = mkBitsFromMask (sparseMask column)

-- | Build a sparse entity universe from the live rows of a nullable reference
-- column.
validityUniverse :: SparseCol e Int -> Universe e
validityUniverse = universeFromMask . sparseMask

-- | Encode a format-v2 header for a physical kind and its two shape counts.
header :: Kind -> Int -> Int -> B.Builder
header kind n m =
  B.byteString magic <> B.word32LE (kindCode kind) <> B.word32LE 0
                     <> B.word64LE (fromIntegral n) <> B.word64LE (fromIntegral m)

-- | Write one complete cache file from a header description and payload.
write :: FilePath -> String -> Kind -> Int -> Int -> B.Builder -> IO ()
write dir name kind n m payload =
  BL.writeFile (dir </> name ++ ".bin")
               (B.toLazyByteString (header kind n m <> payload))

-- | Write a dense signed-integer cache column.
writeInts :: FilePath -> String -> [Int] -> IO ()
writeInts dir name values =
  write dir name DenseI64 (length values) 0 (foldMap (B.int64LE . fromIntegral) values)

-- | Write a dense double-precision cache column.
writeDoubles :: FilePath -> String -> [Double] -> IO ()
writeDoubles dir name values =
  write dir name DenseF64 (length values) 0 (foldMap B.doubleLE values)

-- | Write a nullable entity-reference column using the hole sentinel.
writeIds :: FilePath -> String -> [Maybe (Id t)] -> IO ()
writeIds dir name values =
  write dir name DenseI64 (length values) 0 (foldMap encode values)
  where
    encode = B.word64LE . maybe holeWord (fromIntegral . idIndex)

-- | Write a dense byte-string column as offsets followed by concatenated bytes.
writeStrs :: FilePath -> String -> [ByteString] -> IO ()
writeStrs dir name values = do
  let payload = BS.concat values
      offsets = scanl (+) 0 (map BS.length values)
  ensureWord32 dir name "string payload" (BS.length payload)
  write dir name DenseStr (length values) (BS.length payload)
        (foldMap (B.word32LE . fromIntegral) offsets <> B.byteString payload)

-- | Write a CSR multi-valued signed-integer column.
writeMultiInts :: FilePath -> String -> [[Int]] -> IO ()
writeMultiInts dir name = writeCsrWords dir name . map (map fromIntegral)

-- | Write a CSR multi-valued entity-reference column.
writeMultiIds :: FilePath -> String -> [[Id t]] -> IO ()
writeMultiIds dir name = writeCsrWords dir name . map (map (fromIntegral . idIndex))

-- | Shared writer for CSR 64-bit word payloads, including alignment padding.
writeCsrWords :: FilePath -> String -> [[Int64]] -> IO ()
writeCsrWords dir name rows = do
  let n = length rows
      m = sum (map length rows)
      offsets = scanl (+) 0 (map length rows)
      endOffsets = headerLen + (n + 1) * 4
      padding = replicate (align8 endOffsets - endOffsets) 0
  ensureWord32 dir name "CSR value count" m
  write dir name CsrWords n m
        (foldMap (B.word32LE . fromIntegral) offsets
         <> foldMap B.word8 padding
         <> foldMap B.int64LE (concat rows))

-- | Write a CSR byte-string column with row offsets, string offsets, and bytes.
writeMultiStrs :: FilePath -> String -> [[ByteString]] -> IO ()
writeMultiStrs dir name rows = do
  let strings = concat rows
      n = length rows
      m = length strings
      rowOffsets = scanl (+) 0 (map length rows)
      stringOffsets = scanl (+) 0 (map BS.length strings)
      payload = BS.concat strings
  ensureWord32 dir name "CSR string count" m
  ensureWord32 dir name "CSR string payload" (BS.length payload)
  write dir name CsrStr n m
        (foldMap (B.word32LE . fromIntegral) rowOffsets
         <> foldMap (B.word32LE . fromIntegral) stringOffsets
         <> B.byteString payload)

-- | Reject a shape or byte count which cannot be represented by a 32-bit
-- cache-format offset.
ensureWord32 :: FilePath -> String -> String -> Int -> IO ()
ensureWord32 dir name label value =
  when (toInteger value > toInteger (maxBound :: Word32)) $
    cacheError (dir </> name ++ ".bin") (label ++ " exceeds the cache format's Word32 range")
