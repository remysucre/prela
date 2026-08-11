{-# LANGUAGE RankNTypes #-}

-- | A small, entirely pure database and query program using "Prela.Pull".
module Main where

import Prela.Pull

data Movie
data Keyword

data Demo = Demo
  { movieDomain   :: Universe Movie
  , keywordDomain :: Universe Keyword
  , firstMovie    :: Id Movie
  , thirdMovie    :: Id Movie
  , sequelKeyword :: Id Keyword
  , pixarKeyword  :: Id Keyword
  }

-- Identifier creation is checked against the universe it belongs to.  In real
-- code this is normally done by a loader; the demo makes the boundary visible.
demo :: Maybe Demo
demo = do
  movies <- denseUniverse 4
  keywords <- denseUniverse 3
  m0 <- lookupId movies 0
  m2 <- lookupId movies 2
  k0 <- lookupId keywords 0
  k2 <- lookupId keywords 2
  pure (Demo movies keywords m0 m2 k0 k2)

runDemo :: Demo -> IO ()
runDemo db = do
  putStrLn "all titles:"
  display allTitles
  putStrLn "after 1980:"
  display recentTitles
  putStrLn "after 1980, looked up for movie 2:"
  print (map snd (collect (at recentTitleOf (thirdMovie db))))
  putStrLn "predecessors (missing foreign keys are absent pairs):"
  display predecessors
  putStrLn "movie -> keyword label:"
  display movieKeyword
  putStrLn "movies without a rating:"
  display unrated
  putStrLn "live keyword rows (the sparse universe omits row 1):"
  display liveKeywordText
  where
    titles = ["Alien", "Aliens", "Up", "Toy Story"]
    years = [1979, 1986, 2009, 1995]
    ratings = [Just 9, Nothing, Just 8, Just 7]

    movie :: Mode q => q (Id Movie) (Id Movie)
    movie = universe (movieDomain db)

    title :: Mode q => q (Id Movie) String
    title = column titles

    year :: Mode q => q (Id Movie) Int
    year = column years

    rating :: Mode q => q (Id Movie) Int
    rating = sparseColumn ratings

    -- Movie 1 points to movie 0; the other rows have no predecessor.
    sequelOf :: Mode q => q (Id Movie) (Id Movie)
    sequelOf = sparseColumn
      [Nothing, Just (firstMovie db), Nothing, Nothing]

    keywordOf :: Mode q => q (Id Movie) (Id Keyword)
    keywordOf = multiColumn
      [ [], [sequelKeyword db], []
      , [pixarKeyword db, sequelKeyword db]
      ]

    keywordText :: Mode q => q (Id Keyword) String
    keywordText = column ["sequel", "space", "pixar"]

    liveKeywords :: Mode q => q (Id Keyword) (Id Keyword)
    liveKeywords = universe (universeFromMask [True, False, True])

    allTitles :: Stream (Id Movie) String
    allTitles = compose movie title

    recentTitles :: Stream (Id Movie) String
    recentTitles = compose (restrict movie (gt 1980 year)) title

    recentTitleOf :: Lookup (Id Movie) String
    recentTitleOf = compose (restrict movie (gt 1980 year)) title

    predecessors :: Stream (Id Movie) String
    predecessors = compose sequelOf title

    movieKeyword :: Stream (Id Movie) String
    movieKeyword = compose keywordOf keywordText

    unrated :: Stream (Id Movie) String
    unrated = compose (diff movie rating) title

    liveKeywordText :: Stream (Id Keyword) String
    liveKeywordText = compose liveKeywords keywordText

    display :: (Show d, Show r) => Stream d r -> IO ()
    display = mapM_ print . collect

main :: IO ()
main = maybe (fail "the fixed demo universes are invalid") runDemo demo
