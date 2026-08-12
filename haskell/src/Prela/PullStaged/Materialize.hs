-- | Query-level materializers for staged pull relations.
--
-- Mutable table construction, probing, growth, dense arrays, and generated
-- 'ST' loops are implementation details in
-- "Prela.PullStaged.Materialize.Internal".
module Prela.PullStaged.Materialize
  ( withMaterialize
  , withInv
  , withFold
  , withBufFold
  , withCountDistinct
  , withDenseDistinctCount
  , withDense
  , withDenseOuter
  , withDenseInt
  , withDictionary
  , withBits
  ) where

import Prela.PullStaged.Materialize.Internal
