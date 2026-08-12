{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Ergonomic query construction over the staged pull executor.
--
-- 'Scalar' hides typed quotation for ordinary scalar expressions, 'Relation'
-- hides a relation's rank-n mode polymorphism, and 'Gen' turns the executor's
-- continuation-shaped materializers into pure @do@ notation.  None of these
-- types exists in generated code: they are generation-time wrappers only.
module Prela.PullStaged.Query
  ( -- * Generated scalars
    Scalar
  , lit
  , int
  , pair
  , first
  , second
  , onPair
  , letScalar
  , ifThenElse
  , compare
  , thenCompare
  , (.==.)
  , (./=.)
  , (.<.)
  , (.<=.)
  , (.>.)
  , (.>=.)
  , (.&&.)
  , (.||.)
  , notS
  , fromIntegral
  , div
  , mod
  , member
  , take
  , firstByte
  , isPrefixOf
  , isSuffixOf
  , isInfixOf
  , orderedInfixOf
  , mapList
  , idIndex
  , extent
    -- * Relations
  , Relation
  , stream
  , keyed
    -- * Pure generation
  , Gen
  , share
  , materialize
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
  , regex
    -- * Predicates and value transforms
  , eq
  , ne
  , gt
  , lt
  , ge
  , le
  , oneOf
  , between
  , range
  , filterBy
  , mapKeys
  , mapValues
  , rx
  , nrx
    -- * Consumers
  , foldAll
  , count
  , anyOf
  , collect
  , limit
    -- * Complete queries
  , Query
  , query
  , compile
  ) where

import Data.Hashable (Hashable)
import Data.ByteString (ByteString)
import qualified Data.ByteString.Char8 as BS
import Data.String (IsString (..))
import qualified Data.Vector as BV
import qualified Data.Vector.Unboxed as UV
import Language.Haskell.TH (CodeQ)
import Language.Haskell.TH.Syntax (Lift, liftTyped)
import Prelude hiding (compare, div, fromIntegral, mod, take)
import qualified Prelude as Base
import Text.Regex.TDFA (Regex, RegexLike)

import Prela.Id (Id, Universe, universeSize)
import qualified Prela.Id as Id
import qualified Prela.PullStaged.Materialize as M
import qualified Prela.PullStaged.Ops as O
import Prela.PullStaged.Ops (SMode)
import qualified Prela.PullStaged.Predicate as P
import qualified Prela.PullStaged.Stream as S
import Prela.PullStaged.Stream (Lookup, Stream)
import Prela.Storage (Key)

--------------------------------------------------------------------------------
-- Scalars
--------------------------------------------------------------------------------

-- | Code for one runtime value. The constructor and quotation representation
-- stay private to this module.
newtype Scalar a = Scalar { scalarCode :: CodeQ a }

lit :: Lift a => a -> Scalar a
lit = Scalar . liftTyped

-- | An explicitly typed generated 'Int' literal. Useful when an accumulator's
-- numeric type would otherwise remain ambiguous after splicing.
int :: Int -> Scalar Int
int value = Scalar [|| ($$(liftTyped value) :: Int) ||]

-- | The sole generated product constructor. Larger products are nested binary
-- pairs, matching the representation produced by relational 'O.prod'.
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
-- when the first ties. Useful with 'topK' without exposing generated code.
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

-- | The first byte as an 'Int', or zero for an empty string. Returning a
-- defined default keeps this observation total; schema-specific callers may
-- rely on a stronger non-empty invariant without introducing partial indexing.
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

-- | Whether the first byte string occurs before the second. This is the
-- literal-substring equivalent of @first.*second@, without invoking a regex
-- engine. Empty and missing inputs follow 'ByteString' search semantics.
orderedInfixOf
  :: Scalar ByteString -> Scalar ByteString -> Scalar ByteString -> Scalar Bool
orderedInfixOf (Scalar firstNeedle) (Scalar secondNeedle) (Scalar value) = Scalar
  [|| let !needle1 = $$firstNeedle
          !needle2 = $$secondNeedle
          !input = $$value
      in if BS.null needle1
           then BS.isInfixOf needle2 input
           else case BS.breakSubstring needle1 input of
             (_, suffix)
               | BS.null suffix -> False
               | otherwise -> BS.isInfixOf needle2
                                (BS.drop (BS.length needle1) suffix) ||]

mapList :: (Scalar a -> Scalar b) -> Scalar [a] -> Scalar [b]
mapList transform (Scalar values) = Scalar
  [|| map (\value -> $$(scalarCode (transform (Scalar [|| value ||])))) $$values ||]

-- | Observe an entity identifier's validated zero-based position. This does
-- not expose identifier construction: the result is an ordinary generated
-- 'Int', useful for compact unboxed fold state.
idIndex :: Scalar (Id e) -> Scalar Int
idIndex (Scalar identifier) = Scalar [|| Id.idIndex $$identifier ||]

-- | The generated extent of an entity universe. Schema declarations expose a
-- named accessor built from this primitive.
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

--------------------------------------------------------------------------------
-- Relations and generation
--------------------------------------------------------------------------------

-- | A relation that may be instantiated as either a stream or a keyed lookup.
newtype Relation d r = Relation
  { use :: forall q. SMode q => q d r
  }

-- | Enumerate a generated relation.
stream :: Relation d r -> Stream d r
stream = use

-- | Access a generated relation by key.
keyed :: Relation d r -> Lookup d r
keyed = use

-- | A pure CPS builder. A bind introduces nested generated runtime bindings;
-- it performs no effects while a query runs.
newtype Gen a = Gen
  { runGenWith :: forall w. (a -> CodeQ w) -> CodeQ w
  }

instance Functor Gen where
  fmap f (Gen action) = Gen (\continue -> action (continue . f))

instance Applicative Gen where
  pure value = Gen (\continue -> continue value)
  Gen function <*> Gen argument = Gen $ \continue ->
    function $ \f -> argument (continue . f)

instance Monad Gen where
  Gen action >>= next = Gen $ \continue ->
    action $ \value -> runGenWith (next value) continue

runScalarGen :: Gen (Scalar a) -> CodeQ a
runScalarGen action = runGenWith action scalarCode

-- | Bind a generated scalar once and return a reusable reference to it.
share :: Scalar a -> Gen (Scalar a)
share (Scalar value) = Gen $ \continue ->
  [|| let !shared = $$value
      in $$(continue (Scalar [|| shared ||])) ||]

materialize :: Ord d => Stream d r -> Gen (Relation d r)
materialize rows = Gen $ \continue ->
  M.withMaterialize rows (\relation -> continue (Relation relation))

invert :: Ord r => Stream d r -> Gen (Relation r d)
invert rows = Gen $ \continue ->
  M.withInv rows (\relation -> continue (Relation relation))

groupFold :: (Hashable d, Key d, UV.Unbox acc)
          => (Scalar acc -> Scalar r -> Scalar acc)
          -> Scalar acc -> Stream d r -> Gen (Relation d acc)
groupFold step (Scalar initial) rows = Gen $ \continue ->
  M.withFold (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
             initial rows
             (\relation -> continue (Relation relation))

bufferFold :: Ord d
           => (Scalar [r] -> Scalar acc)
           -> Stream d r -> Gen (Relation d acc)
bufferFold reduce rows = Gen $ \continue ->
  M.withBufFold (scalarCode . reduce . Scalar) rows
                (\relation -> continue (Relation relation))

distinctCount :: (Hashable d, Key d, Hashable r, Key r)
              => Stream d r -> Gen (Relation d Int)
distinctCount rows = Gen $ \continue ->
  M.withCountDistinct rows (\relation -> continue (Relation relation))

-- | Count distinct entity ids within bounded integer groups. Supplying the two
-- extents lets the executor use one packed integer per pair and a dense count
-- array, while preserving the same relation result as 'distinctCount'.
denseDistinctCount :: Scalar Int -> Scalar Int -> Stream Int (Id e)
                   -> Gen (Relation Int Int)
denseDistinctCount (Scalar groups) (Scalar memberExtent) rows = Gen $ \continue ->
  M.withDenseDistinctCount groups memberExtent rows
    (\relation -> continue (Relation relation))

-- | Keys that can safely select a slot in a bounded dense aggregate. Entity
-- identifiers are already non-negative; ordinary integers receive an explicit
-- lower- and upper-bound check in the generated loop.
class DenseKey key where
  withDenseKey
    :: UV.Unbox acc
    => CodeQ Int
    -> (CodeQ acc -> CodeQ r -> CodeQ acc)
    -> CodeQ acc
    -> Stream key r
    -> ((forall q. SMode q => q key acc) -> CodeQ w)
    -> CodeQ w

instance DenseKey (Id e) where
  withDenseKey = M.withDense

instance DenseKey Int where
  withDenseKey = M.withDenseInt

denseFold :: (DenseKey key, UV.Unbox acc)
          => Scalar Int
          -> (Scalar acc -> Scalar r -> Scalar acc)
          -> Scalar acc
          -> Stream key r
          -> Gen (Relation key acc)
denseFold (Scalar size) step (Scalar initial) rows = Gen $ \continue ->
  withDenseKey size
    (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
    initial rows (\relation -> continue (Relation relation))

denseFoldOuter :: UV.Unbox acc
               => Scalar Int
               -> (Scalar acc -> Scalar r -> Scalar acc)
               -> Scalar acc
               -> Stream (Id e) r
               -> Gen (Relation (Id e) acc)
denseFoldOuter (Scalar size) step (Scalar initial) rows = Gen $ \continue ->
  M.withDenseOuter size
    (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
    initial rows (\relation -> continue (Relation relation))

-- | Assign a compact integer to each distinct stream value. The relation maps
-- every input entity to its code; the vector maps codes back to values for final
-- rendering. This keeps repeated strings and compound values out of hot keys.
dictionary :: Ord value
           => Scalar Int -> Stream (Id e) value
           -> Gen (Relation (Id e) Int, Scalar (BV.Vector value))
dictionary (Scalar size) rows = Gen $ \continue ->
  M.withDictionary size rows $ \relation labels ->
    continue (Relation relation, Scalar labels)

bitset :: Scalar Int -> Stream d (Id e) -> Gen (Relation (Id e) (Id e))
bitset (Scalar size) rows = Gen $ \continue ->
  M.withBits size rows (\relation -> continue (Relation relation))

-- | Keep a bounded, ordered result inside generated code and pass its rows to
-- subsequent relational operators. The comparator follows 'Data.List.sortBy':
-- @LT@ means the left row ranks ahead of the right row.
topK
  :: Scalar Int
  -> (Scalar d -> Scalar r -> Scalar d -> Scalar r -> Scalar Ordering)
  -> Stream d r
  -> Gen (Stream d r)
topK (Scalar size) order rows = Gen $ \continue ->
  M.withTopK size
    (\leftRow rightRow ->
      let left = Scalar leftRow
          right = Scalar rightRow
      in scalarCode
           (order (first left) (second left) (first right) (second right)))
    rows continue

regex :: String -> Gen (Scalar Regex)
regex expression = Gen $ \continue ->
  P.withRegex expression (continue . Scalar)

--------------------------------------------------------------------------------
-- Surface predicates and consumers
--------------------------------------------------------------------------------

eq, ne :: (SMode q, Eq r) => Scalar r -> q d r -> q d r
eq (Scalar value) = P.eq value
ne (Scalar value) = P.ne value

gt, lt, ge, le :: (SMode q, Ord r) => Scalar r -> q d r -> q d r
gt (Scalar value) = P.gt value
lt (Scalar value) = P.lt value
ge (Scalar value) = P.ge value
le (Scalar value) = P.le value

oneOf :: (SMode q, Eq r, Lift r) => [r] -> q d r -> q d r
oneOf values = P.isIn (liftTyped values)

between, range :: (SMode q, Ord r)
               => Scalar r -> Scalar r -> q d r -> q d r
between (Scalar low) (Scalar high) = P.between low high
range (Scalar low) (Scalar high) = P.range low high

filterBy :: SMode q => (Scalar r -> Scalar Bool) -> q d r -> q d r
filterBy predicate = O.filt (scalarCode . predicate . Scalar)

-- | Adapt the input key of a lookup. For example, a relation keyed by nation
-- can be attached to @(nation, year)@ groups with @mapKeys first@.
mapKeys :: (Scalar a -> Scalar b) -> Lookup b r -> Lookup a r
mapKeys transform = O.mapLookupKey (scalarCode . transform . Scalar)

mapValues :: SMode q => (Scalar r -> Scalar s) -> q d r -> q d s
mapValues transform = O.mapv (scalarCode . transform . Scalar)

rx, nrx :: (SMode q, RegexLike Regex s) => Scalar Regex -> q d s -> q d s
rx (Scalar expression) = P.rx expression
nrx (Scalar expression) = P.nrx expression

foldAll :: (Scalar acc -> Scalar r -> Scalar acc)
        -> Scalar acc -> Stream d r -> Scalar acc
foldAll step (Scalar initial) rows = Scalar
  (S.foldAll (\acc value -> scalarCode (step (Scalar acc) (Scalar value)))
             initial rows)

count :: Stream d r -> Scalar Int
count = Scalar . S.count

anyOf :: Stream d r -> Scalar Bool
anyOf = Scalar . S.anyOf

collect :: Stream d r -> Scalar [(d, r)]
collect = Scalar . S.collect

limit :: Scalar Int -> Stream d r -> Scalar [(d, r)]
limit (Scalar size) = Scalar . S.limit size

--------------------------------------------------------------------------------
-- Complete queries
--------------------------------------------------------------------------------

-- | A complete staged function from a loaded schema to a result.
newtype Query schema result = Query (CodeQ schema -> Gen (Scalar result))

query :: (CodeQ schema -> Gen (Scalar result)) -> Query schema result
query = Query

compile :: Query schema result -> CodeQ (schema -> result)
compile (Query build) = S.lam1 (runScalarGen . build)
