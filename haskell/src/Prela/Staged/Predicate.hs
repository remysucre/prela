{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Comparison and regex sugar.
--
-- None of these is a new node: each is `filt` from "Prela.Staged.Ops" with a
-- fixed closure. They get a module of their own because they are the surface
-- syntax rather than the machinery — adding a predicate should not mean touching
-- the executor.
--
-- The value being tested is on the left of the comparison and the argument @v@
-- on the right, as in the Prela surface @year > 1980@.
--
-- The bound is a `CodeQ` rather than a plain value, so a query writes
-- @gt [|| 1980 ||] year@ and not @gt 1980 year@. That is the price of staging
-- and it is charged uniformly. The alternative is a `Lift` constraint and
-- @liftTyped@, which would read better at the use site but would not accept a
-- bound computed from the query's own arguments — and TPC-H's bounds are dates
-- and strings built in the query, so that restriction would bite.
module Prela.Staged.Predicate where

import Language.Haskell.TH (CodeQ)
import Text.Regex.TDFA (Regex, RegexLike, makeRegex, matchTest)

import Prela.Staged.Ops

eq, ne :: (SMode q, Eq r) => CodeQ r -> q d r -> q d r
eq v = filt (\x -> [|| $$x == $$v ||])
ne v = filt (\x -> [|| $$x /= $$v ||])

gt, lt, ge, le :: (SMode q, Ord r) => CodeQ r -> q d r -> q d r
gt v = filt (\x -> [|| $$x >  $$v ||])
lt v = filt (\x -> [|| $$x <  $$v ||])
ge v = filt (\x -> [|| $$x >= $$v ||])
le v = filt (\x -> [|| $$x <= $$v ||])

isIn :: (SMode q, Eq r) => CodeQ [r] -> q d r -> q d r
isIn vs = filt (\x -> [|| $$x `elem` $$vs ||])

-- Ranges. `between` is closed at both ends, which is what SQL's BETWEEN means;
-- `range` is half-open, which is what a date bound written as
-- @>= '1994-01-01' AND < '1995-01-01'@ wants. Both are here rather than left to
-- two `filt`s because writing them as @ge lo . le hi@ builds two nodes and reads
-- worse at the use site.
between, range :: (SMode q, Ord r) => CodeQ r -> CodeQ r -> q d r -> q d r
between lo hi = filt (\x -> [|| let !v = $$x in v >= $$lo && v <= $$hi ||])
range   lo hi = filt (\x -> [|| let !v = $$x in v >= $$lo && v <  $$hi ||])

-- | Compile a regex once, in the generated code, and hand the rest of the query
-- a reference to it.
--
-- This is the same continuation shape the materializers in
-- "Prela.Staged.Materialize" use, and for the same reason. Writing
-- @rx :: String -> q d s -> q d s@ and splicing @makeRegex re@ straight into the
-- per-row test would put the compile INSIDE the loop, since a `CodeQ` used once
-- per row is code that runs once per row. GHC used to rescue that by floating
-- the constant out, but the generated loops are compiled with
-- @-fno-full-laziness@, so nothing floats and the regex would be rebuilt per
-- row. Binding it here says what was meant.
withRegex :: String -> (CodeQ Regex -> CodeQ w) -> CodeQ w
withRegex re k = [|| let !r = makeRegex (re :: String) :: Regex in $$(k [|| r ||]) ||]

-- Regex match and its negation, over a regex `withRegex` already compiled.
rx, nrx :: (SMode q, RegexLike Regex s) => CodeQ Regex -> q d s -> q d s
rx  r = filt (\x -> [|| matchTest $$r $$x ||])
nrx r = filt (\x -> [|| not (matchTest $$r $$x) ||])
