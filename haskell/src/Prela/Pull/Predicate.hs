-- | Ordinary-value predicates and transforms for the unstaged executor.
module Prela.Pull.Predicate
  ( eq
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
  ) where

import qualified Prela.Pull.Ops as O
import Prela.Pull.Ops (Mode)
import Prela.Pull.Relation (Probeable, probe)
import Prela.Pull.Stream (Probe)

eq, ne :: (Mode q, Eq v) => v -> q k v -> q k v
eq value = O.filt (== value)
ne value = O.filt (/= value)
{-# INLINE eq #-}
{-# INLINE ne #-}

gt, lt, ge, le :: (Mode q, Ord v) => v -> q k v -> q k v
gt value = O.filt (> value)
lt value = O.filt (< value)
ge value = O.filt (>= value)
le value = O.filt (<= value)
{-# INLINE gt #-}
{-# INLINE lt #-}
{-# INLINE ge #-}
{-# INLINE le #-}

oneOf :: (Mode q, Eq v) => [v] -> q k v -> q k v
oneOf values = O.filt (`elem` values)
{-# INLINE oneOf #-}

between, range :: (Mode q, Ord v) => v -> v -> q k v -> q k v
between low high = O.filt (\value -> value >= low && value <= high)
range low high = O.filt (\value -> value >= low && value < high)
{-# INLINE between #-}
{-# INLINE range #-}

filterBy :: Mode q => (v -> Bool) -> q k v -> q k v
filterBy = O.filt
{-# INLINE filterBy #-}

mapKeys :: Probeable q => (a -> b) -> q b v -> Probe a v
mapKeys transform = O.mapProbeKey transform . probe
{-# INLINE mapKeys #-}

mapValues :: Mode q => (v -> w) -> q k v -> q k w
mapValues = O.mapv
{-# INLINE mapValues #-}
