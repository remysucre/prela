-- | Run the Haskell translation of the Rust README's JOB query 22a.
--
-- @
-- cabal run prela-job-example
-- PRELA_CACHE=/path/to/job/cache cabal run prela-job-example
-- PRELA_LOADER=checked cabal run prela-job-example
-- @
module Main (main) where

import Data.Maybe (fromMaybe)
import System.Environment (lookupEnv)
import System.Exit (die)

import qualified JOB.Staged as Query
import qualified JOB.StagedSchema as Schema

main :: IO ()
main = do
  cacheDir <- fromMaybe "../cache" <$> lookupEnv "PRELA_CACHE"
  loader <- fromMaybe "fast" <$> lookupEnv "PRELA_LOADER"
  loadSchema <- case loader of
    "fast"    -> pure Schema.loadJOBS
    "checked" -> pure Schema.loadJOBSChecked
    other     -> die ("unknown PRELA_LOADER " ++ show other
                      ++ "; expected fast or checked")
  schema <- loadSchema cacheDir
  putStrLn (Query.runQ22a schema)
