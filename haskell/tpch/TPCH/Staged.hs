{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE TemplateHaskell #-}
{-# OPTIONS_GHC -fno-full-laziness #-}

-- | The staged queries as ordinary functions.
--
-- The @-fno-full-laziness@ at the top is not incidental, and every module that
-- splices a query needs it. Full laziness hoists a binding out of the smallest
-- scope that mentions it, and hoisting past a binder means the binding can no
-- longer be a `case`, so it becomes a thunk. In a generated query the thing that
-- gets hoisted is a column read. Q1 probes its four value columns inside the
-- scope of the group key it just computed, none of those reads mention the key,
-- so GHC lifted all four to the top of the row body as thunks — four allocations
-- a row, plus their bounds tests, plus everything downstream that then had to
-- stay boxed. It measured 4.45 GB for one Q1; with the flag it is 0.64 GB.
--
-- Bang patterns do not help. A strict binding is a `case`, and the float is
-- allowed to rewrite a `case` into a `let` precisely when it crosses a binder.
-- The flag is the fix, and it is the same one `vector` recommends for fused
-- pipelines.
--
-- @OverloadedStrings@ is here for a related reason. A literal inside a quote is
-- spliced back as a literal, so @[|| BS.isInfixOf "green" $$v ||]@ in Q9 lands
-- here still needing to mean a `ByteString`. Extensions that change what a piece
-- of syntax MEANS have to hold at both ends.
--
-- @FlexibleContexts@ is the same story one step further out. `withBits` emits a
-- loop whose inferred type mentions @MArray (STUArray s) Bool m@, and the
-- inference happens at the SPLICE site, so the extension that permits such a
-- constraint has to be on here rather than where the quote was written.
--
-- Nothing but splices lives here, and that is forced rather than chosen: a
-- splice cannot name a binding of the module it sits in, so the generators and
-- the formatting they emit calls to have to be somewhere else. That somewhere is
-- "TPCH.StagedQueries".
--
-- `lam1` is what supplies the schema argument. Writing @f s = $$(q6 [|| s ||])@
-- would be the natural thing and does not compile, because @s@ is a runtime
-- binder and the splice runs before it exists; `lam1` has the GENERATOR
-- introduce the binder inside the quote instead.
module TPCH.Staged (queries) where

import Prela.Staged.Stream (lam1)

import TPCH.StagedQueries
import TPCH.StagedSchema (TPCHS)

queries :: [(String, TPCHS -> String)]
queries =
  [ ("1",  $$(lam1 q1))
  , ("2",  $$(lam1 q2))
  , ("3",  $$(lam1 q3))
  , ("4",  $$(lam1 q4))
  , ("5",  $$(lam1 q5))
  , ("6",  $$(lam1 q6))
  , ("7",  $$(lam1 q7))
  , ("8",  $$(lam1 q8))
  , ("9",  $$(lam1 q9))
  , ("10", $$(lam1 q10))
  , ("11", $$(lam1 q11))
  , ("12", $$(lam1 q12))
  , ("13", $$(lam1 q13))
  , ("14", $$(lam1 q14))
  , ("15", $$(lam1 q15))
  , ("16", $$(lam1 q16))
  , ("17", $$(lam1 q17))
  , ("18", $$(lam1 q18))
  , ("19", $$(lam1 q19))
  , ("20", $$(lam1 q20))
  , ("21", $$(lam1 q21))
  , ("22", $$(lam1 q22))
  ]
