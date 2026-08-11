{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}
{-# OPTIONS_GHC -fno-full-laziness #-}

module Main where

import Control.Exception (IOException, try)
import Control.Monad (unless)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.IORef (modifyIORef', newIORef, readIORef)
import Data.Maybe (isNothing)
import System.Directory (createDirectoryIfMissing, getTemporaryDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>))

import Prela.Cache
import Prela.Id
import qualified Prela.Pull as P
import Prela.PullStaged.Stream (lam1)
import Prela.Storage
import StagedQueries (schemaQueriesS)
import TinyStaged (TinyS, link_universe, loadTinyS)

data E
data T

stagedQueries :: TinyS -> ([ByteString], Int, [ByteString], [ByteString], Double)
stagedQueries = $$(lam1 schemaQueriesS)

main :: IO ()
main = do
  failures <- newIORef (0 :: Int)
  let check :: (Eq a, Show a) => String -> a -> a -> IO ()
      check label actual expected = unless (actual == expected) $ do
        putStrLn ("FAIL " ++ label ++ "\n  got  " ++ show actual
                  ++ "\n  want " ++ show expected)
        modifyIORef' failures (+ 1)

  testIdsAndPull check
  testCacheAndStaging check

  count <- readIORef failures
  if count == 0
    then putStrLn "all checks ok"
    else putStrLn (show count ++ " failure(s)") >> exitFailure

testIdsAndPull
  :: (forall a. (Eq a, Show a) => String -> a -> a -> IO ())
  -> IO ()
testIdsAndPull check = do
  movies <- requireUniverse "movies" 4
  m0 <- requireId "movie 0" movies 0
  m1 <- requireId "movie 1" movies 1
  m2 <- requireId "movie 2" movies 2
  m3 <- requireId "movie 3" movies 3

  check "negative dense universe"
    (isNothing (denseUniverse (-1) :: Maybe (Universe E))) True
  check "out-of-range id" (lookupId movies 4) Nothing
  check "negative id" (lookupId movies (-1)) Nothing
  check "round-trip id index" (map idIndex (universeIds movies)) [0, 1, 2, 3]

  let live = universeFromMask [True, False, True, False]
  check "sparse universe enumeration" (universeIds live) [m0, m2]
  check "sparse universe rejects dead id" (lookupId live 1) Nothing
  check "sparse universe contains only live ids"
    (map (containsId live) [m0, m1, m2, m3]) [True, False, True, False]

  let movie :: P.Mode q => q (Id E) (Id E)
      movie = P.universe movies
      title :: P.Mode q => q (Id E) String
      title = P.column ["Alien", "Aliens", "Solaris", "Alien 3"]
      year :: P.Mode q => q (Id E) Int
      year = P.multiColumn [[1979], [1986], [], [1992]]
      predecessor :: P.Mode q => q (Id E) (Id E)
      predecessor = P.sparseColumn [Nothing, Just m0, Nothing, Nothing]
      recent :: P.Stream (Id E) String
      recent = P.compose (P.restrict movie (P.gt 1980 year)) title
      predecessors :: P.Stream (Id E) String
      predecessors = P.compose predecessor title
      sparseIdentity :: P.Lookup (Id E) (Id E)
      sparseIdentity = P.universe live

  check "plain pull query" (P.collect recent)
    [(m1, "Aliens"), (m3, "Alien 3")]
  check "missing foreign key is no pair" (P.collect predecessors)
    [(m1, "Alien")]
  check "sparse lookup agrees with enumeration"
    (map (P.anyOf . P.at sparseIdentity) [m0, m1, m2, m3])
    [True, False, True, False]

testCacheAndStaging
  :: (forall a. (Eq a, Show a) => String -> a -> a -> IO ())
  -> IO ()
testCacheAndStaging check = do
  tmp <- getTemporaryDirectory
  let dir = tmp </> "prela-safe-cache-test"
      schemaDir = tmp </> "prela-safe-schema-test"
  createDirectoryIfMissing True dir
  createDirectoryIfMissing True schemaDir

  writeInts dir "ints" [7, 0, -3]
  ints <- loadInts dir "ints" :: IO (Col E Int)
  check "integer cache" (columnValues ints) [7, 0, -3]

  writeDoubles dir "doubles" [1.5, -0.25]
  doubles <- loadDoubles dir "doubles" :: IO (Col E Double)
  check "double cache" (columnValues doubles) [1.5, -0.25]

  target <- requireUniverse "cache target" 3
  t0 <- requireId "target 0" target 0
  t1 <- requireId "target 1" target 1
  t2 <- requireId "target 2" target 2
  writeIds dir "ids" [Nothing, Just t0, Nothing, Just t2]
  ids <- loadIds dir "ids" :: IO (SparseCol E Int)
  check "nullable id cache" (map (sparseAt ids) [0 .. 3])
    [Nothing, Just 0, Nothing, Just 2]

  writeStrs dir "strings" ["ab", "", "c"]
  strings <- loadStrs dir "strings" :: IO (Col E ByteString)
  check "string cache" (columnValues strings) ["ab", "", "c"]

  wrongKind <- try (loadInts dir "strings") :: IO (Either IOException (Col E Int))
  check "cache kind mismatch" (either (const True) (const False) wrongKind) True
  BS.writeFile (dir </> "bad.bin") "not a cache"
  badMagic <- try (loadStrs dir "bad") :: IO (Either IOException (Col E ByteString))
  check "cache magic mismatch" (either (const True) (const False) badMagic) True

  -- The staged schema stores foreign indices as checked optional integers and
  -- resolves them through the target universe inside generated code.
  kindDomain <- requireUniverse "kinds" 2
  kind0 <- requireId "kind 0" kindDomain 0
  kind1 <- requireId "kind 1" kindDomain 1
  keywordDomain <- requireUniverse "keywords" 2
  keyword0 <- requireId "keyword 0" keywordDomain 0
  keyword1 <- requireId "keyword 1" keywordDomain 1
  movieDomain <- requireUniverse "schema movies" 4
  movie0 <- requireId "schema movie 0" movieDomain 0
  movie3 <- requireId "schema movie 3" movieDomain 3

  writeStrs      schemaDir "Movie_title"   ["Alien", "Aliens", "Solaris", "Alien 3"]
  writeIds       schemaDir "Movie_kind"    [Just kind0, Just kind0, Just kind1, Nothing]
  writeMultiInts schemaDir "Movie_year"    [[1979], [1986], [], [1992]]
  writeMultiIds  schemaDir "Movie_keyword" [[keyword1], [keyword0, keyword1], [], [keyword0]]
  writeDoubles   schemaDir "Movie_rating"  [8.5, 8.4, 8.0, 7.0]
  writeStrs      schemaDir "Keyword_text"  ["sequel", "alien"]
  writeStrs      schemaDir "Kind_text"     ["movie", "tv series"]
  writeIds       schemaDir "Link_about"    [Just movie0, Nothing, Just movie3, Nothing]
  writeStrs      schemaDir "Link_note"     ["first", "", "third", ""]

  schema <- loadTinyS schemaDir
  let expected = (["Aliens", "Alien 3"], 2, ["Solaris"], ["Solaris"], 8.5)
  check "staged schema queries" (stagedQueries schema) expected
  check "sparse schema universe" (map idIndex (universeIds (link_universe schema))) [0, 2]
  check "dead schema id cannot be obtained" (lookupId (link_universe schema) 1) Nothing

  -- Keep all target ids used above live so the test also pins their provenance.
  check "target ids retain their indices" (map idIndex [t0, t1, t2]) [0, 1, 2]

columnValues :: Elem a => Col e a -> [a]
columnValues (Col n values) = map (atStore values) [0 .. n - 1]

requireUniverse :: String -> Int -> IO (Universe e)
requireUniverse label size =
  maybe (fail (label ++ ": invalid extent")) pure (denseUniverse size)

requireId :: String -> Universe e -> Int -> IO (Id e)
requireId label domain index =
  maybe (fail (label ++ ": outside its universe")) pure (lookupId domain index)
