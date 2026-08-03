{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The TPC-H schema for the push engine. The declaration itself is in
-- "TPCH.Decl"; this module is only the splice.
module TPCH.Schema where

import Prela.Schema
import TPCH.Decl (tpchEntities)

declareSchema "TPCH" tpchEntities
