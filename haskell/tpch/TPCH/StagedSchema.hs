{-# LANGUAGE DeriveLift #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The eight TPC-H tables used by the staged pull engine.
module TPCH.StagedSchema where

import Prela.Schema
import TPCH.Decl (tpchEntities)

declareStagedSchema "TPCHS" tpchEntities
