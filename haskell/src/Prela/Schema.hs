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
-- An entity declared with `sparseEntity` gets one more record field, its
-- validity mask, and a universe accessor that reads it.
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
  , Ent, entity, sparseEntity
    -- * The generator
  , declareSchema
  , declareStagedSchema
  , fieldCode
  ) where

import Data.ByteString (ByteString)
import Data.Char (toLower)
import Data.List (intercalate)
import Language.Haskell.TH
import Language.Haskell.TH.Syntax (ModName (..), Module (..), PkgName (..), mkNameG_fld)

import Prela.Cache
import Prela.Ops
import qualified Prela.Staged.Ops as S
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
  , eSparse :: Bool
  }

entity :: String -> String -> [Field] -> Ent
entity n u fs = Ent n u fs False

-- | An entity whose id space has holes: the ids run `0 .. n-1` as usual, but
-- some of them are dead, so driving its universe should skip them. TPC-H's
-- orders are the standard case, being a dense range over a key that is not.
--
-- Which ids are dead is derived rather than declared, from the holes in the
-- entity's FIRST field, which must therefore be a 1:1 `ref`: a foreign key
-- pointing nowhere is what a dead row looks like, and no other column kind can
-- say it. The Rust port writes this as @Order(orders sparse)@ and derives the
-- mask the same way.
--
-- Only the DRIVE changes. Probing the universe stays a range check, because an
-- id arriving in probe position was navigated to from real data and so cannot be
-- dead.
sparseEntity :: String -> String -> [Field] -> Ent
sparseEntity n u fs = Ent n u fs True

--------------------------------------------------------------------------------
-- Generation
--------------------------------------------------------------------------------

-- Which engine the accessors target. Everything else a schema generates — the
-- tag types, the record, the loader — is the same either way, because none of it
-- mentions a relation.
data Flavour = Push | Staged

-- | Splice a schema. The name given becomes the record type, its constructor,
-- and the `load`-prefixed loader.
declareSchema :: String -> [Ent] -> Q [Dec]
declareSchema = declareWith Push

-- | The same schema, with accessors for the staged engine in
-- "Prela.Staged.Ops". The difference is only in the accessors, and it is the
-- difference staging makes everywhere: an accessor takes the CODE of the record
-- rather than the record.
--
-- @
-- title ::           Mode  q =>       Tiny -> q (Id Movie) ByteString
-- title :: 'S.SMode' q => CodeQ Tiny -> q (Id Movie) ByteString
-- @
--
-- A query gets that @CodeQ Tiny@ from `Prela.Staged.Stream.lam1`, which is what
-- introduces a binder the splice is allowed to name.
declareStagedSchema :: String -> [Ent] -> Q [Dec]
declareStagedSchema = declareWith Staged

