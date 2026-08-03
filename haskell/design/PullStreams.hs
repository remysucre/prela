{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE ScopedTypeVariables #-}

-- | Not part of the build. A pull-stream `Drv`, far enough along to run a real
-- query shape and a `zipWith`, which is the thing the current push `Drv` cannot
-- express at all.
--
--   cd haskell && cabal exec -- ghc -O2 -Wall -fforce-recomp design/PullStreams.hs \
--     -outputdir /tmp/pull -o /tmp/pull/pull && /tmp/pull/pull
--
-- The shape of the answer, in three parts.
--
-- 1. A LINEAR producer is a state plus a step function returning Done / Skip /
--    Yield. `Skip` exists because a filter cannot loop to the next match — it
--    does not own a loop — so it must answer "nothing this time, ask again".
--
-- 2. Composition is flat-map, and flat-map does not fit in a linear producer:
--    one outer row can produce many inner rows, so the stream type needs a
--    NESTED case carrying the outer stream and a function from each outer value
--    to an inner producer. This is exactly the Linear/Nested split in strymonas
--    (§6.1 of Stream Fusion, to Completeness), arrived at for the same reason.
--
-- 3. Therefore `Prb` grows a third field. `probe` and `probeAny` stay as they
--    are — membership is not a stream and never needs suspending — but the
--    VALUE-producing side of a compose now also has to be reachable one row at
--    a time, which is `cursor` below. For a 1:1 column that is an at-most-one
--    producer; for a CSR column it is an index range, so the extra state is two
--    Ints and nothing is allocated per outer row.
--
-- What it buys is at the bottom: `zipLin` advances two producers in lockstep,
-- which is a merge join's inner loop. What it costs is the `Skip` branch and
-- the state traffic on every operator, plus the fact (strymonas Thm. 1) that
-- zipping two NESTED streams cannot be fused by anyone — one side has to be
-- reified into a closure. Since compose is Prela's main operator, a merge join
-- between two composed relations lands in exactly that carve-out.
--
-- IT DOES NOT FUSE. This was measured, the same way FUSION.md measures the push
-- engine: a million-row `movie -> keyword : (> 1980) ⊵ count` over a CSR cursor,
-- built at -O2 and dumped. The push version of that query is one join point
-- `Int# -> Int# -> (# Int #)` with nothing allocated per row. The pull version
-- keeps every piece of the abstraction at runtime:
--
--   * `sfold` never inlines into the query — it stays a top-level recursive
--     function that gets CALLED, because it recurses on the Lin/Nest sum;
--   * the inner loop is `case ww3 ww4 of { Done -> ..; Skip -> ..; Yield -> .. }`
--     where `ww3` is a VARIABLE, so every row is an unknown call through a
--     function pointer and a `Step` constructor built and immediately matched;
--   * the accumulator is boxed — `case acc of I# ipv -> .. I# (+# ipv 1#)` —
--     so there is an `I#` allocated per row;
--   * the cursor allocates a fresh existential `Linear` per OUTER row.
--
-- The existential state is the root cause: GHC cannot specialize a loop whose
-- step function is hidden behind `forall s.`, so nothing downstream of it can
-- be seen through. This is the concatMap weak spot of stream fusion, and it is
-- why strymonas needs staging rather than an optimizer — with the loop
-- GENERATED, none of the above is ever built in the first place.
--
-- So the conclusion this file exists to record: pull is fine as a design and
-- gives you zip, but in stock GHC it costs the entire fusion property on the
-- operator Prela uses most. Adopting it means also adopting typed Template
-- Haskell. If all that is actually wanted is a merge join, the cheap route is a
-- pipeline-breaking operator that drives both sides into sorted buffers and
-- merges them — the same shape as `index` and `fold` in Prela.Materialize —
-- which needs no change to the representation at all.
--
-- The staged version is design/Staged.hs, and it does fuse. Read that first; the
-- interest of this file is now the negative result above. Two things about it
-- are worth knowing before opening it. Staging deletes both `Skip` and the
-- existential, so the representation gets SIMPLER, not more complicated — the
-- machinery here exists to work around a runtime step function, and there is no
-- runtime step function once the loop is generated. But staging does not by
-- itself deliver `zipLin`: a generated loop is still owned by one producer, so
-- lockstep needs relations that are addressable by index, which is `SProd` over
-- there and is a strictly smaller class than `Stream` is here.
module Main (main) where

import qualified Data.Vector.Unboxed as U

newtype Id = Id Int deriving (Eq, Show)

--------------------------------------------------------------------------------
-- The representation
--------------------------------------------------------------------------------

data Step s d r = Done | Skip !s | Yield !d !r !s

-- | A producer that advances at most one element per step.
data Linear d r = forall s. Linear !s (s -> Step s d r)

-- | A relation in driven position: linear, or an outer stream each of whose
-- values opens an inner producer. The key of the pair comes from the outer.
data Stream d r
  = Lin (Linear d r)
  | forall e. Nest (Stream d e) (e -> Linear () r)

-- | What the probed side must supply for `compose`: one key in, a producer of
-- that key's values out.
type Cursor d r = d -> Linear () r

--------------------------------------------------------------------------------
-- Leaves
--------------------------------------------------------------------------------

universe :: Int -> Stream Id Id
universe n = Lin (Linear 0 step)
  where
    step i | i >= n    = Done
           | otherwise = Yield (Id i) (Id i) (i + 1)

column :: U.Vector Int -> Stream Id Int
column v = Lin (Linear 0 step)
  where
    n = U.length v
    step i | i >= n    = Done
           | otherwise = Yield (Id i) (U.unsafeIndex v i) (i + 1)

-- | 1:1 column in probed position: at most one value, so the state is just
-- "have I emitted yet".
colCursor :: U.Vector Int -> Cursor Id Int
colCursor v (Id i) = Linear False step
  where
    step done | done || i < 0 || i >= U.length v = Done
              | otherwise                        = Yield () (U.unsafeIndex v i) True

-- | CSR column in probed position: key i owns values[offs[i] .. offs[i+1]-1],
-- so the state is the current offset and the end is captured once.
csrCursor :: U.Vector Int -> U.Vector Int -> Cursor Id Int
csrCursor offs vals (Id i) = Linear (U.unsafeIndex offs i) step
  where
    end = U.unsafeIndex offs (i + 1)
    step j | j >= end  = Done
           | otherwise = Yield () (U.unsafeIndex vals j) (j + 1)

--------------------------------------------------------------------------------
-- Operators
--------------------------------------------------------------------------------

mapLin :: (r -> s) -> Linear d r -> Linear d s
mapLin f (Linear s0 step) = Linear s0 step'
  where
    step' s = case step s of
      Done         -> Done
      Skip s'      -> Skip s'
      Yield d r s' -> Yield d (f r) s'

filtLin :: (r -> Bool) -> Linear d r -> Linear d r
filtLin t (Linear s0 step) = Linear s0 step'
  where
    step' s = case step s of
      Done                     -> Done
      Skip s'                  -> Skip s'
      Yield d r s' | t r       -> Yield d r s'
                   | otherwise -> Skip s'          -- the Skip tax: no inner loop

mapv :: (r -> s) -> Stream d r -> Stream d s
mapv f (Lin l)    = Lin (mapLin f l)
mapv f (Nest o g) = Nest o (mapLin f . g)

filt :: (r -> Bool) -> Stream d r -> Stream d r
filt t (Lin l)    = Lin (filtLin t l)
filt t (Nest o g) = Nest o (filtLin t . g)

-- | Chain through a shared middle value. This is the whole reason `Nest` exists.
compose :: Stream d e -> Cursor e f -> Stream d f
compose s cur = Nest s cur

-- | Keep rows whose value is a member of something. Membership is NOT a stream —
-- it never has to suspend — so this stays a plain predicate, exactly as
-- `probeAny` is today.
restrict :: (r -> Bool) -> Stream d r -> Stream d r
restrict = filt

--------------------------------------------------------------------------------
-- Consumers
--------------------------------------------------------------------------------

foldLin :: (b -> d -> r -> b) -> b -> Linear d r -> b
foldLin f = go
  where
    go !acc (Linear s0 step) = case step s0 of
      Done         -> acc
      Skip s'      -> go acc (Linear s' step)
      Yield d r s' -> go (f acc d r) (Linear s' step)

