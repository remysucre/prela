-- | The `Mode` class, without staging.
--
-- Same shape as "Prela.PullStaged.Ops": one class, exactly two instances
-- (`Stream` and `Lookup`), and a query mentions no mode — the signature at the
-- top picks one. What is different is the leaves. The staged and base ports
-- have nine of them, one per physical column layout ("Prela.Storage"'s
-- `Col`, `SparseCol`, `MultiCol`, `Table`, `Dense`, `Bits`, plus two flavors
-- of `Map`); those layouts exist for speed, and speed is not this module's
-- job. So there is exactly one leaf, `fromPairs`, because that is the
-- definition of a relation: a set of pairs. `universe`, `column`,
-- `sparseColumn`, `multiColumn` below are ordinary functions built from it,
-- kept only because they read the way a query naturally does.
module Prela.Pull.Ops
  ( Mode (..)
  , universe
  , column
  , sparseColumn
  , multiColumn
  , groupBy
  , leftCompose
  , union
  , disj
  ) where

import Prela.Id
import Prela.Pull.Stream

class Mode q where
  -- | The one leaf. A relation is exactly the pairs it is built from.
  fromPairs :: Eq d => [(d, r)] -> q d r

  -- Chain two relations through a shared middle value: `r : d -> e` and
  -- `s : e -> f` give `d -> f`.
  compose  :: q d e -> Lookup e f -> q d f

  -- Pair two relations sharing a domain: for each key, take both values.
  prod     :: q d u -> Lookup d v -> q d (u, v)

  -- Keep each row whose value is a member of the second relation.
  restrict :: q d r -> Lookup r e -> q d r

  -- Keep each row whose key is absent from the second relation. This is
  -- SQL's IS NULL: a missing value is an absent pair.
  diff     :: q d r -> Lookup d e -> q d r

  -- Keep each row whose value passes a test.
  filt     :: (r -> Bool) -> q d r -> q d r

  -- Replace each value, leaving the key alone.
  mapv     :: (r -> s) -> q d r -> q d s

instance Mode Stream where
  fromPairs xs = Lin (pairProd xs)

  compose  a b = Bind a (\d e -> mapkS (const d) (at b e))
  prod     a b = Bind a (\d u -> mapkS (const d) (mapvS (\v -> (u, v)) (at b d)))
  restrict a b = filtKV (\_ r -> anyOf (at b r)) a
  diff     a b = filtKV (\d _ -> not (anyOf (at b d))) a
  filt     t a = filtS t a
  mapv     f a = mapvS f a

instance Mode Lookup where
  fromPairs xs = Lookup (\d -> Lin (listProd [ r | (d', r) <- xs, d' == d ]))

  compose  a b = Lookup (\x -> Bind (at a x) (\_ e -> at b e))
  prod     a b = Lookup (\x -> Bind (at a x) (\_ u -> mapvS (\v -> (u, v)) (at b x)))
  restrict a b = Lookup (\x -> filtS (\r -> anyOf (at b r)) (at a x))
  diff     a b = Lookup (\x -> whenS (not (anyOf (at b x))) (at a x))
  filt     t a = Lookup (\x -> filtS t (at a x))
  mapv     f a = Lookup (\x -> mapvS f (at a x))

--------------------------------------------------------------------------------
-- Leaves, as plain functions over `fromPairs`
--------------------------------------------------------------------------------

-- | The identity relation on the live identifiers in an entity universe.
universe :: Mode q => Universe e -> q (Id e) (Id e)
universe u = fromPairs [ (i, i) | i <- universeIds u ]

-- | A column: `vs !! i` is the value entity `Id i` holds.
column :: Mode q => [r] -> q (Id e) r
column vs = fromPairs (zip (denseIds (length vs)) vs)

-- | A column with holes: `Nothing` is an entity with no value at all, not a
-- value that happens to be absent.
sparseColumn :: Mode q => [Maybe r] -> q (Id e) r
sparseColumn vs = fromPairs [ (i, r) | (i, Just r) <- zip (denseIds (length vs)) vs ]

-- | A one-to-many column: entity `Id i` relates to every value in `vs !! i`.
multiColumn :: Mode q => [[r]] -> q (Id e) r
multiColumn vs = fromPairs [ (i, r) | (i, rs) <- zip (denseIds (length vs)) vs, r <- rs ]

denseIds :: Int -> [Id e]
denseIds n = maybe [] universeIds (denseUniverse n)

--------------------------------------------------------------------------------
-- Fixed-mode operators
--------------------------------------------------------------------------------

-- | Re-key each row of a stream by looking up a second relation at its value,
-- and drop the original key. SQL's GROUP BY.
groupBy :: Stream d r -> Lookup r k -> Stream k r
groupBy s key = Bind s (\_ x -> mapvS (const x) (byValue (at key x)))

-- | Re-key the first stream by its values, then look up the second relation at
-- each original key. This is left composition.
leftCompose :: Stream d e -> Lookup d f -> Stream e f
leftCompose a b = compose (invStream a) b

-- | Concatenate two streams. SQL's UNION ALL (Prela has no
-- deduplicating union — the algebra's `union` already means disjoint sum).
union :: Stream d r -> Stream d r -> Stream d r
union = catS

-- | OR over two membership tests: is `x` in `a` or `b`.
disj :: Lookup d u -> Lookup d v -> Lookup d ()
disj a b = Lookup (\x -> guardS (anyOf (at a x) || anyOf (at b x)))
