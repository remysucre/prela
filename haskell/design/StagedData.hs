-- | Not part of the build. Storage for design/StagedMain.hs, in its own module
-- only because a splice cannot see names defined in the file it appears in.
module StagedData
  ( nMovies
  , keywordOffs
  , keywordVals
  , leftKeys
  , leftVals
  , rightKeys
  , rightVals
  , castMovie
  , sparseOffs
  , sparseVals
  ) where

import qualified Data.Vector.Unboxed as U

nMovies :: Int
nMovies = 1000000
{-# NOINLINE nMovies #-}

-- Movie i owns keywordVals[keywordOffs[i] .. keywordOffs[i+1]-1]. Two keywords
-- per movie, so two million inner rows.
keywordOffs :: U.Vector Int
keywordOffs = U.generate (nMovies + 1) (* 2)
{-# NOINLINE keywordOffs #-}

keywordVals :: U.Vector Int
keywordVals = U.generate (2 * nMovies) (\j -> 1950 + (j `mod` 60))
{-# NOINLINE keywordVals #-}

-- A foreign key into movie, with holes. `noId` is -1 on disk, so one row in
-- seven points nowhere and one in eleven points past the end — both of which a
-- probed leaf must answer with no values rather than a wild read.
castMovie :: U.Vector Int
castMovie = U.generate nMovies $ \i ->
  if i `mod` 7 == 0 then -1
  else if i `mod` 11 == 0 then nMovies + i
  else i
{-# NOINLINE castMovie #-}

-- A CSR column where only every third movie owns anything, so a membership test
-- against it fails two times in three. `keywordOffs` is no use for that: every
-- movie has two keywords there, so `anyOf` would succeed on the first step every
-- time and never exercise the empty case.
sparseOffs :: U.Vector Int
sparseOffs = U.generate (nMovies + 1) (\i -> 4 * (i `div` 3))
{-# NOINLINE sparseOffs #-}

sparseVals :: U.Vector Int
sparseVals = U.generate (4 * ((nMovies + 2) `div` 3)) (\j -> 1900 + (j `mod` 200))
{-# NOINLINE sparseVals #-}

-- Two sorted runs to merge. Every third key on the left, every fifth on the
-- right, so they agree on the multiples of fifteen.
leftKeys, leftVals, rightKeys, rightVals :: U.Vector Int
leftKeys  = U.generate (nMovies `div` 3) (* 3)
leftVals  = U.generate (nMovies `div` 3) id
rightKeys = U.generate (nMovies `div` 5) (* 5)
rightVals = U.generate (nMovies `div` 5) (* 10)
{-# NOINLINE leftKeys #-}
{-# NOINLINE leftVals #-}
{-# NOINLINE rightKeys #-}
{-# NOINLINE rightVals #-}
