{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Not part of the build. The generator half of design/DictProbe.hs, in its own
-- module only because a splice cannot see names defined in the file it appears
-- in. See that file for what this is asking.
module DictProbeGen (sumCol) where

import Language.Haskell.TH (CodeQ)

import Prela.Storage

-- | Generate a loop over a `Col e r`, mapping each value to an `Int` and summing.
--
-- The `Elem r` constraint is the whole point. `atStore` inside the quote is a
-- class method, so the generated code needs a dictionary, and this function does
-- not have a concrete `r` to pick one from. If GHC accepts this, the staged
-- leaves can be written exactly like the push ones.
--
-- The `case $$c of Col n s ->` is inside the quote rather than a pattern on the
-- left, which is the same discipline FUSION.md's first rule states for the push
-- engine, though for a different reason. There the worry was the leaf record
-- becoming a thunk; here the column is a runtime value the generator only has
-- code FOR, so matching it in the generator is not even expressible.
sumCol :: forall e r. Elem r => (CodeQ r -> CodeQ Int) -> CodeQ (Col e r) -> CodeQ Int
sumCol f c =
  [|| case $$c of
        Col n s ->
          let go !i !acc
                | i >= n    = acc
                | otherwise = let !v = atStore s i
                              in go (i + 1) (acc + $$(f [|| v ||]))
          in go 0 0 ||]
