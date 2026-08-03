{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The same schema as TinySchema.hs, spliced for the staged engine.
--
-- It has to be a separate module because the two flavours generate the same
-- accessor names. When the push engine goes away this file replaces
-- TinySchema.hs and the record type goes back to being called @Tiny@.
--
-- Everything except the accessors is identical: the tag types, the record, and
-- `loadTinyS` are generated from the same code, because none of them mentions a
-- relation.
module TinyStaged where

import Prela.Schema

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
