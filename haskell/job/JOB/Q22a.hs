{-# LANGUAGE OverloadedStrings #-}

-- | A direct staged-Haskell translation of the Rust README's JOB query 22a.
module JOB.Q22a
  ( Q22aRow
  , q22a
  , renderQ22a
  ) where

import Data.ByteString (ByteString)
import qualified Data.ByteString.Char8 as BS
import qualified Data.List as List

import JOB.StagedSchema
import Prela.Id (Id)
import Prela.PullStaged.Query
  ( compose, prod, restrict )
import qualified Prela.PullStaged.Query as Q
type Q22aRow = ((ByteString, ByteString), ByteString)

q22a :: Q.Query JOBS [(Id Movie, Q22aRow)]
q22a = Q.query $ \s ->
  Q.regex "\\(USA\\)" >>= \usa ->
  Q.regex "\\(200.*\\)" >>= \year ->
  pure . Q.collect $
    movie s
      `restrict`
        (compose (movieInfo s)
            ( Q.eq "countries" (compose (infoType s) (infoTypeText s))
              `prod` Q.oneOf ["Germany", "German", "USA", "American"] (infoValue s) )
          `prod` Q.oneOf ["murder", "murder-in-title", "blood", "violence"]
                   (compose (movieKeyword s) (keywordText s))
          `prod` Q.gt 2008 (productionYear s)
          `prod` Q.oneOf ["movie", "episode"]
                   (compose (movieKind s) (kindText s)) )
      `compose`
        (movieTitle s
          `prod` (movieData s
                     `restrict` ( Q.lt "7.0" (dataText s)
                                  `prod` Q.eq "rating"
                                    (compose (dataType s) (infoTypeText s)) )
                     `compose` dataText s )
          `prod` (movieCompany s
                     `restrict` ( Q.nrx usa (companyNote s)
                                  `prod` Q.rx year (companyNote s)
                                  `prod` Q.ne "[us]" (companyCountry s)
                                  `prod` Q.eq "production companies"
                                    (compose (companyType s) (companyTypeText s)) )
                     `compose` companyName s ) )

renderQ22a :: [(Id Movie, Q22aRow)] -> String
renderQ22a [] = "(empty)"
renderQ22a ((_, firstRow) : rest) =
  let ((title, rating), company) =
        List.foldl' minColumns firstRow (map snd rest)
  in BS.unpack title ++ " || " ++ BS.unpack rating ++ " || " ++ BS.unpack company
  where
    minColumns ((titleA, ratingA), companyA)
               ((titleB, ratingB), companyB) =
      ((min titleA titleB, min ratingA ratingB), min companyA companyB)
