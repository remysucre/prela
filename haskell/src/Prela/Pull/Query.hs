-- | Author-facing API for the unstaged pull executor.
module Prela.Pull.Query
  ( Relation
  , Drivable
  , Probeable
  , Drive
  , Probe
  , Mode
  , compose
  , prod
  , restrict
  , diff
  , groupBy
  , leftCompose
  , union
  , disj
  , invDrive
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
  , foldAll
  , count
  , anyOf
  , collect
  , limit
  ) where

import Prela.Pull.Consumer
import Prela.Pull.Ops (Mode)
import Prela.Pull.Predicate
import Prela.Pull.Relation
import Prela.Pull.Stream (Drive, Probe)
