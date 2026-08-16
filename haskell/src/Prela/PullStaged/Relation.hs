{-# LANGUAGE RankNTypes #-}

-- | Package-private relation facade and mode-selection evidence.
--
-- "Prela.PullStaged.Query" hides the t'Relation' constructor and the 'drive'
-- and 'probe' methods. They select an executor representation during code
-- generation; they do not scan data or perform a keyed probe themselves. This
-- rank-n facade lets one relation definition be driven on the left of an
-- operator and probed on the right without exposing either representation to a
-- query author.
module Prela.PullStaged.Relation where

import qualified Prela.PullStaged.Ops as O
import Prela.PullStaged.Ops (Mode)
import qualified Prela.PullStaged.Stream as S
import Prela.PullStaged.Stream (Probe, Drive)

-- | A relation that may be instantiated as either a drive or a probe.
newtype Relation d r = Relation
  { useRelation :: forall q. Mode q => q d r
  }

-- | Representations which can supply whole-relation traversal code.
class Drivable q where
  -- | Select the whole-relation drive representation.
  drive :: q d r -> Drive d r

instance Drivable Drive where
  drive = id

instance Drivable Relation where
  drive = useRelation

-- | Representations which can supply keyed-access code.
class Probeable q where
  -- | Select the keyed probe representation.
  probe :: q d r -> Probe d r

instance Probeable Probe where
  probe = id

instance Probeable Relation where
  probe = useRelation

-- A relation is itself a valid algebra mode. Instantiating its rank-n field as
-- t'Drive' or t'Probe' selects the corresponding specialized executor instance.
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

-- | Compose through the left relation's value. The left operand preserves its
-- mode and the right operand is selected as a keyed probe.
compose :: (Mode q, Probeable p) => q d e -> p e f -> q d f
compose rows indexed = O.compose rows (probe indexed)

-- | Pair values from two relations which share a key.
prod :: (Mode q, Probeable p) => q d u -> p d v -> q d (u, v)
prod rows indexed = O.prod rows (probe indexed)

-- | Keep left rows whose value occurs as a key in the right relation.
restrict :: (Mode q, Probeable p) => q d r -> p r e -> q d r
restrict rows predicate = O.restrict rows (probe predicate)

-- | Keep left rows whose key has no match in the right relation.
diff :: (Mode q, Probeable p) => q d r -> p d e -> q d r
diff rows excluded = O.diff rows (probe excluded)

-- | Rekey each driven row using a probe on its value.
groupBy :: (Drivable q, Probeable p) => q d r -> p r k -> Drive k r
groupBy rows key = O.groupBy (drive rows) (probe key)

-- | Rekey the right relation by values enumerated from the left relation.
leftCompose :: (Drivable q, Probeable p) => q d e -> p d f -> Drive e f
leftCompose rows indexed = O.leftCompose (drive rows) (probe indexed)

-- | Run the left drive followed by the right drive.
union :: (Drivable left, Drivable right)
      => left d r -> right d r -> Drive d r
union left right = O.union (drive left) (drive right)

-- | Membership union is intentionally probe-only. It defines no duplicate or
-- ordering semantics for whole-relation traversal.
disj :: (Probeable left, Probeable right)
     => left d r -> right d s -> Probe d ()
disj left right = O.disj (probe left) (probe right)

-- | Swap keys and values for enumeration without building a reverse index.
invDrive :: Drivable q => q d r -> Drive r d
invDrive = S.invDrive . drive
