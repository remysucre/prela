{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The 22 TPC-H queries on the staged pull engine.
--
-- Read three differences before the queries, because they explain the shape of
-- every module in the staged half of the port.
--
-- A query here is a GENERATOR. `q6` does not compute a string, it returns the
-- CODE of something that will, and "TPCH.Staged" is what splices those into real
-- functions. Thus `q6` has type @CodeQ TPCHS -> CodeQ String@, and the schema
-- arrives as the code of the record rather than the record.
--
-- Every materializer takes the rest of the query as an argument. Instead of
-- @let t = fold …@ and used @t@; here it is @withFold … $ \\t -> …@, and the
-- reason is the one rule of staging: a `CodeQ` used twice is code emitted twice,
-- so a materializer that RETURNED its table would rebuild it at every mention.
-- Queries needing several of them read as a stack of continuations, and the
-- reading order is the build order.
--
-- And the formatting helpers live in this module rather than beside the splices.
-- A quote may name a top level binding of its OWN module — that is ordinary
-- cross-stage persistence — but a splice may not name a binding of the module it
-- sits in. So @f2@ and friends live here, on the generator side, and
-- "TPCH.Staged" holds nothing but splices.
module TPCH.StagedQueries
  ( q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11
  , q12, q13, q14, q15, q16, q17, q18, q19, q20, q21, q22
  ) where

import Data.ByteString.Char8 (ByteString)
import qualified Data.ByteString.Char8 as BS
import Data.List (intercalate, sortBy, sortOn)
import Data.Ord (comparing)
import Language.Haskell.TH (CodeQ)

import Prela.PullStaged.Materialize
import Prela.PullStaged.Ops
import Prela.PullStaged.Predicate
import Prela.PullStaged.Stream
import Prela.Id (Id, idIndex, universeSize)

import TPCH.StagedSchema

--------------------------------------------------------------------------------
-- Leaving the algebra
--------------------------------------------------------------------------------

-- These helpers remain unstaged: they run below `collect`, on the handful of
-- rows a query returns, so there is
-- nothing for staging to do to them. They are emitted by name into the generated
-- code and called at run time like any other function.

-- | Two decimal places, matching Rust's @{:.2}@ and DuckDB's money output.
f2 :: Double -> String
f2 x = sign ++ show whole ++ "." ++ pad (show cents)
  where
    r     = round (abs x * 100) :: Integer
    whole = r `div` 100
    cents = r `mod` 100
    sign  = if x < 0 && r /= 0 then "-" else ""
    pad d = replicate (2 - length d) '0' ++ d

-- | The cache stores dates as yyyymmdd integers, so printing one is arithmetic
-- rather than a calendar library.
fmtDate :: Int -> String
fmtDate d = pad 4 (d `div` 10000) ++ "-" ++ pad 2 ((d `div` 100) `mod` 100)
                                 ++ "-" ++ pad 2 (d `mod` 100)
  where pad n v = let t = show v in replicate (n - length t) '0' ++ t

joinLines :: [String] -> String
joinLines = intercalate "\n"

row :: [String] -> String
row = intercalate "|"

bs :: ByteString -> String
bs = BS.unpack

-- Natural keys in the TPC-H source data are 1-based; internal ids are 0-based,
-- so the +1 is an output detail and appears nowhere else.
key1 :: Id e -> String
key1 identifier = show (idIndex identifier + 1)

--------------------------------------------------------------------------------
-- Q1 — pricing summary report
--------------------------------------------------------------------------------

q1 :: CodeQ TPCHS -> CodeQ String
q1 s = withFold step [|| (0, 0, 0, 0, 0, 0) ||] grouped $ \tbl ->
         [|| joinLines (map fmt1 (sortOn fst
               $$(collect (tbl :: Stream (ByteString, ByteString) Q1Acc)))) ||]
  where
    grouped :: Stream (ByteString, ByteString) (((Double, Double), Double), Double)
    grouped =
      compose (groupBy (restrict (lineitem s) (le [|| 19980902 ||] (shipdate s)))
                       (prod (returnflag s) (lineStatus s)))
              (prod (prod (prod (quantity s) (extendedprice s)) (discount s)) (tax s))
    step acc v =
      [|| case ($$acc, $$v) of
            ((!qty, !ext, !di, !dp, !chg, !n), (((q, e), dc), tx)) ->
              let dpInc  = e * (1 - dc)
                  chgInc = dpInc * (1 + tx)
              in (qty + q, ext + e, di + dc, dp + dpInc, chg + chgInc, n + 1) ||]

-- The accumulator needs a name because `withFold` demands `UV.Unbox` on it, and
-- a bare six-tuple has no such instance. This is the one place staging asks for
-- something ordinary generated loops cannot infer implicitly.
type Q1Acc = (Double, Double, Double, Double, Double, Int)

fmt1 :: ((ByteString, ByteString), Q1Acc) -> String
fmt1 ((rf, ls), (qty, ext, di, dp, chg, n)) =
  row [ bs rf, bs ls, f2 qty, f2 ext, f2 dp, f2 chg
      , f2 (qty / nf), f2 (ext / nf), f2 (di / nf), show n ]
  where nf = fromIntegral n

--------------------------------------------------------------------------------
-- Q2 — minimum-cost supplier per part
--------------------------------------------------------------------------------

-- @euPs@ is mentioned twice and is NOT materialized, so it emits two scans of
-- partsupp. An unmaterialized stream
-- used twice is enumerated twice, and Prela materializes only when asked.
q2 :: CodeQ TPCHS -> CodeQ String
q2 s = withFold (\a c -> [|| if $$c < $$a then $$c else $$a ||]) [|| 1 / 0 ||]
                (compose (groupBy euPs (psPart s)) (supplycost s)) $ \minPerPart ->
  let costIsMin :: Lookup (Id PartSupp) (Double, Double)
      costIsMin = filt (\p -> [|| case $$p of (c, m) -> c == m ||])
                       (prod (supplycost s) (compose (psPart s) minPerPart))
      result :: Stream (Id PartSupp) Q2Row
      result = compose (restrict euPs (prod partOk costIsMin)) payload
  in [|| joinLines (map fmt2 (take 100 (sortBy cmp2 (map (flat2 . snd)
           $$(collect result))))) ||]
  where
    euPs :: SMode q => q (Id PartSupp) (Id PartSupp)
    euPs = restrict (partsupp s)
             (eq [|| "EUROPE" ||]
                 (compose (compose (compose (psSupplier s) (supplierNation s))
                                   (nationRegion s))
                          (regionName s)))
    partOk = restrict (psPart s)
               (prod (eq [|| 15 ||] (size s))
                     (filt (\v -> [|| BS.isSuffixOf "BRASS" $$v ||]) (ty s)))
    payload = prod (prod (compose (psSupplier s) suppCols) (psPart s))
                   (compose (psPart s) (mfgr s))
    suppCols = prod (prod (prod (prod (prod (supplierAcctbal s) (supplierName s))
                                      (compose (supplierNation s) (nationName s)))
                                (supplierAddress s))
                          (supplierPhone s))
                    (supplierComment s)

type Q2Row = (((((((Double, ByteString), ByteString), ByteString), ByteString)
              , ByteString), Id Part), ByteString)

-- Flatten the nested pairs once, so the comparator and the formatter both read
-- named components instead of unpicking a seven-deep tuple.
flat2 :: Q2Row -> (Double, ByteString, ByteString, Id Part, ByteString
                  , ByteString, ByteString, ByteString)
flat2 (((((((acct, sname), nat), addr), phone), comm), pk), mfg) =
  (acct, sname, nat, pk, mfg, addr, phone, comm)

cmp2 :: (Double, ByteString, ByteString, Id Part, ByteString, ByteString, ByteString, ByteString)
     -> (Double, ByteString, ByteString, Id Part, ByteString, ByteString, ByteString, ByteString)
     -> Ordering
cmp2 a b = comparing acct b a <> comparing nat a b
                              <> comparing sname a b <> comparing pk a b
  where acct  (x, _, _, _, _, _, _, _) = x
        sname (_, x, _, _, _, _, _, _) = x
        nat   (_, _, x, _, _, _, _, _) = x
        pk    (_, _, _, x, _, _, _, _) = x

fmt2 :: (Double, ByteString, ByteString, Id Part, ByteString, ByteString, ByteString, ByteString)
     -> String
fmt2 (acct, sname, nat, pk, mfg, addr, phone, comm) =
  row [f2 acct, bs sname, bs nat, key1 pk, bs mfg, bs addr, bs phone, bs comm]

--------------------------------------------------------------------------------
-- Q3 — shipping priority
--------------------------------------------------------------------------------

-- SQL groups by (l_orderkey, o_orderdate, o_shippriority). The orderkey fixes
-- the other two, so all three ride in the group key and the output reads them
-- straight off it, with no second lookup.
q3 :: CodeQ TPCHS -> CodeQ String
q3 s = withFold (\a v -> [|| case $$v of (e, dc) -> $$a + e * (1 - dc) ||]) [|| 0 ||]
                grouped $ \revenue ->
  [|| joinLines (map fmt3 (take 10 (sortBy cmp3
        $$(collect (revenue :: Stream ((Id Order, Int), Int) Double))))) ||]
  where
    grouped :: Stream ((Id Order, Int), Int) (Double, Double)
    grouped =
      compose (groupBy (restrict (lineitem s)
                         (prod (prod (gt [|| 19950315 ||] (shipdate s))
                                     (lt [|| 19950315 ||] (compose (liOrder s) (date s))))
                               (eq [|| "BUILDING" ||]
                                   (compose (compose (liOrder s) (orderCustomer s))
                                            (mktsegment s)))))
                       (prod (prod (liOrder s) (compose (liOrder s) (date s)))
                             (compose (liOrder s) (shippriority s))))
              (prod (extendedprice s) (discount s))

cmp3 :: (((Id Order, Int), Int), Double) -> (((Id Order, Int), Int), Double) -> Ordering
cmp3 (((_, d1), _), r1) (((_, d2), _), r2) = compare r2 r1 <> compare d1 d2

fmt3 :: (((Id Order, Int), Int), Double) -> String
fmt3 (((o, d), sp), r) = row [key1 o, f2 r, fmtDate d, show sp]

--------------------------------------------------------------------------------
-- Q4 — order priority checking
--------------------------------------------------------------------------------

q4 :: CodeQ TPCHS -> CodeQ String
q4 s =
  -- Orders with at least one late line, precomputed as a bitset so the EXISTS is
  -- a bit test per order rather than a re-scan.
  withBits [|| universeSize (order_universe $$s) ||]
    (compose (restrict (lineitem s)
               (filt (\p -> [|| case $$p of (c, r) -> c < r ||])
                     (prod (commitdate s) (receiptdate s))))
             (liOrder s)) $ \lateOrder ->
  withFold (\a _ -> [|| $$a + (1 :: Int) ||]) [|| 0 ||]
    (groupBy (restrict (restrict (orders s)
                                 (range [|| 19930701 ||] [|| 19931001 ||] (date s)))
                       lateOrder)
             (priority s)) $ \counts ->
    [|| joinLines (map fmt4 (sortOn fst
          $$(collect (counts :: Stream ByteString Int)))) ||]

fmt4 :: (ByteString, Int) -> String
fmt4 (p, n) = row [bs p, show n]

--------------------------------------------------------------------------------
-- Q5 — local supplier volume
--------------------------------------------------------------------------------

-- The group key carries the work: a lineitem maps to its supplier's nation name,
-- with the ASIA restriction and the customer-nation equality pushed into the key
-- itself. A row whose keyed lookup yields nothing drops out, so the key doubles
-- as the filter and only the date window rides on the receiver.
q5 :: CodeQ TPCHS -> CodeQ String
q5 s = withFold (\a v -> [|| case $$v of (e, dc) -> $$a + e * (1 - dc) ||]) [|| 0 ||]
                grouped $ \result ->
  [|| joinLines (map fmt5 (sortBy (\a b -> comparing snd b a)
        $$(collect (result :: Stream ByteString Double)))) ||]
  where
    grouped :: Stream ByteString (Double, Double)
    grouped =
      compose (groupBy (restrict (lineitem s)
                         (range [|| 19940101 ||] [|| 19950101 ||]
                                (compose (liOrder s) (date s))))
                       (compose sameNation (nationName s)))
              (prod (extendedprice s) (discount s))
    sameNation :: Lookup (Id Lineitem) (Id Nation)
    sameNation = mapv (\p -> [|| fst $$p ||])
      (filt (\p -> [|| case $$p of (a, b) -> a == b ||])
        (prod (restrict (compose (liSupplier s) (supplierNation s))
                        (eq [|| "ASIA" ||] (compose (nationRegion s) (regionName s))))
              (compose (compose (liOrder s) (orderCustomer s)) (customerNation s))))

fmt5 :: (ByteString, Double) -> String
fmt5 (n, v) = row [bs n, f2 v]

--------------------------------------------------------------------------------
-- Q6 — forecasting revenue change
--------------------------------------------------------------------------------

q6 :: CodeQ TPCHS -> CodeQ String
q6 s = [|| f2 $$(foldAll (\a v -> [|| case $$v of (e, dc) -> $$a + e * dc ||])
                         [|| 0 ||] rows) ||]
  where
    rows :: Stream (Id Lineitem) (Double, Double)
    rows = compose (restrict (lineitem s)
                     (prod (prod (range [|| 19940101 ||] [|| 19950101 ||] (shipdate s))
                                 (between [|| 0.05 ||] [|| 0.07 ||] (discount s)))
                           (lt [|| 24.0 ||] (quantity s))))
                   (prod (extendedprice s) (discount s))

--------------------------------------------------------------------------------
-- Q7 — volume shipping between nation pairs
--------------------------------------------------------------------------------

q7 :: CodeQ TPCHS -> CodeQ String
q7 s = withFold (\a v -> [|| case $$v of (e, dc) -> $$a + e * (1 - dc) ||]) [|| 0 ||]
                grouped $ \result ->
  [|| joinLines (map fmt7 (sortBy cmp7
        $$(collect (result :: Stream (Int, (ByteString, ByteString)) Double)))) ||]
  where
    grouped :: Stream (Int, (ByteString, ByteString)) (Double, Double)
    grouped =
      compose (groupBy (lineitem s)
                       (prod (mapv (\v -> [|| $$v `div` 10000 ||])
                                   (between [|| 19950101 ||] [|| 19961231 ||] (shipdate s)))
                             (filt pair (prod supNation custNation))))
              (prod (extendedprice s) (discount s))
    supNation  = compose (compose (liSupplier s) (supplierNation s)) (nationName s)
    custNation = compose (compose (compose (liOrder s) (orderCustomer s))
                                  (customerNation s))
                         (nationName s)
    pair p = [|| case $$p of
                   (a, b) -> (a == "FRANCE" && b == "GERMANY")
                          || (a == "GERMANY" && b == "FRANCE") ||]

cmp7 :: ((Int, (ByteString, ByteString)), Double)
     -> ((Int, (ByteString, ByteString)), Double) -> Ordering
cmp7 ((y1, (s1, c1)), _) ((y2, (s2, c2)), _) =
  compare s1 s2 <> compare c1 c2 <> compare y1 y2

fmt7 :: ((Int, (ByteString, ByteString)), Double) -> String
fmt7 ((y, (n1, n2)), v) = row [bs n1, bs n2, show y, f2 v]

--------------------------------------------------------------------------------
-- Q8 — market share for BRAZIL
--------------------------------------------------------------------------------

-- Per year, the BRAZIL share of the volume on ECONOMY ANODIZED STEEL parts sold
-- to customers in AMERICA. The group key navigates the order once — restricting
-- it and taking its year in one hop — and rows failing that navigation drop out.
q8 :: CodeQ TPCHS -> CodeQ String
q8 s = withFold step [|| (0, 0) ||] grouped $ \tbl ->
  [|| joinLines (map fmt8 (sortOn fst
        $$(collect (mapv (\p -> [|| case $$p of (b, t) -> b / t ||])
                         (tbl :: Stream Int (Double, Double)))))) ||]
  where
    grouped :: Stream Int ((Double, Double), ByteString)
    grouped =
      compose (groupBy (restrict (lineitem s)
                         (eq [|| "ECONOMY ANODIZED STEEL" ||] (compose (liPart s) (ty s))))
                       (mapv (\v -> [|| $$v `div` 10000 ||])
                         (compose (restrict (liOrder s)
                                    (eq [|| "AMERICA" ||]
                                        (compose (compose (compose (orderCustomer s)
                                                                   (customerNation s))
                                                          (nationRegion s))
                                                 (regionName s))))
                                  (between [|| 19950101 ||] [|| 19961231 ||] (date s)))))
              (prod (prod (extendedprice s) (discount s))
                    (compose (compose (liSupplier s) (supplierNation s)) (nationName s)))
    step acc v =
      [|| case ($$acc, $$v) of
            ((!b, !t), ((e, dc), nm)) ->
              let vol = e * (1 - dc)
              in (b + (if nm == "BRAZIL" then vol else 0), t + vol) ||]

fmt8 :: (Int, Double) -> String
fmt8 (y, v) = row [show y, f2 v]

--------------------------------------------------------------------------------
-- Q9 — product type profit measure
--------------------------------------------------------------------------------

-- The query the staged shape was designed for. @sc@ is supply cost per (green
-- part, supplier), it is expensive, and @costPerLi@ reads it — twice, once to
-- cull the lineitem scan and once to fetch the value for the arithmetic.
--
-- Under push that was `materialize` returning a relation, and the sharing was a
-- Haskell @let@ plus a warning in MODES.md not to bind it polymorphically. Here
-- `withMaterialize` takes the rest of the query as an argument, so @scM@ is a
-- reference to one runtime binding no matter how many times it appears, and the
-- warning has nothing left to warn about. Mentioning @costPerLi@ twice does emit
-- the lookup twice, which is right: that is two searches of one shared index,
-- exactly the required correlated lookup.
q9 :: CodeQ TPCHS -> CodeQ String
q9 s = withMaterialize sc $ \scM ->
  let costPerLi :: Lookup (Id Lineitem) Double
      costPerLi = compose (prod (liPart s) (liSupplier s)) scM

      grouped :: Stream (ByteString, Int) (((Double, Double), Double), Double)
      grouped =
        compose (groupBy (restrict (lineitem s) costPerLi)
                         (prod (compose (compose (liSupplier s) (supplierNation s))
                                        (nationName s))
                               (mapv (\v -> [|| $$v `div` 10000 ||])
                                     (compose (liOrder s) (date s)))))
                (prod (prod (prod costPerLi (extendedprice s)) (discount s)) (quantity s))
  in withFold step [|| 0 ||] grouped $ \result ->
       [|| joinLines (map fmt9 (sortBy cmp9
             $$(collect (result :: Stream (ByteString, Int) Double)))) ||]
  where
    sc :: Stream (Id Part, Id Supplier) Double
    sc = compose (groupBy (partsupp s)
                          (prod (restrict (psPart s)
                                  (filt (\v -> [|| BS.isInfixOf "green" $$v ||]) (partName s)))
                                (psSupplier s)))
                 (supplycost s)
    step a v = [|| case $$v of
                     (((cost, e), dc), q) -> $$a + e * (1 - dc) - cost * q ||]

cmp9 :: ((ByteString, Int), Double) -> ((ByteString, Int), Double) -> Ordering
cmp9 ((n1, y1), _) ((n2, y2), _) = compare n1 n2 <> compare y2 y1

fmt9 :: ((ByteString, Int), Double) -> String
fmt9 ((n, y), v) = row [bs n, show y, f2 v]

--------------------------------------------------------------------------------
-- Q10 — returned-item reporting
--------------------------------------------------------------------------------

q10 :: CodeQ TPCHS -> CodeQ String
q10 s = withFold (\a v -> [|| case $$v of (e, dc) -> $$a + e * (1 - dc) ||]) [|| 0 ||]
                 grouped $ \revenue ->
  [|| joinLines (map fmt10 (take 20 (sortBy cmp10
        $$(collect (prod (revenue :: Stream (Id Customer) Double) custCols))))) ||]
  where
    grouped :: Stream (Id Customer) (Double, Double)
    grouped =
      compose (groupBy (restrict (lineitem s)
                         (prod (eq [|| "R" ||] (returnflag s))
                               (range [|| 19931001 ||] [|| 19940101 ||]
                                      (compose (liOrder s) (date s)))))
                       (compose (liOrder s) (orderCustomer s)))
              (prod (extendedprice s) (discount s))
    custCols = prod (prod (prod (prod (prod (customerName s) (customerAcctbal s))
                                      (compose (customerNation s) (nationName s)))
                                (customerAddress s))
                          (customerPhone s))
                    (customerComment s)

type Q10Row = (Double, (((((ByteString, Double), ByteString), ByteString), ByteString)
              , ByteString))

cmp10 :: (Id Customer, Q10Row) -> (Id Customer, Q10Row) -> Ordering
cmp10 (_, (r1, _)) (_, (r2, _)) = compare r2 r1

fmt10 :: (Id Customer, Q10Row) -> String
fmt10 (c, (r, (((((nm, ab), nat), addr), phone), comm)))=
  row [key1 c, bs nm, f2 r, f2 ab, bs nat, bs addr, bs phone, bs comm]

--------------------------------------------------------------------------------
-- Q11 — important stock
--------------------------------------------------------------------------------

-- The threshold is a scalar read off the same table the answer is filtered
-- against, so it has to be named in the GENERATED code rather than the
-- generator: @let !threshold = …@ sits inside the quote and the splice that
-- follows refers to it. That is the same move `lam1` makes, and it is how any
-- staged query introduces a runtime value it needs twice.
q11 :: CodeQ TPCHS -> CodeQ String
q11 s = withFold (\a v -> [|| case $$v of (c, q) -> $$a + c * fromIntegral q ||])
                 [|| 0 ||] grouped $ \valuePerPart ->
  [|| let !threshold = 0.0001 * $$(foldAll (\a v -> [|| $$a + $$v ||]) [|| 0 ||]
                                           (valuePerPart :: Stream (Id Part) Double))
      in joinLines (map fmt11 (sortBy (\a b -> comparing snd b a)
           $$(collect (gt [|| threshold ||] (valuePerPart :: Stream (Id Part) Double))))) ||]
  where
    grouped :: Stream (Id Part) (Double, Int)
    grouped =
      compose (groupBy (restrict (partsupp s)
                         (eq [|| "GERMANY" ||]
                             (compose (compose (psSupplier s) (supplierNation s))
                                      (nationName s))))
                       (psPart s))
              (prod (supplycost s) (availqty s))

fmt11 :: (Id Part, Double) -> String
fmt11 (p, v) = row [key1 p, f2 v]

--------------------------------------------------------------------------------
-- Q12 — shipping modes and order priority
--------------------------------------------------------------------------------

q12 :: CodeQ TPCHS -> CodeQ String
q12 s = withFold step [|| (0, 0) ||] grouped $ \result ->
  [|| joinLines (map fmt12 (sortOn fst
        $$(collect (result :: Stream ByteString (Int, Int))))) ||]
  where
    grouped :: Stream ByteString ByteString
    grouped =
      compose (groupBy (restrict (lineitem s)
                         (prod (prod (isIn [|| ["MAIL", "SHIP"] ||] (shipmode s))
                                     (filt (\p -> [|| case $$p of
                                                        ((sh, c), r) -> sh < c && c < r ||])
                                           (prod (prod (shipdate s) (commitdate s))
                                                 (receiptdate s))))
                               (range [|| 19940101 ||] [|| 19950101 ||] (receiptdate s))))
                       (shipmode s))
              (compose (liOrder s) (priority s))
    step acc pr =
      [|| case ($$acc, $$pr) of
            ((!h, !l), p) | p == "1-URGENT" || p == "2-HIGH" -> (h + 1, l)
                          | otherwise                        -> (h, l + 1) ||]

fmt12 :: (ByteString, (Int, Int)) -> String
fmt12 (m, (h, l)) = row [bs m, show h, show l]

--------------------------------------------------------------------------------
-- Q13 — customer distribution
--------------------------------------------------------------------------------

-- SQL's LEFT JOIN: customers with no qualifying order still count, at zero. That
-- is what `withDenseOuter` is for — it emits every key in the customer id space,
-- seeded with the initial value. `orders` being a sparse universe means driving
-- it already skips the orderkey gaps, so the group key is the bare foreign key
-- with no validity guard of its own.
--
-- The second fold reads `invStream`. Both
-- flip the pairs; `inv` also builds an index for keyed access, which nothing
-- here needs, so the enumeration-only form is the honest choice.
q13 :: CodeQ TPCHS -> CodeQ String
q13 s = withRegex "special.*requests" $ \re ->
  withDenseOuter [|| universeSize (customer_universe $$s) ||]
                 (\a _ -> [|| $$a + (1 :: Int) ||]) [|| 0 ||]
          (groupBy (restrict (orders s) (nrx re (orderComment s)))
                   (orderCustomer s)) $ \countPerCust ->
  withFold (\a _ -> [|| $$a + (1 :: Int) ||]) [|| 0 ||]
           (invStream (countPerCust :: Stream (Id Customer) Int)) $ \dist ->
    [|| joinLines (map fmt13 (sortBy cmp13 $$(collect (dist :: Stream Int Int)))) ||]

cmp13 :: (Int, Int) -> (Int, Int) -> Ordering
cmp13 (k1, v1) (k2, v2) = compare v2 v1 <> compare k2 k1

fmt13 :: (Int, Int) -> String
fmt13 (c, n) = row [show c, show n]

--------------------------------------------------------------------------------
-- Q14 — promo revenue ratio
--------------------------------------------------------------------------------

q14 :: CodeQ TPCHS -> CodeQ String
q14 s = [|| case $$(foldAll step [|| (0, 0) ||] rows) of
              (promo, total) -> f2 (100 * promo / total) ||]
  where
    rows :: Stream (Id Lineitem) ((Double, Double), ByteString)
    rows = compose (restrict (lineitem s)
                     (range [|| 19950901 ||] [|| 19951001 ||] (shipdate s)))
                   (prod (prod (extendedprice s) (discount s))
                         (compose (liPart s) (ty s)))
    step acc v =
      [|| case ($$acc, $$v) of
            ((!p, !t), ((e, dc), typ)) ->
              let dp = e * (1 - dc)
              in (p + (if BS.isPrefixOf "PROMO" typ then dp else 0), t + dp) ||]

--------------------------------------------------------------------------------
-- Q15 — top supplier
--------------------------------------------------------------------------------

q15 :: CodeQ TPCHS -> CodeQ String
q15 s = withFold (\a v -> [|| case $$v of (e, dc) -> $$a + e * (1 - dc) ||]) [|| 0 ||]
                 grouped $ \revenue ->
  [|| let !maxRev = $$(foldAll (\a v -> [|| max $$a $$v ||]) [|| 0 ||]
                               (revenue :: Stream (Id Supplier) Double))
      in joinLines (map fmt15 (sortOn fst
           $$(collect (prod (eq [|| maxRev ||] (revenue :: Stream (Id Supplier) Double))
                            (prod (prod (supplierName s) (supplierAddress s))
                                  (supplierPhone s)))))) ||]
  where
    grouped :: Stream (Id Supplier) (Double, Double)
    grouped =
      compose (groupBy (restrict (lineitem s)
                         (range [|| 19960101 ||] [|| 19960401 ||] (shipdate s)))
                       (liSupplier s))
              (prod (extendedprice s) (discount s))

fmt15 :: (Id Supplier, (Double, ((ByteString, ByteString), ByteString))) -> String
fmt15 (k, (rev, ((nm, addr), phone))) =
  row [key1 k, bs nm, bs addr, bs phone, f2 rev]

--------------------------------------------------------------------------------
-- Q16 — distinct supplier count
--------------------------------------------------------------------------------

q16 :: CodeQ TPCHS -> CodeQ String
q16 s = withRegex "Customer.*Complaints" $ \re ->
  withCountDistinct (grouped re) $ \counts ->
    [|| joinLines (map fmt16 (sortBy cmp16
          $$(collect (counts :: Stream ((ByteString, ByteString), Int) Int)))) ||]
  where
    -- No signature, because writing one would mean naming `Regex` and so taking a
    -- dependency on regex-tdfa in a module that otherwise has no use for it.
    grouped re =
      compose (groupBy (restrict (partsupp s) (prod partOk (suppOk re)))
                       (compose (psPart s) (prod (prod (brand s) (ty s)) (size s))))
              (psSupplier s)
    partOk = restrict (psPart s)
      (prod (prod (ne [|| "Brand#45" ||] (brand s))
                  (filt (\v -> [|| not (BS.isPrefixOf "MEDIUM POLISHED" $$v) ||]) (ty s)))
            (isIn [|| [49, 14, 23, 45, 19, 3, 36, 9] ||] (size s)))
    suppOk re = nrx re (compose (psSupplier s) (supplierComment s))

cmp16 :: (((ByteString, ByteString), Int), Int)
      -> (((ByteString, ByteString), Int), Int) -> Ordering
cmp16 (k1, c1) (k2, c2) = compare c2 c1 <> compare k1 k2

fmt16 :: (((ByteString, ByteString), Int), Int) -> String
fmt16 (((b, t), sz), c) = row [bs b, bs t, show sz, show c]

--------------------------------------------------------------------------------
-- Q17 — small-quantity order revenue
--------------------------------------------------------------------------------

q17 :: CodeQ TPCHS -> CodeQ String
q17 s =
  -- The correlated 0.2 * avg(quantity) per part, built once so the cross-column
  -- comparison below is a keyed lookup and not a re-fold per row.
  withFold (\acc q -> [|| case ($$acc, $$q) of
                            ((!sm, !n), v) -> (sm + v, n + (1 :: Int)) ||])
           [|| (0, 0) ||]
           (compose (groupBy (lineitem s) (liPart s)) (quantity s)) $ \avgTbl ->
  let tpp :: Lookup (Id Part) Double
      tpp = mapv (\p -> [|| case $$p of (sm, n) -> 0.2 * sm / fromIntegral n ||]) avgTbl
      qtyOk = filt (\p -> [|| case $$p of (q, t) -> q < t ||])
                   (prod (quantity s) (compose (liPart s) tpp))
      partOk = restrict (liPart s) (prod (eq [|| "Brand#23" ||] (brand s))
                                         (eq [|| "MED BOX" ||] (container s)))
  in [|| f2 ($$(foldAll (\a v -> [|| $$a + $$v ||]) [|| 0 ||]
                 (compose (restrict (lineitem s) (prod partOk qtyOk))
                          (extendedprice s))) / 7) ||]

--------------------------------------------------------------------------------
-- Q18 — large volume customer
--------------------------------------------------------------------------------

-- Every output column is functionally determined by the order, so they are all
-- attached after the fold rather than dragged through it.
q18 :: CodeQ TPCHS -> CodeQ String
q18 s = withFold (\a v -> [|| $$a + $$v ||]) [|| 0 ||]
                 (compose (groupBy (lineitem s) (liOrder s)) (quantity s)) $ \sumQ ->
  [|| joinLines (map fmt18 (take 100 (sortBy cmp18
        $$(collect (prod (gt [|| 300 ||] (sumQ :: Stream (Id Order) Double))
                         (prod (prod (totalprice s) (date s))
                               (prod (compose (orderCustomer s) (customerName s))
                                     (orderCustomer s)))))))) ||]

type Q18Row = (Double, ((Double, Int), (ByteString, Id Customer)))

cmp18 :: (Id Order, Q18Row) -> (Id Order, Q18Row) -> Ordering
cmp18 (_, (_, ((tp1, d1), _))) (_, (_, ((tp2, d2), _))) =
  compare tp2 tp1 <> compare d1 d2

fmt18 :: (Id Order, Q18Row) -> String
fmt18 (o, (sumQ, ((tp, d), (nm, c)))) =
  row [bs nm, key1 c, key1 o, fmtDate d, f2 tp, f2 sumQ]

--------------------------------------------------------------------------------
-- Q19 — discounted revenue
--------------------------------------------------------------------------------

q19 :: CodeQ TPCHS -> CodeQ String
q19 s = [|| f2 $$(foldAll (\a v -> [|| case $$v of (e, dc) -> $$a + e * (1 - dc) ||])
                          [|| 0 ||] rows) ||]
  where
    rows :: Stream (Id Lineitem) (Double, Double)
    rows = compose (restrict (lineitem s) (prod (prod shipOk instructOk) branches))
                   (prod (extendedprice s) (discount s))
    shipOk     = isIn [|| ["AIR", "AIR REG"] ||] (shipmode s)
    instructOk = eq [|| "DELIVER IN PERSON" ||] (shipinstruct s)
    -- The three-way disjunction cannot be split across conjuncts, so it is one
    -- test over (brand, container, size, quantity).
    branches = filt test (prod (compose (liPart s)
                                        (prod (prod (brand s) (container s)) (size s)))
                               (quantity s))
    test v =
      [|| case $$v of
            (((br, ct), sz), q) ->
                 (br == "Brand#12" && ct `elem` ["SM CASE", "SM BOX", "SM PACK", "SM PKG"]
                    && q >= 1 && q <= 11 && sz >= 1 && sz <= 5)
              || (br == "Brand#23" && ct `elem` ["MED BAG", "MED BOX", "MED PKG", "MED PACK"]
                    && q >= 10 && q <= 20 && sz >= 1 && sz <= 10)
              || (br == "Brand#34" && ct `elem` ["LG CASE", "LG BOX", "LG PACK", "LG PKG"]
                    && q >= 20 && q <= 30 && sz >= 1 && sz <= 15) ||]

--------------------------------------------------------------------------------
-- Q20 — potential part promotion
--------------------------------------------------------------------------------

q20 :: CodeQ TPCHS -> CodeQ String
q20 s =
  -- Per (part, supplier), the quantity shipped in 1994.
  withFold (\a v -> [|| $$a + $$v ||]) [|| 0 ||]
    (compose (groupBy (restrict (lineitem s)
                        (range [|| 19940101 ||] [|| 19950101 ||] (shipdate s)))
                      (prod (liPart s) (liSupplier s)))
             (quantity s)) $ \sumQty ->
  let threshold = mapv (\v -> [|| 0.5 * $$v ||])
                       (compose (prod (psPart s) (psSupplier s)) sumQty)
  in
  -- The qualifying suppliers as a driveable set, so the CANADA filter runs over
  -- those rather than over the whole supplier universe.
  withBits [|| universeSize (supplier_universe $$s) ||]
    (compose (restrict (partsupp s)
               (prod (filt (\v -> [|| BS.isPrefixOf "forest" $$v ||])
                           (compose (psPart s) (partName s)))
                     (filt (\p -> [|| case $$p of (a, t) -> a > t ||])
                           (prod (mapv (\v -> [|| fromIntegral $$v ||]) (availqty s))
                                 threshold))))
             (psSupplier s)) $ \qualSupps ->
    [|| joinLines (map fmt20 (sortOn fst (map snd
          $$(collect (compose (restrict (qualSupps :: Stream (Id Supplier) (Id Supplier))
                                (eq [|| "CANADA" ||]
                                    (compose (supplierNation s) (nationName s))))
                              (prod (supplierName s) (supplierAddress s))))))) ||]

fmt20 :: (ByteString, ByteString) -> String
fmt20 (nm, addr) = row [bs nm, bs addr]

--------------------------------------------------------------------------------
-- Q21 — suppliers who kept orders waiting
--------------------------------------------------------------------------------

-- Three materializers stacked, which is what nesting continuations looks like
-- when a query needs more than one. Each @$ \\name ->@ binds one cache and opens
-- the scope the rest of the query runs in, so the reading order is the build
-- order: both distinct counts first, then the fold that uses them.
--
-- @late@ stays polymorphic and is mentioned three times. That emits three loops
-- over lineitem, and it is meant to: Prela does not materialize unless asked, and
-- it is re-enumerated three times for the same reason.
q21 :: CodeQ TPCHS -> CodeQ String
q21 s =
  withCountDistinct (compose (groupBy (lineitem s) (liOrder s)) (liSupplier s)) $ \allSupp ->
  withCountDistinct (compose (groupBy late (liOrder s)) (liSupplier s)) $ \lateSupp ->
    let -- The order has more than one supplier across all its lines …
        multiSupp :: Lookup (Id Order) Int
        multiSupp = gt [|| 1 ||] allSupp
        -- … but exactly one across its late ones.
        onlyLate :: Lookup (Id Order) Int
        onlyLate = eq [|| 1 ||] lateSupp
        orderOk :: Lookup (Id Lineitem) (Id Order)
        orderOk = restrict (liOrder s)
                    (prod (prod (eq [|| "F" ||] (orderStatus s)) multiSupp) onlyLate)
    in withFold (\a _ -> [|| $$a + (1 :: Int) ||]) [|| 0 ||]
                (groupBy (restrict late (prod natOk orderOk)) (liSupplier s)) $ \tally ->
         [|| joinLines (map fmt21 (take 100 (sortBy cmp21
               $$(collect (prod (tally :: Stream (Id Supplier) Int)
                                (supplierName s)))))) ||]
  where
    late :: SMode q => q (Id Lineitem) (Id Lineitem)
    late = restrict (lineitem s)
             (filt (\v -> [|| case $$v of (c, r) -> c < r ||])
                   (prod (commitdate s) (receiptdate s)))
    natOk :: Lookup (Id Lineitem) ByteString
    natOk = eq [|| "SAUDI ARABIA" ||]
              (compose (compose (liSupplier s) (supplierNation s)) (nationName s))

cmp21 :: (Id Supplier, (Int, ByteString)) -> (Id Supplier, (Int, ByteString)) -> Ordering
cmp21 (_, (c1, n1)) (_, (c2, n2)) = compare c2 c1 <> compare n1 n2

fmt21 :: (Id Supplier, (Int, ByteString)) -> String
fmt21 (_, (c, nm)) = row [bs nm, show c]

--------------------------------------------------------------------------------
-- Q22 — global sales opportunity
--------------------------------------------------------------------------------

-- Two runtime values are introduced inside the generated code rather than the
-- generator: the scalar average, and the bitset of customers who have orders.
-- The average has to be a `let` in the quote because it is a fold over one part
-- of the query used as a bound in another, and the splice that needs it sits
-- inside that @let@'s scope.
q22 :: CodeQ TPCHS -> CodeQ String
q22 s = withBits [|| universeSize (customer_universe $$s) ||] (orderCustomer s) $ \hasOrders ->
  [|| let (sumP, cntP) = $$(foldAll
                             (\acc v -> [|| case $$acc of
                                              (!a, !n) -> (a + $$v, n + (1 :: Int)) ||])
                             [|| (0, 0) ||]
                             (compose (restrict prefixOk (gt [|| 0 ||] (customerAcctbal s)))
                                      (customerAcctbal s)))
          !avg = sumP / fromIntegral cntP
      in $$(withFold (\acc ab -> [|| case $$acc of
                                       (!n, !sm) -> (n + (1 :: Int), sm + $$ab) ||])
                     [|| (0, 0) ||]
                     (compose (groupBy (diff (restrict prefixOk
                                               (gt [|| avg ||] (customerAcctbal s)))
                                             hasOrders)
                                       prefix)
                              (customerAcctbal s))
                     (\counts ->
                        [|| joinLines (map fmt22 (sortOn fst
                              $$(collect (counts :: Stream ByteString (Int, Double))))) ||])) ||]
  where
    prefix :: Lookup (Id Customer) ByteString
    prefix = mapv (\v -> [|| BS.take 2 $$v ||]) (customerPhone s)
    prefixOk :: SMode q => q (Id Customer) (Id Customer)
    prefixOk = restrict (customer s)
                 (isIn [|| ["13", "31", "23", "29", "30", "18", "17"] ||]
                       (mapv (\v -> [|| BS.take 2 $$v ||]) (customerPhone s)))

fmt22 :: (ByteString, (Int, Double)) -> String
fmt22 (c, (n, sm)) = row [bs c, show n, f2 sm]
