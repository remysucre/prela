{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The use site for design/StagedPull.hs: six queries,
-- each printed beside a hand-written version of the same thing, so the answers
-- can be checked and the Core compared.
--
--   cd haskell && mkdir -p /tmp/stagedpull && cabal exec -- ghc -O2 -Wall -fforce-recomp \
--     -ddump-simpl -ddump-to-file -dsuppress-all -dno-suppress-type-signatures \
--     design/StagedPullMain.hs design/StagedPull.hs design/StagedData.hs \
--     -outputdir /tmp/stagedpull -o /tmp/stagedpull/pull && /tmp/stagedpull/pull +RTS -s
--
-- The measurements are at the bottom of this file.
module Main (main) where

import qualified Data.Vector.Unboxed as U
import GHC.Conc (getAllocationCounter, setAllocationCounter)

import StagedPull
import StagedData

--------------------------------------------------------------------------------
-- 1. The CoreProbe query: compose, filter, count
--------------------------------------------------------------------------------

recentKeywords :: Int
recentKeywords =
  $$(count (filtS (\v -> [|| $$v > 1980 ||])
             (compose (universeP [|| nMovies ||])
                      (csrCursor [|| keywordOffs ||] [|| keywordVals ||]))))
{-# NOINLINE recentKeywords #-}

recentKeywordsByHand :: Int
recentKeywordsByHand = outer 0 0
  where
    outer :: Int -> Int -> Int
    outer !i !acc
      | i >= nMovies = acc
      | otherwise    = outer (i + 1) (inner (keywordOffs U.! (i + 1))
                                            (keywordOffs U.! i) acc)
    inner :: Int -> Int -> Int -> Int
    inner !end !j !a
      | j >= end  = a
      | otherwise = inner end (j + 1)
                      (if keywordVals U.! j > 1980 then a + 1 else a)
{-# NOINLINE recentKeywordsByHand #-}

--------------------------------------------------------------------------------
-- 2. Driven through a foreign key with holes
--------------------------------------------------------------------------------

throughHoles :: Int
throughHoles =
  $$(count (compose (columnP [|| castMovie ||])
                    (csrCursor [|| keywordOffs ||] [|| keywordVals ||])))
{-# NOINLINE throughHoles #-}

throughHolesByHand :: Int
throughHolesByHand =
  2 * length [ k | k <- U.toList castMovie, k >= 0, k < nMovies ]
{-# NOINLINE throughHolesByHand #-}

--------------------------------------------------------------------------------
-- 3. Membership, which is early termination in its smallest form
--------------------------------------------------------------------------------

haveKeywords :: Int
haveKeywords =
  $$(count (restrict (universeP [|| nMovies ||])
                     (csrCursor [|| sparseOffs ||] [|| sparseVals ||])))
{-# NOINLINE haveKeywords #-}

haveKeywordsByHand :: Int
haveKeywordsByHand =
  length [ i | i <- [0 .. nMovies - 1]
             , sparseOffs U.! (i + 1) > sparseOffs U.! i ]
{-# NOINLINE haveKeywordsByHand #-}

--------------------------------------------------------------------------------
-- 4. A four-deep prod tower, which is Q1's payload shape
--------------------------------------------------------------------------------

-- The question this answers is whether the intermediate tuples survive. Each is
-- built by `prod` and immediately taken apart by the step function, so GHC's
-- case-of-constructor should cancel all three. If it does not, the value type
-- has to become a generation-time pair rather than code for a pair, which
-- changes every operator's type — much better to know here than at query 14.
prodTower :: Int
prodTower =
  $$(sfold (\acc _ v -> [|| case $$v of
                              (((a, b), c), d) -> $$acc + a + b + c + d ||])
       [|| 0 ||]
       (prod (prod (prod (universeP [|| nMovies ||])
                         (colCursor [|| keywordOffs ||]))
                   (colCursor [|| keywordVals ||]))
             (colCursor [|| castMovie ||])))
{-# NOINLINE prodTower #-}

-- One level and two levels, to say whether the cost is per-`prod` or a fixed
-- price for nesting at all.
prod1 :: Int
prod1 =
  $$(sfold (\acc _ v -> [|| case $$v of (a, b) -> $$acc + a + b ||]) [|| 0 ||]
       (prod (universeP [|| nMovies ||]) (colCursor [|| keywordOffs ||])))
{-# NOINLINE prod1 #-}

prod2 :: Int
prod2 =
  $$(sfold (\acc _ v -> [|| case $$v of ((a, b), c) -> $$acc + a + b + c ||]) [|| 0 ||]
       (prod (prod (universeP [|| nMovies ||]) (colCursor [|| keywordOffs ||]))
             (colCursor [|| keywordVals ||])))
{-# NOINLINE prod2 #-}

prodTowerByHand :: Int
prodTowerByHand = go 0 0
  where
    go :: Int -> Int -> Int
    go !i !acc
      | i >= nMovies = acc
      | otherwise    = go (i + 1) (acc + i
                                       + keywordOffs U.! i
                                       + keywordVals U.! i
                                       + castMovie   U.! i)
{-# NOINLINE prodTowerByHand #-}

--------------------------------------------------------------------------------
-- 5. Lockstep over two FILTERED runs
--------------------------------------------------------------------------------

-- This is the case design/Staged.hs cannot express. `SProd` there needs its
-- inputs addressable by index, and a filter destroys addressability: there is no
-- formula for "the nth surviving row". A pull producer has no such requirement,
-- so the two filters and the zip fuse into one loop with two cursors.
zipped :: Int
zipped =
  $$(sfold (\acc _ v -> [|| $$acc + $$v ||]) [|| 0 ||]
       (linear (zipWithP (\a b -> [|| $$a + $$b ||])
                  (filtProd (\x -> [|| even $$x ||])   (columnProd [|| leftVals ||]))
                  (filtProd (\y -> [|| $$y > 3 ||])    (columnProd [|| rightVals ||])))))
{-# NOINLINE zipped #-}

zippedByHand :: Int
zippedByHand = sum (zipWith (+) (filter even (U.toList leftVals))
                                (filter (> 3) (U.toList rightVals)))
{-# NOINLINE zippedByHand #-}

--------------------------------------------------------------------------------
-- 6. LIMIT, which the push engine cannot express at all
--------------------------------------------------------------------------------

limited :: [(Int, Int)]
limited =
  $$(limit [|| 10 ||]
       (filtS (\v -> [|| $$v > 1980 ||])
         (compose (universeP [|| nMovies ||])
                  (csrCursor [|| keywordOffs ||] [|| keywordVals ||]))))
{-# NOINLINE limited #-}

limitedByHand :: [(Int, Int)]
limitedByHand =
  take 10 [ (i, v)
          | i <- [0 .. nMovies - 1]
          , j <- [keywordOffs U.! i .. keywordOffs U.! (i + 1) - 1]
          , let v = keywordVals U.! j
          , v > 1980 ]
{-# NOINLINE limitedByHand #-}

--------------------------------------------------------------------------------

-- | Bytes allocated while forcing one thunk. Every input vector is forced
-- before the counter starts, so what this reports is the query's own
-- allocation and nothing else. `+RTS -s` cannot separate those, which is why
-- the totals at the bottom of this file are per query rather than one number.
measure :: String -> (a -> Int) -> a -> IO ()
measure nm f x = do
  setAllocationCounter 0
  a0 <- getAllocationCounter
  let !n = f x
  a1 <- getAllocationCounter
  putStrLn (pad nm ++ pad (show n) ++ show (a0 - a1) ++ " bytes")
  where pad s = s ++ replicate (max 1 (22 - length s)) ' '

main :: IO ()
main = do
  -- Force every input vector, so none of it lands in a measurement below.
  mapM_ (\v -> U.length v `seq` U.last v `seq` return ())
        [keywordOffs, keywordVals, castMovie, sparseOffs, sparseVals, leftVals, rightVals]

  -- Measurement first: these are CAFs, so whichever section runs first is the
  -- one that pays for evaluating them.
  putStrLn "-- allocation, staged pull --------------------------------------"
  measure "1 compose+filt+count" id  recentKeywords
  measure "2 through holes"      id  throughHoles
  measure "3 membership"         id  haveKeywords
  measure "4 prod x1"            id  prod1
  measure "4 prod x2"            id  prod2
  measure "4 prod tower"         id  prodTower
  measure "5 zip of two filters" id  zipped
  measure "6 limit 10"           length limited

  putStrLn ""
  putStrLn "-- allocation, by hand ------------------------------------------"
  measure "1 by hand"            id  recentKeywordsByHand
  measure "3 by hand"            id  haveKeywordsByHand
  measure "4 by hand"            id  prodTowerByHand
  measure "6 by hand"            length limitedByHand

  putStrLn ""
  putStrLn "-- answers, staged / by hand ------------------------------------"
  putStrLn ("1 compose+filt+count  " ++ show recentKeywords ++ " / " ++ show recentKeywordsByHand)
  putStrLn ("2 through holes       " ++ show throughHoles   ++ " / " ++ show throughHolesByHand)
  putStrLn ("3 membership          " ++ show haveKeywords   ++ " / " ++ show haveKeywordsByHand)
  putStrLn ("4 prod tower          " ++ show prodTower      ++ " / " ++ show prodTowerByHand)
  putStrLn ("5 zip of two filters  " ++ show zipped         ++ " / " ++ show zippedByHand)
  putStrLn ("6 limit 10            " ++ show limited)
  putStrLn ("6 limit 10 by hand    " ++ show limitedByHand)

--------------------------------------------------------------------------------
-- Measurements
--------------------------------------------------------------------------------
--
-- GHC 9.10.3, -O2, aarch64. Bytes are what the allocation counter charged to
-- forcing that one thunk, with every input vector already forced.
--
--   1 compose+filt+count  966657                32 bytes
--   2 through holes       1558440               32 bytes
--   3 membership          333333                32 bytes
--   4 prod x1             1499998500000         32 bytes
--   4 prod x2             1501977999600         32 bytes
--   4 prod tower          2008470428171         32 bytes
--   5 zip of two filters  166668000002          32 bytes
--   6 limit 10            10                  4576 bytes
--
--   1 by hand             966657                32 bytes
--   3 by hand             333333                32 bytes
--   4 by hand             2008470428171         32 bytes
--   6 by hand             10                  3440 bytes
--
-- IT FUSES. 32 bytes is one boxed `Int` for the answer and nothing else, over
-- one to two million rows, so every one of these allocates ZERO per row and
-- matches the hand-written loop exactly. Query 6 is the exception and has to be:
-- it builds a list, and 4576 bytes is ten cons cells and ten pairs, on the same
-- order as the by-hand `take 10` over a list comprehension.
--
-- Every answer agrees with its hand-written version.
--
-- In the Core, every loop is a worker over unboxed arguments —
--
--   $wgo1 :: Int# -> Int# -> Int#          -- query 1
--   $wgo5 :: Int# -> Int# -> Int#          -- the prod tower
--   $wgo8 :: Int# -> Int -> Int# -> Int#   -- the zip
--
-- with inner levels as `joinrec` reached by `jump`, reading through
-- `indexIntArray#`. No `Step`, no `MutVar#`, no `((), I#`, and no `$fMonad`
-- below the "Tidy Core" line — the nine `$fMonadQ1` occurrences in the dump are
-- all above it, in the splice-time code that GHC runs at compile time and does
-- not emit.
--
-- The prod tower is the one that needed work, and it settles the representation
-- question the plan asked here. The intermediate tuples DO cancel: GHC's
-- case-of-constructor removes all three, and the value type can stay `CodeQ (a,
-- b)` rather than becoming a generation-time pair. What does not cancel on its
-- own is the tuple's COMPONENTS. Each is a checked array read, which has a
-- bottoming branch that GHC will not speculate, so left lazy it crosses the
-- inner loop as a boxed `Int` thunk — 64 bytes a row, 64,000,032 total.
--
-- The fix is two bangs, in `prod` where the pair is built and in `sfoldWhile`'s
-- `Bind` case where the outer value is carried in. NEITHER WORKS ALONE: with
-- only one of them the tower is still at 64,000,032 bytes, and with both it is
-- at 32. One and two levels of `prod` allocate nothing either way, so this is a
-- cliff at three levels rather than a per-level price, and Q1's payload is four
-- levels deep. `-fmax-simplifier-iterations` at 6, 8 and 12 changes nothing, so
-- it is not the simplifier running out of room.
