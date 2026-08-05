{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The staged twin of `Spec.schemaQueries`: the same five queries against the
-- same data, so Spec.hs can check the two engines agree row for row.
--
-- The interesting part is the shape of the module rather than the queries. This
-- is a GENERATOR: `schemaQueriesS` does not run a query, it builds the code of
-- one, and something else splices it. Under the push engine there was no such
-- split — the query was an ordinary function and Spec.hs called it.
--
-- Three things follow from the split, and they are the three things every staged
-- query module will have.
--
-- The record comes in as @CodeQ Sch.TinyS@ rather than @TinyS@, so a leaf like
-- @Sch.title s@ builds the code @title tiny@ instead of reading a column.
--
-- The result is @CodeQ (…)@, and the whole tuple is assembled inside one quote.
-- Each `collect` and each `foldAll` emits its own loop, which is what running
-- five queries means.
--
-- And the @where@ preamble that buys back the bare spelling of each leaf works
-- exactly as it did before, signatures and all, for exactly the same reason:
-- `movie` is driven in one query and probed in another.
module StagedQueries (schemaQueriesS) where

import Data.ByteString (ByteString)
import Language.Haskell.TH (CodeQ)

import Prela.PullStaged.Ops
import Prela.PullStaged.Predicate
import Prela.PullStaged.Stream
import Prela.Storage (Id)

import qualified TinyStaged as Sch

schemaQueriesS :: CodeQ Sch.TinyS
               -> CodeQ ([ByteString], Int, [ByteString], [ByteString], Double)
schemaQueriesS s = [|| ( $$(values sequels)
                       , $$recent
                       , $$(values undated)
                       , $$(values tv)
                       , $$best
                       ) ||]
  where
    values q = [|| map snd $$(collect q) ||]

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

    sequels :: Drive (Id Sch.Movie) ByteString
    sequels = compose (restrict movie (eq [|| "sequel" ||] (compose keyword keywordText)))
                      title

    recent :: CodeQ Int
    recent = foldAll (\n _ -> [|| $$n + 1 ||]) [|| 0 ||]
                     (compose (restrict movie (gt [|| 1980 ||] year)) year)

    undated :: Drive (Id Sch.Movie) ByteString
    undated = compose (diff movie year) title

    tv :: Drive (Id Sch.Movie) ByteString
    tv = compose (restrict movie (eq [|| "tv series" ||] (compose kind kindText))) title

    best :: CodeQ Double
    best = foldAll (\a v -> [|| max $$a $$v ||]) [|| 0 ||] (compose movie rating)
