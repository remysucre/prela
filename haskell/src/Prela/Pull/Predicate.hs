{-# LANGUAGE FlexibleContexts #-}

-- | Comparison and regex sugar, unchanged in spirit from "Prela.Push.Predicate":
-- each of these is `filt` with a fixed closure, not a new kind of node.
module Prela.Pull.Predicate where

import Text.Regex.TDFA (Regex, RegexLike, makeRegex, matchTest)

import Prela.Pull.Ops

eq, ne :: (Mode q, Eq r) => r -> q d r -> q d r
eq v = filt (== v)
ne v = filt (/= v)

gt, lt, ge, le :: (Mode q, Ord r) => r -> q d r -> q d r
gt v = filt (> v)
lt v = filt (< v)
ge v = filt (>= v)
le v = filt (<= v)

isIn :: (Mode q, Eq r) => [r] -> q d r -> q d r
isIn vs = filt (`elem` vs)

-- `between` is closed at both ends (SQL's BETWEEN); `range` is half-open
-- (what a date bound written as `>= lo AND < hi` wants).
between, range :: (Mode q, Ord r) => r -> r -> q d r -> q d r
between lo hi = filt (\x -> x >= lo && x <= hi)
range   lo hi = filt (\x -> x >= lo && x <  hi)

rx, nrx :: (Mode q, RegexLike Regex s) => String -> q d s -> q d s
rx  re = filt (matchTest r)        where r = makeRegex re :: Regex
nrx re = filt (not . matchTest r)  where r = makeRegex re :: Regex
