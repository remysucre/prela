{-# LANGUAGE FlexibleContexts #-}

-- | Comparison and regex sugar.
--
-- None of these is a new node: each is `filt` from "Prela.Ops" with a fixed
-- closure, matching Rust, where `.eq(v)` builds `Filter { p: |x| x == v }`. They
-- get a module of their own because they are the surface syntax rather than the
-- machinery — adding a predicate should not mean touching the executor.
--
-- The value being tested is on the left of the comparison and the argument `v` on
-- the right, as in the Prela surface `year > 1980`.
module Prela.Predicate where

import Text.Regex.TDFA (Regex, RegexLike, makeRegex, matchTest)

import Prela.Ops

eq, ne :: (Mode q, Eq r) => r -> q d r -> q d r
eq v = filt (== v)
ne v = filt (/= v)
{-# INLINE eq #-}
{-# INLINE ne #-}

gt, lt, ge, le :: (Mode q, Ord r) => r -> q d r -> q d r
gt v = filt (> v)
lt v = filt (< v)
ge v = filt (>= v)
le v = filt (<= v)
{-# INLINE gt #-}
{-# INLINE lt #-}
{-# INLINE ge #-}
{-# INLINE le #-}

isIn :: (Mode q, Eq r) => [r] -> q d r -> q d r
isIn vs = filt (`elem` vs)
{-# INLINE isIn #-}

-- Ranges. `between` is closed at both ends, which is Julia's `lo..hi` and what a
-- SQL BETWEEN means; `range` is half-open, which is what a date bound written as
-- `>= '1994-01-01' AND < '1995-01-01'` wants. Both are here rather than left to
-- two `filt`s because writing them as `ge lo . le hi` builds two nodes and reads
-- worse at the use site.
between, range :: (Mode q, Ord r) => r -> r -> q d r -> q d r
between lo hi = filt (\x -> x >= lo && x <= hi)
range   lo hi = filt (\x -> x >= lo && x <  hi)
{-# INLINE between #-}
{-# INLINE range #-}

-- Regex match and its negation (`~` and `≁`). Also just `filt`, but note the
-- absence of an INLINE pragma, which is deliberate: the compiled regex is bound
-- in the `where` so it is built once and shared by every row. Inlining these
-- would copy `makeRegex` into each use site, which is harmless, but there is no
-- loop to fuse into here — the cost is inside `matchTest` either way.
rx, nrx :: (Mode q, RegexLike Regex s) => String -> q d s -> q d s
rx  re = filt (matchTest r)        where r = makeRegex re :: Regex
nrx re = filt (not . matchTest r)  where r = makeRegex re :: Regex
