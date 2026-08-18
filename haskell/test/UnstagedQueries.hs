{-# LANGUAGE OverloadedStrings #-}

-- | Runtime counterparts of the small staged integration queries.
module UnstagedQueries
  ( unstagedQueries
  , unstagedLoadedReferences
  , unstagedReferences
  ) where

import Data.ByteString (ByteString)
import Prela.Id (Id)
import qualified Prela.Pull.Ops as O
import Prela.Pull.Query (Relation, compose, diff, restrict)
import qualified Prela.Pull.Query as Q

import qualified TinyStaged as Sch

unstagedQueries
  :: Sch.TinyS -> ([ByteString], Int, [ByteString], [ByteString], Double)
unstagedQueries schema = (values sequels, recent, values undated, values tv, best)
  where
    values = map snd . Q.collect

    movie :: Relation (Id Sch.Movie) (Id Sch.Movie)
    movie = O.universe (Sch.movie_universe schema)
    title :: Relation (Id Sch.Movie) ByteString
    title = O.column (Sch.movie_title schema)
    year :: Relation (Id Sch.Movie) Int
    year = O.multiColumn (Sch.movie_year schema)
    rating :: Relation (Id Sch.Movie) Double
    rating = O.column (Sch.movie_rating schema)
    keywordIndex :: Relation (Id Sch.Movie) Int
    keywordIndex = O.multiColumn (Sch.movie_keyword schema)
    keyword :: Relation (Id Sch.Movie) (Id Sch.Keyword)
    keyword = compose
      (compose movie keywordIndex)
      (O.resolveId (Sch.keyword_universe schema))
    keywordText :: Relation (Id Sch.Keyword) ByteString
    keywordText = O.column (Sch.keyword_text schema)
    kind :: Relation (Id Sch.Movie) (Id Sch.Kind)
    kind = O.referenceColumn
      (Sch.movie_universe schema)
      (Sch.movie_kind schema)
      (Sch.kind_universe schema)
    kindText :: Relation (Id Sch.Kind) ByteString
    kindText = O.column (Sch.kind_text schema)

    sequels = compose
      (restrict movie (Q.eq "sequel" (compose keyword keywordText)))
      title
    recent = Q.foldAll (\n _ -> n + 1) 0
      (compose (restrict movie (Q.gt 1980 year)) year)
    undated = compose (diff movie year) title
    tv = compose
      (restrict movie (Q.eq "tv series" (compose kind kindText)))
      title
    best = Q.foldAll max 0 (compose movie rating)

unstagedLoadedReferences
  :: Sch.TinyS
  -> ([(Id Sch.Movie, Id Sch.Kind)], [(Id Sch.Movie, Id Sch.Kind)])
unstagedLoadedReferences schema =
  (Q.collect relation, Q.collect (compose movie relation))
  where
    movie :: Relation (Id Sch.Movie) (Id Sch.Movie)
    movie = O.universe (Sch.movie_universe schema)
    relation :: Relation (Id Sch.Movie) (Id Sch.Kind)
    relation = O.referenceColumn
      (Sch.movie_universe schema)
      (Sch.movie_kind schema)
      (Sch.kind_universe schema)

unstagedReferences
  :: Sch.RefFixture
  -> ([(Id Sch.RefSource, Id Sch.RefTarget)],
      [(Id Sch.RefSource, Id Sch.RefTarget)])
unstagedReferences fixture =
  (Q.collect relation, Q.collect (compose source relation))
  where
    source :: Relation (Id Sch.RefSource) (Id Sch.RefSource)
    source = O.universe (Sch.refSourceUniverse fixture)
    relation :: Relation (Id Sch.RefSource) (Id Sch.RefTarget)
    relation = O.referenceColumn
      (Sch.refSourceUniverse fixture)
      (Sch.refBoxedColumn fixture)
      (Sch.refTargetUniverse fixture)
