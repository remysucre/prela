{-# LANGUAGE FunctionalDependencies #-}
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Schema declarations and their staged query interface.
--
-- 'declareStagedSchema' turns a compact list of entities and fields into the
-- boundary between on-disk cache data and staged queries.  It generates
-- phantom entity tags, a strict record of loaded columns, fast and checked
-- loaders, entity universes and extents, and relation-valued field accessors.
-- References are stored physically as integers and exposed as typed 'Id'
-- values only after the generated accessor has checked the target universe.
--
-- The declaration vocabulary in this module describes storage, not a query
-- plan: query execution remains in "Prela.PullStaged".
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
import Language.Haskell.TH.Syntax (Lift, addModFinalizer, liftTyped)

import Prela.Cache
import Prela.Id
import qualified Prela.PullStaged.Ops as O
import qualified Prela.PullStaged.Relation as R
import qualified Prela.PullStaged.Scalar as S
import Prela.Storage

-- | Logical field type understood by the schema generator.
data Ty = TInt | TDouble | TStr | TRef String

-- | Logical string, integer, and double field types, respectively.
str, int, dbl :: Ty
str = TStr
int = TInt
dbl = TDouble

-- | A reference to the entity with the given declaration name.
ref :: String -> Ty
ref = TRef

-- | A field declaration, including its cache name, accessor name, type, and
-- cardinality.
data Field = Field
  { fFile :: String
  , fAs   :: String
  , fTy   :: Ty
  , fMany :: Bool
  }

-- | Declare a one-valued field whose cache and accessor share a name.
one :: String -> Ty -> Field
one name ty = Field name name ty False

-- | Declare a multi-valued field whose cache and accessor share a name.
many :: String -> Ty -> Field
many name ty = Field name name ty True

-- | Override the generated accessor name without changing the cache name.
as :: Field -> String -> Field
as field name = field { fAs = name }

-- | An entity declaration and its generated universe accessor.
data Ent = Ent
  { eName   :: String
  , eUniv   :: String
  , eFields :: [Field]
  , eSparse :: Bool
  }

-- | Declare a dense entity.
entity :: String -> String -> [Field] -> Ent
entity name universeName fields = Ent name universeName fields False

-- | A sparse entity derives its live-ID mask from absence in its first field.
-- That field must therefore be a one-valued reference.
sparseEntity :: String -> String -> [Field] -> Ent
sparseEntity name universeName fields = Ent name universeName fields True

-- | Generate a schema record, loaders, entity tags, and staged accessors.
--
-- Generation fails when an entity has no fields, generated names clash, or a
-- sparse entity cannot derive its live-row mask from its first field.
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

-- | Generate the phantom tag type for an entity.
tagDeclaration :: Ent -> Q Dec
tagDeclaration ent = do
  let name = mkName (eName ent)
  documentDeclaration name
    ("Phantom entity tag generated for the " ++ eName ent ++ " table.")
  dataD (pure []) name [] Nothing [] []

-- | Generate the strict record that owns all loaded universes and columns.
recordDeclaration :: Name -> [Ent] -> Q Dec
recordDeclaration recordName entities = do
  documentDeclaration recordName
    ("Loaded storage record for the generated " ++ nameBase recordName ++ " schema.")
  mapM_ documentEntityFields entities
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
    documentEntityFields ent = do
      documentDeclaration (universeFieldName ent)
        ("Loaded universe backing the " ++ eName ent ++ " entity.")
      mapM_ (\field ->
        documentDeclaration (columnFieldName ent field)
          ("Loaded storage backing " ++ eName ent ++ "." ++ fFile field ++ "."))
        (eFields ent)

-- | Give each generated record selector a small, liftable tag and a 'Project'
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
      documentDeclaration tagName
        ("Internal projection tag for the generated " ++ nameBase selector
          ++ " schema field.")
      tag <- dataD (pure []) tagName [] Nothing
        [normalC tagName []]
        [derivClause Nothing [conT ''Lift]]
      instanceDeclaration <- instanceD (pure [])
        [t| Project $tagType $(conT recordName) $(pure value) |]
        [funD 'projectField
          [clause [wildP, varP (mkName "record")]
            (normalB (appE (varE selector) (varE (mkName "record")))) []]]
      pure [tag, instanceDeclaration]

-- | Derive the private projection-tag name for a generated record selector.
projectionTagName :: Name -> Name -> Name
projectionTagName recordName selector =
  mkName (nameBase recordName ++ upperFirst (nameBase selector) ++ "Field")

-- | Uppercase the first character of a generated name.
upperFirst :: String -> String
upperFirst (first : rest) = toUpper first : rest
upperFirst [] = []

-- | Generate the default mmap loader and the validating checked loader.
loaderDeclarations :: Name -> [Ent] -> Q [Dec]
loaderDeclarations recordName entities = do
  let defaultName = mkName ("load" ++ nameBase recordName)
      checkedName = mkName ("load" ++ nameBase recordName ++ "Checked")
  defaultDeclarations <- makeLoader defaultName loadFunction
    "Load the schema through the trusted memory-mapped cache path."
  checkedDeclarations <- makeLoader checkedName loadCheckedFunction
    "Load the schema through the validating checked cache path."
  pure (defaultDeclarations ++ checkedDeclarations)
  where
    makeLoader loaderName fieldLoader documentation = do
      documentDeclaration loaderName documentation
      directory <- newName "directory"
      groups <- mapM binders entities
      columnStatements <- sequence
        [ bindS (varP value)
            [| $(fieldLoader field) $(varE directory)
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
      signature <- sigD loaderName [t| FilePath -> IO $(conT recordName) |]
      function <- funD loaderName [clause [varP directory] (normalB body) []]
      pure [signature, function]

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

-- | Build a dense entity universe after checking all column lengths agree.
checkedDenseUniverse :: String -> [Int] -> IO (Universe e)
checkedDenseUniverse entityName lengths = do
  size <- checkedExtent entityName lengths
  case denseUniverse size of
    Just result -> pure result
    Nothing -> ioError (userError (entityName ++ ": negative universe size"))

-- | Derive a sparse universe from its first column and validate its extent.
checkedSparseUniverse :: String -> SparseCol e Int -> [Int] -> IO (Universe e)
checkedSparseUniverse entityName firstColumn lengths = do
  size <- checkedExtent entityName lengths
  let result = validityUniverse firstColumn
  unless (universeSize result == size) $
    ioError (userError (entityName ++ ": validity mask length disagrees with its columns"))
  pure result

-- | Return the common column length, or report an inconsistent entity layout.
checkedExtent :: String -> [Int] -> IO Int
checkedExtent entityName lengths = case lengths of
  [] -> ioError (userError (entityName ++ ": entity has no columns"))
  size : rest -> do
    unless (all (== size) rest) $
      ioError (userError (entityName ++ ": columns have inconsistent lengths"))
    pure size

-- | Generate universe, extent, and relation-valued accessors for one entity.
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
      documentDeclaration name
        ("Identity relation over the live " ++ eName ent ++ " identifiers.")
      signature <- sigD name
        [t| $recordCode -> R.Relation (Id $tag) (Id $tag) |]
      function <- funD name [clause [varP record] (normalB [| O.universe $domain |]) []]
      inlinePragma <- pragInlD name Inline FunLike AllPhases
      pure [signature, function, inlinePragma]

    makeExtentAccessor = do
      record <- newName "schema"
      let name = mkName (extentAccessorName ent)
          domain = get record (universeFieldName ent)
      documentDeclaration name
        ("Staged storage extent of the " ++ eName ent ++ " identifier space.")
      signature <- sigD name [t| $recordCode -> S.Scalar Int |]
      function <- funD name
        [clause [varP record] (normalB [| S.extent $domain |]) []]
      inlinePragma <- pragInlD name Inline FunLike AllPhases
      pure [signature, function, inlinePragma]

    makeFieldAccessor field = do
      record <- newName "schema"
      let name = mkName (fAs field)
          sourceDomain = get record (universeFieldName ent)
          column = get record (columnFieldName ent field)
          rawLeaf
            | fMany field = [| O.multiColumn $column |]
            | isReference field = [| O.sparseColumn $column |]
            | otherwise = [| O.column $column |]
          liveRows = [| O.compose (O.universe $sourceDomain) $rawLeaf |]
          body = case (fMany field, fTy field) of
            (False, TRef target) ->
              let targetDomain = get record (universeFieldNameByName target)
              in [| O.referenceColumn $sourceDomain $column $targetDomain |]
            (True, TRef target) ->
              let targetDomain = get record (universeFieldNameByName target)
              in [| O.compose $liveRows (O.resolveId $targetDomain) |]
            _ -> liveRows
      documentDeclaration name
        ("Staged " ++ cardinality field ++ " relation for " ++ eName ent
          ++ "." ++ fFile field ++ ".")
      signature <- sigD name
        [t| $recordCode -> R.Relation (Id $tag) $(elementType (fTy field)) |]
      function <- funD name [clause [varP record] (normalB body) []]
      inlinePragma <- pragInlD name Inline FunLike AllPhases
      pure [signature, function, inlinePragma]

    isReference field = case fTy field of
      TRef _ -> True
      _      -> False

    cardinality field
      | fMany field = "multi-valued"
      | otherwise   = "one-valued"

-- | A typed record projection selected by a generated, liftable field tag.
class Project field record value | field -> record value where
  -- | Select the record field identified by the projection tag.
  projectField :: field -> record -> value

-- | Quote a record projection without an unchecked typed-code coercion.
fieldCode :: (Lift field, Project field record value)
          => field -> CodeQ record -> CodeQ value
fieldCode field record = [|| projectField $$(liftTyped field) $$record ||]

-- | Attach documentation after a generating splice has added its declarations
-- to the module environment.
documentDeclaration :: Name -> String -> Q ()
documentDeclaration name documentation =
  addModFinalizer (putDoc (DeclDoc name) documentation)

-- | Derive a private schema-record selector for a column.
columnFieldName :: Ent -> Field -> Name
columnFieldName ent field = mkName (lowerFirst (eName ent) ++ "_" ++ fFile field)

-- | Derive a private schema-record selector for an entity universe.
universeFieldName :: Ent -> Name
universeFieldName = universeFieldNameByName . eName

-- | Derive a universe selector from an entity declaration name.
universeFieldNameByName :: String -> Name
universeFieldNameByName entityName = mkName (lowerFirst entityName ++ "_universe")

-- | Derive the public staged extent accessor for an entity.
extentAccessorName :: Ent -> String
extentAccessorName ent = lowerFirst (eName ent) ++ "Extent"

-- | Lowercase the first character of a generated name.
lowerFirst :: String -> String
lowerFirst (first : rest) = toLower first : rest
lowerFirst [] = []

-- | Translate a logical field type to its query-facing Haskell type.
elementType :: Ty -> Q Type
elementType TInt = [t| Int |]
elementType TDouble = [t| Double |]
elementType TStr = [t| ByteString |]
elementType (TRef target) = [t| Id $(conT (mkName target)) |]

-- | Translate a logical field type to its cache representation.
physicalElementType :: Ty -> Q Type
physicalElementType (TRef _) = [t| Int |]
physicalElementType ty = elementType ty

-- | Select the storage container generated for a field declaration.
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

-- | Generate an expression that obtains a loaded column's row count.
columnLength :: Field -> Name -> Q Exp
columnLength field value
  | fMany field = [| multiColLen $(varE value) |]
  | isReference = [| sparseColLen $(varE value) |]
  | otherwise = [| colLen $(varE value) |]
  where
    isReference = case fTy field of
      TRef _ -> True
      _      -> False

-- | Select the fast mmap loader for a field declaration.
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

-- | Select the validating loader for a field declaration.
loadCheckedFunction :: Field -> Q Exp
loadCheckedFunction field = case (fMany field, fTy field) of
  (False, TInt)    -> [| loadIntsChecked |]
  (False, TDouble) -> [| loadDoublesChecked |]
  (False, TStr)    -> [| loadStrsChecked |]
  (False, TRef _)  -> [| loadIdsChecked |]
  (True, TInt)     -> [| loadMultiIntsChecked |]
  (True, TStr)     -> [| loadMultiStrsChecked |]
  (True, TRef _)   -> [| loadMultiIdsChecked |]
  (True, TDouble)  ->
    fail ("declareStagedSchema: no multi-valued float cache kind for " ++ show (fFile field))
