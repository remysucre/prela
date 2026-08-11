-- | Public direct pull streams and their query-level operations.
--
-- 'Stream' and 'Lookup' are abstract. Existential producer state, plan
-- constructors, and list-backed producer leaves live in
-- "Prela.Pull.Stream.Internal".
module Prela.Pull.Stream
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
  ) where

import Prela.Pull.Stream.Internal
