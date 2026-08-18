{-# LANGUAGE RankNTypes #-}

-- | Mode-polymorphic relations for the unstaged executor.
module Prela.Pull.Relation
  ( Relation
  , Drivable
  , Probeable
  , drive
  , probe
  , compose
  , prod
  , restrict
  , diff
  , groupBy
  , leftCompose
  , union
  , disj
  , invDrive
  ) where

import qualified Prela.Pull.Ops as O
import Prela.Pull.Ops (Mode)
import qualified Prela.Pull.Stream as S
import Prela.Pull.Stream (Drive, Probe)

-- | A relation which can be instantiated in either executor mode.
newtype Relation k v = Relation
  { useRelation :: forall q. Mode q => q k v
  }

class Drivable q where
  drive :: q k v -> Drive k v

instance Drivable Drive where
  drive = id
  {-# INLINE drive #-}

instance Drivable Relation where
  drive = useRelation
  {-# INLINE drive #-}

class Probeable q where
  probe :: q k v -> Probe k v

instance Probeable Probe where
  probe = id
  {-# INLINE probe #-}

instance Probeable Relation where
  probe = useRelation
  {-# INLINE probe #-}

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
  {-# INLINE universe #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE referenceColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromDenseInt #-}
  {-# INLINE fromTable #-}
  {-# INLINE fromBits #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

compose :: (Mode q, Probeable p) => q k middle -> p middle v -> q k v
compose rows indexed = O.compose rows (probe indexed)
{-# INLINE compose #-}

prod :: (Mode q, Probeable p) => q k left -> p k right -> q k (left, right)
prod rows indexed = O.prod rows (probe indexed)
{-# INLINE prod #-}

restrict :: (Mode q, Probeable p) => q k v -> p v test -> q k v
restrict rows predicate = O.restrict rows (probe predicate)
{-# INLINE restrict #-}

diff :: (Mode q, Probeable p) => q k v -> p k test -> q k v
diff rows excluded = O.diff rows (probe excluded)
{-# INLINE diff #-}

groupBy :: (Drivable q, Probeable p) => q k v -> p v group -> Drive group v
groupBy rows key = O.groupBy (drive rows) (probe key)
{-# INLINE groupBy #-}

leftCompose :: (Drivable q, Probeable p) => q k middle -> p k v -> Drive middle v
leftCompose rows indexed = O.leftCompose (drive rows) (probe indexed)
{-# INLINE leftCompose #-}

union :: (Drivable left, Drivable right)
      => left k v -> right k v -> Drive k v
union left right = O.union (drive left) (drive right)
{-# INLINE union #-}

disj :: (Probeable left, Probeable right)
     => left k u -> right k v -> Probe k ()
disj left right = O.disj (probe left) (probe right)
{-# INLINE disj #-}

invDrive :: Drivable q => q k v -> Drive v k
invDrive = S.invDrive . drive
{-# INLINE invDrive #-}
