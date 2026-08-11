{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE RankNTypes #-}

-- | The staged twin of `Spec.schemaQueries`: the same five queries against the
-- same data, so Spec.hs can check the two engines agree row for row.
--
-- The interesting part is the shape of the module rather than the queries.
-- `schemaQuery` is a pure builder; `Q.compile` at the splice site turns it into
-- an ordinary function.
--
-- Three things follow from the split, and they are the three things every staged
-- query module will have.
--
-- The schema argument is a generation-time reference, so a leaf like
-- @Sch.title s@ describes a generated column read.
--
-- Each `Q.collect` and `Q.foldAll` emits its own loop. `Q.tuple5` assembles the
-- five generated scalar results without exposing quotation syntax.
--
-- And the @where@ preamble that buys back the bare spelling of each leaf works
-- exactly as it did before, signatures and all, for exactly the same reason:
-- `movie` is enumerated in one query and accessed through `Lookup` in another.
module StagedQueries (schemaQuery) where

import Data.ByteString (ByteString)
import Prela.PullStaged.Ops
import Prela.PullStaged.Stream
import qualified Prela.PullStaged.Query as Q
import Prela.Id (Id)

import qualified TinyStaged as Sch

schemaQuery :: Q.Query Sch.TinyS
               ([ByteString], Int, [ByteString], [ByteString], Double)
schemaQuery = Q.query build
  where
    build s =
      pure (Q.tuple5 (values sequels) recent (values undated) (values tv) best)
      where
        values q = Q.mapList Q.second (Q.collect q)

        movie :: SMode q => q (Id Sch.Movie) (Id Sch.Movie)
        movie = Sch.movie s
        title :: SMode q => q (Id Sch.Movie) ByteString
        title = Sch.title s
        year :: SMode q => q (Id Sch.Movie) Int
        year = Sch.year s
        rating :: SMode q => q (Id Sch.Movie) Double
        rating = Sch.rating s
        keyword :: SMode q => q (Id Sch.Movie) (Id Sch.Keyword)
        keyword = Sch.keyword s
        keywordText :: SMode q => q (Id Sch.Keyword) ByteString
        keywordText = Sch.keywordText s
        kind :: SMode q => q (Id Sch.Movie) (Id Sch.Kind)
        kind = Sch.kind s
        kindText :: SMode q => q (Id Sch.Kind) ByteString
        kindText = Sch.kindText s

        sequels :: Stream (Id Sch.Movie) ByteString
        sequels = compose (restrict movie (Q.eq "sequel" (compose keyword keywordText)))
                          title

        recent :: Q.Scalar Int
        recent = Q.foldAll (\n _ -> n + 1) 0
                         (compose (restrict movie (Q.gt 1980 year)) year)

        undated :: Stream (Id Sch.Movie) ByteString
        undated = compose (diff movie year) title

        tv :: Stream (Id Sch.Movie) ByteString
        tv = compose (restrict movie (Q.eq "tv series" (compose kind kindText))) title

        best :: Q.Scalar Double
        best = Q.foldAll (\a v -> Q.ifThenElse (a Q..>. v) a v) 0
                         (compose movie rating)
