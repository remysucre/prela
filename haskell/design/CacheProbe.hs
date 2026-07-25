-- | Not part of the build. End-to-end check of the cache reader at scale: write
-- a million-row column, map it back, and run the same query CoreProbe fuses.
--
--   cd haskell && mkdir -p /tmp/cacheprobe && cabal exec -- \
--     ghc -O2 -isrc -fforce-recomp design/CacheProbe.hs \
--         -outputdir /tmp/cacheprobe -o /tmp/cacheprobe/probe -rtsopts
--   /tmp/cacheprobe/probe /tmp/cacheprobe +RTS -s
--
-- Result: the read-only run does two full passes over a million-row column in
-- 58 KB of total allocation and 43 KB of peak residency, against an 8 MB file.
-- That is startup cost and nothing per row: the column is a view of the mapping,
-- so the data never enters the heap, and the count agrees exactly with the
-- hand-built column in CoreProbe (376926).
module Main where

import Control.Monad (when)
import Prela
import Prela.Cache
import System.Environment (getArgs)

data Movie

n :: Int
n = 1000000

vals :: [Int]
vals = [1900 + (i * 7919) `mod` 130 | i <- [0 .. n - 1]]

-- `probe <dir> write` builds the file; `probe <dir>` only reads it, so the -s
-- numbers for a run without `write` are the query's own cost and nothing else.
main :: IO ()
main = do
  (dir : rest) <- getArgs
  when (rest == ["write"]) (writeInts dir "Movie_year" vals)
  yearCol <- loadInts dir "Movie_year" :: IO (Col Movie Int)
  let movie :: Mode q => q (Id Movie) (Id Movie)
      movie = universe n
      year :: Mode q => q (Id Movie) Int
      year = column yearCol
      -- movie : (year > 1980) → year  ⊵ count
      countRecent = foldAll (\c _ -> c + 1) (0 :: Int)
                            (compose (restrict movie (gt 1980 year)) year)
      -- and once more, to show a second pass over the same mapping is free
      sumYears = foldAll (+) 0 (compose movie year)
  print countRecent
  print sumYears
