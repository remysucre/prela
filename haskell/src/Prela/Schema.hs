{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | Declaring a dataset: entities, their fields, and where the columns come from.
--
-- Everything below this module deals in columns and relations that someone
-- already has in hand. This is the part that turns a schema written down once
-- into the names a query says. JOB has 21 entities and around a hundred fields,
-- so it is written by a generator rather than by hand, the same way the Julia
-- port uses `@entity` and the Rust port uses `schema!`.
--
-- A declaration looks like this, and produces the four things listed under it:
--
-- @
-- declareSchema "Tiny"
--   [ entity "Movie" "movie"
--       [ one  "title"   str
--       , one  "year"    int
--       , many "keyword" (ref "Keyword")
--       ]
--   , entity "Keyword" "keyword"
--       [ one "text" str ]
--   ]
-- @
--
--   * an empty tag type per entity (@data Movie@), which is the phantom in
--     `Id Movie` and what stops movie ids from being used as keyword ids;
--   * one record holding every column (@data Tiny = Tiny { movie_n :: Int,
--     movie_title :: Col Movie ByteString, … }@);
--   * a loader (@loadTiny :: FilePath -> IO Tiny@) reading @Movie_title.bin@ and
--     so on out of a cache directory, with each entity's id space sized from its
--     FIRST declared field, as in the Rust port;
--   * an accessor per field and per universe, taking the record:
--     @title :: Mode q => Tiny -> q (Id Movie) ByteString@.
--
-- The accessors take the record rather than being stored in it, and that is not
-- a stylistic choice. A relation stored in a record field is a polymorphic
-- function inside a data structure, which GHC cannot see through, so the query
-- runs a dictionary call per row and allocates about 250 bytes a row. As
-- top-level functions they inline at the use site and the query fuses exactly as
-- a hand-built one does. Both arrangements were measured; the numbers are in
-- design/SchemaProbe.hs.
--
-- So a query written directly against a schema mentions it once per leaf:
--
-- @
-- countRecent s = foldAll (\\n _ -> n + 1) 0
--                         (compose (restrict (movie s) (gt 1980 (year s))) (year s))
-- @
--
-- and a query module that wants the bare spelling buys it back with one local
-- binding per name, which costs nothing at run time:
--
-- @
-- queries s = [ ... ]
--   where
--     movie :: Mode q => q (Id Movie) (Id Movie)
--     movie = Schema.movie s
--     year  :: Mode q => q (Id Movie) Int
--     year  = Schema.year s
-- @
--
-- The signatures there are load bearing. `TypeFamilies`, which the storage layer
-- needs, turns on `MonoLocalBinds`, so without them each name would be pinned to
-- whichever mode it was first used at.
module Prela.Schema
  ( -- * Field types
    Ty, str, int, dbl, ref
    -- * Fields
  , Field, one, many, as
    -- * Entities
  , Ent, entity
    -- * The generator
  , declareSchema
  ) where

import Data.ByteString (ByteString)
import Data.Char (toLower)
import Data.List (intercalate)
import Language.Haskell.TH

import Prela.Cache
import Prela.Ops
import Prela.Storage

--------------------------------------------------------------------------------
-- The declaration language
--------------------------------------------------------------------------------

-- | What a field's values are. `ref` names another entity, so `ref "Keyword"`
-- gives a column of `Id Keyword`, which is what a foreign key is.
data Ty = TInt | TDouble | TStr | TRef String

str, int, dbl :: Ty
str = TStr
int = TInt
dbl = TDouble

ref :: String -> Ty
ref = TRef

-- | A field carries two names. `fFile` is the name on disk, since the cache
-- files are literally `<Entity>_<field>.bin` and the writer chose them. `fAs` is
-- the accessor, which starts out the same and can be changed with `as`.
data Field = Field
  { fFile :: String
  , fAs   :: String
  , fTy   :: Ty
  , fMany :: Bool
  }

-- | A field with exactly one value per key.
one :: String -> Ty -> Field
one n t = Field n n t False

-- | A field with any number of values per key, stored CSR. This is also how a
-- scalar field with gaps is declared, since the cache has no way to mark a
-- missing number: an absent value is a key with an empty row. JOB's
-- `production_year` is `many` for exactly that reason.
many :: String -> Ty -> Field
many n t = Field n n t True

-- | Rename the accessor without touching the filename. Needed when two entities
-- share a field name, since the accessors are top-level: JOB has both
-- `Movie.info` and `Info.info`, and at most one of them can be called `info`.
as :: Field -> String -> Field
as f n = f { fAs = n }

-- | An entity: its tag type name, the name its universe gets, and its fields.
-- The first field sizes the id space, so make it one that covers every key.
data Ent = Ent
  { eName   :: String
  , eUniv   :: String
  , eFields :: [Field]
  }

entity :: String -> String -> [Field] -> Ent
entity = Ent

--------------------------------------------------------------------------------
-- Generation
--------------------------------------------------------------------------------

-- | Splice a schema. The name given becomes the record type, its constructor,
-- and the `load`-prefixed loader.
declareSchema :: String -> [Ent] -> Q [Dec]
declareSchema sname ents
  | null ents = fail "declareSchema: no entities"
  | any (null . eFields) ents =
      fail "declareSchema: every entity needs at least one field to size its universe"
  -- Accessors are top-level, so two fields wanting the same name is a clash.
  -- GHC would catch it, but it reports "Multiple declarations" pointing at the
  -- splice, which says nothing about which name or which entities. This is a
  -- routine thing to hit, since a universe often wants the name one of its
  -- referring foreign keys also wants.
  | (nm, owners) : _ <- clashes =
      fail ("declareSchema: the name " ++ show nm ++ " is claimed by " ++ intercalate " and " owners
            ++ ".\nRename the universe, or the field with `as`.")
  | otherwise = do
      let recN = mkName sname
      tags <- mapM tagDec ents
      recd <- recDec recN ents
      ldr  <- loaderDecs recN ents
      accs <- concat <$> mapM (accessorDecs recN) ents
      return (tags ++ [recd] ++ ldr ++ accs)
  where
    claimed = [ (eUniv e, "the " ++ eName e ++ " universe") | e <- ents ]
           ++ [ (fAs f, eName e ++ "." ++ fFile f) | e <- ents, f <- eFields e ]
    clashes = [ (nm, owners)
              | (nm, _) <- claimed
              , let owners = [ o | (nm', o) <- claimed, nm' == nm ]
              , length owners > 1 ]

-- `data Movie` — no constructors, because a tag exists only in types.
tagDec :: Ent -> Q Dec
tagDec e = dataD (return []) (mkName (eName e)) [] Nothing [] []

recDec :: Name -> [Ent] -> Q Dec
recDec recN ents = do
  fs <- concat <$> mapM entRecFields ents
  dataD (return []) recN [] Nothing [recC recN (map return fs)] []
  where
    entRecFields e = do
      nT  <- [t| Int |]
      cs  <- mapM (colField e) (eFields e)
      return ((countFieldName e, strict, nT) : cs)
    colField e f = do
      t <- colType e f
      return (colFieldName e f, strict, t)
    strict = Bang NoSourceUnpackedness SourceStrict

loaderDecs :: Name -> [Ent] -> Q [Dec]
loaderDecs recN ents = do
  dir <- newName "dir"
  -- One binder per column, kept grouped by entity so each entity's first
  -- binder is available to size its universe.
  grouped <- mapM (\e -> (,) e <$> mapM (\f -> (,) f <$> newName "c") (eFields e)) ents
  stmts <- sequence [ loadBind dir e f v | (e, fvs) <- grouped, (f, v) <- fvs ]
  args  <- concat <$> mapM ctorArgs grouped
  let body = doE (map return stmts ++ [noBindS (appE [| return |] (return (foldl AppE (ConE recN) args)))])
  sig <- sigD loadN [t| FilePath -> IO $(conT recN) |]
  fun <- funD loadN [clause [varP dir] (normalB body) []]
  return [sig, fun]
  where
    loadN = mkName ("load" ++ nameBase recN)
    loadBind dir e f v =
      bindS (varP v) [| $(loadFn f) $(varE dir) $(litE (stringL (eName e ++ "_" ++ fFile f))) |]
    -- The universe size comes from the entity's first field, so that binder has
    -- to be picked out. `declareSchema` has already rejected a fieldless entity.
    ctorArgs (_, fvs) = case fvs of
      [] -> fail "declareSchema: entity with no fields"
      ((f0, v0) : _) -> do
        n <- if fMany f0 then [| multiColLen $(varE v0) |] else [| colLen $(varE v0) |]
        return (n : map (VarE . snd) fvs)

accessorDecs :: Name -> Ent -> Q [Dec]
accessorDecs recN e = do
  u <- universeAcc
  fs <- concat <$> mapM fieldAcc (eFields e)
  return (u ++ fs)
  where
    tag = conT (mkName (eName e))
    universeAcc = do
      s <- newName "s"
      let n = mkName (eUniv e)
      sig <- sigD n [t| forall q. Mode q => $(conT recN) -> q (Id $tag) (Id $tag) |]
      fun <- funD n [clause [varP s]
                            (normalB [| universe ($(varE (countFieldName e)) $(varE s)) |]) []]
      inl <- pragInlD n Inline FunLike AllPhases
      return [sig, fun, inl]
    fieldAcc f = do
      s <- newName "s"
      let n = mkName (fAs f)
          get = [| $(varE (colFieldName e f)) $(varE s) |]
      sig <- sigD n [t| forall q. Mode q => $(conT recN) -> q (Id $tag) $(elemType (fTy f)) |]
      fun <- funD n [clause [varP s]
                            (normalB (if fMany f then [| multiColumn $get |]
                                                 else [| column $get |])) []]
      inl <- pragInlD n Inline FunLike AllPhases
      return [sig, fun, inl]

--------------------------------------------------------------------------------
-- Names and types
--------------------------------------------------------------------------------

-- Record fields are prefixed by entity so that two entities can both have a
-- `text` column. Only the generated loader and accessors mention them.
colFieldName :: Ent -> Field -> Name
colFieldName e f = mkName (lower1 (eName e) ++ "_" ++ fFile f)

countFieldName :: Ent -> Name
countFieldName e = mkName (lower1 (eName e) ++ "_n")

lower1 :: String -> String
lower1 (c : cs) = toLower c : cs
lower1 []       = []

elemType :: Ty -> Q Type
elemType TInt      = [t| Int |]
elemType TDouble   = [t| Double |]
elemType TStr      = [t| ByteString |]
elemType (TRef r)  = [t| Id $(conT (mkName r)) |]

colType :: Ent -> Field -> Q Type
colType e f
  | fMany f   = [t| MultiCol $tag $el |]
  | otherwise = [t| Col $tag $el |]
  where
    tag = conT (mkName (eName e))
    el  = elemType (fTy f)

-- Which reader in "Prela.Cache" fits. The gap is deliberate rather than an
-- oversight: the cache format has no multi-valued float kind, so nothing can
-- read one, and saying so here beats generating a call that does not typecheck.
loadFn :: Field -> Q Exp
loadFn f = case (fMany f, fTy f) of
  (False, TInt)    -> [| loadInts |]
  (False, TDouble) -> [| loadDoubles |]
  (False, TStr)    -> [| loadStrs |]
  (False, TRef _)  -> [| loadIds |]
  (True,  TInt)    -> [| loadMultiInts |]
  (True,  TStr)    -> [| loadMultiStrs |]
  (True,  TRef _)  -> [| loadMultiIds |]
  (True,  TDouble) -> fail ("declareSchema: no multi-valued float in the cache format, \
                            \so `many " ++ show (fFile f) ++ " dbl` cannot be loaded")
