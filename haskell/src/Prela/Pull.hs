-- | Prela's pull-stream semantics, without staging: the same `Producer`/
-- `Stream`/`Lookup` architecture as "Prela.PullStaged", except every `CodeQ` is
-- gone — a query here is an ordinary function called at run time, not
-- generated code — and there is no physical storage, no fusion, and nothing
-- to build against a cache.
--
-- "Prela.PullStaged" compiles whole-relation enumeration and keyed access into
-- tight loops using explicit code generation. None of that changes what a
-- query means. This module is the small, direct semantics that the staged
-- implementation is answerable to.
--
-- Because there is no physical storage, there is also no on-disk cache and no
-- "Prela.Schema" — a dataset here is just the plain Haskell values a query is
-- written against, the way "app/PullDemo.hs" builds one.
module Prela.Pull
  ( module Prela.Id
  , module Prela.Pull.Ops
  , module Prela.Pull.Predicate
  , module Prela.Pull.Stream
  , module Prela.Pull.Materialize
  ) where

import Prela.Id
import Prela.Pull.Materialize
import Prela.Pull.Ops
import Prela.Pull.Predicate
import Prela.Pull.Stream
