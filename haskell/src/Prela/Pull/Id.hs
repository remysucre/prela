-- | The phantom-typed entity id, split out on its own because it is part of
-- the data model (see CLAUDE.md) and not of physical storage — everything
-- storage-related is what the rest of "Prela.Pull" deliberately has no
-- version of.
module Prela.Pull.Id
  ( Id (..)
  , noId
  ) where

newtype Id e = Id Int deriving (Eq, Ord, Show)

-- | The hole sentinel. Out of range for every id space, so it fails whatever
-- bounds or membership check a probe would otherwise do.
noId :: Id e
noId = Id (-1)
