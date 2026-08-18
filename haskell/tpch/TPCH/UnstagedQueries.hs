{-# LANGUAGE BangPatterns #-}
{-# OPTIONS_GHC -fno-full-laziness #-}

-- | Unstaged counterparts of selected TPC-H queries.
--
-- Q6 is intentionally written in the same relational form as the staged
-- definition.  It uses the same loaded schema and physical columns; only the
-- executor's binding time differs.  The concrete variant below is an
-- optimization diagnostic, not a different query.
module TPCH.UnstagedQueries (q6, q6Concrete) where

import Prela.Id (Id)
import qualified Prela.Pull.Ops as O
import Prela.Pull.Query (Relation, compose, prod, restrict)
import qualified Prela.Pull.Query as Q
import Prela.Pull.Stream (Drive, Probe)

import TPCH.StagedSchema

-- | TPC-H Q6 on the runtime pull executor.
q6 :: TPCHS -> Double
q6 schema = Q.foldAll revenue 0 selected
  where
    revenue !total (!extended, !rate) = total + extended * rate

    rows :: Relation (Id Lineitem) (Id Lineitem)
    rows = O.universe (lineitem_universe schema)
    shipped :: Relation (Id Lineitem) Int
    shipped = O.column (lineitem_shipdate schema)
    rates :: Relation (Id Lineitem) Double
    rates = O.column (lineitem_discount schema)
    quantities :: Relation (Id Lineitem) Double
    quantities = O.column (lineitem_quantity schema)
    prices :: Relation (Id Lineitem) Double
    prices = O.column (lineitem_extendedprice schema)

    eligible :: Relation (Id Lineitem) (Id Lineitem)
    eligible = restrict rows
      ( Q.range 19940101 19950101 shipped
        `prod` Q.between 0.05 0.07 rates
        `prod` Q.lt 24.0 quantities )

    selected :: Relation (Id Lineitem) (Double, Double)
    selected = compose eligible (prices `prod` rates)

{-# NOINLINE q6 #-}

-- | The same query with its two access modes selected explicitly.  Keeping
-- this beside 'q6' lets the benchmark tell us whether the rank-n 'Relation'
-- wrapper prevents GHC from seeing a fusion opportunity.
q6Concrete :: TPCHS -> Double
q6Concrete schema = Q.foldAll revenue 0 selected
  where
    revenue !total (!extended, !rate) = total + extended * rate

    rows :: Drive (Id Lineitem) (Id Lineitem)
    rows = O.universe (lineitem_universe schema)
    shipped :: Probe (Id Lineitem) Int
    shipped = O.column (lineitem_shipdate schema)
    rates :: Probe (Id Lineitem) Double
    rates = O.column (lineitem_discount schema)
    quantities :: Probe (Id Lineitem) Double
    quantities = O.column (lineitem_quantity schema)
    prices :: Probe (Id Lineitem) Double
    prices = O.column (lineitem_extendedprice schema)

    eligible :: Drive (Id Lineitem) (Id Lineitem)
    eligible = restrict rows
      ( Q.range 19940101 19950101 shipped
        `prod` Q.between 0.05 0.07 rates
        `prod` Q.lt 24.0 quantities )

    selected :: Drive (Id Lineitem) (Double, Double)
    selected = compose eligible (prices `prod` rates)

{-# NOINLINE q6Concrete #-}
