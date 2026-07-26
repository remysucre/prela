{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE RecordWildCards #-}

-- | Not part of the build. Does the schema-as-a-record design still fuse?
--
-- CoreProbe checks the same query with its column as a top-level constant. Here
-- the column arrives inside a record, loaded at run time, and the relations are
-- higher-rank fields so that unpacking the record with RecordWildCards puts
-- mode-polymorphic names in scope. That is the whole point of the design, and it
-- is also exactly what could stop GHC specializing: each use of `year` is now a
-- field selection followed by a dictionary application, on a record the compiler
-- has never seen a constructor for.
--
--   cd haskell && mkdir -p /tmp/schemaprobe && cabal exec -- \
--     ghc -O2 -isrc -fforce-recomp design/SchemaProbe.hs \
--         -outputdir /tmp/schemaprobe -o /tmp/schemaprobe/probe \
--         -ddump-simpl -dsuppress-all -ddump-to-file -rtsopts
--   /tmp/schemaprobe/probe /tmp/cacheprobe +RTS -s
--
-- Reads the million-row Movie_year.bin that CacheProbe writes, so run that with
-- `write` first. Expected count is 376926, same as both other probes.
--
-- Result, and it decided the design of Prela.Schema. Three arrangements were
-- measured on the same query and the same file:
--
--   relations stored in the record   249 MB allocated, `Prb` and a Monad
--   as higher-rank fields            dictionary both alive in the Core. BROKEN.
--
--   columns in the record, leaf       57 KB allocated, all markers clean.
--   names bound in a local `where`
--
--   columns in the record, leaves     57 KB allocated, all markers clean.
--   as top-level functions of it
--
-- The first one fails because a polymorphic function stored in a data structure
-- is opaque: GHC reaches the use site holding a field selection rather than a
-- definition, so it cannot see which instance to specialize to, and the query
-- runs through a dictionary per row. The other two both keep the definition
-- visible at the use site, which is all that was needed. `Prela.Schema`
-- generates the third and query modules can add the second on top, since a
-- one-line local binding per name buys back the bare spelling.
module Main where

import Prela
import Prela.Cache
import System.Environment (getArgs)

data Movie

-- The shape a generated schema would have: one higher-rank field per name a
-- query wants to say. The `forall q. Mode q =>` is what makes `year` usable in
-- either mode without the query mentioning modes, and putting it in the field
-- type rather than relying on inference also sidesteps MonoLocalBinds, which
-- would otherwise pin a `where`-bound name to one mode.
data Schema = Schema
  { movieN  :: Int
  , yearCol :: Col Movie Int
  }

loadSchema :: FilePath -> IO Schema
loadSchema dir = do
  c <- loadInts dir "Movie_year" :: IO (Col Movie Int)
  case c of
    Col n _ -> return Schema { movieN = n, yearCol = c }

-- Stands in for the query module: one function taking the schema, every query
-- underneath it reading with bare names. The leaf names are bound locally rather
-- than stored in the record, so at each use site GHC knows the mode statically
-- and can specialize. Signatures are required, not decorative: TypeFamilies
-- turns on MonoLocalBinds, which would otherwise pin each name to one mode.
queries :: Schema -> Int
queries Schema{..} =
    countRecent
  where
    movie :: Mode q => q (Id Movie) (Id Movie)
    movie = universe movieN
    year :: Mode q => q (Id Movie) Int
    year = column yearCol

    -- movie : (year > 1980) → year  ⊵ count
    countRecent = foldAll (\n _ -> n + 1) (0 :: Int)
                          (compose (restrict movie (gt 1980 year)) year)

-- Variant three: no local preamble at all. The leaf is a top-level function of
-- the schema, so the query says `year s` instead of `year`. Noisier to read, but
-- it needs no per-query boilerplate, which matters because everything here is
-- going to be generated.
movie' :: Mode q => Schema -> q (Id Movie) (Id Movie)
movie' s = universe (movieN s)

year' :: Mode q => Schema -> q (Id Movie) Int
year' s = column (yearCol s)

queries' :: Schema -> Int
queries' s = foldAll (\n _ -> n + 1) (0 :: Int)
                     (compose (restrict (movie' s) (gt 1980 (year' s))) (year' s))

main :: IO ()
main = do
  (dir : _) <- getArgs
  s <- loadSchema dir
  print (queries' s)
