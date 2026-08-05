{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The staged engine's answer to design/CoreProbe.hs:
-- the same query, through "Prela.PullStaged.Ops" instead of "Prela.Push.Ops", checked for
-- the same one-loop shape.
--
--   cd haskell && mkdir -p /tmp/stagedprobe && cabal exec -- \
--     ghc -O2 -isrc -Wall -fforce-recomp design/StagedProbe.hs design/StagedProbeGen.hs \
--         -outputdir /tmp/stagedprobe -o /tmp/stagedprobe/probe \
--         -ddump-simpl -dsuppress-all -ddump-to-file
--
-- then read /tmp/stagedprobe/design/StagedProbe.dump-simpl, and run
-- /tmp/stagedprobe/probe +RTS -s for the allocation figure.
--
-- The bar, from the plan: no `Step`, no `Prb`, no `$fMonad`, no `MutVar#`, no
-- `((), I#`, and the loop a `$wgo` over unboxed arguments with `joinrec` inner
-- levels.
--
-- `countRecent` is the comparison against push. `countPerYear` is the one that
-- can fail in a new way: its table has to be built once and read at two modes,
-- and if the continuation shape in "Prela.PullStaged.Materialize" is wrong it will
-- quietly be built twice. `firstTen` is the thing push cannot do at all.
--
-- IT PASSES. All five markers are zero, and so are `atStore`, `insertTable` and
-- `newMTable` — the storage dictionary and the whole table insert specialise
-- away.
--
--   countRecent            32 bytes
--   countPerYear drv    47168 bytes
--   countPerYear prb       16 bytes
--   firstTen             2128 bytes
--
-- `countRecent` is one boxed `Int`, so nothing per row over a million rows.
-- Its loop is `$wgo1 :: Int# -> Int# -> Int#`, with `readIntOffAddr#` and the
-- `> 1980` test inline, and it is the same shape design/CoreProbe.hs gets from
-- the push engine — including GHC's habit of duplicating the loop body into two
-- identical `joinrec`s, which push does too, so that is the simplifier and not
-- staging. The staged one is marginally better at the exit: push boxes into
-- `(# I# sc #)`, this returns a raw `Int#`.
--
-- `countPerYear prb` at 16 bytes is the real result: probing the fold does not
-- rebuild it, so `withFold`'s continuation shares the storage as intended. The
-- 47 KB on the driven side is the table itself (130 groups in a 256-slot table,
-- plus the 64 and 128 slot tables it grew through, plus the frozen copies) and
-- the 130-row collect list. The insert loop compiles to `readWordArray#` and
-- `andI#` with no allocation; `MTable` appears in the dump only inside growth
-- and the final freeze.
--
-- That 47 KB started at 40 MB, which is 40 bytes per INPUT row, and the fix is
-- recorded at `MTable` in "Prela.PullStaged.Materialize": the fold's accumulator was
-- a `(table, count)` tuple, rebuilt every step. Moving the count into a one-slot
-- mutable vector makes the accumulator the same pointer every iteration.
--
-- `firstTen` stops after ten rows out of a million: `limit`'s @([a], Int)@
-- accumulator is unboxed into two loop arguments, so the only allocation is the
-- ten rows themselves.
--
-- Term counts, for the compile-time note the plan asks for: push's CoreProbe is
-- 822 terms for its one query, this is 2552 for three, one of which builds and
-- reads a hash table. Per query they are close.
module Main (main) where

import Control.Exception (evaluate)
import GHC.Conc (getAllocationCounter, setAllocationCounter)

import Prela.Storage

import StagedProbeGen

-- Bytes allocated while forcing one thunk. `+RTS -s` cannot answer this: it is
-- dominated by building `yearCol` from a list, which costs more than every query
-- here put together.
alloc :: String -> IO a -> IO ()
alloc name act = do
  setAllocationCounter 0
  _ <- act
  end <- getAllocationCounter
  putStrLn (name ++ "  " ++ show (negate end) ++ " bytes")

data Movie

n :: Int
n = 1000000
{-# NOINLINE n #-}

yearCol :: Col Movie Int
yearCol = mkCol [1900 + (i * 7919) `mod` 130 | i <- [0 .. 999999 :: Int]]
{-# NOINLINE yearCol #-}

recent :: Int
recent = $$(countRecent [|| n ||] [|| yearCol ||])
{-# NOINLINE recent #-}

perYear :: ([(Int, Int)], Bool)
perYear = $$(countPerYear [|| n ||] [|| yearCol ||])
{-# NOINLINE perYear #-}

firstTenRows :: [(Int, Int)]
firstTenRows = map (\(Id i, y) -> (i, y)) $$(firstTen [|| n ||] [|| yearCol ||])
{-# NOINLINE firstTenRows #-}

-- Order matters: each `alloc` has to be the FIRST force of its thunk, so the
-- printing comes afterwards. The two halves of `perYear` are measured separately
-- on purpose — the second is the probe, and if @withFold@'s continuation shape
-- were wrong it would build the table a second time and the second figure would
-- be as large as the first.
main :: IO ()
main = do
  _ <- evaluate yearCol
  alloc "countRecent      " (evaluate recent)
  alloc "countPerYear drv " (evaluate (length (fst perYear)))
  alloc "countPerYear prb " (evaluate (snd perYear))
  alloc "firstTen         " (evaluate (length firstTenRows))
  putStrLn "-- answers --"
  print recent
  case perYear of
    (groups, has1985) -> print (length groups, sum (map snd groups), has1985)
  print firstTenRows
