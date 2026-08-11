{-# LANGUAGE DeriveLift #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | A small schema used to exercise the staged pull engine.
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
