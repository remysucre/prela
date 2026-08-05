-- | The same tiny movie database as "Demo.hs", run against "Prela.Pull"
-- instead of "Prela": plain lists in place of `Col`/`SparseCol`/`MultiCol`,
-- `String` in place of `ByteString`, and no cache or schema to load. Every
-- query below is copied unchanged from "Demo.hs" — this file exists to show
-- that de-staging and de-storaging the port did not change what a query
-- means, only what it costs to run.
module Main where

import Prela.Pull

--------------------------------------------------------------------------------
-- Entities
--------------------------------------------------------------------------------

data Movie
data Keyword

--------------------------------------------------------------------------------
-- Stored data, built once. The leaves below are views of it.
--------------------------------------------------------------------------------

titleCol :: [String]
titleCol = ["Alien", "Aliens", "Up", "Toy Story"]

yearCol :: [Int]
yearCol = [1979, 1986, 2009, 1995]

franchiseCol :: [String]
franchiseCol = ["Alien", "Alien", "Pixar", "Pixar"]

-- A column with a hole: movie 1 has no rating.
ratingCol :: [Maybe Int]
ratingCol = [Just 9, Nothing, Just 8, Just 7]

-- A foreign key with holes: `noId` is out of range for any table, so it
-- fails the bounds check that every probe already does.
sequelOfCol :: [Id Movie]
sequelOfCol = [noId, Id 0, noId, noId]

-- A multi-valued column: movie 3 has two keywords.
keywordOfCol :: [[Id Keyword]]
keywordOfCol = [[], [Id 0], [], [Id 2, Id 0]]

keywordTextCol :: [String]
keywordTextCol = ["sequel", "space", "pixar"]

--------------------------------------------------------------------------------
-- The schema: mode-polymorphic leaves
--------------------------------------------------------------------------------

movie :: Mode q => q (Id Movie) (Id Movie)
movie = universe 4

-- A universe with gaps, as if row 1 had been deleted: driving it skips the
-- hole, so nothing downstream ever sees a dead id.
liveKeyword :: Mode q => q (Id Keyword) (Id Keyword)
liveKeyword = sparseUniverse [Id 0, Id 2]

title :: Mode q => q (Id Movie) String
title = column titleCol

year :: Mode q => q (Id Movie) Int
year = column yearCol

franchise :: Mode q => q (Id Movie) String
franchise = column franchiseCol

rating :: Mode q => q (Id Movie) Int
rating = sparseColumn ratingCol

sequelOf :: Mode q => q (Id Movie) (Id Movie)
sequelOf = column sequelOfCol

keywordText :: Mode q => q (Id Keyword) String
keywordText = column keywordTextCol

-- A sparse, multi-valued foreign key: only movies 1 and 3 have keywords. A
-- missing keyword is simply an absent pair — the SQL NULL.
keywordOf :: Mode q => q (Id Movie) (Id Keyword)
keywordOf = multiColumn keywordOfCol

--------------------------------------------------------------------------------
-- Queries
--------------------------------------------------------------------------------

-- movie → title
allTitles :: Drive (Id Movie) String
allTitles = compose movie title

-- movie : (year > 1980) → title
recentTitles :: Drive (Id Movie) String
recentTitles = compose (restrict movie (gt 1980 year)) title

-- The same expression, probed. Only the signature differs.
recentTitleOf :: Probe (Id Movie) String
recentTitleOf = compose (restrict movie (gt 1980 year)) title

-- A named subquery left mode-free, so it can be used on either side below.
recent :: Mode q => q (Id Movie) (Id Movie)
recent = restrict movie (gt 1980 year)

-- movie → (title × year)
titleAndYear :: Drive (Id Movie) (String, Int)
titleAndYear = compose movie (prod title year)

-- movie → rating: three rows, not four.
ratings :: Drive (Id Movie) Int
ratings = compose movie rating

-- movie - rating → title: the IS NULL query, as a set difference.
unrated :: Drive (Id Movie) String
unrated = compose (diff movie rating) title

-- sequelOf → title: each movie's predecessor. Only movie 1 has one; the
-- other three hold `noId` and probe to nothing.
predecessors :: Drive (Id Movie) String
predecessors = compose sequelOf title

-- Navigation: movie → keyword → Keyword.text.
movieKeyword :: Drive (Id Movie) String
movieKeyword = compose keywordOf keywordText

-- movie - keywordOf → title: movies with NO keyword.
unkeyworded :: Drive (Id Movie) String
unkeyworded = compose (diff movie keywordOf) title

-- keywordOf' → title: invert the FK, probe by keyword id, get titles.
titlesForKeyword :: Probe (Id Keyword) String
titlesForKeyword = compose (inv keywordOf) title

-- Count movies per franchise: invert franchise, then fold each group with +1.
franchiseCounts :: Drive String Int
franchiseCounts = fold (\n _ -> n + 1) (0 :: Int) (invStream franchise)

-- Each movie's decade.
decades :: Drive (Id Movie) Int
decades = compose movie (mapv (\y -> y `div` 10 * 10) year)

-- Whole-column MIN, escaping to a scalar.
earliest :: Int
earliest = foldAll min maxBound (compose movie year)

-- title ~ "^Alien" and its negation.
alienTitles, nonAlienTitles :: Drive (Id Movie) String
alienTitles    = rx  "^Alien" (compose movie title)
nonAlienTitles = nrx "^Alien" (compose movie title)

