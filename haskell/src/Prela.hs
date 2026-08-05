-- | Prela in Haskell.
--
-- Prela is an embedded query language built on Tarski's algebra of relations.
-- There is exactly one kind of thing: a binary relation — a set of (x, y)
-- pairs — read as a multi-valued function `d -> r` (give it an x, it hands
-- back the y's it relates to: zero, one, or many). Everything else is built by
-- combining relations with operators.
--
-- A relation is accessed in exactly two directions, and which one decides how
-- it executes:
--
--   drive  — enumerate the whole thing: "walk every (x, y) pair." This is the
--            outer-loop role. The left of `→` gets driven.
--   probe  — random access: "here is a key x I already hold; give me its y's."
--            This is the inner role. A filter side gets probed.
--
-- The role is not a property of the relation, it is a property of the POSITION
-- the relation sits in. In `movie : (keyword == "alien") → title`, `movie` is
-- driven while the filter and `title` are probed — same query, both modes.
--
-- The two modes are two record types, and `Mode` is the class with those two
-- records as its instances. Leaves are class methods too, so a leaf is
-- polymorphic and instantiates at whichever mode its position demands, and a
-- query mentions no mode at all — the signature at the top picks one and it
-- flows down. See MODES.md for why it is built this way and what the two
-- rejected alternatives were.
--
-- This module is only a re-export: `import Prela` gets the whole language. The
-- pieces, in the order they depend on each other:
--
--   "Prela.Push.Mode"        the protocol — the two records, `drive`/`probe`, `member`
--   "Prela.Storage"          ids, physical layouts, the three column shapes
--   "Prela.Push.Ops"         the `Mode` class: every leaf, and the mode-free operators
--   "Prela.Push.Predicate"   comparison and regex sugar, all of it `filt`
--   "Prela.Push.Stream"      mode-fixed operators that still fuse, and `foldAll`
--   "Prela.Push.Materialize" the ones that stop the pipeline and hold data
--
-- All five live under "Prela.Push" because their execution protocol is a
-- callback pushed down into a driven relation (`drive :: (d -> r -> m ()) -> m
-- ()`), as opposed to the pull-stream protocol in "Prela.Pull" and
-- "Prela.PullStaged", where the CONSUMER asks a producer for one row at a time.
--
-- Two more modules are deliberately NOT re-exported here, because neither is
-- part of writing a query. "Prela.Cache" reads and writes the on-disk column
-- format, and "Prela.Schema" turns a dataset declaration into leaf names; both
-- are imported explicitly by the one module that loads a dataset. FUSION.md
-- records what the executor's shape depends on and must be read before editing
-- "Prela.Push.Ops" or "Prela.Storage".
module Prela
  ( module Prela.Id
  , module Prela.Push.Mode
  , module Prela.Storage
  , module Prela.Push.Ops
  , module Prela.Push.Predicate
  , module Prela.Push.Stream
  , module Prela.Push.Materialize
  ) where

import Prela.Id
import Prela.Push.Materialize
import Prela.Push.Mode
import Prela.Push.Ops
import Prela.Push.Predicate
import Prela.Push.Stream
import Prela.Storage
