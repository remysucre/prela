{-# LANGUAGE TemplateHaskell #-}

-- | The supported author-facing API for staged pull queries.
--
-- Package-private modules split the implementation by concept. This module
-- deliberately lists the safe vocabulary instead of re-exporting their
-- constructors and representation selectors.
module Prela.PullStaged.Query
  ( -- * Generated scalars
    Scalar
  , lit
  , int
  , pair
  , first
  , second
  , onPair
  , letScalar
  , ifThenElse
  , compare
  , thenCompare
  , (.==.)
  , (./=.)
  , (.<.)
  , (.<=.)
  , (.>.)
  , (.>=.)
  , (.&&.)
  , (.||.)
  , notS
  , fromIntegral
  , div
  , mod
  , member
  , take
  , firstByte
  , isPrefixOf
  , isSuffixOf
  , isInfixOf
  , orderedInfixOf
  , mapList
  , idIndex
  , extent
    -- * Relations
  , Relation
  , Drivable
  , Probeable
  , compose
  , prod
  , restrict
  , diff
  , groupBy
  , leftCompose
  , union
  , disj
  , invStream
    -- * Pure generation and materialization
  , Gen
  , share
  , materialize
  , invert
  , groupFold
  , bufferFold
  , distinctCount
  , denseDistinctCount
  , DenseKey
  , denseFold
  , denseFoldOuter
  , dictionary
  , bitset
  , topK
  , regex
    -- * Predicates and value transforms
  , eq
  , ne
  , gt
  , lt
  , ge
  , le
  , oneOf
  , between
  , range
  , filterBy
  , mapKeys
  , mapValues
  , rx
  , nrx
    -- * Consumers
  , foldAll
  , count
  , anyOf
  , collect
  , limit
    -- * Complete queries
  , Query
  , query
  , compile
  ) where

import Language.Haskell.TH (CodeQ)
import Prelude hiding (compare, div, fromIntegral, mod, take)

import Prela.PullStaged.Consumer
import Prela.PullStaged.Generation
import Prela.PullStaged.Materialize
import Prela.PullStaged.Predicate
import Prela.PullStaged.Relation
import Prela.PullStaged.Scalar
import qualified Prela.PullStaged.Stream as S

-- | A complete staged function from a loaded schema to a result.
newtype Query schema result = Query (CodeQ schema -> Gen (Scalar result))

query :: (CodeQ schema -> Gen (Scalar result)) -> Query schema result
query = Query

compile :: Query schema result -> CodeQ (schema -> result)
compile (Query build) = S.lam1 (runScalarGen . build)