-- movie : ((year > 1980) ∨ keywordOf) → title.
recentOrKeyworded :: Drive (Id Movie) String
recentOrKeyworded = compose (restrict movie (disj (gt 1980 year) keywordOf)) title

-- Two driven relations concatenated. Movie 2 appears via both legs, because
-- union does not de-duplicate.
recentOrUnkeyworded :: Drive (Id Movie) String
recentOrUnkeyworded = union recentTitles unkeyworded

-- franchise ← year: rekey by franchise, then pick up each movie's year.
franchiseYears :: Drive String Int
franchiseYears = leftCompose franchise year

-- Movies per keyword, cached in a 3-slot array rather than a Map. Keyword 1
-- ("space") is attached to nothing, so it does not appear.
keywordCounts :: Drive (Id Keyword) Int
keywordCounts = foldDense 3 (\n _ -> n + 1) (0 :: Int) (invStream keywordOf)

-- The same fold as a left-outer aggregate: keyword 1 now emits its seed, 0.
keywordCountsOuter :: Drive (Id Keyword) Int
keywordCountsOuter = foldDenseOuter 3 (\n _ -> n + 1) (0 :: Int) (invStream keywordOf)

-- bitset(keywordOf, 3) → Keyword.keyword: which keywords are actually used.
usedKeywords :: Drive (Id Keyword) String
usedKeywords = compose (bitset 3 keywordOf) keywordText

-- liveKeyword → Keyword.keyword: the surviving rows of a table with holes.
liveKeywordText :: Drive (Id Keyword) String
liveKeywordText = compose liveKeyword keywordText

--------------------------------------------------------------------------------
-- Running queries
--------------------------------------------------------------------------------

-- `drive`/`probe`/`member` from "Prela.Push.Mode", rebuilt on `collect`/`at`/`anyOf`
-- since a pull stream is consumed by a fold, not a callback. Local to this
-- demo, not part of "Prela.Pull"'s API.
driveEach :: Drive d r -> (d -> r -> IO ()) -> IO ()
driveEach s f = mapM_ (uncurry f) (collect s)

probeEach :: Probe d r -> d -> (r -> IO ()) -> IO ()
probeEach p k f = mapM_ (f . snd) (collect (at p k))

member :: Probe d r -> d -> Bool
member p k = anyOf (at p k)

main :: IO ()
main = do
  putStrLn "all titles:"
  driveEach allTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "after 1980:"
  driveEach recentTitles $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "after 1980, probed for movie 2:"
  probeEach recentTitleOf (Id 2) $ \t -> putStrLn ("  " ++ t)
  putStrLn "is movie 0 recent? (member on the mode-free subquery)"
  print (member (recent :: Probe (Id Movie) (Id Movie)) (Id 0))
  putStrLn "title x year:"
  driveEach titleAndYear $ \m (t, y) -> putStrLn ("  " ++ show m ++ " -> " ++ t ++ " (" ++ show y ++ ")")
  putStrLn "ratings (sparse column, movie 1 has none):"
  driveEach ratings $ \m r -> putStrLn ("  " ++ show m ++ " -> " ++ show r)
  putStrLn "unrated (movie - rating):"
  driveEach unrated $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "predecessors (noId holes in an FK column):"
  driveEach predecessors $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "movie -> keyword label (multi-valued):"
  driveEach movieKeyword $ \m kw -> putStrLn ("  " ++ show m ++ " -> " ++ kw)
  putStrLn "no keyword:"
  driveEach unkeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "titles for keyword 0 (sequel):"
  probeEach titlesForKeyword (Id 0) $ \t -> putStrLn ("  " ++ t)
  putStrLn "movies per franchise:"
  driveEach franchiseCounts $ \f n -> putStrLn ("  " ++ f ++ " -> " ++ show n)
  putStrLn "decades:"
  driveEach decades $ \m d -> putStrLn ("  " ++ show m ++ " -> " ++ show d)
  putStrLn ("earliest year: " ++ show earliest)
  putStrLn "title ~ \"^Alien\":"
  driveEach alienTitles $ \_ t -> putStrLn ("  " ++ t)
  putStrLn "title !~ \"^Alien\":"
  driveEach nonAlienTitles $ \_ t -> putStrLn ("  " ++ t)
  putStrLn "recent OR keyworded:"
  driveEach recentOrKeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "recent UNION unkeyworded (no dedup):"
  driveEach recentOrUnkeyworded $ \m t -> putStrLn ("  " ++ show m ++ " -> " ++ t)
  putStrLn "franchise <- year:"
  driveEach franchiseYears $ \f y -> putStrLn ("  " ++ f ++ " -> " ++ show y)
  putStrLn "movies per keyword (dense fold):"
  driveEach keywordCounts $ \kw n -> putStrLn ("  " ++ show kw ++ " -> " ++ show n)
  putStrLn "movies per keyword (dense fold, outer):"
  driveEach keywordCountsOuter $ \kw n -> putStrLn ("  " ++ show kw ++ " -> " ++ show n)
  putStrLn "keywords actually used (bitset):"
  driveEach usedKeywords $ \kw t -> putStrLn ("  " ++ show kw ++ " -> " ++ t)
  putStrLn "surviving keyword rows (sparse universe):"
  driveEach liveKeywordText $ \kw t -> putStrLn ("  " ++ show kw ++ " -> " ++ t)
