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

-- | The left-associated product produced by @title.and(rating).and(company)@.
type Q22aRow = ((ByteString, ByteString), ByteString)

-- | Select movies matching the README predicates and return title, rating, and
-- production-company name.  The final 'Q.collect' is the sole request to drive
-- the relation; composition, product, and restriction select all probes.
q22a :: Q.Query JOBS [(Id Movie, Q22aRow)]
q22a = Q.query $ \s -> pure . Q.collect $
  movie s
    `restrict`
      ( compose (movieInfo s)
          ( Q.eq "countries" (compose (infoType s) (infoTypeText s))
            `prod` Q.oneOf ["Germany", "German", "USA", "American"] (infoValue s) )
        `prod` Q.oneOf ["murder", "murder-in-title", "blood", "violence"]
                 (compose (movieKeyword s) (keywordText s))
        `prod` Q.gt 2008 (productionYear s)
        `prod` Q.oneOf ["movie", "episode"]
                 (compose (movieKind s) (kindText s)) )
    `compose`
      ( movieTitle s
        `prod` ( movieData s
                   `restrict` ( Q.lt "7.0" (dataText s)
                                `prod` Q.eq "rating"
                                  (compose (dataType s) (infoTypeText s)) )
                   `compose` dataText s )
        `prod` ( movieCompany s
                   `restrict` ( Q.filterBy
                                  (Q.notS . Q.isInfixOf "(USA)") (companyNote s)
                                `prod` Q.filterBy
                                  (Q.orderedInfixOf "(200" ")") (companyNote s)
                                `prod` Q.ne "[us]" (companyCountry s)
                                `prod` Q.eq "production companies"
                                  (compose (companyType s) (companyTypeText s)) )
                   `compose` companyName s ) )

-- | Match the Rust JOB harness: take each output column's minimum independently
-- and join the three values with @ || @.  The algebra has already finished by
-- this point, so rendering remains ordinary unstaged Haskell.
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
