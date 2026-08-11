-- | The public staged Pull query language. Low-level code-generation machinery
-- remains available from the component modules for executor work.
module Prela.PullStaged
  ( module Prela.Id
  , module Prela.PullStaged.Query
  , SMode
  , Stream
  , Lookup
  , compose
  , prod
  , restrict
  , diff
  , groupBy
  , leftCompose
  , union
  , disj
  , invStream
  ) where

import Prela.Id
import Prela.PullStaged.Ops
  ( SMode, compose, diff, disj, groupBy, leftCompose, prod, restrict, union )
import Prela.PullStaged.Query
import Prela.PullStaged.Stream (Lookup, Stream, invStream)
