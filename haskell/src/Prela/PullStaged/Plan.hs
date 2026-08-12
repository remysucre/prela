{-# LANGUAGE DataKinds #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE GADTs #-}
{-# LANGUAGE KindSignatures #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | A pure, typed surface for staged relational plans.
--
-- Plans are ordinary Haskell values. Constructing one performs no generation
-- effects and allocates no runtime query objects. 'compile' lowers the plan to
-- the existing staged pull backend, where stream/lookup selection, fused probes,
-- and materializer scopes remain implementation details.
module Prela.PullStaged.Plan
  ( Access (Read, Scan)
  , Rel
  , Relation
  , Result
  , Scalar
    -- * Leaves and relational algebra
  , leaf
  , compose
  , prod
  , restrict
  , diff
  , filt
  , mapValues
  , groupBy
    -- * Predicates
  , eq
  , ne
  , gt
  , lt
  , ge
  , le
  , oneOf
  , between
  , during
    -- * Materialized plan nodes
  , fold
  , denseFold
  , bitset
    -- * Results and compilation
  , collect
  , foldAll
  , compile
  ) where

import Data.Hashable (Hashable)
import qualified Data.Vector.Unboxed as UV
import Language.Haskell.TH (CodeQ)
import Language.Haskell.TH.Syntax (Lift, liftTyped)

import Prela.Id (Id)
import qualified Prela.PullStaged.Ops as O
import Prela.PullStaged.Ops (SMode)
import qualified Prela.PullStaged.Query as Q
import Prela.PullStaged.Query (DenseKey, Scalar)
import Prela.PullStaged.Stream (Lookup, Stream)
import Prela.Storage (Key)

-- | Whether a plan supports keyed reads as well as scanning. Grouping is the
-- only scan-only operation; a fold turns its result back into a readable
-- materialized relation.
data Access = Read | Scan

class Drive (access :: Access)
instance Drive 'Read
instance Drive 'Scan

-- | A typed logical plan. Constructors are deliberately private so lowering is
-- the sole route to executor values.
data Rel (access :: Access) domain range where
  Leaf
    :: (forall q. SMode q => q domain range)
    -> Rel 'Read domain range
  Compose
    :: Rel access domain middle
    -> Rel 'Read middle range
    -> Rel access domain range
  Product
    :: Rel access domain left
    -> Rel 'Read domain right
    -> Rel access domain (left, right)
  Restrict
    :: Rel access domain range
    -> Rel 'Read range predicate
    -> Rel access domain range
  Difference
    :: Rel access domain range
    -> Rel 'Read domain excluded
    -> Rel access domain range
  Filter
    :: CodeQ (range -> Bool)
    -> Rel access domain range
    -> Rel access domain range
  MapValues
    :: CodeQ (range -> mapped)
    -> Rel access domain range
    -> Rel access domain mapped
  GroupBy
    :: Drive access
    => Rel access domain row
    -> Rel 'Read row key
    -> Rel 'Scan key row
  Fold
    :: (Drive access, Hashable domain, Key domain, UV.Unbox accumulator)
    => CodeQ accumulator
    -> CodeQ (accumulator -> value -> accumulator)
    -> Rel access domain value
    -> Rel 'Read domain accumulator
  DenseFold
    :: (Drive access, DenseKey domain, UV.Unbox accumulator)
    => Scalar Int
    -> CodeQ accumulator
    -> CodeQ (accumulator -> value -> accumulator)
    -> Rel access domain value
    -> Rel 'Read domain accumulator
  Bitset
    :: Drive access
    => Scalar Int
    -> Rel access domain (Id entity)
    -> Rel 'Read (Id entity) (Id entity)

-- | The normal readable relation produced by schema leaves and materializers.
type Relation domain range = Rel 'Read domain range

leaf :: (forall q. SMode q => q domain range) -> Relation domain range
leaf = Leaf

compose
  :: Rel access domain middle
  -> Relation middle range
  -> Rel access domain range
compose = Compose

prod
  :: Rel access domain left
  -> Relation domain right
  -> Rel access domain (left, right)
prod = Product

restrict
  :: Rel access domain range
  -> Relation range predicate
  -> Rel access domain range
restrict = Restrict

diff
  :: Rel access domain range
  -> Relation domain excluded
  -> Rel access domain range
diff = Difference

filt :: CodeQ (range -> Bool) -> Rel access domain range -> Rel access domain range
filt = Filter

mapValues
  :: CodeQ (range -> mapped)
  -> Rel access domain range
  -> Rel access domain mapped
mapValues = MapValues

groupBy
  :: Drive access
  => Rel access domain row
  -> Relation row key
  -> Rel 'Scan key row
groupBy = GroupBy

eq, ne :: (Eq range, Lift range)
       => range -> Rel access domain range -> Rel access domain range
eq value = filt [|| \x -> x == $$(liftTyped value) ||]
ne value = filt [|| \x -> x /= $$(liftTyped value) ||]

gt, lt, ge, le :: (Ord range, Lift range)
               => range -> Rel access domain range -> Rel access domain range
gt value = filt [|| \x -> x >  $$(liftTyped value) ||]
lt value = filt [|| \x -> x <  $$(liftTyped value) ||]
ge value = filt [|| \x -> x >= $$(liftTyped value) ||]
le value = filt [|| \x -> x <= $$(liftTyped value) ||]

oneOf :: (Eq range, Lift range)
      => [range] -> Rel access domain range -> Rel access domain range
oneOf values = filt [|| \x -> x `elem` $$(liftTyped values) ||]

between, during
  :: (Ord range, Lift range)
  => range -> range -> Rel access domain range -> Rel access domain range
between low high = filt
  [|| \x -> let value = x
             in value >= $$(liftTyped low) && value <= $$(liftTyped high) ||]
during low high = filt
  [|| \x -> let value = x
             in value >= $$(liftTyped low) && value < $$(liftTyped high) ||]

fold
  :: (Drive access, Hashable domain, Key domain, UV.Unbox accumulator)
  => CodeQ accumulator
  -> CodeQ (accumulator -> value -> accumulator)
  -> Rel access domain value
  -> Relation domain accumulator
fold = Fold

denseFold
  :: (Drive access, DenseKey domain, UV.Unbox accumulator)
  => Scalar Int
  -> CodeQ accumulator
  -> CodeQ (accumulator -> value -> accumulator)
  -> Rel access domain value
  -> Relation domain accumulator
denseFold = DenseFold

bitset
  :: Drive access
  => Scalar Int
  -> Rel access domain (Id entity)
  -> Relation (Id entity) (Id entity)
bitset = Bitset

-- | A terminal plan. It is pure; generated bindings are introduced only when
-- the terminal is compiled.
data Result result where
  Collect
    :: Drive access
    => Rel access domain range
    -> Result [(domain, range)]
  FoldAll
    :: Drive access
    => CodeQ accumulator
    -> CodeQ (accumulator -> value -> accumulator)
    -> Rel access domain value
    -> Result accumulator

collect :: Drive access => Rel access domain range -> Result [(domain, range)]
collect = Collect

foldAll
  :: Drive access
  => CodeQ accumulator
  -> CodeQ (accumulator -> value -> accumulator)
  -> Rel access domain value
  -> Result accumulator
foldAll = FoldAll

compile :: (CodeQ schema -> Result result) -> CodeQ (schema -> result)
compile build = Q.compile (lowerResult . build)

lowerResult :: Result result -> Q.Gen (Scalar result)
lowerResult (Collect rows) = Q.collect <$> lowerDrive rows
lowerResult (FoldAll initial step rows) = do
  stream <- lowerDrive rows
  pure (Q.foldAll (applyStep step) (Q.fromCode initial) stream)

lowerDrive :: Drive access => Rel access domain range -> Q.Gen (Stream domain range)
lowerDrive (Leaf relation) = pure relation
lowerDrive (Compose left right) = do
  leftStream <- lowerDrive left
  rightLookup <- lowerProbe right
  pure (O.compose leftStream rightLookup)
lowerDrive (Product left right) = do
  leftStream <- lowerDrive left
  rightLookup <- lowerProbe right
  pure (O.prod leftStream rightLookup)
lowerDrive (Restrict rows predicate) = do
  rowStream <- lowerDrive rows
  predicateLookup <- lowerProbe predicate
  pure (O.restrict rowStream predicateLookup)
lowerDrive (Difference rows excluded) = do
  rowStream <- lowerDrive rows
  excludedLookup <- lowerProbe excluded
  pure (O.diff rowStream excludedLookup)
lowerDrive (Filter predicate rows) = do
  stream <- lowerDrive rows
  pure (O.filt (applyPredicate predicate) stream)
lowerDrive (MapValues transform rows) = do
  stream <- lowerDrive rows
  pure (O.mapv (applyTransform transform) stream)
lowerDrive (GroupBy rows key) = do
  stream <- lowerDrive rows
  keyLookup <- lowerProbe key
  pure (O.groupBy stream keyLookup)
lowerDrive (Fold initial step rows) = do
  stream <- lowerDrive rows
  relation <- Q.groupFold (applyStep step) (Q.fromCode initial) stream
  pure (Q.stream relation)
lowerDrive (DenseFold size initial step rows) = do
  stream <- lowerDrive rows
  relation <- Q.denseFold size (applyStep step) (Q.fromCode initial) stream
  pure (Q.stream relation)
lowerDrive (Bitset size rows) = do
  stream <- lowerDrive rows
  relation <- Q.bitset size stream
  pure (Q.stream relation)

lowerProbe :: Relation domain range -> Q.Gen (Lookup domain range)
lowerProbe (Leaf relation) = pure relation
lowerProbe (Compose left right) = do
  leftLookup <- lowerProbe left
  rightLookup <- lowerProbe right
  pure (O.compose leftLookup rightLookup)
lowerProbe (Product left right) = do
  leftLookup <- lowerProbe left
  rightLookup <- lowerProbe right
  pure (O.prod leftLookup rightLookup)
lowerProbe (Restrict rows predicate) = do
  rowLookup <- lowerProbe rows
  predicateLookup <- lowerProbe predicate
  pure (O.restrict rowLookup predicateLookup)
lowerProbe (Difference rows excluded) = do
  rowLookup <- lowerProbe rows
  excludedLookup <- lowerProbe excluded
  pure (O.diff rowLookup excludedLookup)
lowerProbe (Filter predicate rows) = do
  relation <- lowerProbe rows
  pure (O.filt (applyPredicate predicate) relation)
lowerProbe (MapValues transform rows) = do
  relation <- lowerProbe rows
  pure (O.mapv (applyTransform transform) relation)
lowerProbe (Fold initial step rows) = do
  stream <- lowerDrive rows
  relation <- Q.groupFold (applyStep step) (Q.fromCode initial) stream
  pure (Q.keyed relation)
lowerProbe (DenseFold size initial step rows) = do
  stream <- lowerDrive rows
  relation <- Q.denseFold size (applyStep step) (Q.fromCode initial) stream
  pure (Q.keyed relation)
lowerProbe (Bitset size rows) = do
  stream <- lowerDrive rows
  relation <- Q.bitset size stream
  pure (Q.keyed relation)

applyPredicate :: CodeQ (value -> Bool) -> CodeQ value -> CodeQ Bool
applyPredicate predicate value = [|| $$predicate $$value ||]

applyTransform :: CodeQ (value -> mapped) -> CodeQ value -> CodeQ mapped
applyTransform transform value = [|| $$transform $$value ||]

applyStep
  :: CodeQ (accumulator -> value -> accumulator)
  -> Scalar accumulator
  -> Scalar value
  -> Scalar accumulator
applyStep step accumulator value = Q.fromCode
  [|| $$step $$(Q.toCode accumulator) $$(Q.toCode value) ||]
