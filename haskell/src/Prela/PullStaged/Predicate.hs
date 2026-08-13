{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Predicates and value transforms over generated scalar expressions.
--
-- These functions are author-facing syntax over the fixed relational operators.
-- Each predicate becomes code inside the surrounding stream or lookup; there
-- is no predicate AST and no optimizer pass between this layer and execution.
module Prela.PullStaged.Predicate
  ( eq
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
  , regex
  , rx
  , nrx
  ) where

import Language.Haskell.TH (CodeQ)
import Language.Haskell.TH.Syntax (Lift, liftTyped)
import Text.Regex.TDFA (Regex, RegexLike, makeRegex, matchTest)

import Prela.PullStaged.Generation (Gen (..))
import qualified Prela.PullStaged.Ops as O
import Prela.PullStaged.Ops (Mode)
import Prela.PullStaged.Relation (Probeable (asLookup))
import Prela.PullStaged.Scalar
import Prela.PullStaged.Stream (Lookup)

-- | Keep values equal to, or unequal to, the supplied scalar.
eq, ne :: (Mode q, Eq r) => Scalar r -> q d r -> q d r
eq (Scalar value) = O.filt (\x -> [|| $$x == $$value ||])
ne (Scalar value) = O.filt (\x -> [|| $$x /= $$value ||])

-- | Keep values ordered above, below, at least, or at most the supplied scalar.
gt, lt, ge, le :: (Mode q, Ord r) => Scalar r -> q d r -> q d r
gt (Scalar value) = O.filt (\x -> [|| $$x >  $$value ||])
lt (Scalar value) = O.filt (\x -> [|| $$x <  $$value ||])
ge (Scalar value) = O.filt (\x -> [|| $$x >= $$value ||])
le (Scalar value) = O.filt (\x -> [|| $$x <= $$value ||])

-- | Keep values equal to one of the generation-time constants.
oneOf :: (Mode q, Eq r, Lift r) => [r] -> q d r -> q d r
oneOf values = O.filt (\x -> [|| $$x `elem` $$(liftTyped values) ||])

-- | Closed and half-open generated ranges, respectively.
between, range :: (Mode q, Ord r)
               => Scalar r -> Scalar r -> q d r -> q d r
between (Scalar low) (Scalar high) = O.filt
  (\x -> [|| let !value = $$x in value >= $$low && value <= $$high ||])
range (Scalar low) (Scalar high) = O.filt
  (\x -> [|| let !value = $$x in value >= $$low && value < $$high ||])

-- | Keep values satisfying a generated scalar predicate.
filterBy :: Mode q => (Scalar r -> Scalar Bool) -> q d r -> q d r
filterBy predicate = O.filt (scalarCode . predicate . Scalar)

-- | Adapt the input key of a lookup.
mapKeys :: Probeable q => (Scalar a -> Scalar b) -> q b r -> Lookup a r
mapKeys transform = O.mapLookupKey (scalarCode . transform . Scalar) . asLookup

-- | Transform relation values without changing their keys.
mapValues :: Mode q => (Scalar r -> Scalar s) -> q d r -> q d s
mapValues transform = O.mapv (scalarCode . transform . Scalar)

-- | Compile a regex once in generated code and return a reusable scalar handle.
regex :: String -> Gen (Scalar Regex)
regex expression = Gen $ \continue ->
  withRegex expression (continue . Scalar)

-- | Bind one compiled regular expression around generated continuation code.
withRegex :: String -> (CodeQ Regex -> CodeQ w) -> CodeQ w
withRegex expression continue =
  [|| let !compiled = makeRegex (expression :: String) :: Regex
      in $$(continue [|| compiled ||]) ||]

-- | Keep values which do, or do not, match a compiled regular expression.
rx, nrx :: (Mode q, RegexLike Regex s) => Scalar Regex -> q d s -> q d s
rx (Scalar expression) = O.filt (\x -> [|| matchTest $$expression $$x ||])
nrx (Scalar expression) = O.filt (\x -> [|| not (matchTest $$expression $$x) ||])
