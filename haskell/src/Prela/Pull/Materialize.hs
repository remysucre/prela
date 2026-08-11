-- | The operators that stop the pipeline, without staging. Same set as
-- "Prela.PullStaged.Materialize", built directly on `Data.Map`/`Data.Set` rather
-- than a mutable open-addressed table or a dense array — those exist in the
-- staged executor purely for speed, and this module does not have that job.
-- Staged's `with...`-continuation shape (`withFold`, `withMaterialize`, …) is
-- gone too: it only existed to avoid duplicating a `CodeQ` by using it twice,
-- and there is no `CodeQ` here to duplicate, so these just return a value.
module Prela.Pull.Materialize
  ( index
  , materialize
  , inv
  , fold
  , bufFold
  , countDistinct
  , foldDense
  , foldDenseOuter
  , bitset
  ) where

import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set

import Prela.Id
import Prela.Pull.Ops
import Prela.Pull.Stream

-- | Consume a relation and bucket its pairs by key, preserving stream order.
index :: Ord d => Stream d r -> Map d [r]
index = Map.map reverse . sfold (\m d r -> Map.insertWith (++) d [r] m) Map.empty

-- | Force a leg once so reuse is free (in spirit; nothing here memoizes).
materialize :: (Mode q, Eq d) => Stream d r -> q d r
materialize s = fromPairs (collect s)

-- | A materialized inverse: swap the pairs, then rebuild a relation that also
-- supports keyed access.
inv :: (Mode q, Eq r) => Stream d r -> q r d
inv s = fromPairs (collect (invStream s))

-- | Group by key and reduce each group. `count = fold (\n _ -> n + 1) 0`.
fold :: (Mode q, Ord d) => (acc -> r -> acc) -> acc -> Stream d r -> q d acc
fold op ini s = fromPairs [ (d, foldl' op ini rs) | (d, rs) <- Map.toList (index s) ]

-- | The whole-group fold: hand the reducer the entire group at once, in stream
-- order. For anything `fold`'s `(acc -> r -> acc)` shape cannot express.
bufFold :: (Mode q, Ord d) => ([r] -> acc) -> Stream d r -> q d acc
bufFold f s = fromPairs [ (d, f rs) | (d, rs) <- Map.toList (index s) ]

-- | Distinct values per key (SQL's `COUNT(DISTINCT x)`).
countDistinct :: (Mode q, Ord d, Ord r) => Stream d r -> q d Int
countDistinct = bufFold (Set.size . Set.fromList)

-- | The same thing as `fold`, restricted to keys in `0 .. n - 1`. The other
-- ports use this to opt a grouped fold into a dense array; here it is `fold`
-- plus a range check, since there is no array to opt into.
foldDense :: Mode q => Int -> (acc -> r -> acc) -> acc -> Stream (Id e) r -> q (Id e) acc
foldDense n op ini s =
  fromPairs [ (d, foldl' op ini rs) | (d, rs) <- Map.toList (index s), idIndex d < n ]

-- | Like `foldDense`, but every key in `0 .. n - 1` emits, seeded with `ini`
-- where nothing matched — the left-outer-join aggregate.
foldDenseOuter :: Mode q => Int -> (acc -> r -> acc) -> acc -> Stream (Id e) r -> q (Id e) acc
foldDenseOuter n op ini s =
  fromPairs [ (d, foldl' op ini (Map.findWithDefault [] d grouped)) | d <- denseIds n ]
  where
    grouped = index s
    denseIds size = maybe [] universeIds (denseUniverse size)

-- | Consume a relation and note every distinct value it emits, within
-- `0 .. n - 1`: the identity relation on ids actually used.
bitset :: Mode q => Int -> Stream d (Id e) -> q (Id e) (Id e)
bitset n s = fromPairs [ (d, d) | d <- Set.toList seen, idIndex d < n ]
  where seen = Set.fromList (map snd (collect s))
