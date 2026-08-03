{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The use site: one spliced query, so its Core can be
-- compared against the push engine's.
--
--   cd haskell && mkdir -p /tmp/staged && cabal exec -- ghc -O2 -Wall -fforce-recomp \
--     -ddump-simpl -ddump-to-file -dsuppress-all -dno-suppress-type-signatures \
--     design/StagedMain.hs design/Staged.hs design/StagedData.hs \
--     -outputdir /tmp/staged -o /tmp/staged/staged && /tmp/staged/staged
module Main (main) where

import qualified Data.Vector.Unboxed as U

import Staged
import StagedData

-- | @movie -> keyword : (> 1980) ⊵ count@, the CoreProbe query. Everything to
-- the right of the @$$@ runs at compile time and disappears; what is left in the
-- object code is whatever loop it printed.
recentKeywords :: Int
recentKeywords =
  $$(count
       (filt (\v -> [|| $$v > 1980 ||])
         (compose (universe [|| nMovies ||])
                  (csrCursor [|| keywordOffs ||] [|| keywordVals ||]))))
{-# NOINLINE recentKeywords #-}

-- | The same query written by hand, to say what the generated loop ought to be.
byHand :: Int
byHand = outer 0 0
  where
    outer :: Int -> Int -> Int
    outer !i !acc
      | i >= nMovies = acc
      | otherwise    = outer (i + 1) (inner ((U.!) keywordOffs (i + 1))
                                            ((U.!) keywordOffs i)
                                            acc)
    inner :: Int -> Int -> Int -> Int
    inner !end !j !a
      | j >= end  = a
      | otherwise = inner end (j + 1)
                      (if keywordVals U.! j > 1980 then a + 1 else a)
{-# NOINLINE byHand #-}

-- | The same shape, but driven by a foreign key with holes, so the CSR cursor
-- is probed at keys that are negative or past the end. Every such row must
-- contribute nothing. Without the guard in `csrCursor` this reads out of bounds.
throughHoles :: Int
throughHoles =
  $$(count (compose (column [|| castMovie ||])
                    (csrCursor [|| keywordOffs ||] [|| keywordVals ||])))
{-# NOINLINE throughHoles #-}

-- | What that count should be: two keywords for every row whose key resolves.
throughHolesByHand :: Int
throughHolesByHand =
  2 * length [ k | k <- U.toList castMovie, k >= 0, k < nMovies ]
{-# NOINLINE throughHolesByHand #-}

-- | The lockstep case. Two sorted runs merged and summed, in one loop with two
-- cursors: no buffering, no intermediate, and nothing the push engine can write.
merged :: Int
merged =
  $$(foldAll (\acc v -> [|| $$acc + $$v ||]) [|| 0 ||]
       (mergeJoinWith (\a b -> [|| $$a + $$b ||])
          (sortedRun [|| leftKeys ||]  [|| leftVals ||])
          (sortedRun [|| rightKeys ||] [|| rightVals ||])))
{-# NOINLINE merged #-}

-- | The same merge by hand, over lists, to check the answer.
mergedByHand :: Int
mergedByHand = go 0
                 (zip (U.toList leftKeys)  (U.toList leftVals))
                 (zip (U.toList rightKeys) (U.toList rightVals))
  where
    go :: Int -> [(Int, Int)] -> [(Int, Int)] -> Int
    go !acc xs@((k, a) : xs') ys@((k', b) : ys')
      | k < k'    = go acc xs' ys
      | k > k'    = go acc xs ys'
      | otherwise = go (acc + a + b) xs' ys'
    go !acc _ _ = acc
{-# NOINLINE mergedByHand #-}

main :: IO ()
main = do
  print recentKeywords
  print byHand
  print throughHoles
  print throughHolesByHand
  print merged
  print mergedByHand
