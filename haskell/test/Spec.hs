{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}
{-# OPTIONS_GHC -fno-full-laziness #-}

-- | Integration tests for identifiers, cache I/O, generated schemas, and the
-- staged pull executor.
--
-- The suite compares checked and trusted cache loaders, then runs compact
-- generated queries against both representations.  Its fixtures also pin the
-- behavior of sparse universes, malformed references, shared materializers,
-- bounded top-k, and ordered substring scans.
module Main where

import Control.Exception (IOException, try)
import Control.Monad (unless)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Either (isLeft)
import qualified Data.Vector as BV
import Data.IORef (modifyIORef', newIORef, readIORef)
import Data.Maybe (isNothing)
import System.Directory (createDirectoryIfMissing, getTemporaryDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>))

import Prela.Cache
import Prela.Id
import qualified Prela.PullStaged.Query as Q
import Prela.Storage
import StagedQueries
  ( dictionaryQuery, loadedReferenceQuery, referenceQuery, schemaQuery
  , orderedInfixQuery, sharedRelationQuery, topKQuery, tupleHelpersQuery )
import TinyStaged
  ( Kind, Movie, RefFixture, RefSource, RefTarget, TinyS, link_universe, loadTinyS
  , loadTinySChecked, refFixture )

-- | Phantom entity tag used by storage-only fixtures.
data E

-- | Compile the composite tiny-schema query once for the test executable.
stagedQueryProduct
  :: TinyS -> (((([ByteString], Int), [ByteString]), [ByteString]), Double)
stagedQueryProduct = $$(Q.compile schemaQuery)

-- | Flatten the generated query's nested product into assertion-friendly form.
stagedQueries :: TinyS -> ([ByteString], Int, [ByteString], [ByteString], Double)
stagedQueries schema =
  let ((((sequels, recent), undated), television), best) =
        stagedQueryProduct schema
  in (sequels, recent, undated, television, best)

-- | Run compact dictionary coding and dense distinct counting.
stagedDictionary :: TinyS -> (BV.Vector ByteString, [(Int, Int)])
stagedDictionary = $$(Q.compile dictionaryQuery)

-- | Run the bounded top-k test query.
stagedTopK
  :: TinyS -> ([(Id Movie, Double)], [(Id Movie, Double)])
stagedTopK = $$(Q.compile topKQuery)

-- | Run a query that drives and probes one materialized relation.
stagedSharedRelation
  :: TinyS -> ([(Id Movie, Int)], [(Id Movie, Int)])
stagedSharedRelation = $$(Q.compile sharedRelationQuery)

-- | Run ordered-substring cases covering both orders and empty needles.
stagedOrderedInfix :: TinyS -> ((Int, Int), (Int, Int))
stagedOrderedInfix = $$(Q.compile orderedInfixQuery)

-- | Run all generated tuple construction and elimination conveniences.
stagedTupleHelpers :: TinyS -> (((Int, Int), Int), Int)
stagedTupleHelpers = $$(Q.compile tupleHelpersQuery)

-- | Run driven and probed reads of an in-memory malformed reference column.
stagedReferences
  :: RefFixture
  -> ([(Id RefSource, Id RefTarget)], [(Id RefSource, Id RefTarget)])
stagedReferences = $$(Q.compile referenceQuery)

-- | Run driven and probed reads of a cache-loaded reference column.
stagedLoadedReferences
  :: TinyS -> ([(Id Movie, Id Kind)], [(Id Movie, Id Kind)])
stagedLoadedReferences = $$(Q.compile loadedReferenceQuery)

-- | Execute every check and fail the test process if any assertion differs.
main :: IO ()
main = do
  failures <- newIORef (0 :: Int)
  let check :: (Eq a, Show a) => String -> a -> a -> IO ()
      check label actual expected = unless (actual == expected) $ do
        putStrLn ("FAIL " ++ label ++ "\n  got  " ++ show actual
                  ++ "\n  want " ++ show expected)
        modifyIORef' failures (+ 1)

  testIds check
  testCacheAndStaging check

  count <- readIORef failures
  if count == 0
    then putStrLn "all checks ok"
    else putStrLn (show count ++ " failure(s)") >> exitFailure

-- | Check dense and sparse universe construction and identifier validation.
testIds
  :: (forall a. (Eq a, Show a) => String -> a -> a -> IO ())
  -> IO ()
testIds check = do
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

-- | Round-trip every cache shape and exercise the generated tiny schema.
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
  ints <- loadIntsChecked dir "ints" :: IO (Col E Int)
  intsFast <- loadInts dir "ints" :: IO (Col E Int)
  check "integer cache" (colValues ints) [7, 0, -3]
  check "fast integer cache" (colValues intsFast) (colValues ints)
  check "integer column rejects negative rows" (colAt ints (-1)) Nothing
  check "integer column rejects past-end rows" (colAt ints 3) Nothing

  writeDoubles dir "doubles" [1.5, -0.25]
  doubles <- loadDoublesChecked dir "doubles" :: IO (Col E Double)
  doublesFast <- loadDoubles dir "doubles" :: IO (Col E Double)
  check "double cache" (colValues doubles) [1.5, -0.25]
  check "fast double cache" (colValues doublesFast) (colValues doubles)

  target <- requireUniverse "cache target" 3
  t0 <- requireId "target 0" target 0
  t1 <- requireId "target 1" target 1
  t2 <- requireId "target 2" target 2
  writeIds dir "ids" [Nothing, Just t0, Nothing, Just t2]
  ids <- loadIdsChecked dir "ids" :: IO (SparseCol E Int)
  idsFast <- loadIds dir "ids" :: IO (SparseCol E Int)
  check "nullable id cache" (map (sparseAt ids) [0 .. 3])
    [Nothing, Just 0, Nothing, Just 2]
  check "fast nullable id cache" (map (sparseAt idsFast) [0 .. 3])
    (map (sparseAt ids) [0 .. 3])
  check "sparse column rejects negative rows" (sparseAt ids (-1)) Nothing
  check "sparse column rejects past-end rows" (sparseAt ids 4) Nothing

  writeStrs dir "strings" ["ab", "", "c"]
  strings <- loadStrsChecked dir "strings" :: IO (Col E ByteString)
  stringsFast <- loadStrs dir "strings" :: IO (Col E ByteString)
  check "string cache" (colValues strings) ["ab", "", "c"]
  check "fast string cache" (colValues stringsFast) (colValues strings)

  writeMultiInts dir "multi-ints" [[1, 2], [], [-3]]
  multiInts <- loadMultiIntsChecked dir "multi-ints" :: IO (MultiCol E Int)
  multiIntsFast <- loadMultiInts dir "multi-ints" :: IO (MultiCol E Int)
  check "fast multi-integer cache" (multiValues multiIntsFast)
    (multiValues multiInts)
  check "multi column rejects negative rows" (multiAt multiInts (-1)) Nothing
  check "multi column rejects past-end rows" (multiAt multiInts 3) Nothing

  writeMultiIds dir "multi-ids" [[t0, t2], [], [t1]]
  multiIds <- loadMultiIdsChecked dir "multi-ids" :: IO (MultiCol E Int)
  multiIdsFast <- loadMultiIds dir "multi-ids" :: IO (MultiCol E Int)
  check "fast multi-id cache" (multiValues multiIdsFast)
    (multiValues multiIds)

  writeMultiStrs dir "multi-strings" [["ab", ""], [], ["c"]]
  multiStrings <- loadMultiStrsChecked dir "multi-strings" :: IO (MultiCol E ByteString)
  multiStringsFast <- loadMultiStrs dir "multi-strings"
    :: IO (MultiCol E ByteString)
  check "fast multi-string cache" (multiValues multiStringsFast)
    (multiValues multiStrings)

  wrongKind <- try (loadIntsChecked dir "strings") :: IO (Either IOException (Col E Int))
  check "cache kind mismatch" (isLeft wrongKind) True
  BS.writeFile (dir </> "bad.bin") "not a cache"
  badMagic <- try (loadStrsChecked dir "bad") :: IO (Either IOException (Col E ByteString))
  check "cache magic mismatch" (isLeft badMagic) True
  badMagicFast <- try (loadStrs dir "bad")
    :: IO (Either IOException (Col E ByteString))
  check "fast cache magic mismatch"
    (isLeft badMagicFast) True

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

  schema <- loadTinySChecked schemaDir
  fastSchema <- loadTinyS schemaDir
  let expected = (["Aliens", "Alien 3"], 2, ["Solaris"], ["Solaris"], 8.5)
  check "staged schema queries" (stagedQueries schema) expected
  check "fast staged schema queries" (stagedQueries fastSchema) expected
  check "staged dictionary and dense distinct count"
    (stagedDictionary schema) (BV.fromList ["movie", "tv series"], [(0, 2)])
  check "staged bounded top-k"
    (let (best, none) = stagedTopK schema
     in (map (\(movieId, rating) -> (idIndex movieId, rating)) best, none))
    ([(0, 8.5), (1, 8.4)], [])
  check "one staged relation scans and probes"
    (let (scanned, probed) = stagedSharedRelation schema
         indices = map (\(movieId, year) -> (idIndex movieId, year))
     in (indices scanned, indices probed))
    ([(0, 1979), (1, 1986), (3, 1992)],
     [(0, 1979), (1, 1986), (3, 1992)])
  check "ordered substring scan"
    (stagedOrderedInfix schema) ((3, 0), (1, 3))
  check "staged tuple conveniences"
    (stagedTupleHelpers schema) (((6, 10), 15), 21)
  check "sparse schema universe" (map idIndex (universeIds (link_universe schema))) [0, 2]
  check "fast sparse schema universe"
    (map idIndex (universeIds (link_universe fastSchema))) [0, 2]
  check "dead schema id cannot be obtained" (lookupId (link_universe schema) 1) Nothing

  let (boxedDriven, boxedKeyed) = stagedReferences refFixture
      indices = map (\(sourceId, targetId) ->
                       (idIndex sourceId, idIndex targetId))
      expectedReferences = [(0, 0), (2, 2)]
  check "staged boxed reference scan/keyed parity and validation"
    (indices boxedDriven, indices boxedKeyed)
    (expectedReferences, expectedReferences)
  let expectedLoadedReferences = [(0, 0), (1, 0), (2, 1)]
  check "staged checked reference scan/keyed parity"
    (let (driven, keyed) = stagedLoadedReferences schema
     in (indices driven, indices keyed))
    (expectedLoadedReferences, expectedLoadedReferences)
  check "staged word-backed reference scan/keyed parity"
    (let (driven, keyed) = stagedLoadedReferences fastSchema
     in (indices driven, indices keyed))
    (expectedLoadedReferences, expectedLoadedReferences)

  -- Keep all target ids used above live so the test also pins their provenance.
  check "target ids retain their indices" (map idIndex [t0, t1, t2]) [0, 1, 2]

-- | Construct a dense fixture universe or fail with the supplied label.
requireUniverse :: String -> Int -> IO (Universe e)
requireUniverse label size =
  maybe (fail (label ++ ": invalid extent")) pure (denseUniverse size)

-- | Obtain a fixture identifier or fail with the supplied label.
requireId :: String -> Universe e -> Int -> IO (Id e)
requireId label domain index =
  maybe (fail (label ++ ": outside its universe")) pure (lookupId domain index)
