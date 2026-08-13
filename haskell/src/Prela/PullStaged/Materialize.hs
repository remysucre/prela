{-# LANGUAGE RankNTypes #-}

-- | Explicit materialization boundaries for staged pull relations.
--
-- Every operation here consumes a drivable relation into a named physical
-- representation or bounded buffer. Mutable builders and continuation-shaped
-- @with*@ functions remain in "Prela.PullStaged.Materialize.Internal".
module Prela.PullStaged.Materialize
  ( materialize
  , invert
  , groupFold
  , bufferFold
  , distinctCount
  , denseDistinctCount
  , DenseKey
  , denseFold
  , denseFoldOuter
  , dictionary
  , bitset
  , topK
  ) where

import Data.Hashable (Hashable)
import qualified Data.Vector as BV
import qualified Data.Vector.Unboxed as UV
import Language.Haskell.TH (CodeQ)

import Prela.Id (Id)
import Prela.PullStaged.Generation (Gen (..))
import qualified Prela.PullStaged.Materialize.Internal as M
import Prela.PullStaged.Ops (Mode)
import Prela.PullStaged.Relation
  ( Drivable (asStream), Relation (..))
import Prela.PullStaged.Scalar
import Prela.PullStaged.Stream (Stream)
import Prela.Storage (Key)

materialize :: (Ord d, Drivable q) => q d r -> Gen (Relation d r)
materialize rows = Gen $ \continue ->
  M.withMaterialize (asStream rows) (\relation -> continue (Relation relation))

invert :: (Ord r, Drivable q) => q d r -> Gen (Relation r d)
invert rows = Gen $ \continue ->
  M.withInv (asStream rows) (\relation -> continue (Relation relation))

groupFold :: (Drivable q, Hashable d, Key d, UV.Unbox acc)
          => (Scalar acc -> Scalar r -> Scalar acc)
          -> Scalar acc -> q d r -> Gen (Relation d acc)
groupFold step (Scalar initial) rows = Gen $ \continue ->
  M.withFold (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
             initial (asStream rows)
             (\relation -> continue (Relation relation))

bufferFold :: (Drivable q, Ord d)
           => (Scalar [r] -> Scalar acc)
           -> q d r -> Gen (Relation d acc)
bufferFold reduce rows = Gen $ \continue ->
  M.withBufFold (scalarCode . reduce . Scalar) (asStream rows)
                (\relation -> continue (Relation relation))

distinctCount :: (Drivable q, Hashable d, Key d, Hashable r, Key r)
              => q d r -> Gen (Relation d Int)
distinctCount rows = Gen $ \continue ->
  M.withCountDistinct (asStream rows) (\relation -> continue (Relation relation))

-- | Count distinct entity ids within bounded integer groups.
denseDistinctCount :: Drivable q => Scalar Int -> Scalar Int -> q Int (Id e)
                   -> Gen (Relation Int Int)
denseDistinctCount (Scalar groups) (Scalar memberExtent) rows = Gen $ \continue ->
  M.withDenseDistinctCount groups memberExtent (asStream rows)
    (\relation -> continue (Relation relation))

-- | Keys that can safely select a slot in a bounded dense aggregate.
class DenseKey key where
  withDenseKey
    :: UV.Unbox acc
    => CodeQ Int
    -> (CodeQ acc -> CodeQ r -> CodeQ acc)
    -> CodeQ acc
    -> Stream key r
    -> ((forall q. Mode q => q key acc) -> CodeQ w)
    -> CodeQ w

instance DenseKey (Id e) where
  withDenseKey = M.withDense

instance DenseKey Int where
  withDenseKey = M.withDenseInt

denseFold :: (Drivable q, DenseKey key, UV.Unbox acc)
          => Scalar Int
          -> (Scalar acc -> Scalar r -> Scalar acc)
          -> Scalar acc
          -> q key r
          -> Gen (Relation key acc)
denseFold (Scalar size) step (Scalar initial) rows = Gen $ \continue ->
  withDenseKey size
    (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
    initial (asStream rows) (\relation -> continue (Relation relation))

denseFoldOuter :: (Drivable q, UV.Unbox acc)
               => Scalar Int
               -> (Scalar acc -> Scalar r -> Scalar acc)
               -> Scalar acc
               -> q (Id e) r
               -> Gen (Relation (Id e) acc)
denseFoldOuter (Scalar size) step (Scalar initial) rows = Gen $ \continue ->
  M.withDenseOuter size
    (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
    initial (asStream rows) (\relation -> continue (Relation relation))

-- | Assign a compact integer to each distinct stream value and return the
-- reverse dictionary alongside the coded relation.
dictionary :: (Drivable q, Ord value)
           => Scalar Int -> q (Id e) value
           -> Gen (Relation (Id e) Int, Scalar (BV.Vector value))
dictionary (Scalar size) rows = Gen $ \continue ->
  M.withDictionary size (asStream rows) $ \relation labels ->
    continue (Relation relation, Scalar labels)

bitset :: Drivable q
       => Scalar Int -> q d (Id e) -> Gen (Relation (Id e) (Id e))
bitset (Scalar size) rows = Gen $ \continue ->
  M.withBits size (asStream rows) (\relation -> continue (Relation relation))

-- | Retain a bounded ordered result inside generated code.
topK
  :: Drivable q
  => Scalar Int
  -> (Scalar d -> Scalar r -> Scalar d -> Scalar r -> Scalar Ordering)
  -> q d r
  -> Gen (Stream d r)
topK (Scalar size) order rows = Gen $ \continue ->
  M.withTopK size
    (\leftRow rightRow ->
      let left = Scalar leftRow
          right = Scalar rightRow
      in scalarCode
           (order (first left) (second left) (first right) (second right)))
    (asStream rows) continue
