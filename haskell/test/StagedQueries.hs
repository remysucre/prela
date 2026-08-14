{-# LANGUAGE OverloadedStrings #-}

-- | Focused integration queries over the small generated test schema.
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
-- Each `Q.collect` and `Q.foldAll` emits its own loop. Generated tuples retain
-- the left-associated binary-pair representation of relational @prod@ results;
-- the arity conveniences only hide the mechanical construction and elimination.
--
-- The @where@ preamble keeps the bare spelling of each leaf, signatures and
-- all. @movie@ is enumerated in one query and probed in another; the surrounding
-- operator chooses that mode without an explicit conversion.
module StagedQueries
  ( schemaQuery, dictionaryQuery, topKQuery, sharedRelationQuery
  , regexQuery, tupleHelpersQuery, referenceQuery, loadedReferenceQuery
  ) where

import Data.ByteString (ByteString)
import qualified Data.Vector as BV
import qualified Prela.PullStaged.Ops as O
import Prela.PullStaged.Query
  ( Relation, compose, diff, groupBy, restrict )
import Prela.PullStaged.Stream (Lookup, Stream)
import qualified Prela.PullStaged.Query as Q
import Prela.Id (Id)

import qualified TinyStaged as Sch

-- | Combine navigation, restriction, difference, aggregation, and products in
-- one generated tiny-schema query.
schemaQuery :: Q.Query Sch.TinyS
               (((([ByteString], Int), [ByteString]), [ByteString]), Double)
schemaQuery = Q.query $ \s ->
  let values q = Q.mapList Q.second (Q.collect q)

      movie :: Relation (Id Sch.Movie) (Id Sch.Movie)
      movie = Sch.movie s
      title :: Relation (Id Sch.Movie) ByteString
      title = Sch.title s
      year :: Relation (Id Sch.Movie) Int
      year = Sch.year s
      rating :: Relation (Id Sch.Movie) Double
      rating = Sch.rating s
      keyword :: Relation (Id Sch.Movie) (Id Sch.Keyword)
      keyword = Sch.keyword s
      keywordText :: Relation (Id Sch.Keyword) ByteString
      keywordText = Sch.keywordText s
      kind :: Relation (Id Sch.Movie) (Id Sch.Kind)
      kind = Sch.kind s
      kindText :: Relation (Id Sch.Kind) ByteString
      kindText = Sch.kindText s

      sequels :: Relation (Id Sch.Movie) ByteString
      sequels = compose (restrict movie (Q.eq "sequel" (compose keyword keywordText)))
                        title

      recent :: Q.Scalar Int
      recent = Q.foldAll (\n _ -> n + 1) 0
                       (compose (restrict movie (Q.gt 1980 year)) year)

      undated :: Relation (Id Sch.Movie) ByteString
      undated = compose (diff movie year) title

      tv :: Relation (Id Sch.Movie) ByteString
      tv = compose (restrict movie (Q.eq "tv series" (compose kind kindText))) title

      best :: Q.Scalar Double
      best = Q.foldAll (\a v -> Q.ifThenElse (a Q..>. v) a v) 0
                       (compose movie rating)
  in pure (Q.tuple5 (values sequels) recent (values undated) (values tv) best)

-- | Exercise compact value coding and its bounded distinct-count consumer on a
-- tiny schema, independently of the TPC-H integration benchmark.
dictionaryQuery :: Q.Query Sch.TinyS (BV.Vector ByteString, [(Int, Int)])
dictionaryQuery = Q.query $ \s -> do
  (kindCodes, labels) <- Q.dictionary
    (Sch.movieExtent s)
    (compose (Sch.movie s) (compose (Sch.kind s) (Sch.kindText s)))
  let grouped :: Stream Int (Id Sch.Keyword)
      grouped = compose
        (groupBy (Sch.movie s) kindCodes)
        (Sch.keyword s)
  counts <- Q.denseDistinctCount
    (Sch.movieExtent s) (Sch.keywordExtent s) grouped
  pure (Q.pair labels (Q.collect counts))

-- | Check ordered retention, preserved keys, and a zero-capacity top-k buffer.
-- A bounded materializer must order retained rows, preserve their keys for
-- downstream composition, and handle a zero-sized buffer without touching it.
topKQuery
  :: Q.Query Sch.TinyS
       ([(Id Sch.Movie, Double)], [(Id Sch.Movie, Double)])
topKQuery = Q.query $ \s -> do
  let ratings :: Relation (Id Sch.Movie) Double
      ratings = compose (Sch.movie s) (Sch.rating s)
      descending leftKey leftRating rightKey rightRating =
        Q.compare rightRating leftRating `Q.thenCompare`
        Q.compare (Q.idIndex leftKey) (Q.idIndex rightKey)
  best <- Q.topK 2 descending ratings
  none <- Q.topK 0 descending ratings
  pure (Q.pair (Q.collect best) (Q.collect none))

-- | Enumerate and probe the same dense materializer beneath one generated bind.
-- One dense materializer is enumerated on the left and probed on the right.
-- Both uses remain beneath the single generated binding introduced by
-- @denseFold@; no stream/keyed choice appears in the query.
sharedRelationQuery
  :: Q.Query Sch.TinyS
       ([(Id Sch.Movie, Int)], [(Id Sch.Movie, Int)])
sharedRelationQuery = Q.query $ \s -> do
  years <- Q.denseFold (Sch.movieExtent s) (\_ year -> year) 0
    (compose (Sch.movie s) (Sch.year s))
  pure (Q.pair (Q.collect years)
               (Q.collect (compose (Sch.movie s) years)))

-- | Compile two regular expressions once and apply them to a generated column.
regexQuery :: Q.Query Sch.TinyS (Int, Int)
regexQuery = Q.query $ \s ->
  Q.pair <$> (Q.count . (`Q.rx` Sch.title s) <$> Q.regex "^Alien$")
         <*> (Q.count . (`Q.rx` Sch.title s) <$> Q.regex "^Alien.*")

-- | Exercise every generated tuple constructor and matching eliminator.
tupleHelpersQuery :: Q.Query Sch.TinyS (((Int, Int), Int), Int)
tupleHelpersQuery = Q.query $ \_ ->
  let sum3 = Q.onTuple3 (\a b c -> a + b + c) (Q.tuple3 1 2 3)
      sum4 = Q.onTuple4 (\a b c d -> a + b + c + d) (Q.tuple4 1 2 3 4)
      sum5 = Q.onTuple5 (\a b c d e -> a + b + c + d + e)
               (Q.tuple5 1 2 3 4 5)
      sum6 = Q.onTuple6 (\a b c d e f -> a + b + c + d + e + f)
               (Q.tuple6 1 2 3 4 5 6)
  in pure (Q.tuple4 sum3 sum4 sum5 sum6)

-- | Compare driven and keyed access to deliberately malformed boxed references.
--
-- The direct reference leaf must agree in its Stream and Lookup modes, and its
-- checked boxed and trusted word-backed storage representations must have the
-- same semantics for holes, invalid targets, and dead targets.
referenceQuery
  :: Q.Query Sch.RefFixture
       ([(Id Sch.RefSource, Id Sch.RefTarget)],
        [(Id Sch.RefSource, Id Sch.RefTarget)])
referenceQuery = Q.query $ \fixture ->
  let boxedDriven :: Stream (Id Sch.RefSource) (Id Sch.RefTarget)
      boxedDriven = Sch.boxedReference fixture
      boxedKeyed :: Lookup (Id Sch.RefSource) (Id Sch.RefTarget)
      boxedKeyed = Sch.boxedReference fixture
      sources :: Stream (Id Sch.RefSource) (Id Sch.RefSource)
      sources = O.universe (Sch.refSourceDomain fixture)
  in pure (Q.pair (Q.collect boxedDriven)
                  (Q.collect (O.compose sources boxedKeyed)))

-- | Compare driven and keyed access under both generated schema loaders.
--
-- Loaded schemas exercise boxed storage under the checked loader and
-- word-backed storage under the trusted loader. Enumerating the reference leaf
-- directly must agree with probing it from the source universe.
loadedReferenceQuery
  :: Q.Query Sch.TinyS
       ([(Id Sch.Movie, Id Sch.Kind)], [(Id Sch.Movie, Id Sch.Kind)])
loadedReferenceQuery = Q.query $ \schema ->
  let reference = Sch.kind schema
      sources = Sch.movie schema
  in pure (Q.pair (Q.collect reference)
                  (Q.collect (compose sources reference)))
