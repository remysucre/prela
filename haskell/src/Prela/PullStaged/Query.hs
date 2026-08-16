-- | The supported author-facing API for staged pull queries.
--
-- Package-private modules split the implementation by concept. This module
-- deliberately lists the safe vocabulary instead of re-exporting their
-- constructors and representation selectors. Query authors construct a
-- t'Query' with 'query', use t'Gen' sequencing for shared runtime state, terminate
-- with a scalar consumer, and splice the ordinary function returned by
-- 'compile'.
module Prela.PullStaged.Query
  ( -- * Generated scalars
    Scalar
  , lit
  , int
  , pair
  , tuple3
  , tuple4
  , tuple5
  , tuple6
  , first
  , second
  , onPair
  , onTuple3
  , onTuple4
  , onTuple5
  , onTuple6
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
  , invDrive
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

-- | Package a pure query builder. The schema argument denotes the eventual
-- runtime schema inside generated code.
query :: (CodeQ schema -> Gen (Scalar result)) -> Query schema result
query = Query

-- | Generate an ordinary function which executes a complete staged query.
compile :: Query schema result -> CodeQ (schema -> result)
compile (Query generate) = S.lam1 (runScalarGen . generate)
