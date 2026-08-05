{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The generators spliced by design/StagedProbe.hs.
--
-- They live in their own module because of the stage restriction: a splice runs
-- at compile time, so it cannot name anything bound in the module it appears in.
-- That is also why the columns arrive as arguments — the generator is compiled
-- before the data it will read exists, and @CodeQ (Col e Int)@ is how it refers to
-- something it cannot see. Every query here is polymorphic in the entity tag @e@
-- for the same reason.
--
-- This is the shape TPCH/Queries.hs will have after step 3, so it is also a
-- readability check on the new surface syntax.
module StagedProbeGen
  ( countRecent
  , countPerYear
  , firstTen
  ) where

import Language.Haskell.TH (CodeQ)

import Prela.PullStaged.Materialize
import Prela.PullStaged.Ops
import Prela.PullStaged.Predicate
import Prela.PullStaged.Stream
import Prela.Storage

-- | The exact query design/CoreProbe.hs uses, so the two Core dumps can be put
-- side by side:
--
-- > movie : (year > 1980) → year  ⊵ count
countRecent :: forall e. CodeQ Int -> CodeQ (Col e Int) -> CodeQ Int
countRecent n yearCol = foldAll (\a _ -> [|| $$a + 1 ||]) [|| 0 ||] recent
  where
    movie :: SMode q => q (Id e) (Id e)
    movie = universe n

    year :: SMode q => q (Id e) Int
    year = column yearCol

    recent :: Drive (Id e) Int
    recent = compose (restrict movie (gt [|| 1980 ||] year)) year

-- | A grouped fold read back at BOTH modes, which is what @withFold@ exists for.
-- The table is built once; @perYear@ is then driven, to collect every group, and
-- probed, to ask about one. If the continuation shape is wrong this builds it
-- twice and the allocation count says so.
countPerYear :: forall e. CodeQ Int -> CodeQ (Col e Int) -> CodeQ ([(Int, Int)], Bool)
countPerYear n yearCol =
  withFold (\a _ -> [|| $$a + (1 :: Int) ||]) [|| 0 ||] grouped $ \perYear ->
    [|| ( $$(collect (perYear :: Drive Int Int))
        , $$(anyOf (at (perYear :: Probe Int Int) [|| 1985 ||])) ) ||]
  where
    year :: SMode q => q (Id e) Int
    year = column yearCol

    -- Re-key the movie universe by each movie's year: (year, movie id).
    grouped :: Drive Int (Id e)
    grouped = groupBy (universe n) year

-- | Early exit, which is not expressible against the push engine at all: @drive@
-- has no way to be told to stop. The loop must leave after ten rows, not filter a
-- million and take ten.
firstTen :: forall e. CodeQ Int -> CodeQ (Col e Int) -> CodeQ [(Id e, Int)]
firstTen n yearCol = limit [|| 10 ||] recent
  where
    movie :: SMode q => q (Id e) (Id e)
    movie = universe n

    year :: SMode q => q (Id e) Int
    year = column yearCol

    recent :: Drive (Id e) Int
    recent = compose (restrict movie (gt [|| 2020 ||] year)) year
