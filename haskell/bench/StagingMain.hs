{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE TemplateHaskell #-}
{-# OPTIONS_GHC -fno-full-laziness #-}

-- | Focused staged-versus-unstaged TPC-H Q6 benchmark.
module Main (main) where

import Control.Exception (evaluate)
import Control.Monad (forM, unless)
import Data.List (intercalate)
import Data.Word (Word64)
import GHC.Clock (getMonotonicTimeNSec)
import GHC.Stats (RTSStats (allocated_bytes), getRTSStats)
import System.Environment (getArgs, lookupEnv)
import System.Mem (performGC)
import Text.Printf (printf)

import qualified Prela.PullStaged.Query as Q
import qualified TPCH.StagedQueries as Staged
import TPCH.StagedSchema (TPCHS, loadTPCHS)
import qualified TPCH.UnstagedQueries as Unstaged

stagedQ6 :: TPCHS -> Double
stagedQ6 = $$(Q.compile Staged.q6)
{-# NOINLINE stagedQ6 #-}

data Sample = Sample
  { elapsedSeconds :: !Double
  , allocatedBytes :: !Word64
  }

main :: IO ()
main = do
  args <- getArgs
  cacheDirectory <- maybe "../cache" id <$> lookupEnv "PRELA_CACHE"
  let rounds :: Int
      rounds = case args of
        [] -> 7
        [value] -> read value
        _ -> error "usage: prela-staging-bench [rounds]"
  unless (rounds > 0) (error "rounds must be positive")

  schema <- loadTPCHS cacheDirectory
  stagedResult <- evaluate (stagedQ6 schema)
  relationResult <- evaluate (Unstaged.q6 schema)
  concreteResult <- evaluate (Unstaged.q6Concrete schema)
  unless (stagedResult == relationResult && stagedResult == concreteResult) $
    error ("Q6 mismatch: staged=" ++ show stagedResult
           ++ ", unstaged relation=" ++ show relationResult
           ++ ", unstaged concrete=" ++ show concreteResult)

  -- Touch both implementations before collecting samples so page faults and
  -- one-time runtime initialization do not favor either executor.
  _ <- measure stagedQ6 schema
  _ <- measure Unstaged.q6 schema
  _ <- measure Unstaged.q6Concrete schema

  triples <- forM [0 .. rounds - 1] $ \roundNumber ->
    case roundNumber `mod` 3 of
      0 -> do
        staged <- measure stagedQ6 schema
        relation <- measure Unstaged.q6 schema
        concrete <- measure Unstaged.q6Concrete schema
        pure (staged, relation, concrete)
      1 -> do
        relation <- measure Unstaged.q6 schema
        concrete <- measure Unstaged.q6Concrete schema
        staged <- measure stagedQ6 schema
        pure (staged, relation, concrete)
      _ -> do
        concrete <- measure Unstaged.q6Concrete schema
        staged <- measure stagedQ6 schema
        relation <- measure Unstaged.q6 schema
        pure (staged, relation, concrete)
  let stagedSamples = [sample | (sample, _, _) <- triples]
      relationSamples = [sample | (_, sample, _) <- triples]
      concreteSamples = [sample | (_, _, sample) <- triples]

  putStrLn ("Q6 result: " ++ show stagedResult)
  report "staged" stagedSamples
  report "unstaged (Relation)" relationSamples
  report "unstaged (Drive/Probe)" concreteSamples
  let stagedTime = mean (map elapsedSeconds stagedSamples)
      relationTime = mean (map elapsedSeconds relationSamples)
      concreteTime = mean (map elapsedSeconds concreteSamples)
  printf "Relation/staged time: %.2fx\n" (relationTime / stagedTime)
  printf "Drive/Probe/staged time: %.2fx\n" (concreteTime / stagedTime)

measure :: (TPCHS -> Double) -> TPCHS -> IO Sample
measure query schema = do
  performGC
  beforeStats <- getRTSStats
  beforeTime <- getMonotonicTimeNSec
  !result <- evaluate (query schema)
  afterTime <- getMonotonicTimeNSec
  -- 'allocated_bytes' is updated at collections; force one after the timed
  -- region so a sub-nursery staged run is measured instead of reported as zero.
  performGC
  afterStats <- getRTSStats
  -- Keep the result live through both observations.
  _ <- evaluate result
  pure Sample
    { elapsedSeconds = fromIntegral (afterTime - beforeTime) / 1e9
    , allocatedBytes = allocated_bytes afterStats - allocated_bytes beforeStats
    }
{-# NOINLINE measure #-}

report :: String -> [Sample] -> IO ()
report label samples = do
  let times = map elapsedSeconds samples
      allocations = map allocatedBytes samples
  printf "%s mean: %.6f s, %.3f MiB allocated\n"
    label (mean times) (meanWord allocations / (1024 * 1024))
  putStrLn ("  times: " ++ intercalate ", " (map (printf "%.6f") times))

mean :: [Double] -> Double
mean values = sum values / fromIntegral (length values)

meanWord :: [Word64] -> Double
meanWord = mean . map fromIntegral
