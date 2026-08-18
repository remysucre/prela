-- | Terminal consumers for unstaged relations.
module Prela.Pull.Consumer
  ( foldAll
  , count
  , anyOf
  , collect
  , limit
  ) where

import Prela.Pull.Relation (Drivable, drive)
import qualified Prela.Pull.Stream as S

foldAll :: Drivable q => (acc -> v -> acc) -> acc -> q k v -> acc
foldAll step initial = S.foldAll step initial . drive
{-# INLINE foldAll #-}

count :: Drivable q => q k v -> Int
count = S.count . drive
{-# INLINE count #-}

anyOf :: Drivable q => q k v -> Bool
anyOf = S.anyOf . drive
{-# INLINE anyOf #-}

collect :: Drivable q => q k v -> [(k, v)]
collect = S.collect . drive

limit :: Drivable q => Int -> q k v -> [(k, v)]
limit size = S.limit size . drive

