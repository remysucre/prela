-- | The focused Join Order Benchmark schema needed by the README's query 22a.
--
-- The cache names and physical column shapes match the Rust JOB schema.  This
-- intentionally declares only the entities and fields touched by the example;
-- adding the other JOB queries does not require changing the query engine, only
-- extending this schema value with their columns.
module JOB.Decl (jobExampleEntities) where

import Prela.Schema

-- | Cache-backed entities used by JOB 22a.
--
-- Repeated field names receive entity-qualified Haskell accessors while keeping
-- their original cache filenames.  For example, @companyName@ still loads
-- @Company_name.bin@.
jobExampleEntities :: [Ent]
jobExampleEntities =
  [ entity "Movie" "movie"
      [ one  "title"           str             `as` "movieTitle"
      , one  "kind"            (ref "Kind")    `as` "movieKind"
      , many "production_year" int             `as` "productionYear"
      , many "keyword"         (ref "Keyword") `as` "movieKeyword"
      , many "company"         (ref "Company") `as` "movieCompany"
      , many "info"            (ref "Info")    `as` "movieInfo"
      , many "data"            (ref "Data")    `as` "movieData"
      ]
  , entity "Keyword" "keywords"
      [ one "text" str `as` "keywordText" ]
  , entity "Kind" "kinds"
      [ one "text" str `as` "kindText" ]
  , entity "Company" "companies"
      [ one  "name"    str                 `as` "companyName"
      , many "country" str                 `as` "companyCountry"
      , many "note"    str                 `as` "companyNote"
      , one  "ty"      (ref "CompanyType") `as` "companyType"
      ]
  , entity "CompanyType" "companyTypes"
      [ one "text" str `as` "companyTypeText" ]
  , entity "Info" "infos"
      [ one "info" str              `as` "infoValue"
      , one "ty"   (ref "InfoType") `as` "infoType"
      ]
  , entity "InfoType" "infoTypes"
      [ one "text" str `as` "infoTypeText" ]
  , entity "Data" "datas"
      [ one "text" str              `as` "dataText"
      , one "ty"   (ref "InfoType") `as` "dataType"
      ]
  ]
