{-# LANGUAGE RankNTypes #-}

-- | Package-private relation facade and mode-selection evidence.
--
-- 'Prela.PullStaged.Query' hides the 'Relation' constructor and the 'asStream'
-- and 'asLookup' selectors. They select an executor representation during code
-- generation; they do not scan data or perform a keyed lookup themselves.
module Prela.PullStaged.Relation where

import qualified Prela.PullStaged.Ops as O
import Prela.PullStaged.Ops (Mode)
import qualified Prela.PullStaged.Stream as S
import Prela.PullStaged.Stream (Lookup, Stream)

-- | A relation that may be instantiated as either a stream or keyed lookup.
newtype Relation d r = Relation
  { useRelation :: forall q. Mode q => q d r
  }

-- | Representations which can supply whole-relation traversal code.
class Drivable q where
  asStream :: q d r -> Stream d r

instance Drivable Stream where
  asStream = id

instance Drivable Relation where
  asStream = useRelation

-- | Representations which can supply keyed-access code.
class Probeable q where
  asLookup :: q d r -> Lookup d r

instance Probeable Lookup where
  asLookup = id

instance Probeable Relation where
  asLookup = useRelation

-- A relation is itself a valid algebra mode. Instantiating its rank-n field as
-- 'Stream' or 'Lookup' selects the corresponding specialized executor instance.
instance O.Mode Relation where
  universe source = Relation (O.universe source)
  column source = Relation (O.column source)
  sparseColumn source = Relation (O.sparseColumn source)
  referenceColumn sourceDomain raw targetDomain =
    Relation (O.referenceColumn sourceDomain raw targetDomain)
  multiColumn source = Relation (O.multiColumn source)
  fromIndex source = Relation (O.fromIndex source)
  fromCache source = Relation (O.fromCache source)
  fromDense source = Relation (O.fromDense source)
  fromDenseInt source = Relation (O.fromDenseInt source)
  fromTable source = Relation (O.fromTable source)
  fromBits source = Relation (O.fromBits source)
  compose (Relation rows) indexed = Relation (O.compose rows indexed)
  prod (Relation rows) indexed = Relation (O.prod rows indexed)
  restrict (Relation rows) indexed = Relation (O.restrict rows indexed)
  diff (Relation rows) indexed = Relation (O.diff rows indexed)
  filt predicate (Relation rows) = Relation (O.filt predicate rows)
  mapv transform (Relation rows) = Relation (O.mapv transform rows)

-- | Algebra over author-level relations. The left side preserves its mode; the
-- right side is selected as a keyed representation by context.
compose :: (Mode q, Probeable p) => q d e -> p e f -> q d f
compose rows indexed = O.compose rows (asLookup indexed)

prod :: (Mode q, Probeable p) => q d u -> p d v -> q d (u, v)
prod rows indexed = O.prod rows (asLookup indexed)

restrict :: (Mode q, Probeable p) => q d r -> p r e -> q d r
restrict rows predicate = O.restrict rows (asLookup predicate)

diff :: (Mode q, Probeable p) => q d r -> p d e -> q d r
diff rows excluded = O.diff rows (asLookup excluded)

groupBy :: (Drivable q, Probeable p) => q d r -> p r k -> Stream k r
groupBy rows key = O.groupBy (asStream rows) (asLookup key)

leftCompose :: (Drivable q, Probeable p) => q d e -> p d f -> Stream e f
leftCompose rows indexed = O.leftCompose (asStream rows) (asLookup indexed)

union :: (Drivable left, Drivable right)
      => left d r -> right d r -> Stream d r
union left right = O.union (asStream left) (asStream right)

-- | Membership union is intentionally probe-only. It defines no duplicate or
-- ordering semantics for whole-relation traversal.
disj :: (Probeable left, Probeable right)
     => left d r -> right d s -> Lookup d ()
disj left right = O.disj (asLookup left) (asLookup right)

invStream :: Drivable q => q d r -> Stream r d
invStream = S.invStream . asStream