declareWith :: Flavour -> String -> [Ent] -> Q [Dec]
declareWith flav sname ents
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
  -- A sparse entity's validity mask comes from the holes in its first field, and
  -- `validityBits` can only read them out of a 1:1 id column.
  | ((e, f) : _) <- badSparse =
      fail ("declareSchema: " ++ eName e ++ " is sparse, so its first field must be a 1:1 `ref`\
            \ (the mask is derived from that column's holes), but it is " ++ show (fFile f) ++ ".")
  | otherwise = do
      let recN = mkName sname
      tags <- mapM tagDec ents
      recd <- recDec recN ents
      ldr  <- loaderDecs recN ents
      accs <- concat <$> mapM (accessorDecs flav recN) ents
      return (tags ++ [recd] ++ ldr ++ accs)
  where
    claimed = [ (eUniv e, "the " ++ eName e ++ " universe") | e <- ents ]
           ++ [ (fAs f, eName e ++ "." ++ fFile f) | e <- ents, f <- eFields e ]
    clashes = [ (nm, owners)
              | (nm, _) <- claimed
              , let owners = [ o | (nm', o) <- claimed, nm' == nm ]
              , length owners > 1 ]
    badSparse = [ (e, f) | e <- ents, eSparse e, f : _ <- [eFields e], not (isRef1 f) ]
    isRef1 f = case (fMany f, fTy f) of
                 (False, TRef _) -> True
                 _               -> False

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
      -- The validity mask, for a sparse entity only. Deliberately the one LAZY
      -- field in the record: a schema with many sparse entities should not walk
      -- every one of their key columns at load time, and a thunk gets that for
      -- free, once, on first use. The Rust port needs a `OnceLock` to say it.
      lv <- if eSparse e
              then do t <- [t| Bits $(conT (mkName (eName e))) |]
                      return [(liveFieldName e, lazy, t)]
              else return []
      return ((countFieldName e, strict, nT) : lv ++ cs)
    colField e f = do
      t <- colType e f
      return (colFieldName e f, strict, t)
    strict = Bang NoSourceUnpackedness SourceStrict
    lazy   = Bang NoSourceUnpackedness NoSourceStrictness

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
    ctorArgs (e, fvs) = case fvs of
      [] -> fail "declareSchema: entity with no fields"
      ((f0, v0) : _) -> do
        n <- if fMany f0 then [| multiColLen $(varE v0) |] else [| colLen $(varE v0) |]
        -- Sized and masked from the same column: `declareSchema` has already
        -- checked that a sparse entity's first field is a 1:1 `ref`.
        lv <- if eSparse e then (: []) <$> [| validityBits $(varE v0) |] else return []
        return (n : lv ++ map (VarE . snd) fvs)

accessorDecs :: Flavour -> Name -> Ent -> Q [Dec]
accessorDecs flav recN e = do
  u <- universeAcc
  fs <- concat <$> mapM fieldAcc (eFields e)
  return (u ++ fs)
  where
    tag = conT (mkName (eName e))

    -- The record argument, and how a record field is read out of it. Push reads
    -- it directly; staged emits code that will read it later.
    argT t = case flav of
      Push   -> t
      Staged -> [t| CodeQ $t |]
    get s fld = case flav of
      Push   -> [| $(varE fld) $(varE s) |]
      Staged -> do
        -- The record field's ORIGINAL name, package and module and all. See
        -- `fieldCode` for why the bare string will not do.
        Module (PkgName p) (ModName m) <- thisModule
        [| fieldCode p m $(litE (stringL (nameBase recN)))
                     $(litE (stringL (nameBase fld))) $(varE s) |]

    universeAcc = do
      s <- newName "s"
      let n = mkName (eUniv e)
          n' = get s (countFieldName e)
          live = get s (liveFieldName e)
      sig <- case flav of
        Push   -> sigD n [t| forall q. Mode    q => $(argT (conT recN)) -> q (Id $tag) (Id $tag) |]
        Staged -> sigD n [t| forall q. S.SMode q => $(argT (conT recN)) -> q (Id $tag) (Id $tag) |]
      let body = case (flav, eSparse e) of
            (Push,   True)  -> [| sparseUniverse   $live $n' |]
            (Push,   False) -> [| universe              $n' |]
            (Staged, True)  -> [| S.sparseUniverse $live $n' |]
            (Staged, False) -> [| S.universe            $n' |]
      fun <- funD n [clause [varP s] (normalB body) []]
      inl <- pragInlD n Inline FunLike AllPhases
      return [sig, fun, inl]

    fieldAcc f = do
      s <- newName "s"
      let n = mkName (fAs f)
          c = get s (colFieldName e f)
      sig <- case flav of
        Push   -> sigD n [t| forall q. Mode    q => $(argT (conT recN)) -> q (Id $tag) $(elemType (fTy f)) |]
        Staged -> sigD n [t| forall q. S.SMode q => $(argT (conT recN)) -> q (Id $tag) $(elemType (fTy f)) |]
      let body = case (flav, fMany f) of
            (Push,   True)  -> [| multiColumn   $c |]
            (Push,   False) -> [| column        $c |]
            (Staged, True)  -> [| S.multiColumn $c |]
            (Staged, False) -> [| S.column      $c |]
      fun <- funD n [clause [varP s] (normalB body) []]
      inl <- pragInlD n Inline FunLike AllPhases
      return [sig, fun, inl]

-- | Read one record field, in code: given the package, module, record type and
-- name of a selector, @fieldCode … \"movie_year\" s@ is the staged spelling of
-- @movie_year s@. Every staged accessor is built out of this.
--
-- This is a wart, and it is worth being precise about which part. Untyped
-- Template Haskell, which is what `declareSchema` is written in, has no way to
-- build a TYPED quote: there is no @Exp@ constructor for @[|| … ||]@. So the
-- generated body cannot say @[|| movie_year $$s ||]@ directly. It builds the
-- untyped expression instead, which it already knows how to do, and then asserts
-- the type with `unsafeCodeCoerce`. The assertion is the wart — nothing checks
-- that the field really has the type the accessor's signature claims.
--
-- What limits the damage is that both halves come from the same declaration.
-- The record field and the accessor signature are generated from one `Field`, a
-- few lines apart, so a mismatch would be a bug in this module rather than in a
-- schema someone wrote. And it still surfaces: the coerced code is spliced into
-- a query, and the splice is typechecked like any other expression.
--
-- The package and module arguments are not decoration. The obvious version of
-- this takes the field name alone and calls `mkName`, but a `mkName` name is
-- resolved wherever it ends up, and where it ends up is the QUERY module. So it
-- would compile only if the query happened to import the schema's record fields
-- unqualified, which is exactly what a query has no reason to do. Worse, when
-- they are not in scope GHC 9.10 does not report it: it panics with @unfilled
-- unbound-variable evidence@. `mkNameG_fld` names the selector where it actually
-- lives, and `thisModule` at the schema splice is what knows where that is.
--
-- It has to be `mkNameG_fld` and not the `mkNameG_v` that a top-level function
-- would use. Since GHC 9.10 a record selector lives in its own namespace, tagged
-- with the record type it belongs to, and looking one up as an ordinary variable
-- fails with @Can\'t find interface-file declaration@ even though the name is
-- right there in the interface file. That is the reason for the record-type
-- argument.
fieldCode :: String -> String -> String -> String -> CodeQ s -> CodeQ a
fieldCode pkg modu rec fld s =
  unsafeCodeCoerce (appE (varE (mkNameG_fld pkg modu rec fld)) (unTypeCode s))

--------------------------------------------------------------------------------
-- Names and types
--------------------------------------------------------------------------------

-- Record fields are prefixed by entity so that two entities can both have a
-- `text` column. Only the generated loader and accessors mention them.
colFieldName :: Ent -> Field -> Name
colFieldName e f = mkName (lower1 (eName e) ++ "_" ++ fFile f)

countFieldName :: Ent -> Name
countFieldName e = mkName (lower1 (eName e) ++ "_n")

-- Only exists for a sparse entity.
liveFieldName :: Ent -> Name
liveFieldName e = mkName (lower1 (eName e) ++ "_live")

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
