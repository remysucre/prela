-- | The operators that stop the pipeline, without staging. Same set as
-- "Prela.PullStaged.Materialize", built directly on `Data.Map`/`Data.Set` rather
-- than a mutable open-addressed table or a dense array — those exist in the
-- other two ports purely for speed, and this module does not have that job.
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

-- | Drive a relation and bucket its pairs by key, each group in drive order.
index :: Ord d => Drive d r -> Map d [r]
index = sfold (\m d r -> Map.insertWith (flip (++)) d [r] m) Map.empty

-- | Force a leg once so reuse is free (in spirit; nothing here memoizes).
materialize :: (Mode q, Eq d) => Drive d r -> q d r
materialize s = fromPairs (collect s)

-- | The probed inverse: swap the pairs, then rebuild.
inv :: (Mode q, Eq r) => Drive d r -> q r d
inv s = fromPairs (collect (invStream s))

-- | Group by key and reduce each group. `count = fold (\n _ -> n + 1) 0`.
fold :: (Mode q, Ord d) => (acc -> r -> acc) -> acc -> Drive d r -> q d acc
fold op ini s = fromPairs [ (d, foldl' op ini rs) | (d, rs) <- Map.toList (index s) ]

-- | The whole-group fold: hand the reducer the entire group at once, in
-- drive order. For anything `fold`'s `(acc -> r -> acc)` shape cannot express.
bufFold :: (Mode q, Ord d) => ([r] -> acc) -> Drive d r -> q d acc
bufFold f s = fromPairs [ (d, f rs) | (d, rs) <- Map.toList (index s) ]

-- | Distinct values per key (SQL's `COUNT(DISTINCT x)`).
countDistinct :: (Mode q, Ord d, Ord r) => Drive d r -> q d Int
countDistinct = bufFold (Set.size . Set.fromList)

-- | The same thing as `fold`, restricted to keys in `0 .. n - 1`. The other
-- ports use this to opt a grouped fold into a dense array; here it is `fold`
-- plus a range check, since there is no array to opt into.
foldDense :: Mode q => Int -> (acc -> r -> acc) -> acc -> Drive (Id e) r -> q (Id e) acc
foldDense n op ini s =
  fromPairs [ (Id i, foldl' op ini rs) | (Id i, rs) <- Map.toList (index s), 0 <= i, i < n ]

-- | Like `foldDense`, but every key in `0 .. n - 1` emits, seeded with `ini`
-- where nothing matched — the left-outer-join aggregate.
foldDenseOuter :: Mode q => Int -> (acc -> r -> acc) -> acc -> Drive (Id e) r -> q (Id e) acc
foldDenseOuter n op ini s =
  fromPairs [ (Id i, foldl' op ini (Map.findWithDefault [] (Id i) grouped)) | i <- [0 .. n - 1] ]
  where grouped = index s

-- | Drive a relation and note every distinct VALUE it emits, within
-- `0 .. n - 1`: the identity relation on ids actually used.
bitset :: Mode q => Int -> Drive d (Id e) -> q (Id e) (Id e)
bitset n s = fromPairs [ (Id i, Id i) | Id i <- Set.toList seen, 0 <= i, i < n ]
  where seen = Set.fromList (map snd (collect s))
