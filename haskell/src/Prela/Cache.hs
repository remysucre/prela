{-# LANGUAGE ScopedTypeVariables #-}

-- | Checked reading and writing of cache-format-v2 column files.
--
-- The reader validates dimensions and offsets before constructing storage. It
-- copies numeric payloads into vectors instead of reinterpreting mmap memory, so
-- no unchecked pointer or indexing operations are required.
module Prela.Cache
  ( loadInts
  , loadDoubles
  , loadIds
  , loadStrs
  , loadMultiInts
  , loadMultiIds
  , loadMultiStrs
  , validityBits
  , validityUniverse
  , writeInts
  , writeDoubles
  , writeIds
  , writeStrs
  , writeMultiInts
  , writeMultiIds
  , writeMultiStrs
  , Kind (..)
  , magic
  , headerLen
  , holeWord
  , align8
  ) where

import Control.Monad (unless, when)
import Data.Bits (shiftL, (.|.))
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Builder as B
import qualified Data.ByteString.Lazy as BL
import Data.Int (Int64)
import qualified Data.Vector.Storable as V
import Data.Word (Word32, Word64)
import GHC.Float (castWord64ToDouble)
import System.FilePath ((</>))

import Prela.Id
import Prela.Storage

data Kind
  = DenseI64
  | DenseF64
  | DenseStr
  | CsrWords
  | CsrStr
  deriving (Eq, Show)

kindCode :: Kind -> Word32
kindCode DenseI64 = 0
kindCode DenseF64 = 1
kindCode DenseStr = 2
kindCode CsrWords = 3
kindCode CsrStr   = 4

magic :: ByteString
magic = BS.pack [0x70, 0x72, 0x65, 0x6c, 0x61, 0x32, 0x00, 0x00]

headerLen :: Int
headerLen = 32

holeWord :: Word64
holeWord = maxBound

align8 :: Int -> Int
align8 x = (x + 7) `div` 8 * 8

cacheError :: FilePath -> String -> IO a
cacheError path message = ioError (userError (path ++ ": " ++ message))

open :: FilePath -> String -> Kind -> IO (FilePath, Int, Int, ByteString)
open dir name expected = do
  let path = dir </> name ++ ".bin"
  bytes <- BS.readFile path
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

countToInt :: FilePath -> String -> Word64 -> IO Int
countToInt path label value
  | value > fromIntegral (maxBound :: Int) = cacheError path (label ++ " exceeds Int range")
  | otherwise = pure (fromIntegral value)

wordLE :: ByteString -> Int -> Int -> Word64
wordLE bytes offset width =
  foldr step 0 [0 .. width - 1]
  where
    step i acc = acc `shiftL` 8 .|. fromIntegral (BS.index bytes (offset + i))

sliceEnd :: FilePath -> ByteString -> Int -> Int -> Int -> IO Int
sliceEnd path bytes offset count width
  | offset < 0 || count < 0 || width <= 0 = cacheError path "negative payload dimension"
  | offset > BS.length bytes = cacheError path "payload offset lies past end of file"
  | count > (BS.length bytes - offset) `div` width = cacheError path "cache file is truncated"
  | otherwise = pure (offset + count * width)

requireExactEnd :: FilePath -> ByteString -> Int -> IO ()
requireExactEnd path bytes end =
  unless (end == BS.length bytes) (cacheError path "payload length does not match header")

words64 :: ByteString -> Int -> Int -> V.Vector Word64
words64 bytes offset count = V.generate count (\i -> wordLE bytes (offset + i * 8) 8)

words32 :: ByteString -> Int -> Int -> V.Vector Word32
words32 bytes offset count =
  V.generate count (\i -> fromIntegral (wordLE bytes (offset + i * 4) 4))

ints64 :: ByteString -> Int -> Int -> V.Vector Int
ints64 bytes offset count =
  V.map (fromIntegral . (fromIntegral :: Word64 -> Int64)) (words64 bytes offset count)

validateOffsets :: FilePath -> String -> Int -> V.Vector Word32 -> IO ()
validateOffsets path label limit offsets = do
  unless (not (V.null offsets) && V.head offsets == 0) $
    cacheError path (label ++ " offsets must start at zero")
  unless (and (V.toList (V.zipWith (<=) offsets (V.tail offsets)))) $
    cacheError path (label ++ " offsets are not monotonic")
  unless (fromIntegral (V.last offsets) == limit) $
    cacheError path (label ++ " final offset does not match its payload count")

loadInts :: FilePath -> String -> IO (Col e Int)
loadInts dir name = do
  (path, n, m, bytes) <- open dir name DenseI64
  unless (m == 0) (cacheError path "dense integer secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  pure (Col n (IntStore (ints64 bytes headerLen n)))

loadDoubles :: FilePath -> String -> IO (Col e Double)
loadDoubles dir name = do
  (path, n, m, bytes) <- open dir name DenseF64
  unless (m == 0) (cacheError path "dense double secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  pure (Col n (DblStore (V.map castWord64ToDouble (words64 bytes headerLen n))))

-- | Load a nullable foreign-key column. The on-disk all-ones sentinel becomes
-- absence in a sparse relation and is never represented as an 'Id'.
loadIds :: FilePath -> String -> IO (SparseCol e Int)
loadIds dir name = do
  (path, n, m, bytes) <- open dir name DenseI64
  unless (m == 0) (cacheError path "dense id secondary count must be zero")
  end <- sliceEnd path bytes headerLen n 8
  requireExactEnd path bytes end
  values <- traverse (decodeOptionalIndex path) (V.toList (words64 bytes headerLen n))
  pure (mkSparseCol values)

decodeOptionalIndex :: FilePath -> Word64 -> IO (Maybe Int)
decodeOptionalIndex _ value | value == holeWord = pure Nothing
decodeOptionalIndex path value = Just <$> countToInt path "identifier" value

loadStrs :: FilePath -> String -> IO (Col e ByteString)
loadStrs dir name = do
  (path, n, byteCount, bytes) <- open dir name DenseStr
  offsetEnd <- sliceEnd path bytes headerLen (n + 1) 4
  dataEnd <- sliceEnd path bytes offsetEnd byteCount 1
  requireExactEnd path bytes dataEnd
  let offsets = words32 bytes headerLen (n + 1)
      payload = BS.take byteCount (BS.drop offsetEnd bytes)
  validateOffsets path "string" byteCount offsets
  pure (Col n (BsStore offsets payload))

loadMultiInts :: FilePath -> String -> IO (MultiCol e Int)
loadMultiInts dir name = do
  (n, offsets, values) <- csrWords dir name
  pure (MultiCol n offsets (IntStore (V.map fromIntegral values)))

loadMultiIds :: FilePath -> String -> IO (MultiCol e Int)
loadMultiIds dir name = do
  (path, n, offsets, values) <- csrWord64 dir name
  identifiers <- traverse (countToInt path "identifier") (V.toList values)
  pure (MultiCol n offsets (packStore identifiers))

csrWords :: FilePath -> String -> IO (Int, V.Vector Word32, V.Vector Int64)
csrWords dir name = do
  (_path, n, offsets, values) <- csrWord64 dir name
  pure (n, offsets, V.map fromIntegral values)

csrWord64 :: FilePath -> String -> IO (FilePath, Int, V.Vector Word32, V.Vector Word64)
csrWord64 dir name = do
  (path, n, m, bytes) <- open dir name CsrWords
  offsetsEnd <- sliceEnd path bytes headerLen (n + 1) 4
  let valuesAt = align8 offsetsEnd
  when (valuesAt > BS.length bytes) (cacheError path "aligned values offset lies past end of file")
  valuesEnd <- sliceEnd path bytes valuesAt m 8
  requireExactEnd path bytes valuesEnd
  let offsets = words32 bytes headerLen (n + 1)
  validateOffsets path "row" m offsets
  pure (path, n, offsets, words64 bytes valuesAt m)

loadMultiStrs :: FilePath -> String -> IO (MultiCol e ByteString)
loadMultiStrs dir name = do
  (path, n, m, bytes) <- open dir name CsrStr
  rowEnd <- sliceEnd path bytes headerLen (n + 1) 4
  stringEnd <- sliceEnd path bytes rowEnd (m + 1) 4
  let rowOffsets = words32 bytes headerLen (n + 1)
      stringOffsets = words32 bytes rowEnd (m + 1)
      payload = BS.drop stringEnd bytes
  validateOffsets path "row" m rowOffsets
  validateOffsets path "string" (BS.length payload) stringOffsets
  pure (MultiCol n rowOffsets (BsStore stringOffsets payload))

validityBits :: SparseCol e Int -> Bits e
validityBits column = mkBitsFromMask (sparseMask column)

validityUniverse :: SparseCol e Int -> Universe e
validityUniverse = universeFromMask . sparseMask

header :: Kind -> Int -> Int -> B.Builder
header kind n m =
  B.byteString magic <> B.word32LE (kindCode kind) <> B.word32LE 0
                     <> B.word64LE (fromIntegral n) <> B.word64LE (fromIntegral m)

write :: FilePath -> String -> Kind -> Int -> Int -> B.Builder -> IO ()
write dir name kind n m payload =
  BL.writeFile (dir </> name ++ ".bin")
               (B.toLazyByteString (header kind n m <> payload))

writeInts :: FilePath -> String -> [Int] -> IO ()
writeInts dir name values =
  write dir name DenseI64 (length values) 0 (foldMap (B.int64LE . fromIntegral) values)

writeDoubles :: FilePath -> String -> [Double] -> IO ()
writeDoubles dir name values =
  write dir name DenseF64 (length values) 0 (foldMap B.doubleLE values)

writeIds :: FilePath -> String -> [Maybe (Id t)] -> IO ()
writeIds dir name values =
  write dir name DenseI64 (length values) 0 (foldMap encode values)
  where
    encode = B.word64LE . maybe holeWord (fromIntegral . idIndex)

writeStrs :: FilePath -> String -> [ByteString] -> IO ()
writeStrs dir name values = do
  let payload = BS.concat values
      offsets = scanl (+) 0 (map BS.length values)
  ensureWord32 dir name "string payload" (BS.length payload)
  write dir name DenseStr (length values) (BS.length payload)
        (foldMap (B.word32LE . fromIntegral) offsets <> B.byteString payload)

writeMultiInts :: FilePath -> String -> [[Int]] -> IO ()
writeMultiInts dir name = writeCsrWords dir name . map (map fromIntegral)

writeMultiIds :: FilePath -> String -> [[Id t]] -> IO ()
writeMultiIds dir name = writeCsrWords dir name . map (map (fromIntegral . idIndex))

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

ensureWord32 :: FilePath -> String -> String -> Int -> IO ()
ensureWord32 dir name label value =
  when (toInteger value > toInteger (maxBound :: Word32)) $
    cacheError (dir </> name ++ ".bin") (label ++ " exceeds the cache format's Word32 range")
