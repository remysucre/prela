-- | Staged pull executor streams and their mode-free operations.
--
-- t'Stream' and t'Lookup' are deliberately abstract.  The existential producer
-- state and the constructors used to assemble execution plans live in
-- "Prela.PullStaged.Stream.Internal". Executor code uses this module; query
-- authors use "Prela.PullStaged.Query". Re-exported Haddock comments describe
-- the individual operations; this facade exists to keep representation
-- constructors out of downstream modules.
module Prela.PullStaged.Stream
  ( Stream
  , Lookup
  , at
    -- * Stream operators
  , mapvS
  , mapkS
  , mapkVS
  , filtS
  , filtKV
  , invStream
  , byValue
  , whenS
  , guardS
  , catS
    -- * Consumers
  , sfoldWhile
  , sfold
  , foldAll
  , count
  , anyOf
  , collect
  , limit
    -- * Staging utilities
  , lam1
  ) where

import Prela.PullStaged.Stream.Internal
