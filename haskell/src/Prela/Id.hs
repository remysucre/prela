-- | Phantom-typed entity identifiers.
module Prela.Id
  ( Id (..)
  , noId
  ) where

import Data.Hashable (Hashable (..))

-- | A zero-based identifier tagged with its entity type. The tag is erased at
-- runtime but prevents mixing ids from different entities.
newtype Id e = Id Int deriving (Eq, Ord, Show)

instance Hashable (Id e) where
  hashWithSalt salt (Id i) = hashWithSalt salt i
  {-# INLINE hashWithSalt #-}

-- | An identifier that points at no entity. Its value fails every valid range
-- check and matches the all-ones hole stored in cache files.
noId :: Id e
noId = Id (-1)
