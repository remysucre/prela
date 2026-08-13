-- | Safe, representation-independent storage used by schemas and queries.
--
-- Physical vectors, cache-only constructors, mutable key stores, and hash-table
-- machinery live in "Prela.Storage.Internal".  The abstract classes exported
-- here are constraints: clients can select the built-in physical layouts, but
-- cannot manufacture or inspect them directly. Dense, sparse, and multi-valued
-- columns are loaded schema storage; @Dense@, @Table@, and @Bits@ are immutable
-- results produced by staged materializers.
module Prela.Storage
  ( Elem
  , Key
    -- * Columns
  , Col
  , mkCol
  , colLen
  , colAt
  , colValues
  , SparseCol
  , mkSparseCol
  , sparseColLen
  , sparseAt
  , MultiCol
  , mkMultiCol
  , multiColLen
  , multiAt
  , multiValues
    -- * Materialized query storage
  , Dense
  , Table
  , Bits
  , mkBits
  , bitsMember
  ) where

import Prela.Storage.Internal
