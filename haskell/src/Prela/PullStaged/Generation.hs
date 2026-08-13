{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Package-private continuation representation for generated binding scopes.
--
-- t'Gen' is not a runtime effect system. Its sequencing determines where strict
-- bindings and materialized caches appear in generated code. The continuation
-- representation prevents a shared 'CodeQ' expression from being emitted once
-- at every use site. 'Prela.PullStaged.Query' exports the type abstractly and
-- exposes 'share' as its scalar binding operation.
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

-- | Close a scalar generation scope and return its typed generated code.
runScalarGen :: Gen (Scalar a) -> CodeQ a
runScalarGen action = runGenWith action scalarCode

-- | Bind a generated scalar once and return a reusable reference to it.
share :: Scalar a -> Gen (Scalar a)
share (Scalar value) = Gen $ \continue ->
  [|| let !shared = $$value
      in $$(continue (Scalar [|| shared ||])) ||]
