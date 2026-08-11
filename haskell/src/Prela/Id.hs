-- | Phantom-typed entity identifiers and the universes that validate them.
module Prela.Id
  ( Id
  , idIndex
  , Universe
  , denseUniverse
  , universeFromMask
  , universeSize
  , universeIds
  , lookupId
  , containsId
  , boundedId
  ) where

import Data.Array.Unboxed (UArray, (!), listArray)
import Data.Hashable (Hashable (..))
import Data.Maybe (mapMaybe)

-- | A zero-based identifier tagged with its entity type. The constructor is
-- deliberately private: an identifier can only be obtained by looking it up in
-- a 'Universe'.
newtype Id e = Id Int deriving (Eq, Ord, Show)

instance Hashable (Id e) where
  hashWithSalt salt (Id i) = hashWithSalt salt i
  {-# INLINE hashWithSalt #-}

-- | The numeric position of an identifier. This is observation, not
-- construction; callers cannot turn an arbitrary number back into an 'Id'.
idIndex :: Id e -> Int
idIndex (Id i) = i
{-# INLINE idIndex #-}

-- | An entity's valid identifier space. A dense universe accepts every index in
-- its range; a sparse one additionally carries a validity mask.
data Universe e = Universe !Int !(Maybe (UArray Int Bool))

-- | Construct a dense universe. Negative sizes are rejected.
denseUniverse :: Int -> Maybe (Universe e)
denseUniverse n
  | n < 0     = Nothing
  | otherwise = Just (Universe n Nothing)

-- | Construct a sparse universe from one validity flag per possible index.
universeFromMask :: [Bool] -> Universe e
universeFromMask live = Universe n (Just (listArray (0, n - 1) live))
  where
    n = length live

universeSize :: Universe e -> Int
universeSize (Universe n _) = n

-- | Validate an integer against both the range and, for a sparse universe, its
-- validity mask.
lookupId :: Universe e -> Int -> Maybe (Id e)
lookupId (Universe n live) i
  | i < 0 || i >= n = Nothing
  | maybe True (! i) live = Just (Id i)
  | otherwise = Nothing

containsId :: Universe e -> Id e -> Bool
containsId u = maybe False (const True) . lookupId u . idIndex

universeIds :: Universe e -> [Id e]
universeIds u = mapMaybe (lookupId u) [0 .. universeSize u - 1]

-- | Construct an identifier only when its index lies inside the supplied dense
-- extent. Prefer 'lookupId' when an actual entity universe is available.
boundedId :: Int -> Int -> Maybe (Id e)
boundedId n i
  | 0 <= i && i < n = Just (Id i)
  | otherwise = Nothing
