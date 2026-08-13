{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Package-private implementation of generated scalar expressions.
--
-- 'Prela.PullStaged.Query' selectively exports the author-facing operations
-- while keeping the constructor and typed quotation representation private.
module Prela.PullStaged.Scalar where

import Data.ByteString (ByteString)
import qualified Data.ByteString.Char8 as BS
import qualified Data.ByteString.Unsafe as BSU
import Data.Bits ((.&.), (.|.), finiteBitSize, shiftL)
import Data.String (IsString (..))
import Language.Haskell.TH (CodeQ)
import Language.Haskell.TH.Syntax (Lift, liftTyped)
import Prelude hiding (compare, div, fromIntegral, mod, take)
import qualified Prelude as Base

import Prela.Id (Id, Universe, universeSize)
import qualified Prela.Id as Id

-- | Code for one runtime value. This type exists only while generating code.
newtype Scalar a = Scalar { scalarCode :: CodeQ a }

lit :: Lift a => a -> Scalar a
lit = Scalar . liftTyped

-- | An explicitly typed generated 'Int' literal. Useful when an accumulator's
-- numeric type would otherwise remain ambiguous after splicing.
int :: Int -> Scalar Int
int value = Scalar [|| ($$(liftTyped value) :: Int) ||]

-- | The sole generated product constructor. Larger products are nested binary
-- pairs, matching the representation produced by relational product.
pair :: Scalar a -> Scalar b -> Scalar (a, b)
pair (Scalar a) (Scalar b) = Scalar [|| ($$a, $$b) ||]

first :: Scalar (a, b) -> Scalar a
first (Scalar value) = Scalar [|| fst $$value ||]

second :: Scalar (a, b) -> Scalar b
second (Scalar value) = Scalar [|| snd $$value ||]

-- | Eliminate one generated binary product without exposing quotation syntax.
onPair :: (Scalar a -> Scalar b -> Scalar c) -> Scalar (a, b) -> Scalar c
onPair f (Scalar value) = Scalar
  [|| case $$value of
        (a, b) -> $$(scalarCode (f (Scalar [|| a ||]) (Scalar [|| b ||]))) ||]

-- | Name a scalar expression once in generated code.
letScalar :: Scalar a -> (Scalar a -> Scalar b) -> Scalar b
letScalar (Scalar value) continue = Scalar
  [|| let !sharedScalar = $$value
      in $$(scalarCode (continue (Scalar [|| sharedScalar ||]))) ||]

ifThenElse :: Scalar Bool -> Scalar a -> Scalar a -> Scalar a
ifThenElse (Scalar condition) (Scalar yes) (Scalar no) =
  Scalar [|| if $$condition then $$yes else $$no ||]

compare :: Ord a => Scalar a -> Scalar a -> Scalar Ordering
compare (Scalar left) (Scalar right) = Scalar [|| Base.compare $$left $$right ||]

-- | Lexicographic comparator composition: consult the second comparison only
-- when the first ties.
thenCompare :: Scalar Ordering -> Scalar Ordering -> Scalar Ordering
thenCompare (Scalar firstOrdering) (Scalar secondOrdering) = Scalar
  [|| case $$firstOrdering of
        EQ -> $$secondOrdering
        ordering -> ordering ||]

infix 4 .==., ./=., .<., .<=., .>., .>=.
infixr 3 .&&.
infixr 2 .||.

(.==.), (./=.) :: Eq a => Scalar a -> Scalar a -> Scalar Bool
Scalar a .==. Scalar b = Scalar [|| $$a == $$b ||]
Scalar a ./=. Scalar b = Scalar [|| $$a /= $$b ||]

(.<.), (.<=.), (.>.), (.>=.) :: Ord a => Scalar a -> Scalar a -> Scalar Bool
Scalar a .<.  Scalar b = Scalar [|| $$a <  $$b ||]
Scalar a .<=. Scalar b = Scalar [|| $$a <= $$b ||]
Scalar a .>.  Scalar b = Scalar [|| $$a >  $$b ||]
Scalar a .>=. Scalar b = Scalar [|| $$a >= $$b ||]

(.&&.), (.||.) :: Scalar Bool -> Scalar Bool -> Scalar Bool
Scalar a .&&. Scalar b = Scalar [|| $$a && $$b ||]
Scalar a .||. Scalar b = Scalar [|| $$a || $$b ||]

notS :: Scalar Bool -> Scalar Bool
notS (Scalar value) = Scalar [|| not $$value ||]

fromIntegral :: (Integral a, Num b) => Scalar a -> Scalar b
fromIntegral (Scalar value) = Scalar [|| Base.fromIntegral $$value ||]

div :: Integral a => Scalar a -> Scalar a -> Scalar a
div (Scalar numerator) (Scalar denominator) =
  Scalar [|| $$numerator `Base.div` $$denominator ||]

mod :: Integral a => Scalar a -> Scalar a -> Scalar a
mod (Scalar numerator) (Scalar denominator) =
  Scalar [|| $$numerator `Base.mod` $$denominator ||]

member :: (Eq a, Lift a) => Scalar a -> [a] -> Scalar Bool
member (Scalar value) choices =
  Scalar [|| $$value `elem` $$(liftTyped choices) ||]

take :: Scalar Int -> Scalar ByteString -> Scalar ByteString
take (Scalar size) (Scalar value) = Scalar [|| BS.take $$size $$value ||]

-- | The first byte as an 'Int', or zero for an empty string.
firstByte :: Scalar ByteString -> Scalar Int
firstByte (Scalar value) = Scalar
  [|| case BS.uncons $$value of
        Nothing       -> 0
        Just (byte, _) -> Base.fromEnum byte ||]

isPrefixOf, isSuffixOf, isInfixOf
  :: Scalar ByteString -> Scalar ByteString -> Scalar Bool
isPrefixOf (Scalar needle) (Scalar value) =
  Scalar [|| BS.isPrefixOf $$needle $$value ||]
isSuffixOf (Scalar needle) (Scalar value) =
  Scalar [|| BS.isSuffixOf $$needle $$value ||]
isInfixOf (Scalar needle) (Scalar value) =
  Scalar [|| BS.isInfixOf $$needle $$value ||]

-- | Whether the first byte string occurs before the second.
orderedInfixOf
  :: Scalar ByteString -> Scalar ByteString -> Scalar ByteString -> Scalar Bool
orderedInfixOf (Scalar firstNeedle) (Scalar secondNeedle) (Scalar value) = Scalar
  [|| orderedInfixOfBytes $$firstNeedle $$secondNeedle $$value ||]

-- Search by index rather than constructing the prefix, suffix, and post-match
-- slices returned by 'BS.breakSubstring'. This predicate runs once per candidate
-- row in TPC-H Q13, so even constant-size slice allocation becomes substantial.
orderedInfixOfBytes :: ByteString -> ByteString -> ByteString -> Bool
orderedInfixOfBytes firstNeedle secondNeedle input
  | BS.null firstNeedle = containsFrom secondNeedle input 0
  | firstAt < 0 = False
  | otherwise = containsFrom secondNeedle input (firstAt + BS.length firstNeedle)
  where
    firstAt = findSubstringFrom firstNeedle input 0
{-# INLINE orderedInfixOfBytes #-}

containsFrom :: ByteString -> ByteString -> Int -> Bool
containsFrom needle input start = findSubstringFrom needle input start >= 0
{-# INLINE containsFrom #-}

-- Return the first matching byte offset, or -1. The bounds checks are hoisted
-- out of 'matchesAt', making its unsafe indexing safe by construction.
findSubstringFrom :: ByteString -> ByteString -> Int -> Int
findSubstringFrom needle input start
  | needleLength == 0 = min start inputLength
  | start < 0 = findSubstringFrom needle input 0
  | start > lastStart = -1
  | needleLength <= finiteBitSize (0 :: Word) `Base.div` 8 = packedSearch
  | otherwise = search start
  where
    !needleLength = BS.length needle
    !inputLength = BS.length input
    !lastStart = inputLength - needleLength
    !firstByteOfNeedle = BSU.unsafeIndex needle 0

    -- For short needles, keep the current input window in one machine word.
    -- Q13's two literals are seven and eight bytes, so each new candidate costs
    -- one byte load, a shift, and a comparison rather than a nested byte loop.
    !needleWord = wordAt needle 0 needleLength
    !wordMask = (1 `shiftL` (8 * needleLength)) - 1
    packedSearch = packed (wordAt input start needleLength)
                          (start + needleLength)

    packed !window !end
      | window == needleWord = end - needleLength
      | end >= inputLength = -1
      | otherwise =
          let !next = Base.fromIntegral (BSU.unsafeIndex input end)
              !window' = wordMask .&. ((window `shiftL` 8) .|. next)
          in packed window' (end + 1)

    search !inputAt
      | inputAt > lastStart = -1
      | BSU.unsafeIndex input inputAt == firstByteOfNeedle
          && matchesAt inputAt 1 = inputAt
      | otherwise = search (inputAt + 1)

    matchesAt !inputAt !needleAt
      | needleAt >= needleLength = True
      | BSU.unsafeIndex input (inputAt + needleAt)
          == BSU.unsafeIndex needle needleAt = matchesAt inputAt (needleAt + 1)
      | otherwise = False
{-# INLINE findSubstringFrom #-}

wordAt :: ByteString -> Int -> Int -> Word
wordAt bytes start count = go 0 0
  where
    go !word !offset
      | offset >= count = word
      | otherwise =
          go ((word `shiftL` 8)
                .|. Base.fromIntegral (BSU.unsafeIndex bytes (start + offset)))
             (offset + 1)
{-# INLINE wordAt #-}

mapList :: (Scalar a -> Scalar b) -> Scalar [a] -> Scalar [b]
mapList transform (Scalar values) = Scalar
  [|| map (\value -> $$(scalarCode (transform (Scalar [|| value ||])))) $$values ||]

-- | Observe an entity identifier's validated zero-based position.
idIndex :: Scalar (Id e) -> Scalar Int
idIndex (Scalar identifier) = Scalar [|| Id.idIndex $$identifier ||]

-- | The generated extent of an entity universe.
extent :: CodeQ (Universe e) -> Scalar Int
extent universeCode = Scalar [|| universeSize $$universeCode ||]

instance Num a => Num (Scalar a) where
  Scalar a + Scalar b = Scalar [|| $$a + $$b ||]
  Scalar a - Scalar b = Scalar [|| $$a - $$b ||]
  Scalar a * Scalar b = Scalar [|| $$a * $$b ||]
  negate (Scalar a) = Scalar [|| negate $$a ||]
  abs (Scalar a) = Scalar [|| abs $$a ||]
  signum (Scalar a) = Scalar [|| signum $$a ||]
  fromInteger value = Scalar [|| fromInteger $$(liftTyped value) ||]

instance Fractional a => Fractional (Scalar a) where
  Scalar a / Scalar b = Scalar [|| $$a / $$b ||]
  recip (Scalar a) = Scalar [|| recip $$a ||]
  fromRational value =
    Scalar [|| fromRational $$(liftTyped value) ||]

instance IsString a => IsString (Scalar a) where
  fromString value =
    Scalar [|| fromString $$(liftTyped value) ||]
