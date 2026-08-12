-- | Internal representation of validated entity identifiers.
--
-- Public code receives the abstract type from "Prela.Id". Executor storage may
-- unpack an identifier and later rebuild that same validated index; keeping the
-- constructor here makes that proof-preserving operation possible without an
-- unsafe coercion or exposing arbitrary identifier construction to users.
module Prela.Id.Internal
  ( Id (..)
  , idIndex
  ) where

import Data.Hashable (Hashable (..))

newtype Id e = Id Int deriving (Eq, Ord, Show)

instance Hashable (Id e) where
  hashWithSalt salt (Id i) = hashWithSalt salt i
  {-# INLINE hashWithSalt #-}

idIndex :: Id e -> Int
idIndex (Id i) = i
{-# INLINE idIndex #-}
