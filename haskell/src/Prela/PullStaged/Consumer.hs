-- | Terminal consumers which leave the staged relation algebra.
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

foldAll :: Drivable q => (Scalar acc -> Scalar r -> Scalar acc)
        -> Scalar acc -> q d r -> Scalar acc
foldAll step (Scalar initial) rows = Scalar
  (S.foldAll (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
             initial (R.asStream rows))

count :: Drivable q => q d r -> Scalar Int
count = Scalar . S.count . R.asStream

anyOf :: Drivable q => q d r -> Scalar Bool
anyOf = Scalar . S.anyOf . R.asStream

collect :: Drivable q => q d r -> Scalar [(d, r)]
collect = Scalar . S.collect . R.asStream

limit :: Drivable q => Scalar Int -> q d r -> Scalar [(d, r)]
limit (Scalar size) = Scalar . S.limit size . R.asStream
