{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE OverloadedStrings #-}

-- | The 22 TPC-H queries, ported from the Rust port's idiomatic variant
-- (src/tpch/common.rs on `main`), which was itself a port of the original Julia
-- ones. "Idiomatic" means the algebra is written the way the language wants it
-- rather than the way a cost-based optimizer would rearrange it: there is no
-- optimizer, so the expression IS the plan, and the shape below is the shape
-- that runs.
--
-- Two conventions run through all of them.
--
-- A query takes the loaded schema `s` and reaches every column through a
-- generated accessor applied to it, `quantity s` and `liOrder s`. The accessors
-- are top-level functions rather than record fields precisely so this inlines;
-- see the note in "Prela.Schema".
--
-- Sorting and formatting happen at the very end, on the handful of rows a query
-- actually returns, because SQL's ORDER BY is not a relational operator and
-- Prela has nothing to say about it. `collect` is the boundary: everything above
-- it is fused relational algebra, everything below it is a list.
module TPCH.Queries (queries, inlineOracles) where

import Data.ByteString.Char8 (ByteString)
import qualified Data.ByteString.Char8 as BS
import Data.List (intercalate, sortBy, sortOn)
import Data.Ord (comparing)

import Prela
import TPCH.Schema

--------------------------------------------------------------------------------
-- Leaving the algebra
--------------------------------------------------------------------------------

-- | Drive a relation into a list of pairs, in drive order. Every query ends
-- here: the rows are few by this point, and ordering them is a list problem.
collect :: Drv d r -> [(d, r)]
collect q = reverse (snd (runAcc (drive q step) []))
  where step x y = Acc (\acc -> ((), (x, y) : acc))

-- | Two decimal places, matching Rust's @{:.2}@ and DuckDB's money output.
f2 :: Double -> String
f2 x = sign ++ show whole ++ "." ++ pad (show cents)
  where
    r     = round (abs x * 100) :: Integer
    whole = r `div` 100
    cents = r `mod` 100
    sign  = if x < 0 && r /= 0 then "-" else ""
    pad d = replicate (2 - length d) '0' ++ d

-- | The cache stores dates as yyyymmdd integers (regen parses them once at load
-- time), so printing one is arithmetic rather than a calendar library.
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
key1 (Id i) = show (i + 1)

--------------------------------------------------------------------------------
-- Q1 — pricing summary report
--------------------------------------------------------------------------------

q1 :: TPCH -> String
q1 s = joinLines (map fmt (sortOn fst (collect grouped)))
  where
    grouped :: Drv (ByteString, ByteString) (Double, Double, Double, Double, Double, Int)
    grouped = fold step (0, 0, 0, 0, 0, 0)
      (compose (groupBy (restrict (lineitem s) (le 19980902 (shipdate s)))
                        (prod (returnflag s) (lineStatus s)))
               (prod (prod (prod (quantity s) (extendedprice s)) (discount s)) (tax s)))
    step (!qty, !ext, !di, !dp, !chg, !n) (((q, e), dc), tx) =
      let dpInc  = e * (1 - dc)
          chgInc = dpInc * (1 + tx)
      in (qty + q, ext + e, di + dc, dp + dpInc, chg + chgInc, n + 1)
    fmt ((rf, ls), (qty, ext, di, dp, chg, n)) =
      row [ bs rf, bs ls, f2 qty, f2 ext, f2 dp, f2 chg
          , f2 (qty / nf), f2 (ext / nf), f2 (di / nf), show n ]
      where nf = fromIntegral n

--------------------------------------------------------------------------------
-- Q2 — minimum-cost supplier per part
--------------------------------------------------------------------------------

q2 :: TPCH -> String
q2 s = joinLines (map fmt (take 100 (sortBy cmp (map (flat . snd) (collect result)))))
  where
    -- Partsupp rows whose supplier sits in EUROPE, driven twice: once to find
    -- the cheapest cost per part, once for the answer itself.
    euPs :: Drv (Id PartSupp) (Id PartSupp)
    euPs = restrict (partsupp s)
             (eq "EUROPE" (compose (compose (compose (psSupplier s) (supplierNation s))
                                            (nationRegion s))
                                   (regionName s)))
    minPerPart :: Prb (Id Part) Double
    minPerPart = fold (\a c -> if c < a then c else a) (1 / 0)
                      (compose (groupBy euPs (psPart s)) (supplycost s))
    result :: Drv (Id PartSupp) (((((((Double, ByteString), ByteString), ByteString), ByteString), ByteString), Id Part), ByteString)
    result = compose (restrict euPs (prod partOk costIsMin)) payload
    partOk = restrict (psPart s) (prod (eq 15 (size s))
                                       (filt (BS.isSuffixOf "BRASS") (ty s)))
    costIsMin = filt (\(c, m) -> c == m)
                     (prod (supplycost s) (compose (psPart s) minPerPart))
    payload = prod (prod (compose (psSupplier s) suppCols) (psPart s))
                   (compose (psPart s) (mfgr s))
    suppCols = prod (prod (prod (prod (prod (supplierAcctbal s) (supplierName s))
                                      (compose (supplierNation s) (nationName s)))
                                (supplierAddress s))
                          (supplierPhone s))
                    (supplierComment s)
    -- Flatten the nested pairs once, so the comparator and the formatter both
    -- read named components instead of unpicking a seven-deep tuple.
    flat ((((((((acct, sname), nat), addr), phone), comm), pk), mfg)) =
      (acct, sname, nat, pk, mfg, addr, phone, comm)
    cmp a b = comparing acct b a <> comparing nat a b
                                 <> comparing sname a b <> comparing pk a b
      where acct  (x, _, _, _, _, _, _, _) = x
            sname (_, x, _, _, _, _, _, _) = x
            nat   (_, _, x, _, _, _, _, _) = x
            pk    (_, _, _, x, _, _, _, _) = x
    fmt (acct, sname, nat, pk, mfg, addr, phone, comm) =
      row [f2 acct, bs sname, bs nat, key1 pk, bs mfg, bs addr, bs phone, bs comm]

--------------------------------------------------------------------------------
-- Q3 — shipping priority
--------------------------------------------------------------------------------

-- SQL groups by (l_orderkey, o_orderdate, o_shippriority). The orderkey fixes
-- the other two, so all three ride in the group key and the output reads them
-- straight off it, with no second lookup.
q3 :: TPCH -> String
q3 s = joinLines (map fmt (take 10 (sortBy cmp (collect revenue))))
  where
    revenue :: Drv ((Id Order, Int), Int) Double
    revenue = fold (\a (e, dc) -> a + e * (1 - dc)) 0
      (compose (groupBy (restrict (lineitem s)
                          (prod (prod (gt 19950315 (shipdate s))
                                      (lt 19950315 (compose (liOrder s) (date s))))
                                (eq "BUILDING" (compose (compose (liOrder s) (orderCustomer s))
                                                        (mktsegment s)))))
                        (prod (prod (liOrder s) (compose (liOrder s) (date s)))
                              (compose (liOrder s) (shippriority s))))
               (prod (extendedprice s) (discount s)))
    cmp (((_, d1), _), r1) (((_, d2), _), r2) = compare r2 r1 <> compare d1 d2
    fmt (((o, d), sp), r) = row [key1 o, f2 r, fmtDate d, show sp]

--------------------------------------------------------------------------------
-- Q4 — order priority checking
--------------------------------------------------------------------------------

q4 :: TPCH -> String
q4 s = joinLines (map fmt (sortOn fst (collect counts)))
  where
    -- Orders with at least one late line, precomputed as a bitset so the
    -- EXISTS is a bit test per order rather than a re-scan.
    lateOrder :: Prb (Id Order) (Id Order)
    lateOrder = bitset (order_n s)
      (compose (restrict (lineitem s)
                 (filt (\(c, r) -> c < r) (prod (commitdate s) (receiptdate s))))
               (liOrder s))
    counts :: Drv ByteString Int
    counts = fold (\a _ -> a + 1) 0
      (groupBy (restrict (restrict (orders s) (range 19930701 19931001 (date s))) lateOrder)
               (priority s))
    fmt (p, n) = row [bs p, show n]

--------------------------------------------------------------------------------
-- Q5 — local supplier volume
--------------------------------------------------------------------------------

-- The group key carries the work: a lineitem maps to its supplier's nation name,
-- with the ASIA restriction and the customer-nation equality pushed into the key
-- itself. A row whose key probe yields nothing drops out, so the key doubles as
-- the filter and only the date window rides on the receiver.
q5 :: TPCH -> String
q5 s = joinLines (map fmt (sortBy (\a b -> comparing snd b a) (collect result)))
  where
    result :: Drv ByteString Double
    result = fold (\a (e, dc) -> a + e * (1 - dc)) 0
      (compose (groupBy (restrict (lineitem s)
                          (range 19940101 19950101 (compose (liOrder s) (date s))))
                        (compose sameNation (nationName s)))
               (prod (extendedprice s) (discount s)))
    sameNation = mapv fst (filt (uncurry (==))
      (prod (restrict (compose (liSupplier s) (supplierNation s))
                      (eq "ASIA" (compose (nationRegion s) (regionName s))))
            (compose (compose (liOrder s) (orderCustomer s)) (customerNation s))))
    fmt (n, v) = row [bs n, f2 v]

--------------------------------------------------------------------------------
-- Q6 — forecasting revenue change
--------------------------------------------------------------------------------

q6 :: TPCH -> String
q6 s = f2 (foldAll (\a (e, dc) -> a + e * dc) 0
  (compose (restrict (lineitem s)
             (prod (prod (range 19940101 19950101 (shipdate s))
                         (between 0.05 0.07 (discount s)))
                   (lt 24.0 (quantity s))))
           (prod (extendedprice s) (discount s))))

--------------------------------------------------------------------------------
-- Q7 — volume shipping between nation pairs
--------------------------------------------------------------------------------

q7 :: TPCH -> String
q7 s = joinLines (map fmt (sortBy cmp (collect result)))
  where
    result :: Drv (Int, (ByteString, ByteString)) Double
    result = fold (\a (e, dc) -> a + e * (1 - dc)) 0
      (compose (groupBy (lineitem s)
                        (prod (mapv (`div` 10000) (between 19950101 19961231 (shipdate s)))
                              (filt pair (prod supNation custNation))))
               (prod (extendedprice s) (discount s)))
    supNation  = compose (compose (liSupplier s) (supplierNation s)) (nationName s)
    custNation = compose (compose (compose (liOrder s) (orderCustomer s)) (customerNation s))
                         (nationName s)
    pair (a, b) = (a == "FRANCE" && b == "GERMANY") || (a == "GERMANY" && b == "FRANCE")
    cmp ((y1, (s1, c1)), _) ((y2, (s2, c2)), _) =
      compare s1 s2 <> compare c1 c2 <> compare y1 y2
    fmt ((y, (n1, n2)), v) = row [bs n1, bs n2, show y, f2 v]

--------------------------------------------------------------------------------
-- Q8 — market share for BRAZIL
--------------------------------------------------------------------------------

-- Per year, the BRAZIL share of the volume on ECONOMY ANODIZED STEEL parts sold
-- to customers in AMERICA. The group key navigates the order once — restricting
-- it and taking its year in one hop — and rows failing that navigation drop out.
q8 :: TPCH -> String
q8 s = joinLines (map fmt (sortOn fst (collect result)))
  where
    result :: Drv Int Double
    result = mapv (\(b, t) -> b / t) (fold step (0, 0)
      (compose (groupBy (restrict (lineitem s)
                          (eq "ECONOMY ANODIZED STEEL" (compose (liPart s) (ty s))))
                        (mapv (`div` 10000)
                          (compose (restrict (liOrder s)
                                     (eq "AMERICA" (compose (compose (compose (orderCustomer s)
                                                                              (customerNation s))
                                                                     (nationRegion s))
                                                            (regionName s))))
                                   (between 19950101 19961231 (date s)))))
               (prod (prod (extendedprice s) (discount s))
                     (compose (compose (liSupplier s) (supplierNation s)) (nationName s)))))
    step (!b, !t) ((e, dc), nm) =
      let v = e * (1 - dc)
      in (b + (if nm == "BRAZIL" then v else 0), t + v)
    fmt (y, v) = row [show y, f2 v]

--------------------------------------------------------------------------------
-- Q9 — product type profit measure
--------------------------------------------------------------------------------

q9 :: TPCH -> String
q9 s = joinLines (map fmt (sortBy cmp (collect result)))
  where
    -- Supply cost per (green part, supplier). The probe misses on every
    -- non-green part, so restricting the scan by it culls most of the lineitems
    -- before the group key's navigation runs; the payload re-probes for the value.
    sc :: Prb (Id Part, Id Supplier) Double
    sc = materialize
      (compose (groupBy (partsupp s)
                        (prod (restrict (psPart s) (filt (BS.isInfixOf "green") (partName s)))
                              (psSupplier s)))
               (supplycost s))
    costPerLi :: Prb (Id Lineitem) Double
    costPerLi = compose (prod (liPart s) (liSupplier s)) sc
    result :: Drv (ByteString, Int) Double
    result = fold (\a (((cost, e), dc), q) -> a + e * (1 - dc) - cost * q) 0
      (compose (groupBy (restrict (lineitem s) costPerLi)
                        (prod (compose (compose (liSupplier s) (supplierNation s)) (nationName s))
                              (mapv (`div` 10000) (compose (liOrder s) (date s)))))
               (prod (prod (prod costPerLi (extendedprice s)) (discount s)) (quantity s)))
    cmp ((n1, y1), _) ((n2, y2), _) = compare n1 n2 <> compare y2 y1
    fmt ((n, y), v) = row [bs n, show y, f2 v]

--------------------------------------------------------------------------------
-- Q10 — returned-item reporting
--------------------------------------------------------------------------------

q10 :: TPCH -> String
q10 s = joinLines (map fmt (take 20 (sortBy cmp (collect revenue))))
  where
    revenue :: Drv (Id Customer) (Double, (((((ByteString, Double), ByteString), ByteString), ByteString), ByteString))
    revenue = prod (fold (\a (e, dc) -> a + e * (1 - dc)) 0
                     (compose (groupBy (restrict (lineitem s)
                                         (prod (eq "R" (returnflag s))
                                               (range 19931001 19940101
                                                      (compose (liOrder s) (date s)))))
                                       (compose (liOrder s) (orderCustomer s)))
                              (prod (extendedprice s) (discount s))))
                   custCols
    custCols = prod (prod (prod (prod (prod (customerName s) (customerAcctbal s))
                                      (compose (customerNation s) (nationName s)))
                                (customerAddress s))
                          (customerPhone s))
                    (customerComment s)
    cmp (_, (r1, _)) (_, (r2, _)) = compare r2 r1
    fmt (c, (r, (((((nm, ab), nat), addr), phone), comm))) =
      row [key1 c, bs nm, f2 r, f2 ab, bs nat, bs addr, bs phone, bs comm]

--------------------------------------------------------------------------------
-- Q11 — important stock
--------------------------------------------------------------------------------

q11 :: TPCH -> String
q11 s = joinLines (map fmt (sortBy (\a b -> comparing snd b a) (collect above)))
  where
    -- Driven twice, so it is bound at one mode and its cache is shared: once to
    -- total it for the threshold, once to filter against that threshold.
    valuePerPart :: Drv (Id Part) Double
    valuePerPart = fold (\a (c, q) -> a + c * fromIntegral q) 0
      (compose (groupBy (restrict (partsupp s)
                          (eq "GERMANY" (compose (compose (psSupplier s) (supplierNation s))
                                                 (nationName s))))
                        (psPart s))
               (prod (supplycost s) (availqty s)))
    threshold = 0.0001 * foldAll (+) 0 valuePerPart
    above :: Drv (Id Part) Double
    above = gt threshold valuePerPart
    fmt (p, v) = row [key1 p, f2 v]

--------------------------------------------------------------------------------
-- Q12 — shipping modes and order priority
--------------------------------------------------------------------------------

q12 :: TPCH -> String
q12 s = joinLines (map fmt (sortOn fst (collect result)))
  where
    result :: Drv ByteString (Int, Int)
    result = fold step (0, 0)
      (compose (groupBy (restrict (lineitem s)
                          (prod (prod (isIn ["MAIL", "SHIP"] (shipmode s))
                                      (filt (\((sh, c), r) -> sh < c && c < r)
                                            (prod (prod (shipdate s) (commitdate s))
                                                  (receiptdate s))))
                                (range 19940101 19950101 (receiptdate s))))
                        (shipmode s))
               (compose (liOrder s) (priority s)))
    step (!h, !l) pr | pr == "1-URGENT" || pr == "2-HIGH" = (h + 1, l)
                     | otherwise                          = (h, l + 1)
    fmt (m, (h, l)) = row [bs m, show h, show l]

--------------------------------------------------------------------------------
-- Q13 — customer distribution
--------------------------------------------------------------------------------

-- SQL's LEFT JOIN: customers with no qualifying order still count, at zero. That
-- is what `foldDenseOuter` is for — it emits every key in the customer id space,
-- seeded with the initial value. `orders` being a sparse universe means driving
-- it already skips the orderkey gaps, so the group key is the bare foreign key
-- with no validity guard of its own.
q13 :: TPCH -> String
q13 s = joinLines (map fmt (sortBy cmp (collect dist)))
  where
    countPerCust :: Drv (Id Customer) Int
    countPerCust = foldDenseOuter (customer_n s) (\a _ -> a + 1) 0
      (groupBy (restrict (orders s) (nrx "special.*requests" (orderComment s)))
               (orderCustomer s))
    dist :: Drv Int Int
    dist = fold (\a _ -> a + 1) 0 (inv countPerCust)
    cmp (k1, v1) (k2, v2) = compare v2 v1 <> compare k2 k1
    fmt (c, n) = row [show c, show n]

--------------------------------------------------------------------------------
-- Q14 — promo revenue ratio
--------------------------------------------------------------------------------

q14 :: TPCH -> String
q14 s = f2 (100 * promo / total)
  where
    (promo, total) = foldAll step (0, 0)
      (compose (restrict (lineitem s) (range 19950901 19951001 (shipdate s)))
               (prod (prod (extendedprice s) (discount s)) (compose (liPart s) (ty s))))
    step (!p, !t) ((e, dc), typ) =
      let dp = e * (1 - dc)
      in (p + (if BS.isPrefixOf "PROMO" typ then dp else 0), t + dp)

--------------------------------------------------------------------------------
-- Q15 — top supplier
--------------------------------------------------------------------------------

q15 :: TPCH -> String
q15 s = joinLines (map fmt (sortOn fst (collect result)))
  where
    revenue :: Drv (Id Supplier) Double
    revenue = fold (\a (e, dc) -> a + e * (1 - dc)) 0
      (compose (groupBy (restrict (lineitem s) (range 19960101 19960401 (shipdate s)))
                        (liSupplier s))
               (prod (extendedprice s) (discount s)))
    maxRev = foldAll max 0 revenue
    result :: Drv (Id Supplier) (Double, ((ByteString, ByteString), ByteString))
    result = prod (eq maxRev revenue)
                  (prod (prod (supplierName s) (supplierAddress s)) (supplierPhone s))
    fmt (k, (rev, ((nm, addr), phone))) = row [key1 k, bs nm, bs addr, bs phone, f2 rev]

--------------------------------------------------------------------------------
-- Q16 — distinct supplier count
--------------------------------------------------------------------------------

q16 :: TPCH -> String
q16 s = joinLines (map fmt (sortBy cmp (collect counts)))
  where
    counts :: Drv ((ByteString, ByteString), Int) Int
    counts = countDistinct
      (compose (groupBy (restrict (partsupp s) (prod partOk suppOk))
                        (compose (psPart s) (prod (prod (brand s) (ty s)) (size s))))
               (psSupplier s))
    partOk = restrict (psPart s)
      (prod (prod (ne "Brand#45" (brand s))
                  (filt (not . BS.isPrefixOf "MEDIUM POLISHED") (ty s)))
            (isIn [49, 14, 23, 45, 19, 3, 36, 9] (size s)))
    suppOk = nrx "Customer.*Complaints" (compose (psSupplier s) (supplierComment s))
    cmp (k1, c1) (k2, c2) = compare c2 c1 <> compare k1 k2
    fmt (((b, t), sz), c) = row [bs b, bs t, show sz, show c]

--------------------------------------------------------------------------------
-- Q17 — small-quantity order revenue
--------------------------------------------------------------------------------

q17 :: TPCH -> String
q17 s = f2 (total / 7)
  where
    -- The correlated 0.2 * avg(quantity) per part, materialized once so the
    -- cross-column comparison below is a probe and not a re-fold per row.
    tpp :: Prb (Id Part) Double
    tpp = mapv (\(sm, n) -> 0.2 * sm / fromIntegral n)
               (fold (\(!sm, !n) q -> (sm + q, n + 1 :: Int)) (0, 0)
                     (compose (groupBy (lineitem s) (liPart s)) (quantity s)))
    total = foldAll (+) 0
      (compose (restrict (lineitem s) (prod partOk qtyOk)) (extendedprice s))
    partOk = restrict (liPart s) (prod (eq "Brand#23" (brand s)) (eq "MED BOX" (container s)))
    qtyOk = filt (\(q, t) -> q < t) (prod (quantity s) (compose (liPart s) tpp))

--------------------------------------------------------------------------------
-- Q18 — large volume customer
--------------------------------------------------------------------------------

-- Every output column is functionally determined by the order, so they are all
-- attached after the fold rather than dragged through it.
q18 :: TPCH -> String
q18 s = joinLines (map fmt (take 100 (sortBy cmp (collect result))))
  where
    result :: Drv (Id Order) (Double, ((Double, Int), (ByteString, Id Customer)))
    result = prod (gt 300 (fold (+) 0 (compose (groupBy (lineitem s) (liOrder s))
                                               (quantity s))))
                  (prod (prod (totalprice s) (date s))
                        (prod (compose (orderCustomer s) (customerName s)) (orderCustomer s)))
    cmp (_, (_, ((tp1, d1), _))) (_, (_, ((tp2, d2), _))) = compare tp2 tp1 <> compare d1 d2
    fmt (o, (sumQ, ((tp, d), (nm, c)))) =
      row [bs nm, key1 c, key1 o, fmtDate d, f2 tp, f2 sumQ]

--------------------------------------------------------------------------------
-- Q19 — discounted revenue
--------------------------------------------------------------------------------

q19 :: TPCH -> String
q19 s = f2 (foldAll (\a (e, dc) -> a + e * (1 - dc)) 0
  (compose (restrict (lineitem s) (prod (prod shipOk instructOk) branches))
           (prod (extendedprice s) (discount s))))
  where
    shipOk     = isIn ["AIR", "AIR REG"] (shipmode s)
    instructOk = eq "DELIVER IN PERSON" (shipinstruct s)
    -- The three-way disjunction cannot be split across conjuncts, so it is one
    -- closure over (brand, container, size, quantity).
    branches = filt test (prod (compose (liPart s) (prod (prod (brand s) (container s))
                                                         (size s)))
                               (quantity s))
    test (((br, ct), sz), q) =
         (br == "Brand#12" && ct `elem` ["SM CASE", "SM BOX", "SM PACK", "SM PKG"]
            && q >= 1 && q <= 11 && sz >= 1 && sz <= 5)
      || (br == "Brand#23" && ct `elem` ["MED BAG", "MED BOX", "MED PKG", "MED PACK"]
            && q >= 10 && q <= 20 && sz >= 1 && sz <= 10)
      || (br == "Brand#34" && ct `elem` ["LG CASE", "LG BOX", "LG PACK", "LG PKG"]
            && q >= 20 && q <= 30 && sz >= 1 && sz <= 15)

--------------------------------------------------------------------------------
-- Q20 — potential part promotion
--------------------------------------------------------------------------------

q20 :: TPCH -> String
q20 s = joinLines (map fmt (sortOn fst (map snd (collect result))))
  where
    -- Per (part, supplier), the quantity shipped in 1994.
    sumQty :: Prb (Id Part, Id Supplier) Double
    sumQty = fold (+) 0
      (compose (groupBy (restrict (lineitem s) (range 19940101 19950101 (shipdate s)))
                        (prod (liPart s) (liSupplier s)))
               (quantity s))
    threshold = mapv (0.5 *) (compose (prod (psPart s) (psSupplier s)) sumQty)
    -- The qualifying suppliers as a driveable set, so the CANADA filter runs
    -- over those rather than over the whole supplier universe.
    qualSupps :: Drv (Id Supplier) (Id Supplier)
    qualSupps = bitset (supplier_n s)
      (compose (restrict (partsupp s)
                 (prod (filt (BS.isPrefixOf "forest") (compose (psPart s) (partName s)))
                       (filt (\(a, t) -> a > t)
                             (prod (mapv fromIntegral (availqty s)) threshold))))
               (psSupplier s))
    result :: Drv (Id Supplier) (ByteString, ByteString)
    result = compose (restrict qualSupps
                       (eq "CANADA" (compose (supplierNation s) (nationName s))))
                     (prod (supplierName s) (supplierAddress s))
    fmt (nm, addr) = row [bs nm, bs addr]

--------------------------------------------------------------------------------
-- Q21 — suppliers who kept orders waiting
--------------------------------------------------------------------------------

q21 :: TPCH -> String
q21 s = joinLines (map fmt (take 100 (sortBy cmp (collect counts))))
  where
    late :: Drv (Id Lineitem) (Id Lineitem)
    late = restrict (lineitem s)
             (filt (\(c, r) -> c < r) (prod (commitdate s) (receiptdate s)))
    -- The order has more than one supplier across all its lines …
    multiSupp :: Prb (Id Order) Int
    multiSupp = gt 1 (countDistinct (compose (groupBy (lineitem s) (liOrder s))
                                             (liSupplier s)))
    -- … but exactly one across its late ones.
    onlyLate :: Prb (Id Order) Int
    onlyLate = eq 1 (countDistinct (compose (groupBy late (liOrder s)) (liSupplier s)))
    counts :: Drv (Id Supplier) (Int, ByteString)
    counts = prod (fold (\a _ -> a + 1) 0
                    (groupBy (restrict late (prod natOk orderOk)) (liSupplier s)))
                  (supplierName s)
    natOk = eq "SAUDI ARABIA" (compose (compose (liSupplier s) (supplierNation s))
                                       (nationName s))
    orderOk = restrict (liOrder s)
                (prod (prod (eq "F" (orderStatus s)) multiSupp) onlyLate)
    cmp (_, (c1, n1)) (_, (c2, n2)) = compare c2 c1 <> compare n1 n2
    fmt (_, (c, nm)) = row [bs nm, show c]

--------------------------------------------------------------------------------
-- Q22 — global sales opportunity
--------------------------------------------------------------------------------

q22 :: TPCH -> String
q22 s = joinLines (map fmt (sortOn fst (collect counts)))
  where
    prefix :: Prb (Id Customer) ByteString
    prefix = mapv (BS.take 2) (customerPhone s)
    prefixOk :: Drv (Id Customer) (Id Customer)
    prefixOk = restrict (customer s)
                 (isIn ["13", "31", "23", "29", "30", "18", "17"]
                       (mapv (BS.take 2) (customerPhone s)))
    -- The scalar subquery: average balance over the positive-balance customers
    -- with one of those phone prefixes.
    (sumP, cntP) = foldAll (\(!a, !n) v -> (a + v, n + 1 :: Int)) (0, 0)
      (compose (restrict prefixOk (gt 0 (customerAcctbal s))) (customerAcctbal s))
    avg = sumP / fromIntegral cntP
    hasOrders :: Prb (Id Customer) (Id Customer)
    hasOrders = bitset (customer_n s) (orderCustomer s)
    counts :: Drv ByteString (Int, Double)
    counts = fold (\(!n, !sm) ab -> (n + 1, sm + ab)) (0, 0)
      (compose (groupBy (diff (restrict prefixOk (gt avg (customerAcctbal s))) hasOrders)
                        prefix)
               (customerAcctbal s))
    fmt (c, (n, sm)) = row [bs c, show n, f2 sm]

--------------------------------------------------------------------------------
-- The registry
--------------------------------------------------------------------------------

-- | Every query, in TPC-H numbering order, paired with its runner. The oracle
-- for each is either inlined below (when it is a few lines) or read out of
-- ../oracles/tpch/Q<n>.txt by the runner, which is how the Rust port splits them
-- too — the two ports check against the same recorded DuckDB output.
queries :: [(String, TPCH -> String)]
queries =
  [ ("1", q1),   ("2", q2),   ("3", q3),   ("4", q4),   ("5", q5)
  , ("6", q6),   ("7", q7),   ("8", q8),   ("9", q9),   ("10", q10)
  , ("11", q11), ("12", q12), ("13", q13), ("14", q14), ("15", q15)
  , ("16", q16), ("17", q17), ("18", q18), ("19", q19), ("20", q20)
  , ("21", q21), ("22", q22)
  ]

-- | The short oracles. Anything not here is a file.
inlineOracles :: [(String, String)]
inlineOracles =
  [ ("1", joinLines
      [ "A|F|37734107.00|56586554400.73|53758257134.87|55909065222.83|25.52|38273.13|0.05|1478493"
      , "N|F|991417.00|1487504710.38|1413082168.05|1469649223.19|25.52|38284.47|0.05|38854"
      , "N|O|74476040.00|111701729697.74|106118230307.61|110367043872.49|25.50|38249.12|0.05|2920374"
      , "R|F|37719753.00|56568041380.90|53741292684.60|55889619119.83|25.51|38250.85|0.05|1478870"
      ])
  , ("3", joinLines
      [ "2456423|406181.01|1995-03-05|0"
      , "3459808|405838.70|1995-03-04|0"
      , "492164|390324.06|1995-02-19|0"
      , "1188320|384537.94|1995-03-09|0"
      , "2435712|378673.06|1995-02-26|0"
      , "4878020|378376.80|1995-03-12|0"
      , "5521732|375153.92|1995-03-13|0"
      , "2628192|373133.31|1995-02-22|0"
      , "993600|371407.46|1995-03-05|0"
      , "2300070|367371.15|1995-03-13|0"
      ])
  , ("4", joinLines
      [ "1-URGENT|10594"
      , "2-HIGH|10476"
      , "3-MEDIUM|10410"
      , "4-NOT SPECIFIED|10556"
      , "5-LOW|10487"
      ])
  , ("5", joinLines
      [ "INDONESIA|55502041.17"
      , "VIETNAM|55295087.00"
      , "CHINA|53724494.26"
      , "INDIA|52035512.00"
      , "JAPAN|45410175.70"
      ])
  , ("6", "123141078.23")
  , ("10", joinLines
      [ "57040|Customer#000057040|734235.25|632.87|JAPAN|nICtsILWBB|22-895-641-3466|ep. blithely regular foxes promise slyly furiously ironic depend"
      , "143347|Customer#000143347|721002.69|2557.47|EGYPT|,Q9Ml3w0gvX|14-742-935-3718|endencies sleep. slyly express deposits nag carefully around the even tithes. slyly regular "
      , "60838|Customer#000060838|679127.31|2454.77|BRAZIL|VWmQhWweqj5hFpcvhGFBeOY9hJ4m|12-913-494-9813|tes. final instructions nag quickly according to"
      , "101998|Customer#000101998|637029.57|3790.89|UNITED KINGDOM|0,ORojfDdyMca2E2H|33-593-865-6378|ost carefully. slyly regular packages cajole about the blithely final ideas. permanently daring deposit"
      , "125341|Customer#000125341|633508.09|4983.51|GERMANY|9YRcnoUPOM7Sa8xymhsDHdQg|17-582-695-5962|ly furiously brave packages. quickly regular dugouts kindle furiously carefully bold theodolites. "
      , "25501|Customer#000025501|620269.78|7725.04|ETHIOPIA|sr4VVVe3xCJQ2oo2QEhi19D,pXqo6kOGaSn2|15-874-808-6793|y ironic foxes hinder according to the furiously permanent dolphins. pending ideas integrate blithely from "
      , "115831|Customer#000115831|596423.87|5098.10|FRANCE|AlMpPnmtGrOFrDMUs5VLo EIA,Cg,Rw5TBuBoKiO|16-715-386-3788|unts nag carefully final packages. express theodolites are regular ac"
      , "84223|Customer#000084223|594998.02|528.65|UNITED KINGDOM|Eq51o UpQ4RBr  fYTdrZApDsPV4pQyuPq|33-442-824-8191|longside of the slyly final deposits. blithely final platelets about the blithely i"
      , "54289|Customer#000054289|585603.39|5583.02|IRAN|x3ouCpz6,pRNVhajr0CCQG1|20-834-292-4707| cajole furiously after the quickly unusual fo"
      , "39922|Customer#000039922|584878.11|7321.11|GERMANY|2KtWzW,FYkhdWBfobp6SFXWYKjvU9|17-147-757-8036|ironic deposits sublate furiously. carefully regular theodolites along the b"
      , "6226|Customer#000006226|576783.76|2230.09|UNITED KINGDOM|TKbxS1dbkGMtaa,KOi26lbip4P0tPbWK0|33-657-701-3391|nal packages are alongside of the quickly bold deposits. carefully "
      , "922|Customer#000000922|576767.53|3869.25|GERMANY|rsR9lRxyTdHbDOVt8nYbwjK5vAWH9sB|17-945-916-9648|cuses cajole carefully regular idea"
      , "147946|Customer#000147946|576455.13|2030.13|ALGERIA|Jqdt1kHAJtuTqHQK,B7 3tJh|10-886-956-3143|ly pending platelets. ironic requests haggle alongside of the furiou"
      , "115640|Customer#000115640|569341.19|6436.10|ARGENTINA|6yKLIRRAirUmBjKNO6Z3|11-411-543-4901|ffily ironic deposits. blithely specia"
      , "73606|Customer#000073606|568656.86|1785.67|JAPAN|vx9,7ACVtoKnLcoAHGNYDF|22-437-653-6966|uests cajole according to the foxe"
      , "110246|Customer#000110246|566842.98|7763.35|VIETNAM|UgsLFL3rendATzcHi|31-943-426-9837|ow carefully. blithely careful packages hag"
      , "142549|Customer#000142549|563537.24|5085.99|INDONESIA|pJAmChWXct HNjPzgoBUOgAHduwwIR|19-955-562-2398|. slyly bold packages nag quickly against the unusual deposits. express asymptotes detect furiously pending, eve"
      , "146149|Customer#000146149|557254.99|1791.55|ROMANIA| STLwtlaB6|29-744-164-6487|nic, special instructions. multipliers run carefully blithely iro"
      , "52528|Customer#000052528|556397.35|551.79|ARGENTINA|elsyt8c9Z,7ch|11-208-192-3205|olphins. blithely silent platelets affix carefully even platelets. ca"
      , "23431|Customer#000023431|554269.54|3381.86|ROMANIA|kKI5,CJAJQjQRQtOdCiFQ|29-915-458-2654|the final sentiments. carefully ironic packages"
      ])
  , ("12", joinLines
      [ "MAIL|6202|9324"
      , "SHIP|6200|9262"
      ])
  , ("14", "16.38")
  ]