-- | Folding a nested stream generates the loop nest: the outer fold's step
-- function is itself a fold over the inner producer.
sfold :: (b -> d -> r -> b) -> b -> Stream d r -> b
sfold f z (Lin l)    = foldLin f z l
sfold f z (Nest o g) = sfold (\acc d e -> foldLin (\acc' () r -> f acc' d r) acc (g e)) z o

toList :: Stream d r -> [(d, r)]
toList = reverse . sfold (\acc d r -> (d, r) : acc) []

--------------------------------------------------------------------------------
-- What pull buys: lockstep consumption
--------------------------------------------------------------------------------

-- | Advance two producers together. Impossible with the push `Drv`: once you
-- call `drive` it runs to completion. The `Maybe` buffers the left element
-- while the right side works through its Skips.
zipLin :: (a -> b -> c) -> Linear d a -> Linear e b -> Linear (d, e) c
zipLin f (Linear l0 lstep) (Linear r0 rstep) = Linear (l0, r0, Nothing) step
  where
    step (ls, rs, Nothing) = case lstep ls of
      Done          -> Done
      Skip ls'      -> Skip (ls', rs, Nothing)
      Yield d a ls' -> Skip (ls', rs, Just (d, a))
    step (ls, rs, Just (d, a)) = case rstep rs of
      Done          -> Done
      Skip rs'      -> Skip (ls, rs', Just (d, a))
      Yield e b rs' -> Yield (d, e) (f a b) (ls, rs', Nothing)

