{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | A schema small enough to check by hand, used by Spec.hs. Also the worked
-- example of what a declaration looks like, since the JOB one will be this
-- shape with twenty more entities.
--
-- The three pragmas above are what a schema module needs: TemplateHaskell for
-- the splice, RankNTypes for the generated `forall q. Mode q =>` signatures.
module TinySchema where

import Prela.Schema

-- Movie is declared first, but `ref "Keyword"` names an entity whose tag does
-- not exist yet. That resolves because a splice produces all its declarations at
-- once and Haskell's top level has no ordering, so mutual references between
-- entities need no forward declaration. Julia needs `@declare` for this.
declareSchema "Tiny"
  [ entity "Movie" "movie"
      [ one  "title"    str
      , one  "kind"     (ref "Kind")
      , many "year"     int
      , many "keyword"  (ref "Keyword")
      , one  "rating"   dbl
      ]
  -- The universes are plural because Movie's foreign keys claim the singular
  -- names. The Rust JOB schema resolves the same collision the same way.
  , entity "Keyword" "keywords"
      [ one "text" str `as` "keywordText" ]
  , entity "Kind" "kinds"
      [ one "text" str `as` "kindText" ]
  -- A sparse entity: its ids run 0..n-1 like any other, but the ones whose
  -- `about` foreign key is a hole are dead and driving `links` skips them.
  , sparseEntity "Link" "links"
      [ one "about" (ref "Movie")
      , one "note"  str
      ]
  ]
