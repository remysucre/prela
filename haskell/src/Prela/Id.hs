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
import Data.Maybe (mapMaybe)
import Prela.Id.Internal (Id (..), idIndex)

-- The 'Id' constructor is deliberately omitted from this module's export list:
-- callers can observe an index but can obtain an identifier only through a
-- universe check. Executor internals import "Prela.Id.Internal" when storing an
-- already-validated identifier as a machine integer.

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
{-# INLINE lookupId #-}

containsId :: Universe e -> Id e -> Bool
containsId (Universe n live) (Id i) =
  0 <= i && i < n && maybe True (! i) live
{-# INLINE containsId #-}

universeIds :: Universe e -> [Id e]
universeIds u = mapMaybe (lookupId u) [0 .. universeSize u - 1]

-- | Construct an identifier only when its index lies inside the supplied dense
-- extent. Prefer 'lookupId' when an actual entity universe is available.
boundedId :: Int -> Int -> Maybe (Id e)
boundedId n i
  | 0 <= i && i < n = Just (Id i)
  | otherwise = Nothing
{-# INLINE boundedId #-}
