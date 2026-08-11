{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE FunctionalDependencies #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Generate staged schema storage, checked loaders, and query accessors from a
-- compact entity declaration.
module Prela.Schema
  ( Ty, str, int, dbl, ref
  , Field, one, many, as
  , Ent, entity, sparseEntity
  , declareStagedSchema
  , Project(..), fieldCode
  ) where

import Control.Monad (unless)
import Data.ByteString (ByteString)
import Data.Char (toLower, toUpper)
import Data.List (intercalate)
import Language.Haskell.TH
import Language.Haskell.TH.Syntax (Lift, liftTyped)

import Prela.Cache
import Prela.Id
import qualified Prela.PullStaged.Ops as S
import qualified Prela.PullStaged.Query as Q
import Prela.Storage

data Ty = TInt | TDouble | TStr | TRef String

str, int, dbl :: Ty
str = TStr
int = TInt
dbl = TDouble

ref :: String -> Ty
ref = TRef

data Field = Field
  { fFile :: String
  , fAs   :: String
  , fTy   :: Ty
  , fMany :: Bool
  }

one :: String -> Ty -> Field
one name ty = Field name name ty False

many :: String -> Ty -> Field
many name ty = Field name name ty True

as :: Field -> String -> Field
as field name = field { fAs = name }

data Ent = Ent
  { eName   :: String
  , eUniv   :: String
  , eFields :: [Field]
  , eSparse :: Bool
  }

entity :: String -> String -> [Field] -> Ent
entity name universeName fields = Ent name universeName fields False

-- | A sparse entity derives its live-ID mask from absence in its first field.
-- That field must therefore be a one-valued reference.
sparseEntity :: String -> String -> [Field] -> Ent
sparseEntity name universeName fields = Ent name universeName fields True

declareStagedSchema :: String -> [Ent] -> Q [Dec]
declareStagedSchema schemaName entities
  | null entities = fail "declareStagedSchema: no entities"
  | any (null . eFields) entities =
      fail "declareStagedSchema: every entity needs at least one field"
  | (name, owners) : _ <- clashes =
      fail ("declareStagedSchema: the name " ++ show name ++ " is claimed by "
            ++ intercalate " and " owners ++ ".\nRename the universe or field accessor.")
  | (ent, field) : _ <- invalidSparse =
      fail ("declareStagedSchema: " ++ eName ent
            ++ " is sparse, so its first field must be a one-valued reference, but it is "
            ++ show (fFile field))
  | otherwise = do
      let recordName = mkName schemaName
      tags <- mapM tagDeclaration entities
      record <- recordDeclaration recordName entities
      projections <- projectionDeclarations recordName entities
      loader <- loaderDeclarations recordName entities
      accessors <- concat <$> mapM (accessorDeclarations recordName) entities
      pure (tags ++ [record] ++ projections ++ loader ++ accessors)
  where
    claimed = [ (eUniv ent, "the " ++ eName ent ++ " universe") | ent <- entities ]
           ++ [ (extentAccessorName ent, "the " ++ eName ent ++ " extent")
              | ent <- entities ]
           ++ [ (fAs field, eName ent ++ "." ++ fFile field)
              | ent <- entities, field <- eFields ent ]
    clashes = [ (name, owners)
              | (name, _) <- claimed
              , let owners = [owner | (other, owner) <- claimed, other == name]
              , length owners > 1 ]
    invalidSparse =
      [ (ent, field)
      | ent <- entities
      , eSparse ent
      , field : _ <- [eFields ent]
      , case (fMany field, fTy field) of
          (False, TRef _) -> False
          _               -> True
      ]

tagDeclaration :: Ent -> Q Dec
tagDeclaration ent = dataD (pure []) (mkName (eName ent)) [] Nothing [] []

recordDeclaration :: Name -> [Ent] -> Q Dec
recordDeclaration recordName entities = do
  fields <- concat <$> mapM entityFields entities
  dataD (pure []) recordName [] Nothing [recC recordName (map pure fields)] []
  where
    strict = Bang NoSourceUnpackedness SourceStrict
    entityFields ent = do
      universeType <- [t| Universe $(conT (mkName (eName ent))) |]
      columns <- mapM (columnField ent) (eFields ent)
      pure ((universeFieldName ent, strict, universeType) : columns)
    columnField ent field = do
      ty <- columnType ent field
      pure (columnFieldName ent field, strict, ty)

-- Each generated record selector gets a small, liftable tag and a 'Project'
-- instance.  Accessors pass that concrete tag to 'fieldCode'.  This keeps the
-- generated selection fully typechecked without constructing typed TH syntax
-- by hand or coercing an untyped expression into 'CodeQ'.
projectionDeclarations :: Name -> [Ent] -> Q [Dec]
projectionDeclarations recordName entities = concat <$> sequence
  [ entityProjections ent | ent <- entities ]
  where
    entityProjections ent = do
      universeType <- [t| Universe $(conT (mkName (eName ent))) |]
      universeProjection <- projection (universeFieldName ent) (pure universeType)
      columnProjections <- sequence
        [ projection (columnFieldName ent field) (columnType ent field)
        | field <- eFields ent
        ]
      pure (universeProjection ++ concat columnProjections)

    projection selector valueType = do
      value <- valueType
      let tagName = projectionTagName recordName selector
          tagType = conT tagName
      tag <- dataD (pure []) tagName [] Nothing
        [normalC tagName []]
        [derivClause Nothing [conT ''Lift]]
      instanceDeclaration <- instanceD (pure [])
        [t| Project $tagType $(conT recordName) $(pure value) |]
        [funD 'projectField
          [clause [wildP, varP (mkName "record")]
            (normalB (appE (varE selector) (varE (mkName "record")))) []]]
      pure [tag, instanceDeclaration]

projectionTagName :: Name -> Name -> Name
projectionTagName recordName selector =
  mkName (nameBase recordName ++ upperFirst (nameBase selector) ++ "Field")

upperFirst :: String -> String
upperFirst (first : rest) = toUpper first : rest
upperFirst [] = []

loaderDeclarations :: Name -> [Ent] -> Q [Dec]
loaderDeclarations recordName entities = do
  directory <- newName "directory"
  groups <- mapM binders entities
  columnStatements <- sequence
    [ bindS (varP value)
        [| $(loadFunction field) $(varE directory)
             $(litE (stringL (eName ent ++ "_" ++ fFile field))) |]
    | (ent, _, fields) <- groups
    , (field, value) <- fields
    ]
  universeStatements <- mapM universeStatement groups
  let arguments = concat
        [ VarE universeValue : map (VarE . snd) fields
        | (_, universeValue, fields) <- groups
        ]
      constructor = foldl AppE (ConE recordName) arguments
      body = doE (map pure (columnStatements ++ universeStatements)
                  ++ [noBindS (appE [| pure |] (pure constructor))])
      loaderName = mkName ("load" ++ nameBase recordName)
  signature <- sigD loaderName [t| FilePath -> IO $(conT recordName) |]
  function <- funD loaderName [clause [varP directory] (normalB body) []]
  pure [signature, function]
  where
    binders ent = do
      universeValue <- newName "universe"
      fields <- mapM (\field -> (,) field <$> newName "column") (eFields ent)
      pure (ent, universeValue, fields)

    universeStatement (ent, universeValue, fields) = do
      lengths <- listE [columnLength field value | (field, value) <- fields]
      expression <- case fields of
        [] -> fail "declareStagedSchema: entity with no fields"
        ((_, firstValue) : _) | eSparse ent ->
          [| checkedSparseUniverse $(litE (stringL (eName ent)))
                                    $(varE firstValue) $(pure lengths) |]
        _ -> [| checkedDenseUniverse $(litE (stringL (eName ent))) $(pure lengths) |]
      bindS (varP universeValue) (pure expression)

checkedDenseUniverse :: String -> [Int] -> IO (Universe e)
checkedDenseUniverse entityName lengths = do
  size <- checkedExtent entityName lengths
  case denseUniverse size of
    Just result -> pure result
    Nothing -> ioError (userError (entityName ++ ": negative universe size"))

checkedSparseUniverse :: String -> SparseCol e Int -> [Int] -> IO (Universe e)
checkedSparseUniverse entityName firstColumn lengths = do
  size <- checkedExtent entityName lengths
  let result = validityUniverse firstColumn
  unless (universeSize result == size) $
    ioError (userError (entityName ++ ": validity mask length disagrees with its columns"))
  pure result

checkedExtent :: String -> [Int] -> IO Int
checkedExtent entityName lengths = case lengths of
  [] -> ioError (userError (entityName ++ ": entity has no columns"))
  size : rest -> do
    unless (all (== size) rest) $
      ioError (userError (entityName ++ ": columns have inconsistent lengths"))
    pure size

accessorDeclarations :: Name -> Ent -> Q [Dec]
accessorDeclarations recordName ent = do
  universeAccessor <- makeUniverseAccessor
  extentAccessor <- makeExtentAccessor
  fieldAccessors <- concat <$> mapM makeFieldAccessor (eFields ent)
  pure (universeAccessor ++ extentAccessor ++ fieldAccessors)
  where
    tag = conT (mkName (eName ent))
    recordCode = [t| CodeQ $(conT recordName) |]

    get record field =
      appE (appE [| fieldCode |]
                 (conE (projectionTagName recordName field)))
           (varE record)

    makeUniverseAccessor = do
      record <- newName "schema"
      let name = mkName (eUniv ent)
          domain = get record (universeFieldName ent)
      signature <- sigD name
        [t| forall q. S.SMode q => $recordCode -> q (Id $tag) (Id $tag) |]
      function <- funD name [clause [varP record] (normalB [| S.universe $domain |]) []]
      inlinePragma <- pragInlD name Inline FunLike AllPhases
      pure [signature, function, inlinePragma]

    makeExtentAccessor = do
      record <- newName "schema"
      let name = mkName (extentAccessorName ent)
          domain = get record (universeFieldName ent)
      signature <- sigD name [t| $recordCode -> Q.Scalar Int |]
      function <- funD name
        [clause [varP record] (normalB [| Q.extent $domain |]) []]
      inlinePragma <- pragInlD name Inline FunLike AllPhases
      pure [signature, function, inlinePragma]

    makeFieldAccessor field = do
      record <- newName "schema"
      let name = mkName (fAs field)
          sourceDomain = get record (universeFieldName ent)
          column = get record (columnFieldName ent field)
          rawLeaf
            | fMany field = [| S.multiColumn $column |]
            | isReference field = [| S.sparseColumn $column |]
            | otherwise = [| S.column $column |]
          liveRows = [| S.compose (S.universe $sourceDomain) $rawLeaf |]
          body = case fTy field of
            TRef target ->
              let targetDomain = get record (universeFieldNameByName target)
              in [| S.compose $liveRows (S.resolveId $targetDomain) |]
            _ -> liveRows
      signature <- sigD name
        [t| forall q. S.SMode q => $recordCode -> q (Id $tag) $(elementType (fTy field)) |]
      function <- funD name [clause [varP record] (normalB body) []]
      inlinePragma <- pragInlD name Inline FunLike AllPhases
      pure [signature, function, inlinePragma]

    isReference field = case fTy field of
      TRef _ -> True
      _      -> False

-- | A typed record projection selected by a generated, liftable field tag.
class Project field record value | field -> record value where
  projectField :: field -> record -> value

-- | Quote a record projection without an unchecked typed-code coercion.
fieldCode :: (Lift field, Project field record value)
          => field -> CodeQ record -> CodeQ value
fieldCode field record = [|| projectField $$(liftTyped field) $$record ||]

columnFieldName :: Ent -> Field -> Name
columnFieldName ent field = mkName (lowerFirst (eName ent) ++ "_" ++ fFile field)

universeFieldName :: Ent -> Name
universeFieldName = universeFieldNameByName . eName

universeFieldNameByName :: String -> Name
universeFieldNameByName entityName = mkName (lowerFirst entityName ++ "_universe")

extentAccessorName :: Ent -> String
extentAccessorName ent = lowerFirst (eName ent) ++ "Extent"

lowerFirst :: String -> String
lowerFirst (first : rest) = toLower first : rest
lowerFirst [] = []

elementType :: Ty -> Q Type
elementType TInt = [t| Int |]
elementType TDouble = [t| Double |]
elementType TStr = [t| ByteString |]
elementType (TRef target) = [t| Id $(conT (mkName target)) |]

physicalElementType :: Ty -> Q Type
physicalElementType (TRef _) = [t| Int |]
physicalElementType ty = elementType ty

columnType :: Ent -> Field -> Q Type
columnType ent field
  | fMany field = [t| MultiCol $tag $element |]
  | isReference = [t| SparseCol $tag Int |]
  | otherwise = [t| Col $tag $element |]
  where
    tag = conT (mkName (eName ent))
    element = physicalElementType (fTy field)
    isReference = case fTy field of
      TRef _ -> True
      _      -> False

columnLength :: Field -> Name -> Q Exp
columnLength field value
  | fMany field = [| multiColLen $(varE value) |]
  | isReference = [| sparseColLen $(varE value) |]
  | otherwise = [| colLen $(varE value) |]
  where
    isReference = case fTy field of
      TRef _ -> True
      _      -> False

loadFunction :: Field -> Q Exp
loadFunction field = case (fMany field, fTy field) of
  (False, TInt)    -> [| loadInts |]
  (False, TDouble) -> [| loadDoubles |]
  (False, TStr)    -> [| loadStrs |]
  (False, TRef _)  -> [| loadIds |]
  (True, TInt)     -> [| loadMultiInts |]
  (True, TStr)     -> [| loadMultiStrs |]
  (True, TRef _)   -> [| loadMultiIds |]
  (True, TDouble)  ->
    fail ("declareStagedSchema: no multi-valued float cache kind for " ++ show (fFile field))
