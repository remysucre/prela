{-# LANGUAGE TypeFamilies #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE UndecidableInstances #-}

-- Option one: mirror Rust. Each operator is its own data type; the modes are
-- classes; the algebra's rules live in the instance contexts.
module ModesDeep where

import Control.Monad (when)

newtype Id e = Id Int deriving (Eq, Ord, Show)

--------------------------------------------------------------------------------
-- The classes. `D` and `R` are associated type families, standing in for Rust's
-- associated types on the `Query` trait.
--------------------------------------------------------------------------------

class Query q where
  type D q
  type R q

class Query q => Drive q where
  drive :: Monad m => q -> (D q -> R q -> m ()) -> m ()

class Query q => Probe q where
  probe    :: Monad m => q -> D q -> (R q -> m ()) -> m ()
  probeAny :: q -> D q -> (R q -> Bool) -> Bool

member :: Probe q => q -> D q -> Bool
member q x = probeAny q x (const True)

--------------------------------------------------------------------------------
-- Leaves
--------------------------------------------------------------------------------

newtype Universe e = Universe Int
instance Query (Universe e) where
  type D (Universe e) = Id e
  type R (Universe e) = Id e
instance Drive (Universe e) where
  drive (Universe n) k = mapM_ (\i -> k (Id i) (Id i)) [0 .. n - 1]
instance Probe (Universe e) where
  probe    (Universe n) x k = when (inRange n x) (k x)
  probeAny (Universe n) x p = inRange n x && p x

inRange :: Int -> Id e -> Bool
inRange n (Id i) = 0 <= i && i < n

newtype Column e r = Column [r]
instance Query (Column e r) where
  type D (Column e r) = Id e
  type R (Column e r) = r
instance Drive (Column e r) where
  drive (Column vs) k = mapM_ (\(i, v) -> k (Id i) v) (zip [0 ..] vs)
instance Probe (Column e r) where
  probe    (Column vs) (Id i) k = when (i < length vs) (k (vs !! i))
  probeAny (Column vs) (Id i) p = i < length vs && p (vs !! i)

--------------------------------------------------------------------------------
-- Operators. One data type each; the mode rules are the instance contexts,
-- which is exactly where Rust puts them.
--------------------------------------------------------------------------------

data Compose a b = Compose a b
-- Mirrors `impl<A: Query, B: Query<D = A::R>> Query for Compose<A, B>`.
instance (Query a, Query b, R a ~ D b) => Query (Compose a b) where
  type D (Compose a b) = D a
  type R (Compose a b) = R b
-- Driving a composition drives the left and probes the right.
instance (Drive a, Probe b, R a ~ D b) => Drive (Compose a b) where
  drive (Compose a b) k = drive a (\x y -> probe b y (\z -> k x z))
-- Probing one probes both.
instance (Probe a, Probe b, R a ~ D b) => Probe (Compose a b) where
  probe    (Compose a b) x k = probe    a x (\y -> probe    b y k)
  probeAny (Compose a b) x p = probeAny a x (\y -> probeAny b y p)

data Restrict a b = Restrict a b
instance (Query a, Query b, R a ~ D b) => Query (Restrict a b) where
  type D (Restrict a b) = D a
  type R (Restrict a b) = R a
instance (Drive a, Probe b, R a ~ D b) => Drive (Restrict a b) where
  drive (Restrict a b) k = drive a (\x y -> when (member b y) (k x y))
instance (Probe a, Probe b, R a ~ D b) => Probe (Restrict a b) where
  probe    (Restrict a b) x k = probe    a x (\y -> when (member b y) (k y))
  probeAny (Restrict a b) x p = probeAny a x (\y -> member b y && p y)

data Filt a = Filt (R a -> Bool) a
instance Query a => Query (Filt a) where
  type D (Filt a) = D a
  type R (Filt a) = R a
instance Drive a => Drive (Filt a) where
  drive (Filt t a) k = drive a (\x y -> when (t y) (k x y))
instance Probe a => Probe (Filt a) where
  probe    (Filt t a) x k = probe    a x (\y -> when (t y) (k y))
  probeAny (Filt t a) x p = probeAny a x (\y -> t y && p y)

data Diff a b = Diff a b
instance (Query a, Query b, D a ~ D b) => Query (Diff a b) where
  type D (Diff a b) = D a
  type R (Diff a b) = R a
instance (Drive a, Probe b, D a ~ D b) => Drive (Diff a b) where
  drive (Diff a b) k = drive a (\x v -> when (not (member b x)) (k x v))
instance (Probe a, Probe b, D a ~ D b) => Probe (Diff a b) where
  probe    (Diff a b) x k = when (not (member b x)) (probe a x k)
  probeAny (Diff a b) x p = not (member b x) && probeAny a x p

-- The payoff: InvStream has a Drive instance and NO Probe instance, so it
-- simply cannot appear where probing is required.
newtype InvStream a = InvStream a
instance Query a => Query (InvStream a) where
  type D (InvStream a) = R a
  type R (InvStream a) = D a
instance Drive a => Drive (InvStream a) where
  drive (InvStream a) k = drive a (\x y -> k y x)

--------------------------------------------------------------------------------
-- Smart constructors keep the use sites identical to the other design.
--------------------------------------------------------------------------------

compose :: a -> b -> Compose a b
compose = Compose

restrict :: a -> b -> Restrict a b
restrict = Restrict

diff :: a -> b -> Diff a b
diff = Diff

gt :: (Query a, Ord (R a)) => R a -> a -> Filt a
gt v = Filt (> v)

--------------------------------------------------------------------------------
-- A query. Note the TYPE: the plan is spelled out in it.
--------------------------------------------------------------------------------

data Movie
data Keyword

movie :: Universe Movie
movie = Universe 3

title :: Column Movie String
title = Column ["Alien", "Aliens", "Up"]

year :: Column Movie Int
year = Column [1979, 1986, 2009]

keywordOf :: Column Movie (Id Keyword)
keywordOf = Column [Id 0, Id 0, Id 1]

recentTitles :: Compose (Restrict (Universe Movie) (Filt (Column Movie Int)))
                        (Column Movie String)
recentTitles = compose (restrict movie (gt 1980 year)) title

unkeyworded :: Compose (Diff (Universe Movie) (Column Movie (Id Keyword)))
                       (Column Movie String)
unkeyworded = compose (diff movie keywordOf) title

-- REJECTED: no Probe instance for InvStream, so this has no Probe instance
-- either, and driving is the only thing you can do with it.
--   badProbe :: IO ()
--   badProbe = probe (compose (InvStream keywordOf) title) (Id 0) putStrLn

main :: IO ()
main = do
  drive recentTitles $ \m t -> putStrLn (show m ++ " -> " ++ t)
  drive unkeyworded  $ \m t -> putStrLn (show m ++ " -> " ++ t)
  drive (InvStream keywordOf) $ \k m -> putStrLn (show k ++ " <- " ++ show m)
