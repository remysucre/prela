-- | Not part of the build. A single realistic query, used to check whether the
-- operator chain fuses into one loop nest under -O2.
--
--   cd haskell && ghc -O2 -isrc -fforce-recomp design/CoreProbe.hs \
--        -outputdir /tmp/coreprobe -o /tmp/coreprobe/probe \
--        -ddump-simpl -dsuppress-all -ddump-to-file
--
-- then read design/CoreProbe.dump-simpl.
--
-- Result, at plain -O2 with no extra flags: the whole chain becomes one join
-- point `$s$wgo3 :: Int# -> Int# -> (# Int #)` — unboxed row index, unboxed
-- accumulator, indexArray# and the `> 1980` test inline in the loop body, no
-- allocation per row, no Mode dictionary, no Prb record, no intermediate.
--
-- Two things had to be fixed to get there, both recorded in Prela.hs:
--   * `column` must not match its Col on the left of the `=`, or the Prb is a
--     thunk and GHC calls the probe through a shared record field per row.
--   * `foldAll` must thread its accumulator through a state monad, not an
--     STRef, or every row does a readMutVar/writeMutVar on a boxed Int.
module Main where

import Prela

data Movie

yearCol :: Col Movie Int
yearCol = mkCol [1900 + (i * 7919) `mod` 130 | i <- [0 .. 999999 :: Int]]

movie :: Mode q => q (Id Movie) (Id Movie)
movie = universe 1000000

year :: Mode q => q (Id Movie) Int
year = column yearCol

-- movie : (year > 1980) → year  ⊵ count
countRecent :: Int
countRecent = foldAll (\n _ -> n + 1) (0 :: Int)
                      (compose (restrict movie (gt 1980 year)) year)

main :: IO ()
main = print countRecent
