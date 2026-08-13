{-# LANGUAGE DeriveLift #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Generated schema and adversarial fixtures for staged integration tests.
--
-- The declaration covers dense and sparse entities, scalar and multi-valued
-- columns, renamed accessors, and entity references.  The definitions below it
-- construct malformed references directly so both executor modes can be tested
-- at the storage/query boundary.
module TinyStaged where

import Language.Haskell.TH (CodeQ)
import Prela.Id (Id, Universe, denseUniverse, universeFromMask)
import Prela.PullStaged.Ops (Mode, referenceColumn)
import Prela.Schema
import Prela.Storage (SparseCol, mkSparseCol)

-- | Generate the tiny schema record, loaders, universes, and relation accessors.
declareStagedSchema "TinyS"
  [ entity "Movie" "movie"
      [ one  "title"    str
      , one  "kind"     (ref "Kind")
      , many "year"     int
      , many "keyword"  (ref "Keyword")
      , one  "rating"   dbl
      ]
  , entity "Keyword" "keywords"
      [ one "text" str `as` "keywordText" ]
  , entity "Kind" "kinds"
      [ one "text" str `as` "kindText" ]
  , sparseEntity "Link" "links"
      [ one "about" (ref "Movie")
      , one "note"  str
      ]
  ]

-- | Phantom source entity for the malformed-reference fixture.
data RefSource

-- | Phantom target entity for the malformed-reference fixture.
data RefTarget

-- | Storage needed to exercise one reference leaf independently of a schema.
data RefFixture = RefFixture
  { refSourceUniverse :: Universe RefSource
  , refBoxedColumn     :: SparseCol RefSource Int
  , refTargetUniverse :: Universe RefTarget
  }

-- | A deliberately malformed-reference fixture for the staged reference leaf.
--
-- Row 1 is a hole, row 3 points outside the target extent, and row 4 points to
-- a dead target. The boxed representation must drop those rows and agree
-- between driven enumeration and keyed access; loaded-schema tests below this
-- module exercise the same parity for both boxed and word-backed storage.
refFixture :: RefFixture
refFixture = RefFixture source boxed target
  where
    source = case denseUniverse 5 of
      Just universe -> universe
      Nothing -> error "refFixture: impossible negative source extent"
    target = universeFromMask [True, False, True]
    boxed = mkSparseCol [Just 0, Nothing, Just 2, Just 5, Just 1]

-- | Project the fixture's source universe into generated code.
refSourceDomain :: CodeQ RefFixture -> CodeQ (Universe RefSource)
refSourceDomain fixture = [|| refSourceUniverse $$fixture ||]

-- | Expose the fixture's boxed integer column as a validated typed reference.
boxedReference
  :: Mode q => CodeQ RefFixture -> q (Id RefSource) (Id RefTarget)
boxedReference fixture = referenceColumn
  (refSourceDomain fixture)
  [|| refBoxedColumn $$fixture ||]
  [|| refTargetUniverse $$fixture ||]
