{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The same eight tables as "TPCH.Schema", spliced for the staged engine.
--
-- The record is called @TPCHS@ and the loader @loadTPCHS@, because the two
-- splices would otherwise collide, and the tag types are distinct too: an
-- @Id TPCH.Schema.Lineitem@ and an @Id TPCH.StagedSchema.Lineitem@ are different
-- types. Nothing needs them to agree, since no query mixes the engines.
module TPCH.StagedSchema where

import Prela.Schema
import TPCH.Decl (tpchEntities)

declareStagedSchema "TPCHS" tpchEntities
