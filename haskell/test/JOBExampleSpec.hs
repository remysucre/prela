{-# LANGUAGE OverloadedStrings #-}

-- | End-to-end fixture for the README JOB query translation.
module Main (main) where

import Control.Monad (unless)
import System.Directory (createDirectoryIfMissing, getTemporaryDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>))

import qualified JOB.Staged as Query
import JOB.StagedSchema
import Prela.Cache
import Prela.Id (Id, Universe, denseUniverse, lookupId)

main :: IO ()
main = do
  tmp <- getTemporaryDirectory
  let cacheDir = tmp </> "prela-job-readme-example-test"
  createDirectoryIfMissing True cacheDir
  writeFixture cacheDir

  checked <- loadJOBSChecked cacheDir
  fast <- loadJOBS cacheDir
  let expected = "Alpha || 5.5 || Alpha Studios"
      actualChecked = Query.runQ22a checked
      actualFast = Query.runQ22a fast
  unless (actualChecked == expected && actualFast == expected) $ do
    putStrLn ("FAIL JOB README q22a\n  checked " ++ show actualChecked
              ++ "\n  fast    " ++ show actualFast
              ++ "\n  want    " ++ show expected)
    exitFailure
  putStrLn "JOB README q22a check ok"

-- | Write three movies: two qualifying rows and one row rejected by the year,
-- keyword, and country predicates.  The two survivors also verify Rust's
-- column-wise-minimum rendering rather than lexicographic row minimum.
writeFixture :: FilePath -> IO ()
writeFixture cacheDir = do
  kindDomain <- universe "kinds" 2
  movieKindId <- identifier "movie kind" kindDomain 0
  episodeKindId <- identifier "episode kind" kindDomain 1

  keywordDomain <- universe "keywords" 3
  murderId <- identifier "murder keyword" keywordDomain 0
  violenceId <- identifier "violence keyword" keywordDomain 1
  romanceId <- identifier "romance keyword" keywordDomain 2

  companyDomain <- universe "companies" 3
  zetaCompany <- identifier "zeta company" companyDomain 0
  alphaCompany <- identifier "alpha company" companyDomain 1
  usCompany <- identifier "US company" companyDomain 2

  infoDomain <- universe "infos" 3
  germanyInfo <- identifier "Germany info" infoDomain 0
  usaInfo <- identifier "USA info" infoDomain 1
  franceInfo <- identifier "France info" infoDomain 2

  dataDomain <- universe "data" 3
  rating65 <- identifier "6.5 rating" dataDomain 0
  rating80 <- identifier "8.0 rating" dataDomain 1
  rating55 <- identifier "5.5 rating" dataDomain 2

  companyTypeDomain <- universe "company types" 2
  productionCompany <- identifier "production company type" companyTypeDomain 0
  distributor <- identifier "distributor type" companyTypeDomain 1

  infoTypeDomain <- universe "info types" 2
  countries <- identifier "countries info type" infoTypeDomain 0
  rating <- identifier "rating info type" infoTypeDomain 1

  writeStrs cacheDir "Movie_title" ["Zulu", "Alpha", "Excluded"]
  writeIds cacheDir "Movie_kind"
    [Just movieKindId, Just episodeKindId, Just movieKindId]
  writeMultiInts cacheDir "Movie_production_year" [[2009], [2012], [2007]]
  writeMultiIds cacheDir "Movie_keyword" [[murderId], [violenceId], [romanceId]]
  writeMultiIds cacheDir "Movie_company" [[zetaCompany], [alphaCompany], [usCompany]]
  writeMultiIds cacheDir "Movie_info" [[germanyInfo], [usaInfo], [franceInfo]]
  writeMultiIds cacheDir "Movie_data" [[rating65], [rating55], [rating80]]

  writeStrs cacheDir "Keyword_text" ["murder", "violence", "romance"]
  writeStrs cacheDir "Kind_text" ["movie", "episode"]

  writeStrs cacheDir "Company_name" ["Zeta Films", "Alpha Studios", "US Company"]
  writeMultiStrs cacheDir "Company_country" [["[ca]"], ["[de]"], ["[us]"]]
  writeMultiStrs cacheDir "Company_note" [["(2005)"], ["(2001)"], ["(2008)"]]
  writeIds cacheDir "Company_ty"
    [Just productionCompany, Just productionCompany, Just distributor]
  writeStrs cacheDir "CompanyType_text" ["production companies", "distributors"]

  writeStrs cacheDir "Info_info" ["Germany", "USA", "France"]
  writeIds cacheDir "Info_ty" [Just countries, Just countries, Just countries]
  writeStrs cacheDir "InfoType_text" ["countries", "rating"]

  writeStrs cacheDir "Data_text" ["6.5", "8.0", "5.5"]
  writeIds cacheDir "Data_ty" [Just rating, Just rating, Just rating]

universe :: String -> Int -> IO (Universe entity)
universe label size =
  maybe (fail (label ++ ": invalid universe size")) pure (denseUniverse size)

identifier :: String -> Universe entity -> Int -> IO (Id entity)
identifier label domain index =
  maybe (fail (label ++ ": invalid identifier")) pure (lookupId domain index)
