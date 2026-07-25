-- | Round-trip tests for the cache reader: write a file of each kind, map it
-- back, and check that driving the resulting column gives the pairs that went
-- in. These mirror the Rust port's cache tests, which is deliberate — the two
-- readers consume the same bytes, so a disagreement about the format is a bug in
-- one of them and the tests are the only place that shows up until there is a
-- real cache on disk to read.
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ScopedTypeVariables #-}
module Main where

import Control.Exception (ErrorCall, evaluate, try)
import Control.Monad (unless)
import Data.ByteString (ByteString)
import Data.IORef (modifyIORef', newIORef, readIORef)
import System.Directory (createDirectoryIfMissing, getTemporaryDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>))

import Prela
import Prela.Cache

data E
data T

-- Collect every pair a driven relation emits, in order.
pairs :: Drv d r -> IO [(d, r)]
pairs q = do
  ref <- newIORef []
  drive q (\x y -> modifyIORef' ref ((x, y) :))
  reverse <$> readIORef ref

main :: IO ()
main = do
  tmp <- getTemporaryDirectory
  let dir = tmp </> "prela-cache-test"
  createDirectoryIfMissing True dir
  fails <- newIORef (0 :: Int)
  let check :: (Eq a, Show a) => String -> a -> a -> IO ()
      check what got want = unless (got == want) $ do
        putStrLn ("FAIL " ++ what ++ "\n  got  " ++ show got ++ "\n  want " ++ show want)
        modifyIORef' fails (+ 1)

  -- dense integers, negatives included
  writeInts dir "ints" [7, 0, -3]
  ints <- loadInts dir "ints" :: IO (Col E Int)
  got1 <- pairs (column ints)
  check "dense ints" got1 [(Id 0, 7), (Id 1, 0), (Id 2, -3)]

  -- dense doubles
  writeDoubles dir "dbls" [1.5, -0.25]
  dbls <- loadDoubles dir "dbls" :: IO (Col E Double)
  got2 <- pairs (column dbls)
  check "dense doubles" got2 [(Id 0, 1.5), (Id 1, -0.25)]

  -- dense ids: the holes must come back as noId, bit-for-bit, or a loaded FK
  -- column and a hand-built one would disagree about what a hole is
  writeIds dir "fk" [Nothing, Just (Id 0), Nothing, Just (Id 2)]
  fk <- loadIds dir "fk" :: IO (Col E (Id T))
  got3 <- pairs (column fk)
  check "dense ids" got3
    [(Id 0, noId), (Id 1, Id 0), (Id 2, noId), (Id 3, Id 2)]

  -- ...and a hole must resolve to nothing when composed onto the target table,
  -- with no presence bit anywhere: this is the whole point of the sentinel
  writeStrs dir "label" ["zero", "one", "two"]
  label <- loadStrs dir "label" :: IO (Col T ByteString)
  got4 <- pairs (compose (column fk) (column label))
  check "ids compose past holes" got4 [(Id 1, "zero"), (Id 3, "two")]

  -- dense strings, with a hole as the empty string
  writeStrs dir "strs" ["ab", "", "c"]
  strs <- loadStrs dir "strs" :: IO (Col E ByteString)
  got5 <- pairs (column strs)
  check "dense strings" got5 [(Id 0, "ab"), (Id 1, ""), (Id 2, "c")]

  -- CSR words: an empty row in the middle, which is how the format spells both
  -- "no value" and "many values"
  writeMultiInts dir "csr" [[5, 6], [], [9]]
  csr <- loadMultiInts dir "csr" :: IO (MultiCol E Int)
  got6 <- pairs (multiColumn csr)
  check "csr words" got6 [(Id 0, 5), (Id 0, 6), (Id 2, 9)]

  -- an odd key count pushes the values past a 4-byte boundary, so this is the
  -- test that the writer's pad and the reader's align8 agree
  writeMultiIds dir "csrid" [[Id 1], [], [Id 0, Id 2], []]
  csrid <- loadMultiIds dir "csrid" :: IO (MultiCol E (Id T))
  got7 <- pairs (multiColumn csrid)
  check "csr ids" got7 [(Id 0, Id 1), (Id 2, Id 0), (Id 2, Id 2)]

  -- CSR strings: row offsets over string offsets over bytes
  writeMultiStrs dir "csrstr" [["a", "bb"], [], ["c"]]
  csrstr <- loadMultiStrs dir "csrstr" :: IO (MultiCol E ByteString)
  got8 <- pairs (multiColumn csrstr)
  check "csr strings" got8 [(Id 0, "a"), (Id 0, "bb"), (Id 2, "c")]

  -- a sparse universe derived from the holes in an FK column
  let live = validityBits fk
  got9 <- pairs (sparseUniverse live 4 :: Drv (Id E) (Id E))
  check "validity bits" got9 [(Id 1, Id 1), (Id 3, Id 3)]

  -- Probing that same universe consults only the range, NOT the mask, so id 0 —
  -- a hole — is a member. The asymmetry with drive above is deliberate and is
  -- inherited from the Rust port: a probe of a universe is only ever reached with
  -- an id that came from a real column of that table, so a dead id cannot arrive,
  -- and leaving the mask out keeps the probe down to one comparison. Pinned here
  -- because it is the kind of thing a later "cleanup" would quietly change.
  check "sparse universe probe is a range check only"
    (map (member (sparseUniverse live 4 :: Prb (Id E) (Id E)) . Id) [0, 1, 3, 4])
    [True, True, True, False]

  -- a file of the wrong kind must fail loudly rather than reinterpret bytes
  bad <- try (loadInts dir "strs" >>= evaluate) :: IO (Either ErrorCall (Col E Int))
  check "kind mismatch is loud" (either (const True) (const False) bad) True

  -- and so must a file that is not a v2 cache at all
  writeFile (dir </> "notcache.bin") "definitely not a cache file"
  bad2 <- try (loadStrs dir "notcache" >>= evaluate)
            :: IO (Either ErrorCall (Col E ByteString))
  check "bad magic is loud" (either (const True) (const False) bad2) True

  n <- readIORef fails
  if n == 0
    then putStrLn "cache: all round trips ok"
    else putStrLn (show n ++ " failure(s)") >> exitFailure
