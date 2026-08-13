{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE TemplateHaskell #-}
{-# OPTIONS_GHC -fno-full-laziness #-}

-- | Template Haskell compilation boundary for JOB query 22a.
module JOB.Staged (runQ22a) where

import JOB.Q22a (q22a, renderQ22a)
import JOB.StagedSchema (JOBS)
import qualified Prela.PullStaged.Query as Q

-- | Run the compiled query and render it exactly like the Rust JOB harness.
runQ22a :: JOBS -> String
runQ22a = renderQ22a . $$(Q.compile q22a)
