-- | Two sets of tests.
--
-- First, round trips for the cache reader: write a file of each kind, map it
-- back, and check that driving the resulting column gives the pairs that went
-- in. These mirror the Rust port's cache tests, which is deliberate — the two
-- readers consume the same bytes, so a disagreement about the format is a bug in
-- one of them and the tests are the only place that shows up until there is a
-- real cache on disk to read.
--
-- Then the schema layer, using the declaration in TinySchema: write a cache
-- named the way the generated loader expects, load it, and ask questions through
-- the generated accessors. That covers the two things a generator can plausibly
-- get wrong, the filenames it reads and the sizes it gives the universes,
-- neither of which typechecking says anything about.
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ScopedTypeVariables #-}
module Main where

import Control.Exception (ErrorCall, evaluate, try)
import Control.Monad (unless)
import Data.ByteString (ByteString)
import Data.IORef (modifyIORef', newIORef, readIORef)
import System.Directory (createDirectoryIfMissing, getTemporaryDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>))

import Prela
import Prela.Cache
import qualified TinySchema as Sch

data E
data T

-- Collect every pair a driven relation emits, in order.
pairs :: Drv d r -> IO [(d, r)]
pairs q = do
  ref <- newIORef []
  drive q (\x y -> modifyIORef' ref ((x, y) :))
  reverse <$> readIORef ref

-- The same thing without IO, for the queries below. `foldAll` threads its
-- accumulator through the fused loop, so no ref is needed.
values :: Drv d r -> [r]
values q = reverse (foldAll (\acc y -> y : acc) [] q)

-- A stand-in for a query module, written the way Prela.Schema's doc recommends:
-- one function taking the record, and a `where` preamble that buys back the bare
-- spelling of each leaf. `Sch.title s` becomes `title`, which is what a query
-- against JOB will want when it mentions thirty of them.
--
-- The signatures on those bindings are load bearing rather than decorative. The
-- storage layer needs TypeFamilies, which turns on MonoLocalBinds, so without a
-- signature GHC pins each name to whichever mode it is first used at — and
-- `movie` below is driven while `year` is probed, in the same expression.
schemaQueries :: Sch.Tiny -> ([ByteString], Int, [ByteString], [ByteString], Double)
schemaQueries s = (sequels, recent, undated, tv, best)
  where
    movie :: Mode q => q (Id Sch.Movie) (Id Sch.Movie)
    movie = Sch.movie s
    title :: Mode q => q (Id Sch.Movie) ByteString
    title = Sch.title s
    year :: Mode q => q (Id Sch.Movie) Int
    year = Sch.year s
    rating :: Mode q => q (Id Sch.Movie) Double
    rating = Sch.rating s
    keyword :: Mode q => q (Id Sch.Movie) (Id Sch.Keyword)
    keyword = Sch.keyword s
    keywordText :: Mode q => q (Id Sch.Keyword) ByteString
    keywordText = Sch.keywordText s
    kind :: Mode q => q (Id Sch.Movie) (Id Sch.Kind)
    kind = Sch.kind s
    kindText :: Mode q => q (Id Sch.Kind) ByteString
    kindText = Sch.kindText s

    -- movie : (keyword → keywordText == "sequel") → title
    sequels = values (compose (restrict movie (eq "sequel" (compose keyword keywordText)))
                             title)

    -- movie : (year > 1980) → year  ⊵ count
    recent = foldAll (\n _ -> n + 1) (0 :: Int) (compose (restrict movie (gt 1980 year)) year)

    -- (movie - year) → title, which is SQL's `year IS NULL`: a movie with no
    -- year is a key the CSR column has no values for.
    undated = values (compose (diff movie year) title)

    -- the same navigation as `sequels` but through a 1:1 foreign key, so this is
    -- also the check that a hole in that column reaches nothing
    tv = values (compose (restrict movie (eq "tv series" (compose kind kindText))) title)

    best = foldAll max 0 (compose movie rating)

main :: IO ()
main = do
  tmp <- getTemporaryDirectory
  let dir = tmp </> "prela-cache-test"
  createDirectoryIfMissing True dir
  fails <- newIORef (0 :: Int)
  let check :: (Eq a, Show a) => String -> a -> a -> IO ()
      check what got want = unless (got == want) $ do
        putStrLn ("FAIL " ++ what ++ "\n  got  " ++ show got ++ "\n  want " ++ show want)
        modifyIORef' fails (+ 1)

  -- dense integers, negatives included
  writeInts dir "ints" [7, 0, -3]
  ints <- loadInts dir "ints" :: IO (Col E Int)
  got1 <- pairs (column ints)
  check "dense ints" got1 [(Id 0, 7), (Id 1, 0), (Id 2, -3)]

  -- dense doubles
  writeDoubles dir "dbls" [1.5, -0.25]
  dbls <- loadDoubles dir "dbls" :: IO (Col E Double)
  got2 <- pairs (column dbls)
  check "dense doubles" got2 [(Id 0, 1.5), (Id 1, -0.25)]

  -- dense ids: the holes must come back as noId, bit-for-bit, or a loaded FK
  -- column and a hand-built one would disagree about what a hole is
  writeIds dir "fk" [Nothing, Just (Id 0), Nothing, Just (Id 2)]
  fk <- loadIds dir "fk" :: IO (Col E (Id T))
  got3 <- pairs (column fk)
  check "dense ids" got3
    [(Id 0, noId), (Id 1, Id 0), (Id 2, noId), (Id 3, Id 2)]

  -- ...and a hole must resolve to nothing when composed onto the target table,
  -- with no presence bit anywhere: this is the whole point of the sentinel
  writeStrs dir "label" ["zero", "one", "two"]
  label <- loadStrs dir "label" :: IO (Col T ByteString)
  got4 <- pairs (compose (column fk) (column label))
  check "ids compose past holes" got4 [(Id 1, "zero"), (Id 3, "two")]

  -- dense strings, with a hole as the empty string
  writeStrs dir "strs" ["ab", "", "c"]
  strs <- loadStrs dir "strs" :: IO (Col E ByteString)
  got5 <- pairs (column strs)
  check "dense strings" got5 [(Id 0, "ab"), (Id 1, ""), (Id 2, "c")]

  -- CSR words: an empty row in the middle, which is how the format spells both
  -- "no value" and "many values"
  writeMultiInts dir "csr" [[5, 6], [], [9]]
  csr <- loadMultiInts dir "csr" :: IO (MultiCol E Int)
  got6 <- pairs (multiColumn csr)
  check "csr words" got6 [(Id 0, 5), (Id 0, 6), (Id 2, 9)]

  -- an odd key count pushes the values past a 4-byte boundary, so this is the
  -- test that the writer's pad and the reader's align8 agree
  writeMultiIds dir "csrid" [[Id 1], [], [Id 0, Id 2], []]
  csrid <- loadMultiIds dir "csrid" :: IO (MultiCol E (Id T))
  got7 <- pairs (multiColumn csrid)
  check "csr ids" got7 [(Id 0, Id 1), (Id 2, Id 0), (Id 2, Id 2)]

  -- CSR strings: row offsets over string offsets over bytes
  writeMultiStrs dir "csrstr" [["a", "bb"], [], ["c"]]
  csrstr <- loadMultiStrs dir "csrstr" :: IO (MultiCol E ByteString)
  got8 <- pairs (multiColumn csrstr)
  check "csr strings" got8 [(Id 0, "a"), (Id 0, "bb"), (Id 2, "c")]

  -- a sparse universe derived from the holes in an FK column
  let live = validityBits fk
  got9 <- pairs (sparseUniverse live 4 :: Drv (Id E) (Id E))
  check "validity bits" got9 [(Id 1, Id 1), (Id 3, Id 3)]

  -- Probing that same universe consults only the range, NOT the mask, so id 0 —
  -- a hole — is a member. The asymmetry with drive above is deliberate and is
  -- inherited from the Rust port: a probe of a universe is only ever reached with
  -- an id that came from a real column of that table, so a dead id cannot arrive,
  -- and leaving the mask out keeps the probe down to one comparison. Pinned here
  -- because it is the kind of thing a later "cleanup" would quietly change.
  check "sparse universe probe is a range check only"
    (map (member (sparseUniverse live 4 :: Prb (Id E) (Id E)) . Id) [0, 1, 3, 4])
    [True, True, True, False]

  -- a file of the wrong kind must fail loudly rather than reinterpret bytes
  bad <- try (loadInts dir "strs" >>= evaluate) :: IO (Either ErrorCall (Col E Int))
  check "kind mismatch is loud" (either (const True) (const False) bad) True

  -- and so must a file that is not a v2 cache at all
  writeFile (dir </> "notcache.bin") "definitely not a cache file"
  bad2 <- try (loadStrs dir "notcache" >>= evaluate)
            :: IO (Either ErrorCall (Col E ByteString))
  check "bad magic is loud" (either (const True) (const False) bad2) True

  ------------------------------------------------------------------------------
  -- The schema layer
  ------------------------------------------------------------------------------

  -- Four movies. Movie 2 has no year and no keywords, movie 3's kind is a hole,
  -- so the two ways of spelling "missing" both appear.
  let sdir = tmp </> "prela-schema-test"
  createDirectoryIfMissing True sdir
  writeStrs      sdir "Movie_title"   ["Alien", "Aliens", "Solaris", "Alien 3"]
  writeIds       sdir "Movie_kind"    [Just (Id 0), Just (Id 0), Just (Id 1), Nothing]
  writeMultiInts sdir "Movie_year"    [[1979], [1986], [], [1992]]
  writeMultiIds  sdir "Movie_keyword" [[Id 1], [Id 0, Id 1], [], [Id 0]]
  writeDoubles   sdir "Movie_rating"  [8.5, 8.4, 8.0, 7.0]
  writeStrs      sdir "Keyword_text"  ["sequel", "alien"]
  writeStrs      sdir "Kind_text"     ["movie", "tv series"]
  -- Four link ids, of which 1 and 3 are dead: their FK is a hole.
  writeIds       sdir "Link_about"    [Just (Id 0), Nothing, Just (Id 3), Nothing]
  writeStrs      sdir "Link_note"     ["first", "", "third", ""]
  s <- Sch.loadTiny sdir

  -- The loader read the right filenames, in the right kinds.
  gotS1 <- pairs (Sch.title s)
  check "schema: 1:1 string column" gotS1
    [(Id 0, "Alien"), (Id 1, "Aliens"), (Id 2, "Solaris"), (Id 3, "Alien 3")]
  gotS2 <- pairs (Sch.year s)
  check "schema: multi-valued column" gotS2 [(Id 0, 1979), (Id 1, 1986), (Id 3, 1992)]

  -- Each universe is sized from its entity's FIRST declared field, and nothing
  -- else in the record says how big an id space is, so this is the check that the
  -- generated `colLen` call picked the right column.
  gotS3 <- pairs (Sch.movie s)
  check "schema: universe from first column" gotS3 [(Id i, Id i) | i <- [0 .. 3]]
  check "schema: universe sizes"
    ( length (values (Sch.keywords s)), length (values (Sch.kinds s)) )
    (2, 2)

  -- Two entities both have a field named `text` on disk; `as` renamed the
  -- accessors, and the record fields are prefixed by entity, so both survive.
  gotS4 <- pairs (Sch.keywordText s)
  check "schema: renamed accessors" gotS4 [(Id 0, "sequel"), (Id 1, "alien")]

  -- A sparse entity's universe: the mask is derived from the holes in its first
  -- field, so driving it skips the dead ids...
  gotS5 <- pairs (Sch.links s)
  check "schema: sparse universe drive" gotS5 [(Id 0, Id 0), (Id 2, Id 2)]

  -- ...while probing it stays a plain range check, dead id and all. Same
  -- asymmetry as the hand-built case above, now reached through the generator.
  check "schema: sparse universe probe" (map (member (Sch.links s) . Id) [0, 1, 3, 4])
    [True, True, True, False]

  -- GROUP BY a non-key column: re-key the movies by their kind, then reduce.
  --
  -- Movie 3's kind is a hole, and it lands in a group of its OWN rather than
  -- being dropped. That is not a bug and it is not special to `groupBy`: a hole
  -- disappears when it is used as a KEY, because the range check then fails, and
  -- here it is being used as a value. Rust's `group_by` does the same. Pinned
  -- because the shape of the answer depends on it.
  let byKind = groupBy (Sch.movie s) (Sch.kind s)
  gotG1 <- pairs (fold (\n _ -> n + 1) (0 :: Int) byKind)
  check "groupBy then fold, hole is its own group" gotG1 [(noId, 1), (Id 0, 2), (Id 1, 1)]

  -- The idiom for excluding it: keep only the movies whose kind resolves to a
  -- real kind. `compose (kind s) (kinds s)` is that test, and it is the ordinary
  -- membership check rather than anything to do with grouping.
  let byLiveKind = groupBy (restrict (Sch.movie s) (compose (Sch.kind s) (Sch.kinds s)))
                           (Sch.kind s)
  gotG2 <- pairs (fold (\n _ -> n + 1) (0 :: Int) byLiveKind)
  check "groupBy over resolvable keys only" gotG2 [(Id 0, 2), (Id 1, 1)]

  -- The whole-group fold, and its main instance. Kind 1's movie has no keywords,
  -- so that group has no rows at all and does not appear.
  gotG3 <- pairs (bufFold id (compose byLiveKind (Sch.title s)))
  check "bufFold keeps drive order" gotG3 [(Id 0, ["Alien", "Aliens"]), (Id 1, ["Solaris"])]
  gotG4 <- pairs (countDistinct (compose byLiveKind (Sch.keyword s)))
  check "countDistinct" gotG4 [(Id 0, 2 :: Int)]

  -- Ranges: closed against half-open, on the same three years.
  check "between is closed" (values (between 1979 1986 (compose (Sch.movie s) (Sch.year s))))
    [1979, 1986]
  check "range is half-open" (values (range 1979 1986 (compose (Sch.movie s) (Sch.year s))))
    [1979]

  check "schema: queries" (schemaQueries s)
    ( ["Aliens", "Alien 3"]   -- keyword "sequel"
    , 2                       -- year > 1980
    , ["Solaris"]             -- no year at all
    , ["Solaris"]             -- kind is "tv series", reached through the FK
    , 8.5                     -- highest rating
    )

  n <- readIORef fails
  if n == 0
    then putStrLn "cache and schema: all checks ok"
    else putStrLn (show n ++ " failure(s)") >> exitFailure
