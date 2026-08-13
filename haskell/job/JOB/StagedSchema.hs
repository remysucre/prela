{-# LANGUAGE DeriveLift #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Generated storage record, loaders, universes, and relation accessors for
-- the README's focused JOB example.
module JOB.StagedSchema where

import JOB.Decl (jobExampleEntities)
import Prela.Schema

declareStagedSchema "JOBS" jobExampleEntities
