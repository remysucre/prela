-- | A tiny movie database and a runnable query, exercising the Prela core.
module Main where

import Prela
import Control.Monad (when)

--------------------------------------------------------------------------------
-- Entities
--------------------------------------------------------------------------------

-- Empty tag types: they carry no values, they exist only to label ids so the
-- type checker can tell a movie key from a keyword key.
data Movie
data Keyword

--------------------------------------------------------------------------------
-- Tiny movie database
--------------------------------------------------------------------------------

movie :: Rel (Id Movie) (Id Movie)     -- the universe: movies 0, 1, 2
movie = universe 3

title :: Rel (Id Movie) String
title = column ["Alien", "Aliens", "Up"]

year :: Rel (Id Movie) Int             -- value is a scalar, not an id
year = column [1979, 1986, 2009]

-- The Keyword entity is stored "shredded" into two relations, exactly the data
-- model in CLAUDE.md: a foreign key from movie to keyword, and the keyword's
-- own label column keyed by keyword id.
keywordText :: Rel (Id Keyword) String
keywordText = column ["sequel", "space", "pixar"]

-- Sparse FK: only movie 1 has a keyword (keyword 0, "sequel"). A missing
-- keyword is simply an absent pair — the SQL NULL. Hand-written because our
-- `column` leaf is dense.
keywordOf :: Rel (Id Movie) (Id Keyword)
keywordOf = Rel
  { drive    = \k   -> k (Id 1) (Id 0)
  , probe    = \x k -> when (x == Id 1) (k (Id 0))
  , probeAny = \x p -> x == Id 1 && p (Id 0)
  }

--------------------------------------------------------------------------------
-- Queries
--------------------------------------------------------------------------------

-- `movie → title`: every movie's title.
allTitles :: Rel (Id Movie) String
allTitles = compose movie title

-- `movie : (year > 1980) → title`: titles of movies after 1980.
recentTitles :: Rel (Id Movie) String
recentTitles = compose (restrict movie (gt 1980 year)) title

-- `movie → (title × year)`: each movie paired with both columns at once.
titleAndYear :: Rel (Id Movie) (String, Int)
titleAndYear = compose movie (prod title year)

-- Navigation: `movie → keyword → Keyword.text`. Hop the FK to the keyword id,
-- then that id to its label. This is field navigation = composition, and it is
-- the whole point of the entity layer: keywordOf's VALUE is `Id Keyword`, which
-- is exactly keywordText's KEY, so they compose. Wire it to the wrong label
-- column and the ids would not match and it would not compile.
movieKeyword :: Rel (Id Movie) String
movieKeyword = compose keywordOf keywordText

-- ...the compile error, spelled out. `title` is keyed by Id Movie, but
-- keywordOf hands out Id Keyword, so this does not type-check — uncomment to see
-- GHC reject it ("Couldn't match Id Keyword with Id Movie"):
--
--   broken = compose keywordOf title

-- `movie - keywordOf → title`: titles of movies with NO keyword. diff tests the
-- KEY, so keywordOf's value type (Id Keyword) is irrelevant here.
unkeyworded :: Rel (Id Movie) String
unkeyworded = compose (diff movie keywordOf) title

-- `keywordOf' → title`: invert the FK to go keyword → movie, probe by a keyword
-- id to get its movies, then their titles.
movizForKeyword :: Rel (Id Keyword) String
movizForKeyword = compose (inv keywordOf) title

-- each movie's franchise; Alien and Aliens share one.
franchise :: Rel (Id Movie) String
franchise = column ["Alien", "Alien", "Pixar"]

-- count movies per franchise: invert franchise (string -> movie), then fold
-- each group with +1. `franchise ▷ count`, essentially.
franchiseCounts :: Rel String Int
franchiseCounts = fold (\n _ -> n + 1) 0 (inv franchise)

-- `movie → (year ↦ decade)`: each movie's decade.
decades :: Rel (Id Movie) Int
decades = compose movie (mapv (\y -> y `div` 10 * 10) year)

main :: IO ()
main = do
  putStrLn "all titles:"
  drive allTitles    $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "after 1980:"
  drive recentTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "title × year:"
  drive titleAndYear $ \m (t, y) -> putStrLn ("  " ++ show m ++ " -> " ++ t ++ " (" ++ show y ++ ")")
  putStrLn "movie → keyword label:"
  drive movieKeyword $ \m kw -> putStrLn ("  " ++ show m ++ " -> " ++ kw)
  putStrLn "no keyword:"
  drive unkeyworded  $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "titles for keyword 0 (sequel):"
  probe movizForKeyword (Id 0) $ \t -> putStrLn ("  " ++ t)
  putStrLn "movies per franchise:"
  drive franchiseCounts $ \f n -> putStrLn ("  " ++ f ++ " -> " ++ show n)
  putStrLn "decades:"
  drive decades $ \m d -> putStrLn ("  " ++ show m ++ " -> " ++ show d)
  putStrLn ("earliest year: " ++ show (foldAll min maxBound (compose movie year)))
