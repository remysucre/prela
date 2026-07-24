{-# LANGUAGE RankNTypes #-}

-- Third try: no coercion class. The mode is one class whose instances are the
-- two modes, and LEAVES are class methods too, so a leaf is polymorphic and
-- instantiates at whichever mode its position demands.
module ModesFinal where

import Control.Monad (when)
import Control.Monad.ST (runST)
import Data.STRef (newSTRef, modifySTRef', readSTRef)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

newtype Drv d r = Drv
  { drive :: forall m. Monad m => (d -> r -> m ()) -> m () }

data Prb d r = Prb
  { probe    :: forall m. Monad m => d -> (r -> m ()) -> m ()
  , probeAny :: d -> (r -> Bool) -> Bool
  }

member :: Prb d r -> d -> Bool
member q x = probeAny q x (const True)

newtype Id e = Id Int deriving (Eq, Ord, Show)

--------------------------------------------------------------------------------
-- One class. Two instances. Leaves and mode-polymorphic operators are methods;
-- the right-hand argument of a binary operator is always concretely `Prb`,
-- because that side is always probed regardless of the result's mode.
--------------------------------------------------------------------------------

class Mode q where
  universe  :: Int -> q (Id e) (Id e)
  column    :: [r] -> q (Id e) r
  fromIndex :: Ord d => Map d [r] -> q d r

  compose   :: q d e -> Prb e f -> q d f
  prod      :: q d u -> Prb d v -> q d (u, v)
  restrict  :: q d r -> Prb r e -> q d r
  diff      :: q d r -> Prb d e -> q d r
  filt      :: (r -> Bool) -> q d r -> q d r
  mapv      :: (r -> s) -> q d r -> q d s

instance Mode Drv where
  universe n  = Drv (\k -> mapM_ (\i -> k (Id i) (Id i)) [0 .. n - 1])
  column vals = Drv (\k -> mapM_ (\(i, v) -> k (Id i) v) (zip [0 ..] vals))
  fromIndex m = Drv (\k -> mapM_ (\(d, rs) -> mapM_ (k d) rs) (Map.toList m))

  compose  a b = Drv (\k -> drive a (\x y -> probe b y (\z -> k x z)))
  prod     a b = Drv (\k -> drive a (\x u -> probe b x (\v -> k x (u, v))))
  restrict a b = Drv (\k -> drive a (\x y -> when (member b y) (k x y)))
  diff     a b = Drv (\k -> drive a (\x v -> when (not (member b x)) (k x v)))
  filt   t a   = Drv (\k -> drive a (\x y -> when (t y) (k x y)))
  mapv   f a   = Drv (\k -> drive a (\x v -> k x (f v)))

instance Mode Prb where
  universe n = Prb { probe    = \x k -> when (inRange n x) (k x)
                   , probeAny = \x p -> inRange n x && p x }
  column vals = Prb { probe    = \(Id i) k -> when (i < length vals) (k (vals !! i))
                    , probeAny = \(Id i) p -> i < length vals && p (vals !! i) }
  fromIndex m = Prb { probe    = \x k -> maybe (return ()) (mapM_ k) (Map.lookup x m)
                    , probeAny = \x p -> maybe False (any p) (Map.lookup x m) }

  compose a b = Prb
    { probe    = \x k -> probe    a x (\y -> probe    b y k)
    , probeAny = \x p -> probeAny a x (\y -> probeAny b y p)
    }
  prod a b = Prb
    { probe    = \x k -> probe    a x (\u -> probe    b x (\v -> k (u, v)))
    , probeAny = \x p -> probeAny a x (\u -> probeAny b x (\v -> p (u, v)))
    }
  restrict a b = Prb
    { probe    = \x k -> probe    a x (\y -> when (member b y) (k y))
    , probeAny = \x p -> probeAny a x (\y -> member b y && p y)
    }
  diff a b = Prb
    { probe    = \x k -> when (not (member b x)) (probe a x k)
    , probeAny = \x p -> not (member b x) && probeAny a x p
    }
  filt t a = Prb
    { probe    = \x k -> probe    a x (\y -> when (t y) (k y))
    , probeAny = \x p -> probeAny a x (\y -> t y && p y)
    }
  mapv f a = Prb
    { probe    = \x k -> probe    a x (\v -> k (f v))
    , probeAny = \x p -> probeAny a x (\v -> p (f v))
    }

inRange :: Int -> Id e -> Bool
inRange n (Id i) = 0 <= i && i < n

gt :: (Mode q, Ord r) => r -> q d r -> q d r
gt v = filt (> v)

--------------------------------------------------------------------------------
-- Mode-fixed operators: exactly the ones whose mode is NOT free.
--------------------------------------------------------------------------------

-- Takes something driven, gives something driven. Cannot reach probe position.
invStream :: Drv d r -> Drv r d
invStream a = Drv (\k -> drive a (\x y -> k y x))

index :: Ord d => Drv d r -> Map d [r]
index q = runST $ do
  ref <- newSTRef Map.empty
  drive q (\d r -> modifySTRef' ref (Map.insertWith (++) d [r]))
  readSTRef ref

-- Consumes a driven relation, produces one usable in either mode.
materialize :: (Mode q, Ord d) => Drv d r -> q d r
materialize = fromIndex . index

inv :: (Mode q, Ord r) => Drv d r -> q r d
inv = fromIndex . index . invStream

--------------------------------------------------------------------------------
-- A schema and queries. Note that NOTHING below names a mode except the
-- top-level signatures, which is the point.
--------------------------------------------------------------------------------

data Movie
data Keyword

movie :: Mode q => q (Id Movie) (Id Movie)
movie = universe 3

title :: Mode q => q (Id Movie) String
title = column ["Alien", "Aliens", "Up"]

year :: Mode q => q (Id Movie) Int
year = column [1979, 1986, 2009]

keywordOf :: Mode q => q (Id Movie) (Id Keyword)
keywordOf = fromIndex (Map.fromList [(Id 1, [Id 0])])

-- movie : (year > 1980) → title, driven.
recentTitles :: Drv (Id Movie) String
recentTitles = compose (restrict movie (gt 1980 year)) title

-- The very same expression, probed. Only the signature differs.
recentTitleOf :: Prb (Id Movie) String
recentTitleOf = compose (restrict movie (gt 1980 year)) title

-- A named subquery, still mode-free, usable on either side below.
recent :: Mode q => q (Id Movie) (Id Movie)
recent = restrict movie (gt 1980 year)

unkeyworded :: Drv (Id Movie) String
unkeyworded = compose (diff movie keywordOf) title

titlesForKeyword :: Prb (Id Keyword) String
titlesForKeyword = compose (inv keywordOf) title

-- REJECTED: invStream is Drv-only, so this composition cannot be a Prb.
--   bad :: Prb (Id Keyword) String
--   bad = compose (invStream keywordOf) title

main :: IO ()
main = do
  drive recentTitles $ \m t -> putStrLn (show m ++ " -> " ++ t)
  drive unkeyworded  $ \m t -> putStrLn (show m ++ " -> " ++ t)
  drive (recent :: Drv (Id Movie) (Id Movie)) $ \m _ -> putStrLn (show m)
  probe recentTitleOf (Id 2) putStrLn
  probe titlesForKeyword (Id 0) putStrLn
