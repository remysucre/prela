-- | Prela's pull-stream semantics, without staging: the same `Producer`/
-- `Drive`/`Probe` architecture as "Prela.PullStaged", except every `CodeQ` is
-- gone — a query here is an ordinary function called at run time, not
-- generated code — and there is no physical storage, no fusion, and nothing
-- to build against a cache.
--
-- "Prela.Push" and "Prela.PullStaged" both exist to make `drive`/`probe`
-- compile into a tight loop, and that goal shapes almost everything about
-- them: "Prela.Push.Mode"'s continuation-passing records, "Prela.Storage"'s
-- columnar layouts, "Prela.PullStaged"'s explicit code generation. None of
-- that changes what a query MEANS. This module is what is left after
-- removing all of it — the initial spec the other two are answerable to.
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
