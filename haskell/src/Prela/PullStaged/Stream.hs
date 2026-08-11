{-# LANGUAGE TemplateHaskell #-}

-- | Public staged pull streams and their query-level operations.
--
-- 'Stream' and 'Lookup' are deliberately abstract.  The existential producer
-- state and the constructors used to assemble execution plans live in
-- "Prela.PullStaged.Stream.Internal"; ordinary queries consume and transform
-- streams through this module instead.
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
