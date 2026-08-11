{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE RankNTypes #-}

-- | The 22 TPC-H queries on the staged pull engine.
--
-- Each definition is a pure, quote-free `Q.Query` builder. `Q.compile` (in
-- "TPCH.Staged") turns it into an ordinary function. Generated scalars use
-- normal numeric operations, predicates accept ordinary literals, and
-- materializers bind with @<-@ in `do` notation.
--
-- The generation monad is pure: its sequencing describes the scopes of runtime
-- caches. A relation bound by `Q.groupFold`, `Q.materialize`, or `Q.bitset` is
-- constructed once in generated code and may safely be used more than once.
--
-- Queries return typed rows or scalars. The `renderN` functions below them are
-- ordinary Haskell applied after compilation; sorting, limiting, and report
-- formatting therefore never cross the staging boundary.
module TPCH.StagedQueries
  ( q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11
  , q12, q13, q14, q15, q16, q17, q18, q19, q20, q21, q22
  , render1, render2, render3, render4, render5, render6, render7, render8
  , render9, render10, render11, render12, render13, render14, render15
  , render16, render17, render18, render19, render20, render21, render22
  ) where

import Data.ByteString.Char8 (ByteString)
import qualified Data.ByteString.Char8 as BS
import Data.List (intercalate, sortBy, sortOn)
import Data.Ord (comparing)
import Prela.PullStaged.Ops
import Prela.PullStaged.Stream
import qualified Prela.PullStaged.Query as Q
import Prela.Id (Id, idIndex)

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

q1 :: Q.Query TPCHS [((ByteString, ByteString), Q1Acc)]
q1 = Q.query build
  where
    build s = do
      tbl <- Q.groupFold step (Q.tuple6 0 0 0 0 0 0) grouped
      pure (Q.collect (Q.stream tbl))
      where
        grouped :: Stream (ByteString, ByteString) (((Double, Double), Double), Double)
        grouped =
          compose (groupBy (restrict (lineitem s) (Q.le 19980902 (shipdate s)))
                           (prod (returnflag s) (lineStatus s)))
                  (prod (prod (prod (quantity s) (extendedprice s)) (discount s)) (tax s))
        step acc = Q.onPair $ \edc tx ->
          Q.onPair (\qe dc ->
            Q.onPair (\q e ->
              Q.onTuple6 (\qty ext di dp chg n ->
                Q.letScalar (e * (1 - dc)) $ \dpInc ->
                Q.letScalar (dpInc * (1 + tx)) $ \chgInc ->
                  Q.tuple6 (qty + q) (ext + e) (di + dc)
                           (dp + dpInc) (chg + chgInc) (n + 1)) acc) qe) edc

-- The accumulator needs a name because `Q.groupFold` demands `UV.Unbox` on it, and
-- a bare six-tuple has no such instance. This is the one place staging asks for
-- something ordinary generated loops cannot infer implicitly.
type Q1Acc = (Double, Double, Double, Double, Double, Int)

fmt1 :: ((ByteString, ByteString), Q1Acc) -> String
fmt1 ((rf, ls), (qty, ext, di, dp, chg, n)) =
  row [ bs rf, bs ls, f2 qty, f2 ext, f2 dp, f2 chg
      , f2 (qty / nf), f2 (ext / nf), f2 (di / nf), show n ]
  where nf = fromIntegral n

render1 :: [((ByteString, ByteString), Q1Acc)] -> String
render1 = joinLines . map fmt1 . sortOn fst

--------------------------------------------------------------------------------
-- Q2 — minimum-cost supplier per part
--------------------------------------------------------------------------------

-- @euPs@ is mentioned twice and is NOT materialized, so it emits two scans of
-- partsupp. An unmaterialized stream
-- used twice is enumerated twice, and Prela materializes only when asked.
q2 :: Q.Query TPCHS [(Id PartSupp, Q2Row)]
q2 = Q.query build
  where
    build s = do
      minPerPart <- Q.groupFold
        (\a c -> Q.ifThenElse (c Q..<. a) c a) (1 / 0)
        (compose (groupBy euPs (psPart s)) (supplycost s))
      let costIsMin :: Lookup (Id PartSupp) (Double, Double)
          costIsMin = Q.filterBy (Q.onPair (Q..==.))
                         (prod (supplycost s) (compose (psPart s) (Q.keyed minPerPart)))
          result :: Stream (Id PartSupp) Q2Row
          result = compose (restrict euPs (prod partOk costIsMin)) payload
      pure (Q.collect result)
      where
        euPs :: SMode q => q (Id PartSupp) (Id PartSupp)
        euPs = restrict (partsupp s)
                 (Q.eq "EUROPE"
                     (compose (compose (compose (psSupplier s) (supplierNation s))
                                       (nationRegion s))
                              (regionName s)))
        partOk = restrict (psPart s)
                   (prod (Q.eq 15 (size s))
                         (Q.filterBy (Q.isSuffixOf "BRASS") (ty s)))
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

render2 :: [(Id PartSupp, Q2Row)] -> String
render2 = joinLines . map fmt2 . take 100 . sortBy cmp2 . map (flat2 . snd)

--------------------------------------------------------------------------------
-- Q3 — shipping priority
--------------------------------------------------------------------------------

-- SQL groups by (l_orderkey, o_orderdate, o_shippriority). The orderkey fixes
-- the other two, so all three ride in the group key and the output reads them
-- straight off it, with no second lookup.
q3 :: Q.Query TPCHS [(((Id Order, Int), Int), Double)]
q3 = Q.query build
  where
    build s = do
      revenue <- Q.groupFold
        (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 grouped
      pure (Q.collect (Q.stream revenue))
      where
        grouped :: Stream ((Id Order, Int), Int) (Double, Double)
        grouped =
          compose (groupBy (restrict (lineitem s)
                             (prod (prod (Q.gt 19950315 (shipdate s))
                                         (Q.lt 19950315 (compose (liOrder s) (date s))))
                                   (Q.eq "BUILDING"
                                       (compose (compose (liOrder s) (orderCustomer s))
                                                (mktsegment s)))))
                           (prod (prod (liOrder s) (compose (liOrder s) (date s)))
                                 (compose (liOrder s) (shippriority s))))
                  (prod (extendedprice s) (discount s))

cmp3 :: (((Id Order, Int), Int), Double) -> (((Id Order, Int), Int), Double) -> Ordering
cmp3 (((_, d1), _), r1) (((_, d2), _), r2) = compare r2 r1 <> compare d1 d2

fmt3 :: (((Id Order, Int), Int), Double) -> String
fmt3 (((o, d), sp), r) = row [key1 o, f2 r, fmtDate d, show sp]

render3 :: [(((Id Order, Int), Int), Double)] -> String
render3 = joinLines . map fmt3 . take 10 . sortBy cmp3

--------------------------------------------------------------------------------
-- Q4 — order priority checking
--------------------------------------------------------------------------------

q4 :: Q.Query TPCHS [(ByteString, Int)]
q4 = Q.query $ \s -> do
  -- Orders with at least one late line, precomputed as a bitset so the EXISTS is
  -- a bit test per order rather than a re-scan.
  lateOrder <- Q.bitset (orderExtent s)
    (compose (restrict (lineitem s)
               (Q.filterBy (Q.onPair (Q..<.))
                 (prod (commitdate s) (receiptdate s))))
             (liOrder s))
  counts <- Q.groupFold (\a _ -> a + 1) 0
    (groupBy (restrict (restrict (orders s)
                                 (Q.range 19930701 19931001 (date s)))
                       (Q.keyed lateOrder))
             (priority s))
  pure (Q.collect (Q.stream counts))

fmt4 :: (ByteString, Int) -> String
fmt4 (p, n) = row [bs p, show n]

render4 :: [(ByteString, Int)] -> String
render4 = joinLines . map fmt4 . sortOn fst

--------------------------------------------------------------------------------
-- Q5 — local supplier volume
--------------------------------------------------------------------------------

-- The group key carries the work: a lineitem maps to its supplier's nation name,
-- with the ASIA restriction and the customer-nation equality pushed into the key
-- itself. A row whose keyed lookup yields nothing drops out, so the key doubles
-- as the filter and only the date window rides on the receiver.
q5 :: Q.Query TPCHS [(ByteString, Double)]
q5 = Q.query build
  where
    build s = do
      result <- Q.groupFold
        (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 grouped
      pure (Q.collect (Q.stream result))
      where
        grouped :: Stream ByteString (Double, Double)
        grouped =
          compose (groupBy (restrict (lineitem s)
                             (Q.range 19940101 19950101
                                      (compose (liOrder s) (date s))))
                           (compose sameNation (nationName s)))
                  (prod (extendedprice s) (discount s))
        sameNation :: Lookup (Id Lineitem) (Id Nation)
        sameNation = Q.mapValues Q.first
          (Q.filterBy (Q.onPair (Q..==.))
            (prod (restrict (compose (liSupplier s) (supplierNation s))
                            (Q.eq "ASIA" (compose (nationRegion s) (regionName s))))
                  (compose (compose (liOrder s) (orderCustomer s)) (customerNation s))))

fmt5 :: (ByteString, Double) -> String
fmt5 (n, v) = row [bs n, f2 v]

render5 :: [(ByteString, Double)] -> String
render5 = joinLines . map fmt5 . sortBy (\a b -> comparing snd b a)

--------------------------------------------------------------------------------
-- Q6 — forecasting revenue change
--------------------------------------------------------------------------------

q6 :: Q.Query TPCHS Double
q6 = Q.query $ \s ->
  let rows :: Stream (Id Lineitem) (Double, Double)
      rows = compose (restrict (lineitem s)
                       (prod (prod (Q.range 19940101 19950101 (shipdate s))
                                   (Q.between 0.05 0.07 (discount s)))
                             (Q.lt 24.0 (quantity s))))
                     (prod (extendedprice s) (discount s))
      revenue = Q.foldAll
        (\a -> Q.onPair (\e dc -> a + e * dc)) 0 rows
  in pure revenue

render6 :: Double -> String
render6 = f2

--------------------------------------------------------------------------------
-- Q7 — volume shipping between nation pairs
--------------------------------------------------------------------------------

q7 :: Q.Query TPCHS [((Int, (ByteString, ByteString)), Double)]
q7 = Q.query build
  where
    build s = do
      result <- Q.groupFold
        (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 grouped
      pure (Q.collect (Q.stream result))
      where
        grouped :: Stream (Int, (ByteString, ByteString)) (Double, Double)
        grouped =
          compose (groupBy (lineitem s)
                           (prod (Q.mapValues (`Q.div` 10000)
                                   (Q.between 19950101 19961231 (shipdate s)))
                                 (Q.filterBy pair (prod supNation custNation))))
                  (prod (extendedprice s) (discount s))
        supNation  = compose (compose (liSupplier s) (supplierNation s)) (nationName s)
        custNation = compose (compose (compose (liOrder s) (orderCustomer s))
                                      (customerNation s))
                             (nationName s)
        pair = Q.onPair $ \a b ->
          ((a Q..==. "FRANCE") Q..&&. (b Q..==. "GERMANY")) Q..||.
          ((a Q..==. "GERMANY") Q..&&. (b Q..==. "FRANCE"))

cmp7 :: ((Int, (ByteString, ByteString)), Double)
     -> ((Int, (ByteString, ByteString)), Double) -> Ordering
cmp7 ((y1, (s1, c1)), _) ((y2, (s2, c2)), _) =
  compare s1 s2 <> compare c1 c2 <> compare y1 y2

fmt7 :: ((Int, (ByteString, ByteString)), Double) -> String
fmt7 ((y, (n1, n2)), v) = row [bs n1, bs n2, show y, f2 v]

render7 :: [((Int, (ByteString, ByteString)), Double)] -> String
render7 = joinLines . map fmt7 . sortBy cmp7

--------------------------------------------------------------------------------
-- Q8 — market share for BRAZIL
--------------------------------------------------------------------------------

-- Per year, the BRAZIL share of the volume on ECONOMY ANODIZED STEEL parts sold
-- to customers in AMERICA. The group key navigates the order once — restricting
-- it and taking its year in one hop — and rows failing that navigation drop out.
q8 :: Q.Query TPCHS [(Int, Double)]
q8 = Q.query build
  where
    build s = do
      tbl <- Q.groupFold step (Q.pair 0 0) grouped
      let shares = Q.mapValues
            (Q.onPair (/))
            (Q.stream tbl)
      pure (Q.collect shares)
      where
        grouped :: Stream Int ((Double, Double), ByteString)
        grouped =
          compose (groupBy (restrict (lineitem s)
                             (Q.eq "ECONOMY ANODIZED STEEL"
                                   (compose (liPart s) (ty s))))
                           (Q.mapValues (`Q.div` 10000)
                             (compose (restrict (liOrder s)
                                        (Q.eq "AMERICA"
                                            (compose (compose (compose (orderCustomer s)
                                                                       (customerNation s))
                                                              (nationRegion s))
                                                     (regionName s))))
                                      (Q.between 19950101 19961231 (date s)))))
                  (prod (prod (extendedprice s) (discount s))
                        (compose (compose (liSupplier s) (supplierNation s)) (nationName s)))
        step acc = Q.onPair $ \ed nm ->
          Q.onPair (\e dc ->
            Q.onPair (\b t ->
                  Q.letScalar (e * (1 - dc)) $ \vol ->
                Q.pair (b + Q.ifThenElse (nm Q..==. "BRAZIL") vol 0)
                       (t + vol)) acc) ed

fmt8 :: (Int, Double) -> String
fmt8 (y, v) = row [show y, f2 v]

render8 :: [(Int, Double)] -> String
render8 = joinLines . map fmt8 . sortOn fst

--------------------------------------------------------------------------------
-- Q9 — product type profit measure
--------------------------------------------------------------------------------

-- The query the staged shape was designed for. @sc@ is supply cost per (green
-- part, supplier), it is expensive, and @costPerLi@ reads it — twice, once to
-- cull the lineitem scan and once to fetch the value for the arithmetic.
--
-- `Q.materialize` binds @scM@ in the pure generation monad, so it is a reference
-- to one runtime cache no matter how many times it appears. Mentioning
-- @costPerLi@ twice does emit
-- the lookup twice, which is right: that is two searches of one shared index,
-- exactly the required correlated lookup.
q9 :: Q.Query TPCHS [((ByteString, Int), Double)]
q9 = Q.query $ \s -> do
  let sc :: Stream (Id Part, Id Supplier) Double
      sc = compose (groupBy (partsupp s)
                            (prod (restrict (psPart s)
                                    (Q.filterBy (Q.isInfixOf "green")
                                       (partName s)))
                                  (psSupplier s)))
                   (supplycost s)
  scM <- Q.materialize sc
  let costPerLi :: Lookup (Id Lineitem) Double
      costPerLi = compose (prod (liPart s) (liSupplier s)) (Q.keyed scM)

      grouped :: Stream (ByteString, Int) (((Double, Double), Double), Double)
      grouped =
        compose (groupBy (restrict (lineitem s) costPerLi)
                         (prod (compose (compose (liSupplier s) (supplierNation s))
                                        (nationName s))
                               (Q.mapValues (`Q.div` 10000)
                                  (compose (liOrder s) (date s)))))
                (prod (prod (prod costPerLi (extendedprice s)) (discount s)) (quantity s))
      step a = Q.onPair $ \ced q ->
        Q.onPair (\ce dc ->
          Q.onPair (\cost e -> a + e * (1 - dc) - cost * q) ce) ced
  result <- Q.groupFold step 0 grouped
  pure (Q.collect (Q.stream result))

cmp9 :: ((ByteString, Int), Double) -> ((ByteString, Int), Double) -> Ordering
cmp9 ((n1, y1), _) ((n2, y2), _) = compare n1 n2 <> compare y2 y1

fmt9 :: ((ByteString, Int), Double) -> String
fmt9 ((n, y), v) = row [bs n, show y, f2 v]

render9 :: [((ByteString, Int), Double)] -> String
render9 = joinLines . map fmt9 . sortBy cmp9

--------------------------------------------------------------------------------
-- Q10 — returned-item reporting
--------------------------------------------------------------------------------

q10 :: Q.Query TPCHS [(Id Customer, Q10Row)]
q10 = Q.query build
  where
    build s = do
      revenue <- Q.groupFold
        (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 grouped
      pure (Q.collect (prod (Q.stream revenue) custCols))
      where
        grouped :: Stream (Id Customer) (Double, Double)
        grouped =
          compose (groupBy (restrict (lineitem s)
                             (prod (Q.eq "R" (returnflag s))
                                   (Q.range 19931001 19940101
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

render10 :: [(Id Customer, Q10Row)] -> String
render10 = joinLines . map fmt10 . take 20 . sortBy cmp10

--------------------------------------------------------------------------------
-- Q11 — important stock
--------------------------------------------------------------------------------

-- The threshold is a scalar read off the same table the answer is filtered
-- against. `Q.share` emits one strict runtime binding and returns a reusable
-- scalar reference to it, without exposing that generated binding here.
q11 :: Q.Query TPCHS [(Id Part, Double)]
q11 = Q.query $ \s -> do
  let grouped :: Stream (Id Part) (Double, Int)
      grouped =
        compose (groupBy (restrict (partsupp s)
                           (Q.eq "GERMANY"
                                 (compose (compose (psSupplier s) (supplierNation s))
                                          (nationName s))))
                         (psPart s))
                (prod (supplycost s) (availqty s))
  valuePerPart <- Q.groupFold
    (\a -> Q.onPair (\c q -> a + c * Q.fromIntegral q)) 0 grouped
  threshold <- Q.share
    (0.0001 * Q.foldAll (+) 0 (Q.stream valuePerPart))
  pure (Q.collect (Q.gt threshold (Q.stream valuePerPart)))

fmt11 :: (Id Part, Double) -> String
fmt11 (p, v) = row [key1 p, f2 v]

render11 :: [(Id Part, Double)] -> String
render11 = joinLines . map fmt11 . sortBy (\a b -> comparing snd b a)

--------------------------------------------------------------------------------
-- Q12 — shipping modes and order priority
--------------------------------------------------------------------------------

q12 :: Q.Query TPCHS [(ByteString, (Int, Int))]
q12 = Q.query build
  where
    build s = do
      result <- Q.groupFold step (Q.pair 0 0) grouped
      pure (Q.collect (Q.stream result))
      where
        grouped :: Stream ByteString ByteString
        grouped =
          compose (groupBy (restrict (lineitem s)
                             (prod (prod (Q.oneOf ["MAIL", "SHIP"] (shipmode s))
                                         (Q.filterBy (Q.onPair $ \shc r ->
                                            Q.onPair (\sh c ->
                                              (sh Q..<. c) Q..&&. (c Q..<. r)) shc)
                                           (prod (prod (shipdate s) (commitdate s))
                                                 (receiptdate s))))
                                   (Q.range 19940101 19950101 (receiptdate s))))
                           (shipmode s))
                  (compose (liOrder s) (priority s))
        step acc pr = Q.onPair (\h l ->
          Q.ifThenElse
            ((pr Q..==. "1-URGENT") Q..||. (pr Q..==. "2-HIGH"))
            (Q.pair (h + 1) l)
            (Q.pair h (l + 1))) acc

fmt12 :: (ByteString, (Int, Int)) -> String
fmt12 (m, (h, l)) = row [bs m, show h, show l]

render12 :: [(ByteString, (Int, Int))] -> String
render12 = joinLines . map fmt12 . sortOn fst

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
q13 :: Q.Query TPCHS [(Int, Int)]
q13 = Q.query $ \s -> do
  re <- Q.regex "special.*requests"
  countPerCust <- Q.denseFoldOuter
    (customerExtent s)
    (\a _ -> a + 1) 0
    (groupBy (restrict (orders s) (Q.nrx re (orderComment s)))
             (orderCustomer s))
  dist <- Q.groupFold (\a _ -> a + 1) 0
    (invStream (Q.stream countPerCust))
  pure (Q.collect (Q.stream dist))

cmp13 :: (Int, Int) -> (Int, Int) -> Ordering
cmp13 (k1, v1) (k2, v2) = compare v2 v1 <> compare k2 k1

fmt13 :: (Int, Int) -> String
fmt13 (c, n) = row [show c, show n]

render13 :: [(Int, Int)] -> String
render13 = joinLines . map fmt13 . sortBy cmp13

--------------------------------------------------------------------------------
-- Q14 — promo revenue ratio
--------------------------------------------------------------------------------

q14 :: Q.Query TPCHS Double
q14 = Q.query $ \s ->
  let rows :: Stream (Id Lineitem) ((Double, Double), ByteString)
      rows = compose (restrict (lineitem s)
                       (Q.range 19950901 19951001 (shipdate s)))
                     (prod (prod (extendedprice s) (discount s))
                           (compose (liPart s) (ty s)))
      totals = Q.foldAll step (Q.pair 0 0) rows
      step acc = Q.onPair $ \ed typ ->
        Q.onPair (\e dc ->
          Q.onPair (\promo total ->
            Q.letScalar (e * (1 - dc)) $ \price ->
              Q.pair
                (promo + Q.ifThenElse (Q.isPrefixOf "PROMO" typ) price 0)
                (total + price)) acc) ed
  in pure (Q.onPair (\promo total -> 100 * promo / total) totals)

render14 :: Double -> String
render14 = f2

--------------------------------------------------------------------------------
-- Q15 — top supplier
--------------------------------------------------------------------------------

q15 :: Q.Query TPCHS
         [(Id Supplier, (Double, ((ByteString, ByteString), ByteString)))]
q15 = Q.query build
  where
    build s = do
      revenue <- Q.groupFold
        (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 grouped
      maxRev <- Q.share (Q.foldAll
        (\a v -> Q.ifThenElse (v Q..>. a) v a) 0
        (Q.stream revenue))
      let result = prod
            (Q.eq maxRev (Q.stream revenue))
            (prod (prod (supplierName s) (supplierAddress s)) (supplierPhone s))
      pure (Q.collect result)
      where
        grouped :: Stream (Id Supplier) (Double, Double)
        grouped =
          compose (groupBy (restrict (lineitem s)
                             (Q.range 19960101 19960401 (shipdate s)))
                           (liSupplier s))
                  (prod (extendedprice s) (discount s))

fmt15 :: (Id Supplier, (Double, ((ByteString, ByteString), ByteString))) -> String
fmt15 (k, (rev, ((nm, addr), phone))) =
  row [key1 k, bs nm, bs addr, bs phone, f2 rev]

render15
  :: [(Id Supplier, (Double, ((ByteString, ByteString), ByteString)))] -> String
render15 = joinLines . map fmt15 . sortOn fst

--------------------------------------------------------------------------------
-- Q16 — distinct supplier count
--------------------------------------------------------------------------------

q16 :: Q.Query TPCHS [(((ByteString, ByteString), Int), Int)]
q16 = Q.query $ \s -> do
  re <- Q.regex "Customer.*Complaints"
  counts <- Q.distinctCount (grouped s re)
  pure (Q.collect (Q.stream counts))
  where
    grouped s re =
      compose (groupBy (restrict (partsupp s) (prod partOk (suppOk re)))
                       (compose (psPart s) (prod (prod (brand s) (ty s)) (size s))))
              (psSupplier s)
      where
        partOk = restrict (psPart s)
          (prod (prod (Q.ne "Brand#45" (brand s))
                      (Q.filterBy
                        (Q.notS . Q.isPrefixOf "MEDIUM POLISHED") (ty s)))
                (Q.oneOf [49, 14, 23, 45, 19, 3, 36, 9] (size s)))
        suppOk re' = Q.nrx re' (compose (psSupplier s) (supplierComment s))

cmp16 :: (((ByteString, ByteString), Int), Int)
      -> (((ByteString, ByteString), Int), Int) -> Ordering
cmp16 (k1, c1) (k2, c2) = compare c2 c1 <> compare k1 k2

fmt16 :: (((ByteString, ByteString), Int), Int) -> String
fmt16 (((b, t), sz), c) = row [bs b, bs t, show sz, show c]

render16 :: [(((ByteString, ByteString), Int), Int)] -> String
render16 = joinLines . map fmt16 . sortBy cmp16

--------------------------------------------------------------------------------
-- Q17 — small-quantity order revenue
--------------------------------------------------------------------------------

q17 :: Q.Query TPCHS Double
q17 = Q.query $ \s -> do
  -- The correlated 0.2 * avg(quantity) per part, built once so the cross-column
  -- comparison below is a keyed lookup and not a re-fold per row.
  avgTbl <- Q.groupFold
    (\acc q -> Q.onPair (\sm n -> Q.pair (sm + q) (n + Q.int 1)) acc)
    (Q.pair 0 (Q.int 0))
    (compose (groupBy (lineitem s) (liPart s)) (quantity s))
  let tpp :: Lookup (Id Part) Double
      tpp = Q.mapValues
            (Q.onPair (\sm n -> 0.2 * sm / Q.fromIntegral n))
            (Q.keyed avgTbl)
      qtyOk = Q.filterBy (Q.onPair (Q..<.))
                (prod (quantity s) (compose (liPart s) tpp))
      partOk = restrict (liPart s)
                 (prod (Q.eq "Brand#23" (brand s))
                       (Q.eq "MED BOX" (container s)))
      total = Q.foldAll (+) 0
        (compose (restrict (lineitem s) (prod partOk qtyOk)) (extendedprice s))
  pure (total / 7)

render17 :: Double -> String
render17 = f2

--------------------------------------------------------------------------------
-- Q18 — large volume customer
--------------------------------------------------------------------------------

-- Every output column is functionally determined by the order, so they are all
-- attached after the fold rather than dragged through it.
q18 :: Q.Query TPCHS [(Id Order, Q18Row)]
q18 = Q.query $ \s -> do
  sumQ <- Q.groupFold (+) 0
    (compose (groupBy (lineitem s) (liOrder s)) (quantity s))
  let result = prod (Q.gt 300 (Q.stream sumQ))
                    (prod (prod (totalprice s) (date s))
                          (prod (compose (orderCustomer s) (customerName s))
                                (orderCustomer s)))
  pure (Q.collect result)

type Q18Row = (Double, ((Double, Int), (ByteString, Id Customer)))

cmp18 :: (Id Order, Q18Row) -> (Id Order, Q18Row) -> Ordering
cmp18 (_, (_, ((tp1, d1), _))) (_, (_, ((tp2, d2), _))) =
  compare tp2 tp1 <> compare d1 d2

fmt18 :: (Id Order, Q18Row) -> String
fmt18 (o, (sumQ, ((tp, d), (nm, c)))) =
  row [bs nm, key1 c, key1 o, fmtDate d, f2 tp, f2 sumQ]

render18 :: [(Id Order, Q18Row)] -> String
render18 = joinLines . map fmt18 . take 100 . sortBy cmp18

--------------------------------------------------------------------------------
-- Q19 — discounted revenue
--------------------------------------------------------------------------------

q19 :: Q.Query TPCHS Double
q19 = Q.query $ \s ->
  let rows :: Stream (Id Lineitem) (Double, Double)
      rows = compose (restrict (lineitem s) (prod (prod shipOk instructOk) branches))
                     (prod (extendedprice s) (discount s))
      shipOk     = Q.oneOf ["AIR", "AIR REG"] (shipmode s)
      instructOk = Q.eq "DELIVER IN PERSON" (shipinstruct s)
      -- The three-way disjunction cannot be split across conjuncts, so it is one
      -- test over (brand, container, size, quantity).
      branches = Q.filterBy test
        (prod (compose (liPart s) (prod (prod (brand s) (container s)) (size s)))
              (quantity s))
      test = Q.onPair $ \bcs q ->
        Q.onPair (\bc sz ->
          Q.onPair (\br ct ->
            let branch wantedBrand containers lowQ highQ highSize =
                  (br Q..==. wantedBrand) Q..&&. Q.member ct containers Q..&&.
                  (q Q..>=. lowQ) Q..&&. (q Q..<=. highQ) Q..&&.
                  (sz Q..>=. 1) Q..&&. (sz Q..<=. highSize)
            in branch "Brand#12" ["SM CASE", "SM BOX", "SM PACK", "SM PKG"] 1 11 5
               Q..||. branch "Brand#23" ["MED BAG", "MED BOX", "MED PKG", "MED PACK"] 10 20 10
               Q..||. branch "Brand#34" ["LG CASE", "LG BOX", "LG PACK", "LG PKG"] 20 30 15)
            bc) bcs
      revenue = Q.foldAll
        (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 rows
  in pure revenue

render19 :: Double -> String
render19 = f2

--------------------------------------------------------------------------------
-- Q20 — potential part promotion
--------------------------------------------------------------------------------

q20 :: Q.Query TPCHS [(Id Supplier, (ByteString, ByteString))]
q20 = Q.query $ \s -> do
  -- Per (part, supplier), the quantity shipped in 1994.
  sumQty <- Q.groupFold (+) 0
    (compose (groupBy (restrict (lineitem s)
                        (Q.range 19940101 19950101 (shipdate s)))
                      (prod (liPart s) (liSupplier s)))
             (quantity s))
  let threshold = Q.mapValues (0.5 *)
        (compose (prod (psPart s) (psSupplier s))
                 (Q.keyed sumQty))
      candidates = compose
        (restrict (partsupp s)
          (prod (Q.filterBy (Q.isPrefixOf "forest")
                 (compose (psPart s) (partName s)))
                (Q.filterBy (Q.onPair (Q..>.))
                  (prod (Q.mapValues Q.fromIntegral (availqty s))
                        threshold))))
        (psSupplier s)
  -- The qualifying suppliers as a driveable set, so the CANADA filter runs over
  -- those rather than over the whole supplier universe.
  qualSupps <- Q.bitset
    (supplierExtent s) candidates
  let result = compose
        (restrict (Q.stream qualSupps)
          (Q.eq "CANADA" (compose (supplierNation s) (nationName s))))
        (prod (supplierName s) (supplierAddress s))
  pure (Q.collect result)

fmt20 :: (ByteString, ByteString) -> String
fmt20 (nm, addr) = row [bs nm, bs addr]

render20 :: [(Id Supplier, (ByteString, ByteString))] -> String
render20 = joinLines . map fmt20 . sortOn fst . map snd

--------------------------------------------------------------------------------
-- Q21 — suppliers who kept orders waiting
--------------------------------------------------------------------------------

-- Three materializers sequenced in `do` notation. Each @<-@ binds one generated
-- runtime cache, so the reading order is the build order: both distinct counts
-- first, then the fold that uses them.
--
-- @late@ stays polymorphic and is mentioned three times. That emits three loops
-- over lineitem, and it is meant to: Prela does not materialize unless asked, and
-- it is re-enumerated three times for the same reason.
q21 :: Q.Query TPCHS [(Id Supplier, (Int, ByteString))]
q21 = Q.query build
  where
    build s = do
      allSupp <- Q.distinctCount
        (compose (groupBy (lineitem s) (liOrder s)) (liSupplier s))
      lateSupp <- Q.distinctCount
        (compose (groupBy late (liOrder s)) (liSupplier s))
      let -- The order has more than one supplier across all its lines …
          multiSupp :: Lookup (Id Order) Int
          multiSupp = Q.gt 1 (Q.keyed allSupp)
          -- … but exactly one across its late ones.
          onlyLate :: Lookup (Id Order) Int
          onlyLate = Q.eq 1 (Q.keyed lateSupp)
          orderOk :: Lookup (Id Lineitem) (Id Order)
          orderOk = restrict (liOrder s)
                      (prod (prod (Q.eq "F" (orderStatus s)) multiSupp) onlyLate)
      tally <- Q.groupFold (\a _ -> a + 1) 0
        (groupBy (restrict late (prod natOk orderOk)) (liSupplier s))
      let result = prod (Q.stream tally) (supplierName s)
      pure (Q.collect result)
      where
        late :: SMode q => q (Id Lineitem) (Id Lineitem)
        late = restrict (lineitem s)
                 (Q.filterBy (Q.onPair (Q..<.))
                   (prod (commitdate s) (receiptdate s)))
        natOk :: Lookup (Id Lineitem) ByteString
        natOk = Q.eq "SAUDI ARABIA"
                  (compose (compose (liSupplier s) (supplierNation s)) (nationName s))

cmp21 :: (Id Supplier, (Int, ByteString)) -> (Id Supplier, (Int, ByteString)) -> Ordering
cmp21 (_, (c1, n1)) (_, (c2, n2)) = compare c2 c1 <> compare n1 n2

fmt21 :: (Id Supplier, (Int, ByteString)) -> String
fmt21 (_, (c, nm)) = row [bs nm, show c]

render21 :: [(Id Supplier, (Int, ByteString))] -> String
render21 = joinLines . map fmt21 . take 100 . sortBy cmp21

--------------------------------------------------------------------------------
-- Q22 — global sales opportunity
--------------------------------------------------------------------------------

-- Two runtime values are shared across later work: the scalar average and the
-- bitset of customers who have orders. `do` notation expresses both generated
-- scopes without mixing quotation syntax into the query.
q22 :: Q.Query TPCHS [(ByteString, (Int, Double))]
q22 = Q.query build
  where
    build s = do
      hasOrders <- Q.bitset
        (customerExtent s) (orderCustomer s)
      let totals = Q.foldAll
            (\acc v -> Q.onPair (\a n -> Q.pair (a + v) (n + Q.int 1)) acc)
            (Q.pair 0 (Q.int 0))
            (compose (restrict prefixOk (Q.gt 0 (customerAcctbal s)))
                     (customerAcctbal s))
      avg <- Q.share (Q.onPair (\sm n -> sm / Q.fromIntegral n) totals)
      counts <- Q.groupFold
        (\acc ab -> Q.onPair (\n sm -> Q.pair (n + Q.int 1) (sm + ab)) acc)
        (Q.pair (Q.int 0) 0)
        (compose (groupBy
                    (diff (restrict prefixOk (Q.gt avg (customerAcctbal s)))
                          (Q.keyed hasOrders))
                    prefix)
                 (customerAcctbal s))
      pure (Q.collect (Q.stream counts))
      where
        prefix :: Lookup (Id Customer) ByteString
        prefix = Q.mapValues (Q.take 2) (customerPhone s)
        prefixOk :: SMode q => q (Id Customer) (Id Customer)
        prefixOk = restrict (customer s)
                     (Q.oneOf ["13", "31", "23", "29", "30", "18", "17"]
                       (Q.mapValues (Q.take 2) (customerPhone s)))

fmt22 :: (ByteString, (Int, Double)) -> String
fmt22 (c, (n, sm)) = row [bs c, show n, f2 sm]

render22 :: [(ByteString, (Int, Double))] -> String
render22 = joinLines . map fmt22 . sortOn fst
