{-# LANGUAGE DeriveLift #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Generated storage record and staged accessors for the eight TPC-H tables.
--
-- The schema value lives in "TPCH.Decl"; this module is solely its Template
-- Haskell expansion boundary.  It exports the generated @TPCHS@ record, fast
-- and checked loaders, entity tags, universes, extents, and field relations.
module TPCH.StagedSchema where

import Prela.Schema
import TPCH.Decl (tpchEntities)

-- | Generate the complete TPC-H schema API from the shared declarations.
declareStagedSchema "TPCHS" tpchEntities