linear :: Stream d r -> Maybe (Linear d r)
linear (Lin l)    = Just l
linear (Nest _ _) = Nothing

--------------------------------------------------------------------------------
-- Check it against hand-written answers
--------------------------------------------------------------------------------

years :: U.Vector Int
years = U.fromList [1975, 1982, 1990, 1968, 2001]

-- Movie i owns keywordVals[keywordOffs[i] .. keywordOffs[i+1]-1].
keywordOffs :: U.Vector Int
keywordOffs = U.fromList [0, 2, 2, 5, 6, 6]

keywordVals :: U.Vector Int
keywordVals = U.fromList [10, 11, 20, 21, 22, 30]

main :: IO ()
main = do
  -- movie : (year > 1980) -> year, folded to a count. The CoreProbe query.
  let recent = filt (> 1980) (compose (universe 5) (colCursor years))
      n      = sfold (\acc _ _ -> acc + (1 :: Int)) 0 recent
  putStrLn ("count recent      = " ++ show n ++ "   (want 3)")

  -- movie -> keyword, the nested case: five movies, six keyword rows, one movie
  -- with no keywords at all.
  let kws = compose (universe 5) (csrCursor keywordOffs keywordVals)
  putStrLn ("movie -> keyword  = " ++ show (toList kws))

  -- Nesting composes: movie -> keyword, then filter and map the inner values.
  putStrLn ("  ... filtered    = " ++ show (toList (restrict (> 15) kws)))
  putStrLn ("  ... mapped      = " ++ show (toList (mapv negate kws)))

  -- And the payoff. Two linear producers advanced in lockstep — a merge join's
  -- inner loop, which the push Drv cannot express.
  case (linear (column years), linear (universe 5)) of
    (Just ys, Just us) -> do
      let zipped = zipLin (\y (Id i) -> (i, y)) ys us
      putStrLn ("zipped            = " ++ show (reverse (foldLin (\acc _ r -> r : acc) [] zipped)))
    _ -> putStrLn "zipped            = unavailable: a nested stream must be reified first"
