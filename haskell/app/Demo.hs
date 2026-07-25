-- | A tiny movie database and runnable queries, exercising the Prela core.
--
-- The point to notice: not one query below mentions a mode. `recentTitles` and
-- `recentTitleOf` are the same expression; their signatures are what make one
-- an enumeration and the other a lookup.
{-# LANGUAGE OverloadedStrings #-}
module Main where

import Prela
import Data.ByteString (ByteString)
import qualified Data.ByteString.Char8 as BC

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

-- Strings are stored as one packed buffer plus an offsets array, so a title is
-- a slice rather than a list of boxed characters. `Col` picks that layout from
-- the element type alone; the declaration below looks the same as `yearCol`.
titleCol :: Col Movie ByteString
titleCol = mkCol ["Alien", "Aliens", "Up", "Toy Story"]

yearCol :: Col Movie Int
yearCol = mkCol [1979, 1986, 2009, 1995]

franchiseCol :: Col Movie ByteString
franchiseCol = mkCol ["Alien", "Alien", "Pixar", "Pixar"]

-- A column with a hole: movie 1 has no rating. The fill value is only there to
-- keep the array total; the presence mask is what decides, so a missing rating
-- is not a rating of 0.
ratingCol :: SparseCol Movie Int
ratingCol = mkSparseCol 0 [Just 9, Nothing, Just 8, Just 7]

-- A foreign key with holes, which need no mask: `noId` is out of range for any
-- table, so it fails the bounds check that every probe already does.
sequelOfCol :: Col Movie (Id Movie)
sequelOfCol = mkCol [noId, Id 0, noId, noId]

-- A multi-valued column, stored as CSR: one offsets array plus one flat run of
-- values. Movie 3 has two keywords.
keywordOfCol :: MultiCol Movie (Id Keyword)
keywordOfCol = mkMultiCol 4 [(1, Id 0), (3, Id 2), (3, Id 0)]

keywordTextCol :: Col Keyword ByteString
keywordTextCol = mkCol ["sequel", "space", "pixar"]

--------------------------------------------------------------------------------
-- The schema: mode-polymorphic leaves
--------------------------------------------------------------------------------

movie :: Mode q => q (Id Movie) (Id Movie)
movie = universe 4

-- A universe with gaps, as if rows 1 had been deleted: driving it skips the
-- hole, so nothing downstream ever sees a dead id.
liveKeyword :: Mode q => q (Id Keyword) (Id Keyword)
liveKeyword = sparseUniverse (mkBits 3 [Id 0, Id 2]) 3

title :: Mode q => q (Id Movie) ByteString
title = column titleCol

year :: Mode q => q (Id Movie) Int
year = column yearCol

franchise :: Mode q => q (Id Movie) ByteString
franchise = column franchiseCol

rating :: Mode q => q (Id Movie) Int
rating = sparseColumn ratingCol

sequelOf :: Mode q => q (Id Movie) (Id Movie)
sequelOf = column sequelOfCol

keywordText :: Mode q => q (Id Keyword) ByteString
keywordText = column keywordTextCol

-- A sparse, multi-valued foreign key: only movies 1 and 3 have keywords. A
-- missing keyword is simply an absent pair — the SQL NULL.
keywordOf :: Mode q => q (Id Movie) (Id Keyword)
keywordOf = multiColumn keywordOfCol

--------------------------------------------------------------------------------
-- Queries
--------------------------------------------------------------------------------

-- movie → title
allTitles :: Drv (Id Movie) ByteString
allTitles = compose movie title

-- movie : (year > 1980) → title
recentTitles :: Drv (Id Movie) ByteString
recentTitles = compose (restrict movie (gt 1980 year)) title

-- The same expression, probed. Only the signature differs.
recentTitleOf :: Prb (Id Movie) ByteString
recentTitleOf = compose (restrict movie (gt 1980 year)) title

-- A named subquery left mode-free, so it can be used on either side below.
recent :: Mode q => q (Id Movie) (Id Movie)
recent = restrict movie (gt 1980 year)

-- movie → (title × year)
titleAndYear :: Drv (Id Movie) (ByteString, Int)
titleAndYear = compose movie (prod title year)

-- movie → rating: three rows, not four.
ratings :: Drv (Id Movie) Int
ratings = compose movie rating

-- movie - rating → title: the IS NULL query, as a set difference.
unrated :: Drv (Id Movie) ByteString
unrated = compose (diff movie rating) title

-- sequelOf → title: each movie's predecessor. Only movie 1 has one; the other
-- three hold `noId` and probe to nothing.
predecessors :: Drv (Id Movie) ByteString
predecessors = compose sequelOf title

-- Navigation: movie → keyword → Keyword.text. keywordOf's VALUE is Id Keyword,
-- which is exactly keywordText's KEY, so they compose. Wire it to the wrong
-- label column and it would not compile.
movieKeyword :: Drv (Id Movie) ByteString
movieKeyword = compose keywordOf keywordText

-- ...the compile error, spelled out. `title` is keyed by Id Movie but keywordOf
-- hands out Id Keyword, so this does not type-check:
--
--   broken = compose keywordOf title

-- movie - keywordOf → title: movies with NO keyword. diff tests the KEY, so
-- keywordOf's value type is irrelevant here.
unkeyworded :: Drv (Id Movie) ByteString
unkeyworded = compose (diff movie keywordOf) title

-- keywordOf' → title: invert the FK, probe by keyword id, get titles.
titlesForKeyword :: Prb (Id Keyword) ByteString
titlesForKeyword = compose (inv keywordOf) title

-- Count movies per franchise: invert franchise, then fold each group with +1.
franchiseCounts :: Drv ByteString Int
franchiseCounts = fold (\n _ -> n + 1) (0 :: Int) (invStream franchise)

-- Each movie's decade.
decades :: Drv (Id Movie) Int
decades = compose movie (mapv (\y -> y `div` 10 * 10) year)

-- Whole-column MIN, escaping to a scalar.
earliest :: Int
earliest = foldAll min maxBound (compose movie year)

-- title ~ "^Alien" and its negation.
alienTitles, nonAlienTitles :: Drv (Id Movie) ByteString
alienTitles    = rx  "^Alien" (compose movie title)
nonAlienTitles = nrx "^Alien" (compose movie title)

-- movie : ((year > 1980) ∨ keywordOf) → title. The two sides of the ∨ relate a
-- movie to different things — a year and a keyword id — which is fine, because
-- only membership is ever asked of it. Its value type is () and cannot be read.
recentOrKeyworded :: Drv (Id Movie) ByteString
recentOrKeyworded = compose (restrict movie (disj (gt 1980 year) keywordOf)) title

-- Two driven relations concatenated. Movie 2 appears via both legs, because
-- union does not de-duplicate.
recentOrUnkeyworded :: Drv (Id Movie) ByteString
recentOrUnkeyworded = union recentTitles unkeyworded

-- franchise ← year: rekey by franchise, then pick up each movie's year. Note
-- `franchise` lands in drive position and `year` in probe position, from the
-- signature of leftCompose alone.
franchiseYears :: Drv ByteString Int
franchiseYears = leftCompose franchise year

-- Movies per keyword, cached in a 3-slot array rather than a Map. Keyword 1
-- ("space") is attached to nothing, so it does not appear.
keywordCounts :: Drv (Id Keyword) Int
keywordCounts = foldDense 3 (\n _ -> n + 1) (0 :: Int) (invStream keywordOf)

-- The same fold as a left-outer aggregate: keyword 1 now emits its seed, 0.
keywordCountsOuter :: Drv (Id Keyword) Int
keywordCountsOuter = foldDenseOuter 3 (\n _ -> n + 1) (0 :: Int) (invStream keywordOf)

-- bitset(keywordOf, 3) → Keyword.keyword: which keywords are actually used.
-- The bitset sets a bit at each VALUE keywordOf emits, so it is the identity
-- relation on used keyword ids.
usedKeywords :: Drv (Id Keyword) ByteString
usedKeywords = compose (bitset 3 keywordOf) keywordText

-- liveKeyword → Keyword.keyword: the surviving rows of a table with holes.
liveKeywordText :: Drv (Id Keyword) ByteString
liveKeywordText = compose liveKeyword keywordText

main :: IO ()
main = do
  putStrLn "all titles:"
  drive allTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "after 1980:"
  drive recentTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "after 1980, probed for movie 2:"
  probe recentTitleOf (Id 2) $ \t -> putStrLn ("  " ++ str t)
  putStrLn "is movie 0 recent? (member on the mode-free subquery)"
  print (member (recent :: Prb (Id Movie) (Id Movie)) (Id 0))
  putStrLn "title x year:"
  drive titleAndYear $ \m (t, y) -> putStrLn ("  " ++ show m ++ " -> " ++ str t ++ " (" ++ show y ++ ")")
  putStrLn "ratings (sparse column, movie 1 has none):"
  drive ratings $ \m r -> putStrLn ("  " ++ show m ++ " -> " ++ show r)
  putStrLn "unrated (movie - rating):"
  drive unrated $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "predecessors (noId holes in an FK column):"
  drive predecessors $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "movie -> keyword label (multi-valued):"
  drive movieKeyword $ \m kw -> putStrLn ("  " ++ show m ++ " -> " ++ str kw)
  putStrLn "no keyword:"
  drive unkeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "titles for keyword 0 (sequel):"
  probe titlesForKeyword (Id 0) $ \t -> putStrLn ("  " ++ str t)
  putStrLn "movies per franchise:"
  drive franchiseCounts $ \f n -> putStrLn ("  " ++ str f ++ " -> " ++ show n)
  putStrLn "decades:"
  drive decades $ \m d -> putStrLn ("  " ++ show m ++ " -> " ++ show d)
  putStrLn ("earliest year: " ++ show earliest)
  putStrLn "title ~ \"^Alien\":"
  drive alienTitles $ \_ t -> putStrLn ("  " ++ str t)
  putStrLn "title !~ \"^Alien\":"
  drive nonAlienTitles $ \_ t -> putStrLn ("  " ++ str t)
  putStrLn "recent OR keyworded:"
  drive recentOrKeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "recent UNION unkeyworded (no dedup):"
  drive recentOrUnkeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ str t)
  putStrLn "franchise <- year:"
  drive franchiseYears $ \f y -> putStrLn ("  " ++ str f ++ " -> " ++ show y)
  putStrLn "movies per keyword (dense fold):"
  drive keywordCounts $ \kw n -> putStrLn ("  " ++ show kw ++ " -> " ++ show n)
  putStrLn "movies per keyword (dense fold, outer):"
  drive keywordCountsOuter $ \kw n -> putStrLn ("  " ++ show kw ++ " -> " ++ show n)
  putStrLn "keywords actually used (bitset):"
  drive usedKeywords $ \kw t -> putStrLn ("  " ++ show kw ++ " -> " ++ str t)
  putStrLn "surviving keyword rows (sparse universe):"
  drive liveKeywordText $ \kw t -> putStrLn ("  " ++ show kw ++ " -> " ++ str t)

str :: ByteString -> String
str = BC.unpack
