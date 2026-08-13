-- | Terminal consumers which leave the staged relation algebra.
--
-- Each function converts any 'Drivable' relation into a generated scalar. The
-- traversal is emitted directly by the pull-stream consumer, so filters,
-- composition, and value transforms upstream remain in the same generated
-- loop. Only 'collect' and 'limit' allocate result lists.
module Prela.PullStaged.Consumer
  ( foldAll
  , count
  , anyOf
  , collect
  , limit
  ) where

import qualified Prela.PullStaged.Relation as R
import Prela.PullStaged.Relation (Drivable)
import Prela.PullStaged.Scalar
import qualified Prela.PullStaged.Stream as S

-- | Reduce all values into one scalar, ignoring relation keys.
foldAll :: Drivable q => (Scalar acc -> Scalar r -> Scalar acc)
        -> Scalar acc -> q d r -> Scalar acc
foldAll step (Scalar initial) rows = Scalar
  (S.foldAll (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
             initial (R.asStream rows))

-- | Count the rows produced by a relation.
count :: Drivable q => q d r -> Scalar Int
count = Scalar . S.count . R.asStream

-- | Test whether a relation produces a row, stopping after the first match.
anyOf :: Drivable q => q d r -> Scalar Bool
anyOf = Scalar . S.anyOf . R.asStream

-- | Collect all generated @(key, value)@ pairs in stream order.
collect :: Drivable q => q d r -> Scalar [(d, r)]
collect = Scalar . S.collect . R.asStream

-- | Collect at most the requested number of pairs and stop the source early.
-- Non-positive limits produce an empty list.
limit :: Drivable q => Scalar Int -> q d r -> Scalar [(d, r)]
limit (Scalar size) = Scalar . S.limit size . R.asStream
