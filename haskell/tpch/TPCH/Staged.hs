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
-- gets hoisted is a column read. Q1 performs keyed access to its four value
-- columns inside the scope of the group key it just computed. None of those
-- reads mention the key, so GHC lifted all four to the top of the row body as
-- thunks — four allocations a row, plus their bounds tests, plus everything
-- downstream that then had to stay boxed. It measured 4.45 GB for one Q1; with
-- the flag it is 0.64 GB.
--
-- Bang patterns do not help. A strict binding is a `case`, and the float is
-- allowed to rewrite a `case` into a `let` precisely when it crosses a binder.
-- The flag is the fix, and it is the same one @vector@ recommends for fused
-- pipelines.
--
-- @OverloadedStrings@ is here for a related reason. The façade emits string
-- literals into this module, where they still need to mean @ByteString@.
--
-- @FlexibleContexts@ is the same story one step further out. `Q.bitset` emits a
-- loop whose inferred type mentions @MArray (STUArray s) Bool m@, and the
-- inference happens at the SPLICE site, so the extension that permits such a
-- constraint has to be on here rather than where the quote was written.
--
-- This module is the explicit boundary: each splice produces a function that
-- computes typed rows, then an ordinary @renderN@ function sorts, limits, and
-- formats them. Query definitions themselves contain no quotation syntax.
--
-- `Q.compile` supplies the schema argument inside generated code. Query authors
-- therefore write an ordinary @Q.query $ \s -> ...@ builder; the public surface
-- keeps the binder-introduction trick inside the compiler.
module TPCH.Staged (queries) where

import qualified Prela.PullStaged.Query as Q

import TPCH.StagedQueries
import TPCH.StagedSchema (TPCHS)

-- | Every compiled TPC-H query paired with its number and result renderer.
queries :: [(String, TPCHS -> String)]
queries =
  [ ("1",  render1  . $$(Q.compile q1))
  , ("2",  render2  . $$(Q.compile q2))
  , ("3",  render3  . $$(Q.compile q3))
  , ("4",  render4  . $$(Q.compile q4))
  , ("5",  render5  . $$(Q.compile q5))
  , ("6",  render6  . $$(Q.compile q6))
  , ("7",  render7  . $$(Q.compile q7))
  , ("8",  render8  . $$(Q.compile q8))
  , ("9",  render9  . $$(Q.compile q9))
  , ("10", render10 . $$(Q.compile q10))
  , ("11", render11 . $$(Q.compile q11))
  , ("12", render12 . $$(Q.compile q12))
  , ("13", render13 . $$(Q.compile q13))
  , ("14", render14 . $$(Q.compile q14))
  , ("15", render15 . $$(Q.compile q15))
  , ("16", render16 . $$(Q.compile q16))
  , ("17", render17 . $$(Q.compile q17))
  , ("18", render18 . $$(Q.compile q18))
  , ("19", render19 . $$(Q.compile q19))
  , ("20", render20 . $$(Q.compile q20))
  , ("21", render21 . $$(Q.compile q21))
  , ("22", render22 . $$(Q.compile q22))
  ]
