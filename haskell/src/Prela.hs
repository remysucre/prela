{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE FlexibleContexts #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TypeFamilies #-}

-- | Prela in Haskell — the core execution protocol.
--
-- Prela is an embedded query language built on Tarski's algebra of relations.
-- There is exactly one kind of thing: a binary relation — a set of (x, y)
-- pairs — read as a multi-valued function `d -> r` (give it an x, it hands
-- back the y's it relates to: zero, one, or many). Everything else is built by
-- combining relations with operators.
--
-- A relation is accessed in exactly two directions, and which one decides how
-- it executes:
--
--   drive  — enumerate the whole thing: "walk every (x, y) pair." This is the
--            outer-loop role. The left of `→` gets driven.
--   probe  — random access: "here is a key x I already hold; give me its y's."
--            This is the inner role. A filter side gets probed.
--
-- The role is not a property of the relation, it is a property of the POSITION
-- the relation sits in. In `movie : (keyword == "alien") → title`, `movie` is
-- driven while the filter and `title` are probed — same query, both modes.
--
-- The two modes are two record types, and `Mode` is the class with those two
-- records as its instances. Leaves are class methods too, so a leaf is
-- polymorphic and instantiates at whichever mode its position demands, and a
-- query mentions no mode at all — the signature at the top picks one and it
-- flows down. See MODES.md for why it is built this way and what the two
-- rejected alternatives were.

module Prela where

import Control.Monad (when)
import Control.Monad.ST (ST, runST)
import Data.STRef (newSTRef, modifySTRef', readSTRef)
import Data.Array (Array, accumArray, elems, (!))
import Data.Array.Base (unsafeAt)
import Data.Array.ST (STArray, STUArray, newArray, readArray, writeArray, freeze, runSTUArray)
import Data.Array.Unboxed (UArray)
import qualified Data.Array.Unboxed as U
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Unsafe as BSU
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Maybe (isJust)
import qualified Data.Vector.Storable as V
import Data.Word (Word32)
import Text.Regex.TDFA (Regex, RegexLike, makeRegex, matchTest)

--------------------------------------------------------------------------------
-- The two modes
--------------------------------------------------------------------------------

-- | Enumerate: run the action on every (x, y) pair.
--
-- The `forall m.` inside the field (higher-rank, hence RankNTypes) is load
-- bearing: one relation must be drivable in different monads — driven into an
-- index in `ST`, printed in `IO`. The continuations are monadic actions rather
-- than pure accumulator-transformers so the operator bodies stay free of
-- accumulator plumbing.
newtype Drv d r = Drv
  { drive :: forall m. Monad m => (d -> r -> m ()) -> m () }

-- | Look up: given a key x, run the action on every related y.
--
-- `probeAny` short-circuits at the first hit and is PURE: a membership test
-- needs no effect, so it stays out of the monad and drops straight into `when`
-- and `&&` in the streaming operators.
data Prb d r = Prb
  { probe    :: forall m. Monad m => d -> (r -> m ()) -> m ()
  , probeAny :: d -> (r -> Bool) -> Bool
  }

-- | "Is x in this relation at all?" — what a filter actually calls.
member :: Prb d r -> d -> Bool
member q x = probeAny q x (const True)
{-# INLINE member #-}

--------------------------------------------------------------------------------
-- Leaf storage
--------------------------------------------------------------------------------

-- An entity id. Under the hood a 0-based Int, but the phantom tag e (Movie,
-- Keyword, …) rides along in the type and never appears in a value, so
-- `Id Movie` and `Id Keyword` are the same bits and do not unify: composing a
-- Movie-keyed relation onto a Person-keyed one is a compile error, not a
-- silent wrong answer. Erased at runtime, so the safety is free.
newtype Id e = Id Int deriving (Eq, Ord, Show)

-- | The missing-id sentinel — a foreign key that points at nothing. The value is
-- not arbitrary: it fails every `0 <= i && i < n` range check, so a hole probes
-- to nothing without any branch of its own, which is why Prela can claim there
-- are no NULLs and still store fixed-width id columns.
--
-- It is @-1@ rather than `maxBound` so that it is bit-for-bit the all-ones hole
-- word the cache writes (see "Prela.Cache"), read back as a signed machine word.
-- A loaded column and a hand-built one then hold the same bits, which they must,
-- since nothing downstream knows where a column came from. Same trick as the
-- Rust port's `NO_ID`, which is `usize::MAX` for the same reason read unsigned.
noId :: Id e
noId = Id (-1)

--------------------------------------------------------------------------------
-- How element types are physically stored
--------------------------------------------------------------------------------

-- Rust gets flat storage for free: `Vec<i64>` is already a contiguous array of
-- machine words. Haskell does not — `Array Int Int` is an array of POINTERS to
-- boxed Ints, which is a cache miss per row and defeats the point of a column
-- store. An unboxed array is flat, but only accepts primitive element types, so
-- one uniform column type cannot serve both an Int column and a string column.
--
-- This class is the way out: an associated DATA family, so each element type
-- names its own physical layout while the leaves below see one interface.
--
-- The layouts chosen here are deliberately the ON-DISK layouts of the cache
-- format (see Prela.Cache): 8-byte words for numbers and ids, 4-byte offsets
-- plus one packed buffer for strings. That is what lets loading be a VIEW of a
-- memory-mapped file rather than a conversion of it — `Storable` vectors are a
-- pointer, an offset and a length, so a column can point straight into the
-- mapping the way the Rust port's `&'static str` does. The difference is that
-- Rust has to leak the mmap to get a `'static` lifetime, whereas here the
-- vectors hold the mapping's finalizer and it is released when the last column
-- referring to it is collected.
--
-- `atStore` is UNCHECKED. Every caller is a leaf that has already done its own
-- range test, and doing it twice is exactly the kind of per-row cost this whole
-- design exists to avoid.
class Elem r where
  data Store r
  packStore :: [r] -> Store r
  storeLen  :: Store r -> Int
  atStore   :: Store r -> Int -> r

packU :: U.IArray UArray r => [r] -> UArray Int r
packU vs = U.listArray (0, length vs - 1) vs

packV :: V.Storable r => [r] -> V.Vector r
packV = V.fromList

-- Offsets are 32-bit, as on disk. Widening to an index is a zero-extend.
off32 :: V.Vector Word32 -> Int -> Int
off32 v i = fromIntegral (V.unsafeIndex v i)
{-# INLINE off32 #-}

instance Elem Int where
  newtype Store Int = IntStore (V.Vector Int)
  packStore = IntStore . packV
  storeLen (IntStore a) = V.length a
  atStore (IntStore a) i = V.unsafeIndex a i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

instance Elem Double where
  newtype Store Double = DblStore (V.Vector Double)
  packStore = DblStore . packV
  storeLen (DblStore a) = V.length a
  atStore (DblStore a) i = V.unsafeIndex a i
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

-- An id column is a word array reinterpreted, which is what makes a foreign key
-- as cheap to store as a number. The Rust port gets this from
-- `#[repr(transparent)]`; here the newtype is erased, so it is the same bits.
instance Elem (Id e) where
  newtype Store (Id e) = IdStore (V.Vector Int)
  packStore = IdStore . packV . map (\(Id i) -> i)
  storeLen (IdStore a) = V.length a
  atStore (IdStore a) i = Id (V.unsafeIndex a i)
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

-- One concatenated buffer plus n+1 offsets. `unsafeTake`/`unsafeDrop` are
-- pointer arithmetic on the shared buffer, so reading a string copies nothing.
instance Elem ByteString where
  data Store ByteString = BsStore !(V.Vector Word32) !ByteString
  packStore vs = BsStore (packV (scanl (+) 0 (map (fromIntegral . BS.length) vs)))
                         (BS.concat vs)
  storeLen (BsStore offs _) = V.length offs - 1
  atStore (BsStore offs buf) i =
    case (off32 offs i, off32 offs (i + 1)) of
      (o, o') -> BSU.unsafeTake (o' - o) (BSU.unsafeDrop o buf)
  {-# INLINE storeLen #-}
  {-# INLINE atStore #-}

--------------------------------------------------------------------------------
-- The three column shapes
--------------------------------------------------------------------------------

-- Storage is its own type, built once, and the leaf constructors below are
-- VIEWS of it. That matters: a leaf is mode-polymorphic, and a polymorphic
-- binding is re-elaborated per instantiation, so if the array were built inside
-- the class method a column used in both modes would be built twice.
--
-- These are three separate types rather than one type with a shape field, for
-- the same reason the sparse universe is separate below: dispatch happens once
-- at compile time, so a dense column's loop carries no branch testing whether
-- it might have been sparse.

-- | Total 1:1: every key in `0 .. n-1` has exactly one value.
data Col e r = Col !Int !(Store r)

mkCol :: Elem r => [r] -> Col e r
mkCol vs = Col (storeLen s) s where s = packStore vs

-- | 1:1 with holes: the values array stays full width, and a presence bit per
-- key says which slots are real. For an id-valued column `noId` in the holes
-- would do the same job for free, but a scalar column has no spare value — a
-- missing year is not year 0 — so the mask is what keeps "absent" from becoming
-- a wrong answer. Julia carries the same presence bitvector for this reason.
data SparseCol e r = SparseCol !Int !(Store r) !(UArray Int Bool)

-- | `fill` occupies the holes and is never read; only its width matters.
mkSparseCol :: Elem r => r -> [Maybe r] -> SparseCol e r
mkSparseCol fill ms =
  SparseCol (length ms) (packStore (map (maybe fill id) ms)) (packU (map isJust ms))

-- | Multi-valued, stored CSR: key i owns `values[offsets[i] .. offsets[i+1]-1]`,
-- and a key with no values is simply an empty range, so partial and multi-valued
-- are the same representation.
data MultiCol e r = MultiCol !Int !(V.Vector Word32) !(Store r)

-- | Pairs with a key outside `0 .. n-1` are dropped. Per-key value order follows
-- the input.
mkMultiCol :: Elem r => Int -> [(Int, r)] -> MultiCol e r
mkMultiCol n prs = MultiCol n (packV (scanl (+) 0 (map (fromIntegral . length) buckets)))
                             (packStore (concat buckets))
  where
    buckets = map reverse (elems (accumArray (flip (:)) [] (0, n - 1)
                                             [p | p@(k, _) <- prs, 0 <= k, k < n]))

-- One reduced value per key over a dense key space `0 .. n-1`: what a grouped
-- fold produces when the keys are entity ids rather than arbitrary values. The
-- presence array is what distinguishes "this key folded to init" from "this key
-- was never seen", and it is also how the outer variant is expressed — seeding
-- presence to all-True makes every key emit, so there is no separate flag.
data Dense e t = Dense !Int !(Array Int t) !(UArray Int Bool)

-- A dense membership set: the identity relation on whichever ids are present.
-- `UArray Int Bool` is bit-packed by GHC, so a test is a word load and a shift,
-- which is the whole point of it over a `Map`. Driving it scans every bit rather
-- than using Rust's trailing-zeros skip, so it is best used probed.
newtype Bits e = Bits (UArray Int Bool)

-- | A membership set over `0 .. n-1` from the ids that are present. Ids outside
-- the range are dropped, `noId` among them.
mkBits :: Int -> [Id e] -> Bits e
mkBits n ids = Bits (runSTUArray (do
  bs <- newBits n False
  mapM_ (\(Id i) -> when (0 <= i && i < n) (writeArray bs i True)) ids
  return bs))

-- Typed wrappers so the two arrays below are built in one pass over the input.
-- `freeze` alone leaves the mutable array type ambiguous; these pin it.
newBoxed :: Int -> t -> ST s (STArray s Int t)
newBoxed n v = newArray (0, n - 1) v

newBits :: Int -> Bool -> ST s (STUArray s Int Bool)
newBits n v = newArray (0, n - 1) v

--------------------------------------------------------------------------------
-- The mode class: leaves and every operator whose mode is free
--------------------------------------------------------------------------------

-- The result mode `q` flows through each operator. The right-hand argument of
-- a binary operator is concretely `Prb`, because that side is probed whatever
-- mode the result is in — that single fact is what the whole class turns on.
class Mode q where
  -- Leaves.
  universe  :: Int -> q (Id e) (Id e)
  -- A dense id space that carries holes. Its DRIVE walks only the live ids, but
  -- its PROBE checks the range and not the mask, which is not an oversight: an
  -- id reaching a universe in probe position was obtained by navigating from
  -- real data, so it is live by construction, and re-testing the bit would be
  -- work with no possible effect. The Rust port takes the same shortcut.
  sparseUniverse :: Bits e -> Int -> q (Id e) (Id e)
  column    :: Elem r => Col e r -> q (Id e) r
  sparseColumn :: Elem r => SparseCol e r -> q (Id e) r
  multiColumn  :: Elem r => MultiCol e r -> q (Id e) r
  fromIndex :: Ord d => Map d [r] -> q d r
  fromCache :: Ord d => Map d s -> q d s
  fromDense :: Dense e t -> q (Id e) t
  fromBits  :: Bits e -> q (Id e) (Id e)

  -- Chain two relations through a shared middle value: `r : d -> e` and
  -- `s : e -> f` give `d -> f`. Also field navigation.
  compose   :: q d e -> Prb e f -> q d f

  -- Pair two relations sharing a DOMAIN: for each key, take both values.
  -- Used in member position this is conjunction — `probeAny` collapses to a
  -- short-circuiting AND and the pair is never built.
  prod      :: q d u -> Prb d v -> q d (u, v)

  -- Keep each row whose VALUE is a member of the second relation.
  restrict  :: q d r -> Prb r e -> q d r

  -- Keep each row whose KEY is absent from the second relation. This is SQL's
  -- IS NULL: a missing value is an absent pair.
  diff      :: q d r -> Prb d e -> q d r

  -- Keep each row whose value passes a test.
  filt      :: (r -> Bool) -> q d r -> q d r

  -- Replace each value, leaving the key alone.
  mapv      :: (r -> s) -> q d r -> q d s

instance Mode Drv where
  universe n = Drv (\k -> mapM_ (\i -> k (Id i) (Id i)) [0 .. n - 1])
  -- Same thunk hazard as the Prb instance below: match the Col inside the
  -- lambda, not on the left of the `=`.
  sparseUniverse b _ = Drv (\k -> case b of
                                    Bits bs -> mapM_ (\i -> when (unsafeAt bs i) (k (Id i) (Id i)))
                                                     (U.indices bs))
  column c = Drv (\k -> case c of
                          Col n s -> mapM_ (\i -> k (Id i) (atStore s i)) [0 .. n - 1])
  sparseColumn c =
    Drv (\k -> case c of
                 SparseCol n s pres ->
                   mapM_ (\i -> when (unsafeAt pres i) (k (Id i) (atStore s i))) [0 .. n - 1])
  multiColumn c =
    Drv (\k -> case c of
                 MultiCol n offs s ->
                   mapM_ (\i -> mapM_ (\j -> k (Id i) (atStore s j))
                                      [off32 offs i .. off32 offs (i + 1) - 1])
                         [0 .. n - 1])
  fromIndex m = Drv (\k -> mapM_ (\(d, rs) -> mapM_ (k d) rs) (Map.toList m))
  fromCache m = Drv (\k -> mapM_ (uncurry k) (Map.toList m))
  fromDense c = Drv (\k -> case c of
                             Dense n vals seen ->
                               mapM_ (\i -> when (seen U.! i) (k (Id i) (vals ! i)))
                                     [0 .. n - 1])
  fromBits b = Drv (\k -> case b of
                            Bits bs -> mapM_ (\i -> when (bs U.! i) (k (Id i) (Id i)))
                                             (U.indices bs))

  compose  a b = Drv (\k -> drive a (\x y -> probe b y (\z -> k x z)))
  prod     a b = Drv (\k -> drive a (\x u -> probe b x (\v -> k x (u, v))))
  restrict a b = Drv (\k -> drive a (\x y -> when (member b y) (k x y)))
  diff     a b = Drv (\k -> drive a (\x v -> when (not (member b x)) (k x v)))
  filt   t a   = Drv (\k -> drive a (\x y -> when (t y) (k x y)))
  mapv   f a   = Drv (\k -> drive a (\x v -> k x (f v)))
  {-# INLINE universe #-}
  {-# INLINE sparseUniverse #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromBits #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

instance Mode Prb where
  universe n = Prb { probe    = \x k -> when (inUniverse n x) (k x)
                   , probeAny = \x p -> inUniverse n x && p x }
  -- NOTE: the `Col` is deliberately NOT matched on the left of the `=`. Doing
  -- that would make the record a thunk (it must force the column before it can
  -- return), and GHC will not duplicate a thunk into a loop — the probe would
  -- then be an unknown call through a shared record field on every row. Built
  -- this way the record is already a constructor application, so it inlines at
  -- each use site and the array read lands directly in the loop. Verified in
  -- Core; see design/CoreProbe.hs.
  sparseUniverse _ n = Prb { probe    = \x k -> when (inUniverse n x) (k x)
                           , probeAny = \x p -> inUniverse n x && p x }
  column c =
    Prb { probe    = \(Id i) k -> case c of
                                    Col n s -> when (0 <= i && i < n) (k (atStore s i))
        , probeAny = \(Id i) p -> case c of
                                    Col n s -> 0 <= i && i < n && p (atStore s i) }
  sparseColumn c =
    Prb { probe    = \(Id i) k -> case c of
            SparseCol n s pres -> when (0 <= i && i < n && unsafeAt pres i) (k (atStore s i))
        , probeAny = \(Id i) p -> case c of
            SparseCol n s pres -> 0 <= i && i < n && unsafeAt pres i && p (atStore s i) }
  multiColumn c =
    Prb { probe    = \(Id i) k -> case c of
            MultiCol n offs s ->
              when (0 <= i && i < n)
                   (mapM_ (\j -> k (atStore s j))
                          [off32 offs i .. off32 offs (i + 1) - 1])
        , probeAny = \(Id i) p -> case c of
            MultiCol n offs s ->
              0 <= i && i < n
                && any (\j -> p (atStore s j))
                       [off32 offs i .. off32 offs (i + 1) - 1] }
  fromIndex m = Prb { probe    = \x k -> maybe (return ()) (mapM_ k) (Map.lookup x m)
                    , probeAny = \x p -> maybe False (any p) (Map.lookup x m) }
  fromCache m = Prb { probe    = \x k -> maybe (return ()) k (Map.lookup x m)
                    , probeAny = \x p -> maybe False p (Map.lookup x m) }
  fromDense c =
    Prb { probe    = \(Id i) k -> case c of
                                    Dense n vals seen ->
                                      when (0 <= i && i < n && seen U.! i) (k (vals ! i))
        , probeAny = \(Id i) p -> case c of
                                    Dense n vals seen ->
                                      0 <= i && i < n && seen U.! i && p (vals ! i) }
  fromBits b =
    Prb { probe    = \x k -> case b of Bits bs -> when (hasBit bs x) (k x)
        , probeAny = \x p -> case b of Bits bs -> hasBit bs x && p x }

  compose a b = Prb
    { probe    = \x k -> probe    a x (\y -> probe    b y k)
    , probeAny = \x p -> probeAny a x (\y -> probeAny b y p)
    }
  prod a b = Prb
    { probe    = \x k -> probe    a x (\u -> probe    b x (\v -> k (u, v)))
    , probeAny = \x p -> probeAny a x (\u -> probeAny b x (\v -> p (u, v)))
    }
  restrict a b = Prb
    { probe    = \x k -> probe    a x (\y -> when (member b y) (k y))
    , probeAny = \x p -> probeAny a x (\y -> member b y && p y)
    }
  diff a b = Prb
    { probe    = \x k -> when (not (member b x)) (probe a x k)
    , probeAny = \x p -> not (member b x) && probeAny a x p
    }
  filt t a = Prb
    { probe    = \x k -> probe    a x (\y -> when (t y) (k y))
    , probeAny = \x p -> probeAny a x (\y -> t y && p y)
    }
  mapv f a = Prb
    { probe    = \x k -> probe    a x (\v -> k (f v))
    , probeAny = \x p -> probeAny a x (\v -> p (f v))
    }
  {-# INLINE universe #-}
  {-# INLINE sparseUniverse #-}
  {-# INLINE column #-}
  {-# INLINE sparseColumn #-}
  {-# INLINE multiColumn #-}
  {-# INLINE fromIndex #-}
  {-# INLINE fromCache #-}
  {-# INLINE fromDense #-}
  {-# INLINE fromBits #-}
  {-# INLINE compose #-}
  {-# INLINE prod #-}
  {-# INLINE restrict #-}
  {-# INLINE diff #-}
  {-# INLINE filt #-}
  {-# INLINE mapv #-}

inUniverse :: Int -> Id e -> Bool
inUniverse n (Id i) = 0 <= i && i < n
{-# INLINE inUniverse #-}

hasBit :: UArray Int Bool -> Id e -> Bool
hasBit bs (Id i) = case U.bounds bs of
                     (lo, hi) -> lo <= i && i <= hi && bs U.! i
{-# INLINE hasBit #-}

--------------------------------------------------------------------------------
-- Predicate sugar
--------------------------------------------------------------------------------

-- The comparison operators are not new nodes: each is `filt` with a fixed
-- closure, matching Rust, where `.eq(v)` builds `Filter { p: |x| x == v }`.
-- The value is on the left and v on the right, as in the Prela surface
-- `year > 1980`.
eq, ne :: (Mode q, Eq r) => r -> q d r -> q d r
eq v = filt (== v)
ne v = filt (/= v)
{-# INLINE eq #-}
{-# INLINE ne #-}

gt, lt, ge, le :: (Mode q, Ord r) => r -> q d r -> q d r
gt v = filt (> v)
lt v = filt (< v)
ge v = filt (>= v)
le v = filt (<= v)
{-# INLINE gt #-}
{-# INLINE lt #-}
{-# INLINE ge #-}
{-# INLINE le #-}

isIn :: (Mode q, Eq r) => [r] -> q d r -> q d r
isIn vs = filt (`elem` vs)
{-# INLINE isIn #-}

-- Regex match and its negation (`~` and `≁`). Also just `filt`, but note the
-- absence of an INLINE pragma, which is deliberate: the compiled regex is bound
-- in the `where` so it is built once and shared by every row. Inlining these
-- would copy `makeRegex` into each use site, which is harmless, but there is no
-- loop to fuse into here — the cost is inside `matchTest` either way.
rx, nrx :: (Mode q, RegexLike Regex s) => String -> q d s -> q d s
rx  re = filt (matchTest r)        where r = makeRegex re :: Regex
nrx re = filt (not . matchTest r)  where r = makeRegex re :: Regex

--------------------------------------------------------------------------------
-- The operators whose mode is NOT free
--------------------------------------------------------------------------------

-- These are exactly the ones the class cannot hold, and keeping them out of it
-- is the point of the design. Each either demands a driven input, or produces
-- a fixed mode, or both.

-- | Flip a relation's direction. Driven, that is free: stream the pairs and
-- swap each one. It has NO probed form — probing an inverse means building a
-- reverse index, which is `inv` below. Since this returns `Drv`, a query that
-- tries to probe it does not compile.
invStream :: Drv d r -> Drv r d
invStream a = Drv (\k -> drive a (\x y -> k y x))
{-# INLINE invStream #-}

-- | Left-compose (`←`), which is defined as `r ← s ≡ r' → s`: rekey the first
-- relation by its own value, then chain the second onto it. Both arguments share
-- a DOMAIN and the result is keyed by the first one's VALUE. Being built on
-- `invStream` it inherits its mode: driven only, no probed form.
leftCompose :: Drv d e -> Prb d f -> Drv e f
leftCompose a b = compose (invStream a) b
{-# INLINE leftCompose #-}

-- | Union of two relations over the same domain and value type (`Drive`-only in
-- the Rust port too): drive one and then the other. There is no probed form,
-- because probing a union would have to probe both sides and de-duplicate, and
-- nothing in Prela asks for that — what queries actually want from OR is
-- membership, which is `disj` below.
union :: Drv d r -> Drv d r -> Drv d r
union a b = Drv (\k -> drive a k >> drive b k)
{-# INLINE union #-}

-- | Membership union (`∨`), the OR of two filters. Never enumerated: only
-- whether a key is present in either side is defined, so the value type is `()`.
-- That is not a placeholder, it is the constraint made visible — you cannot
-- navigate through a `∨` or read a value out of one, and the type says so, which
-- is how the Rust port gets the same guarantee by implementing only `Member`.
-- The two sides may relate their keys to entirely different things.
disj :: Prb d u -> Prb d v -> Prb d ()
disj a b = Prb
  { probe    = \x k -> when (member a x || member b x) (k ())
  , probeAny = \x p -> (member a x || member b x) && p ()
  }
{-# INLINE disj #-}

-- | Drive a relation once and bucket its pairs by key. This is where a query
-- stops being pure closures and holds real data, and it is why `drive` is
-- `forall m. Monad m` rather than a pure fold: we instantiate m = ST, mutate a
-- ref while driving, and freeze the result.
index :: Ord d => Drv d r -> Map d [r]
index q = runST $ do
  ref <- newSTRef Map.empty
  drive q (\d r -> modifySTRef' ref (Map.insertWith (++) d [r]))
  readSTRef ref

-- | Force a leg once so reuse is free. Same pairs, now backed by the index.
-- Bind the RESULT monomorphically: it is mode-polymorphic, and a polymorphic
-- binding used at two modes builds its index twice, which is the exact cost
-- this exists to remove.
materialize :: (Mode q, Ord d) => Drv d r -> q d r
materialize = fromIndex . index

-- | The probed inverse: index the flipped pairs, and the result serves either
-- mode because the work has already been paid for.
inv :: (Mode q, Ord r) => Drv d r -> q r d
inv = fromIndex . index . invStream

-- | Group by key and reduce each group (`q ▷ (op, init)`). A fold cannot
-- stream — reducing a group means seeing the whole group — so like the index
-- it drives once into a cache, but the cache holds ONE value per key.
-- `count = fold (\n _ -> n + 1) 0`. The result is an ordinary relation, so a
-- fold composes and probes like anything else.
fold :: (Mode q, Ord d) => (s -> r -> s) -> s -> Drv d r -> q d s
fold op ini q = fromCache cache
  where
    cache = runST $ do
      ref <- newSTRef Map.empty
      drive q (\d v -> modifySTRef' ref (Map.alter (Just . flip op v . maybe ini id) d))
      readSTRef ref

-- | The dense-array grouped fold (`q ▷ (op, init, n)`): the same thing as
-- `fold`, opted into a different physical cache. When the keys are entity ids
-- over a known `0 .. n-1` the per-key slot can be an array index instead of a
-- map entry, which removes a hash and an allocation from every reduce step. Keys
-- outside the range are dropped rather than growing the store.
foldDense :: Mode q => Int -> (t -> r -> t) -> t -> Drv (Id e) r -> q (Id e) t
foldDense n op ini = fromDense . buildDense n op ini False

-- | Like `foldDense`, but every key in `0 .. n-1` emits, seeded with `init`
-- where nothing matched — the left-outer-join aggregate. Correct ONLY when
-- `0 .. n-1` is exactly the key universe; over a sparse key space it invents
-- rows for keys that do not exist.
foldDenseOuter :: Mode q => Int -> (t -> r -> t) -> t -> Drv (Id e) r -> q (Id e) t
foldDenseOuter n op ini = fromDense . buildDense n op ini True

buildDense :: Int -> (t -> r -> t) -> t -> Bool -> Drv (Id e) r -> Dense e t
buildDense n op ini pre q = runST $ do
  vals <- newBoxed n ini
  seen <- newBits n pre
  drive q (\(Id i) v -> when (0 <= i && i < n) (do
             acc <- readArray vals i
             writeArray vals i (op acc v)
             writeArray seen i True))
  Dense n <$> freeze vals <*> freeze seen

-- | Precompute dense membership (`bitset(q, n)`): drive a relation and set a bit
-- at each VALUE it emits, giving back the identity relation on those ids. This is
-- the one operator that exists purely for speed — it is semantically the same as
-- restricting against the relation's values, but a bit test beats re-running a
-- subquery or hashing into a set, so it pays off on a filter probed many times.
bitset :: Mode q => Int -> Drv d (Id e) -> q (Id e) (Id e)
bitset n q = fromBits (Bits (runSTUArray (do
  bs <- newBits n False
  drive q (\_ (Id i) -> when (0 <= i && i < n) (writeArray bs i True))
  return bs)))

-- | A minimal strict state monad, used only by `foldAll`.
--
-- `drive` hands its continuation a monadic action, so a whole-relation fold has
-- to carry the accumulator in the monad. Doing that with an `STRef` costs a
-- heap cell and a boxed write per row; threading it as a state function instead
-- lets GHC turn the accumulator into an argument of the fused loop and unbox it.
newtype Acc s a = Acc { runAcc :: s -> (a, s) }

instance Functor (Acc s) where
  fmap f (Acc g) = Acc (\s -> case g s of (a, s') -> (f a, s'))
  {-# INLINE fmap #-}

instance Applicative (Acc s) where
  pure a = Acc (\s -> (a, s))
  {-# INLINE pure #-}
  Acc f <*> Acc g = Acc (\s -> case f s of
                                 (h, s')  -> case g s' of
                                   (a, s'') -> (h a, s''))
  {-# INLINE (<*>) #-}

instance Monad (Acc s) where
  Acc g >>= f = Acc (\s -> case g s of (a, s') -> runAcc (f a) s')
  {-# INLINE (>>=) #-}

-- | No-group fold (`q ⊵ (op, init)`, already unwrapped): ignore the keys, fold
-- every value into one accumulator, return the bare scalar.
foldAll :: (s -> r -> s) -> s -> Drv d r -> s
foldAll op ini q =
  snd (runAcc (drive q (\_ v -> Acc (\acc -> let !acc' = op acc v in ((), acc')))) ini)
{-# INLINE foldAll #-}
