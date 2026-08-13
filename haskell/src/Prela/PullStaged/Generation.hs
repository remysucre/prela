{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Package-private continuation representation for generated binding scopes.
-- 'Prela.PullStaged.Query' exports 'Gen' abstractly and exposes only 'share'.
module Prela.PullStaged.Generation where

import Language.Haskell.TH (CodeQ)

import Prela.PullStaged.Scalar

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
