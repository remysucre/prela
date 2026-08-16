-- | Staged streams, drives, probes, and their mode-free operations.
--
-- t'Drive' and t'Probe' are deliberately abstract. The existential stream
-- state and the constructors used to assemble execution plans live in
-- "Prela.PullStaged.Stream.Internal". Executor code uses this module; query
-- authors use "Prela.PullStaged.Query". Re-exported Haddock comments describe
-- the individual operations; this facade exists to keep representation
-- constructors out of downstream modules.
module Prela.PullStaged.Stream
  ( Drive
  , Probe
  , at
    -- * Drive operators
  , mapvD
  , mapkD
  , mapkVD
  , filtD
  , filtDKV
  , invDrive
  , byValue
  , whenD
  , guardD
  , catD
    -- * Consumers
  , dfoldWhile
  , dfold
  , foldAll
  , count
  , anyOf
  , collect
  , limit
    -- * Staging utilities
  , lam1
  ) where

import Prela.PullStaged.Stream.Internal
