{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Package-private implementation of generated scalar expressions.
--
-- 'Prela.PullStaged.Query' selectively exports the author-facing operations
-- while keeping the constructor and typed quotation representation private. A
-- t'Scalar' is code to be inserted into the compiled query, not a value computed
-- during query construction. Its numeric, comparison, string, and product
-- operations therefore compose typed Template Haskell expressions while
-- preserving the ordinary Haskell surface syntax used by query authors.
module Prela.PullStaged.Scalar where

import Data.ByteString (ByteString)
import qualified Data.ByteString.Char8 as BS
import Data.String (IsString (..))
import Language.Haskell.TH (CodeQ)
import Language.Haskell.TH.Syntax (Lift, liftTyped)
import Prelude hiding (compare, div, fromIntegral, mod, take)
import qualified Prelude as Base

import Prela.Id (Id, Universe, universeSize)
import qualified Prela.Id as Id

-- | Code for one runtime value. This type exists only while generating code.
newtype Scalar a = Scalar { scalarCode :: CodeQ a }

-- | Lift a generation-time constant into generated code.
lit :: Lift a => a -> Scalar a
lit = Scalar . liftTyped

-- | An explicitly typed generated 'Int' literal. Useful when an accumulator's
-- numeric type would otherwise remain ambiguous after splicing.
int :: Int -> Scalar Int
int value = Scalar [|| ($$(liftTyped value) :: Int) ||]

-- | Construct one generated binary product.
pair :: Scalar a -> Scalar b -> Scalar (a, b)
pair (Scalar a) (Scalar b) = Scalar [|| ($$a, $$b) ||]

-- | Construct a left-associated generated triple.
tuple3 :: Scalar a -> Scalar b -> Scalar c -> Scalar ((a, b), c)
tuple3 a b c = pair (pair a b) c

-- | Construct a left-associated generated four-tuple.
tuple4
  :: Scalar a -> Scalar b -> Scalar c -> Scalar d
  -> Scalar (((a, b), c), d)
tuple4 a b c d = pair (tuple3 a b c) d

-- | Construct a left-associated generated five-tuple.
tuple5
  :: Scalar a -> Scalar b -> Scalar c -> Scalar d -> Scalar e
  -> Scalar ((((a, b), c), d), e)
tuple5 a b c d e = pair (tuple4 a b c d) e

-- | Construct a left-associated generated six-tuple.
tuple6
  :: Scalar a -> Scalar b -> Scalar c -> Scalar d -> Scalar e -> Scalar f
  -> Scalar (((((a, b), c), d), e), f)
tuple6 a b c d e f = pair (tuple5 a b c d e) f

-- | Project the first component of a generated pair.
first :: Scalar (a, b) -> Scalar a
first (Scalar value) = Scalar [|| fst $$value ||]

-- | Project the second component of a generated pair.
second :: Scalar (a, b) -> Scalar b
second (Scalar value) = Scalar [|| snd $$value ||]

-- | Eliminate one generated binary product without exposing quotation syntax.
onPair :: (Scalar a -> Scalar b -> Scalar c) -> Scalar (a, b) -> Scalar c
onPair f (Scalar value) = Scalar
  [|| case $$value of
        (a, b) -> $$(scalarCode (f (Scalar [|| a ||]) (Scalar [|| b ||]))) ||]

-- | Eliminate a left-associated generated triple.
onTuple3
  :: (Scalar a -> Scalar b -> Scalar c -> Scalar result)
  -> Scalar ((a, b), c) -> Scalar result
onTuple3 use (Scalar value) = Scalar
  [|| case $$value of
        ((a, b), c) ->
          $$(scalarCode (use (Scalar [|| a ||]) (Scalar [|| b ||])
                             (Scalar [|| c ||]))) ||]

-- | Eliminate a left-associated generated four-tuple.
onTuple4
  :: (Scalar a -> Scalar b -> Scalar c -> Scalar d -> Scalar result)
  -> Scalar (((a, b), c), d) -> Scalar result
onTuple4 use (Scalar value) = Scalar
  [|| case $$value of
        (((a, b), c), d) ->
          $$(scalarCode (use (Scalar [|| a ||]) (Scalar [|| b ||])
                             (Scalar [|| c ||]) (Scalar [|| d ||]))) ||]

-- | Eliminate a left-associated generated five-tuple.
onTuple5
  :: (Scalar a -> Scalar b -> Scalar c -> Scalar d -> Scalar e
      -> Scalar result)
  -> Scalar ((((a, b), c), d), e) -> Scalar result
onTuple5 use (Scalar value) = Scalar
  [|| case $$value of
        ((((a, b), c), d), e) ->
          $$(scalarCode (use (Scalar [|| a ||]) (Scalar [|| b ||])
                             (Scalar [|| c ||]) (Scalar [|| d ||])
                             (Scalar [|| e ||]))) ||]

-- | Eliminate a left-associated generated six-tuple.
onTuple6
  :: (Scalar a -> Scalar b -> Scalar c -> Scalar d -> Scalar e -> Scalar f
      -> Scalar result)
  -> Scalar (((((a, b), c), d), e), f) -> Scalar result
onTuple6 use (Scalar value) = Scalar
  [|| case $$value of
        (((((a, b), c), d), e), f) ->
          $$(scalarCode (use (Scalar [|| a ||]) (Scalar [|| b ||])
                             (Scalar [|| c ||]) (Scalar [|| d ||])
                             (Scalar [|| e ||]) (Scalar [|| f ||]))) ||]

-- | Name a scalar expression once in generated code.
letScalar :: Scalar a -> (Scalar a -> Scalar b) -> Scalar b
letScalar (Scalar value) continue = Scalar
  [|| let !sharedScalar = $$value
      in $$(scalarCode (continue (Scalar [|| sharedScalar ||]))) ||]

-- | Select one of two generated expressions at query runtime.
ifThenElse :: Scalar Bool -> Scalar a -> Scalar a -> Scalar a
ifThenElse (Scalar condition) (Scalar yes) (Scalar no) =
  Scalar [|| if $$condition then $$yes else $$no ||]

-- | Compare two generated values.
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

-- | Generated equality and inequality.
(.==.), (./=.) :: Eq a => Scalar a -> Scalar a -> Scalar Bool
Scalar a .==. Scalar b = Scalar [|| $$a == $$b ||]
Scalar a ./=. Scalar b = Scalar [|| $$a /= $$b ||]

-- | Generated ordered comparisons.
(.<.), (.<=.), (.>.), (.>=.) :: Ord a => Scalar a -> Scalar a -> Scalar Bool
Scalar a .<.  Scalar b = Scalar [|| $$a <  $$b ||]
Scalar a .<=. Scalar b = Scalar [|| $$a <= $$b ||]
Scalar a .>.  Scalar b = Scalar [|| $$a >  $$b ||]
Scalar a .>=. Scalar b = Scalar [|| $$a >= $$b ||]

-- | Generated short-circuiting Boolean conjunction and disjunction.
(.&&.), (.||.) :: Scalar Bool -> Scalar Bool -> Scalar Bool
Scalar a .&&. Scalar b = Scalar [|| $$a && $$b ||]
Scalar a .||. Scalar b = Scalar [|| $$a || $$b ||]

-- | Negate a generated Boolean.
notS :: Scalar Bool -> Scalar Bool
notS (Scalar value) = Scalar [|| not $$value ||]

-- | Convert an integral generated value to another numeric type.
fromIntegral :: (Integral a, Num b) => Scalar a -> Scalar b
fromIntegral (Scalar value) = Scalar [|| Base.fromIntegral $$value ||]

-- | Perform generated integral division.
div :: Integral a => Scalar a -> Scalar a -> Scalar a
div (Scalar numerator) (Scalar denominator) =
  Scalar [|| $$numerator `Base.div` $$denominator ||]

-- | Return the generated integral remainder.
mod :: Integral a => Scalar a -> Scalar a -> Scalar a
mod (Scalar numerator) (Scalar denominator) =
  Scalar [|| $$numerator `Base.mod` $$denominator ||]

-- | Test a generated value against a generation-time list of constants.
member :: (Eq a, Lift a) => Scalar a -> [a] -> Scalar Bool
member (Scalar value) choices =
  Scalar [|| $$value `elem` $$(liftTyped choices) ||]

-- | Take a generated number of bytes from the front of a byte string.
take :: Scalar Int -> Scalar ByteString -> Scalar ByteString
take (Scalar size) (Scalar value) = Scalar [|| BS.take $$size $$value ||]

-- | The first byte as an 'Int', or zero for an empty string.
firstByte :: Scalar ByteString -> Scalar Int
firstByte (Scalar value) = Scalar
  [|| case BS.uncons $$value of
        Nothing       -> 0
        Just (byte, _) -> Base.fromEnum byte ||]

-- | Test generated byte strings for prefix and suffix membership.
isPrefixOf, isSuffixOf
  :: Scalar ByteString -> Scalar ByteString -> Scalar Bool
isPrefixOf (Scalar needle) (Scalar value) =
  Scalar [|| BS.isPrefixOf $$needle $$value ||]
isSuffixOf (Scalar needle) (Scalar value) =
  Scalar [|| BS.isSuffixOf $$needle $$value ||]

-- | Apply a generated scalar transform to every element of a runtime list.
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
