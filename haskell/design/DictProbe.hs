{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The one thing that has to work before the staged
-- engine can be written against the real storage types, and the one thing the
-- design prototypes never tested: can a typed quote mention a class method whose
-- dictionary is not known where the quote is WRITTEN?
--
-- Every prototype so far used `Data.Vector.Unboxed Int`, which is monomorphic.
-- The real leaves are not. "Prela.Storage" gives each element type its own
-- physical layout through an associated data family:
--
--   class Elem r where
--     data Store r
--     atStore :: Store r -> Int -> r
--
-- so a staged `column` has to emit code containing `atStore`, from a generator
-- that is polymorphic in `r`. Whether that is allowed is a question about GHC,
-- not about Prela. Typed splices are typechecked at the SPLICE site, where `r`
-- is concrete, so the dictionary ought to resolve there — but a quote also
-- cannot capture a locally bound dictionary, and which of those two rules
-- applies here is exactly what this file asks.
--
-- IT WORKS, AND IT SPECIALISES. One generator, `sumCol` in design/DictProbeGen.hs,
-- spliced at three element types, and in every case the dictionary is gone from
-- the compiled program: `grep atStore` over the Tidy Core finds nothing. Each
-- site is a `joinrec $wgo :: Int# -> Int# -> Int` with the array read inlined,
-- and each allocates only the boxed `Int` it returns.
--
--   Int         500000500000 / 500000500000
--   Id Movie    499999500000 / 499999500000
--   ByteString  2893 / 2893
--
-- The `Id` newtype and the `Store` data family both erase to casts. The
-- ByteString case does better than asked: `atStore` there builds a slice of the
-- packed buffer, and since the only thing done with the slice is take its
-- length, the slice is never built — what is left is two `readWord32OffAddr#`
-- and a subtraction.
--
-- The reads are `readIntOffAddr#` paired with `touch#`, not `indexIntArray#` as
-- in the design prototypes. That is the difference between `Data.Vector.Storable`
-- and `Data.Vector.Unboxed`: `Storable` points into the mmap through a
-- `ForeignPtr` and the `touch#` keeps the mapping alive across the read. It is a
-- keepalive, not an allocation, and it is what the push engine already compiles
-- to today. So the prototypes' use of `Unboxed` was not hiding anything.
--
--   cd haskell && mkdir -p /tmp/dictprobe && cabal exec -- ghc -O2 -isrc -Wall \
--     -fforce-recomp design/DictProbe.hs design/DictProbeGen.hs \
--     -outputdir /tmp/dictprobe -o /tmp/dictprobe/probe && /tmp/dictprobe/probe
module Main (main) where

import Data.ByteString.Char8 ()
import qualified Data.ByteString.Char8 as BC

import Prela.Storage

import DictProbeGen

-- Three element types with three different physical layouts: `Int` is a
-- `Storable` vector of words, `Id e` is the same words reinterpreted, and
-- `ByteString` is offsets plus one packed buffer. If the generator works it
-- works for all three from one definition.
data Movie

ints :: Col Movie Int
ints = mkCol [1 .. 1000000]
{-# NOINLINE ints #-}

ids :: Col Movie (Id Movie)
ids = mkCol (map Id [0 .. 999999])
{-# NOINLINE ids #-}

names :: Col Movie BC.ByteString
names = mkCol (map (BC.pack . show) [1 .. 1000 :: Int])
{-# NOINLINE names #-}

-- | Sum a column of Ints. The generator never knew `r` was `Int`.
sumInts :: Int
sumInts = $$(sumCol (\v -> [|| $$v ||]) [|| ints ||])
{-# NOINLINE sumInts #-}

-- | The same generator at `Id Movie`, whose `atStore` wraps in a newtype.
sumIds :: Int
sumIds = $$(sumCol (\v -> [|| case $$v of Id i -> i ||]) [|| ids ||])
{-# NOINLINE sumIds #-}

-- | And at `ByteString`, whose `atStore` is two offset reads and a slice.
sumNameLens :: Int
sumNameLens = $$(sumCol (\v -> [|| BC.length $$v ||]) [|| names ||])
{-# NOINLINE sumNameLens #-}

main :: IO ()
main = do
  putStrLn ("Int         " ++ show sumInts       ++ " / " ++ show expectInts)
  putStrLn ("Id Movie    " ++ show sumIds        ++ " / " ++ show expectIds)
  putStrLn ("ByteString  " ++ show sumNameLens   ++ " / " ++ show expectLens)
  where
    expectInts = sum [1 .. 1000000 :: Int]
    expectIds  = sum [0 .. 999999 :: Int]
    expectLens = sum (map (BC.length . BC.pack . show) [1 .. 1000 :: Int])
