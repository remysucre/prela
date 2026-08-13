-- | The public staged Pull query language. Low-level code-generation machinery
-- remains available from the component modules for executor work.
module Prela.PullStaged
  ( module Prela.Id
  , module Prela.PullStaged.Query
  , Stream
  , Lookup
  ) where

import Prela.Id
-- The umbrella keeps 'Prela.Id.idIndex' as its unqualified observation.
-- Generated scalar code uses the deliberately qualified 'Q.idIndex'.
import Prela.PullStaged.Query hiding (idIndex)
import Prela.PullStaged.Stream (Lookup, Stream)
