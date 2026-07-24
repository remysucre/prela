-- | A tiny movie database and runnable queries, exercising the Prela core.
--
-- The point to notice: not one query below mentions a mode. `recentTitles` and
-- `recentTitleOf` are the same expression; their signatures are what make one
-- an enumeration and the other a lookup.
module Main where

import Prela
import qualified Data.Map.Strict as Map

--------------------------------------------------------------------------------
-- Entities
--------------------------------------------------------------------------------

-- Empty tag types: they carry no values and exist only to label ids so the
-- type checker can tell a movie key from a keyword key.
data Movie
data Keyword

--------------------------------------------------------------------------------
-- Stored data, built once. The leaves below are views of it.
--------------------------------------------------------------------------------

titleCol :: Col Movie String
titleCol = mkCol ["Alien", "Aliens", "Up", "Toy Story"]

yearCol :: Col Movie Int
yearCol = mkCol [1979, 1986, 2009, 1995]

franchiseCol :: Col Movie String
franchiseCol = mkCol ["Alien", "Alien", "Pixar", "Pixar"]

keywordTextCol :: Col Keyword String
keywordTextCol = mkCol ["sequel", "space", "pixar"]

--------------------------------------------------------------------------------
-- The schema: mode-polymorphic leaves
--------------------------------------------------------------------------------

movie :: Mode q => q (Id Movie) (Id Movie)
movie = universe 4

title :: Mode q => q (Id Movie) String
title = column titleCol

year :: Mode q => q (Id Movie) Int
year = column yearCol

franchise :: Mode q => q (Id Movie) String
franchise = column franchiseCol

keywordText :: Mode q => q (Id Keyword) String
keywordText = column keywordTextCol

-- A sparse foreign key: only movies 1 and 3 have a keyword. A missing keyword
-- is simply an absent pair — the SQL NULL.
keywordOf :: Mode q => q (Id Movie) (Id Keyword)
keywordOf = fromIndex (Map.fromList [(Id 1, [Id 0]), (Id 3, [Id 2])])

--------------------------------------------------------------------------------
-- Queries
--------------------------------------------------------------------------------

-- movie → title
allTitles :: Drv (Id Movie) String
allTitles = compose movie title

-- movie : (year > 1980) → title
recentTitles :: Drv (Id Movie) String
recentTitles = compose (restrict movie (gt 1980 year)) title

-- The same expression, probed. Only the signature differs.
recentTitleOf :: Prb (Id Movie) String
recentTitleOf = compose (restrict movie (gt 1980 year)) title

-- A named subquery left mode-free, so it can be used on either side below.
recent :: Mode q => q (Id Movie) (Id Movie)
recent = restrict movie (gt 1980 year)

-- movie → (title × year)
titleAndYear :: Drv (Id Movie) (String, Int)
titleAndYear = compose movie (prod title year)

-- Navigation: movie → keyword → Keyword.text. keywordOf's VALUE is Id Keyword,
-- which is exactly keywordText's KEY, so they compose. Wire it to the wrong
-- label column and it would not compile.
movieKeyword :: Drv (Id Movie) String
movieKeyword = compose keywordOf keywordText

-- ...the compile error, spelled out. `title` is keyed by Id Movie but keywordOf
-- hands out Id Keyword, so this does not type-check:
--
--   broken = compose keywordOf title

-- movie - keywordOf → title: movies with NO keyword. diff tests the KEY, so
-- keywordOf's value type is irrelevant here.
unkeyworded :: Drv (Id Movie) String
unkeyworded = compose (diff movie keywordOf) title

-- keywordOf' → title: invert the FK, probe by keyword id, get titles.
titlesForKeyword :: Prb (Id Keyword) String
titlesForKeyword = compose (inv keywordOf) title

-- Count movies per franchise: invert franchise, then fold each group with +1.
franchiseCounts :: Drv String Int
franchiseCounts = fold (\n _ -> n + 1) (0 :: Int) (invStream franchise)

-- Each movie's decade.
decades :: Drv (Id Movie) Int
decades = compose movie (mapv (\y -> y `div` 10 * 10) year)

-- Whole-column MIN, escaping to a scalar.
earliest :: Int
earliest = foldAll min maxBound (compose movie year)

main :: IO ()
main = do
  putStrLn "all titles:"
  drive allTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "after 1980:"
  drive recentTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "after 1980, probed for movie 2:"
  probe recentTitleOf (Id 2) $ \t -> putStrLn ("  " ++ t)
  putStrLn "is movie 0 recent? (member on the mode-free subquery)"
  print (member (recent :: Prb (Id Movie) (Id Movie)) (Id 0))
  putStrLn "title x year:"
  drive titleAndYear $ \m (t, y) -> putStrLn ("  " ++ show m ++ " -> " ++ t ++ " (" ++ show y ++ ")")
  putStrLn "movie -> keyword label:"
  drive movieKeyword $ \m kw -> putStrLn ("  " ++ show m ++ " -> " ++ kw)
  putStrLn "no keyword:"
  drive unkeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "titles for keyword 0 (sequel):"
  probe titlesForKeyword (Id 0) $ \t -> putStrLn ("  " ++ t)
  putStrLn "movies per franchise:"
  drive franchiseCounts $ \f n -> putStrLn ("  " ++ f ++ " -> " ++ show n)
  putStrLn "decades:"
  drive decades $ \m d -> putStrLn ("  " ++ show m ++ " -> " ++ show d)
  putStrLn ("earliest year: " ++ show earliest)
