{-# LANGUAGE OverloadedStrings #-}

-- | The 22 TPC-H queries on the staged pull engine.
--
-- Each definition is a pure, quote-free `Q.Query` builder. `Q.compile` (in
-- "TPCH.Staged") turns it into an ordinary function. Generated scalars use
-- normal numeric operations, predicates accept ordinary literals, and
-- materializer scopes compose with ordinary 'Functor' and 'Monad' operators.
--
-- The generation monad is pure: its sequencing describes the scopes of runtime
-- caches. A relation bound by `Q.groupFold`, `Q.materialize`, or `Q.bitset` is
-- constructed once in generated code and may safely be used more than once.
--
-- Queries return typed rows or scalars. The @renderN@ functions below them are
-- ordinary Haskell applied after compilation; sorting, limiting, and report
-- formatting therefore never cross the staging boundary.
module TPCH.StagedQueries
  ( Q1Acc, Q2Row, Q10Row, Q16Key, Q18Row
  , q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11
  , q12, q13, q14, q15, q16, q17, q18, q19, q20, q21, q22
  , render1, render2, render3, render4, render5, render6, render7, render8
  , render9, render10, render11, render12, render13, render14, render15
  , render16, render17, render18, render19, render20, render21, render22
  ) where

import Data.ByteString.Char8 (ByteString)
import qualified Data.ByteString.Char8 as BS
import Data.List (intercalate, sortBy, sortOn)
import Data.Ord (comparing)
import qualified Data.Vector as BV
import Prela.PullStaged.Query
  ( Relation, compose, diff, disj, groupBy, invStream, prod, restrict )
import Prela.PullStaged.Stream (Lookup, Stream)
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

-- | Join rendered rows without a trailing newline.
joinLines :: [String] -> String
joinLines = intercalate "\n"

-- | Join the fields of one TPC-H result row.
row :: [String] -> String
row = intercalate "|"

-- | Decode the benchmark's ASCII bytes for reporting.
bs :: ByteString -> String
bs = BS.unpack

-- | Render an internal identifier as its one-based TPC-H natural key.
--
-- Natural keys in the TPC-H source data are 1-based; internal ids are 0-based,
-- so the +1 is an output detail and appears nowhere else.
key1 :: Id e -> String
key1 identifier = show (idIndex identifier + 1)

--------------------------------------------------------------------------------
-- Q1 — pricing summary report
--------------------------------------------------------------------------------

-- | Summarize shipped quantity, prices, discounts, taxes, averages, and line
-- counts by return flag and line status for shipments through 2 September 1998.
q1 :: Q.Query TPCHS [(Int, Q1Acc)]
q1 = Q.query $ \s ->
  Q.collect <$> Q.denseFold 288 step (Q.tuple6 0 0 0 0 0 (Q.int 0)) (grouped s)
  where
    grouped s =
      groupBy (restrict (lineitem s) (Q.le 19980902 (shipdate s)))
              (Q.mapValues encodeFlags (prod (returnflag s) (lineStatus s)))
        `compose` (quantity s `prod` extendedprice s `prod` discount s `prod` tax s)
    encodeFlags = Q.onPair $ \returnFlag status ->
      (Q.firstByte returnFlag - 65) * 16 + (Q.firstByte status - 70)
    step acc = Q.onTuple4 $ \q e dc tx ->
      Q.onTuple6 (\qty ext di dp chg n ->
        Q.letScalar (e * (1 - dc)) $ \dpInc ->
        Q.letScalar (dpInc * (1 + tx)) $ \chgInc ->
          Q.tuple6 (qty + q) (ext + e) (di + dc)
                   (dp + dpInc) (chg + chgInc) (n + Q.int 1)) acc

-- | Six-field accumulator used by Q1's dense group fold.
--
-- The generic tuple conveniences hide its left-associated binary-pair storage
-- from the update expression.
type Q1Acc = (((((Double, Double), Double), Double), Double), Int)

-- | Format one Q1 aggregate row.
fmt1 :: (Int, Q1Acc) -> String
fmt1 (flags, (((((qty, ext), di), dp), chg), n)) =
  row [ bs rf, bs ls, f2 qty, f2 ext, f2 dp, f2 chg
      , f2 (qty / nf), f2 (ext / nf), f2 (di / nf), show n ]
  where
    rf = BS.singleton (toEnum (flags `div` 16 + 65))
    ls = BS.singleton (toEnum (flags `mod` 16 + 70))
    nf = fromIntegral n

-- | Sort and render the complete Q1 result.
render1 :: [(Int, Q1Acc)] -> String
render1 = joinLines . map fmt1 . sortOn fst

--------------------------------------------------------------------------------
-- Q2 — minimum-cost supplier per part
--------------------------------------------------------------------------------

-- | Find the European suppliers offering each size-15 brass part at its lowest
-- supply cost, then report the top 100 offers by account balance and name.
-- Hoist both selective dimension predicates into bitsets. The two intentional
-- partsupp scans then perform bounded id probes, and the minimum-cost fold only
-- sees rows for qualifying parts rather than aggregating every European offer.
q2 :: Q.Query TPCHS [(Id PartSupp, Q2Row)]
q2 = Q.query $ \s ->
  Q.bitset (supplierExtent s)
    (restrict (supplier s)
      (Q.eq "EUROPE" $ compose
        (compose (supplierNation s) (nationRegion s)) (regionName s)))
  >>= \europeanSuppliers ->
  Q.bitset (partExtent s)
    (restrict (part s)
      (Q.eq 15 (size s) `prod` Q.filterBy (Q.isSuffixOf "BRASS") (ty s)))
  >>= \qualifyingParts ->
  let eligiblePs = restrict (partsupp s)
        ( compose (psSupplier s) europeanSuppliers
          `prod` compose (psPart s) qualifyingParts )
  in Q.denseFold (partExtent s)
       (\a c -> Q.ifThenElse (c Q..<. a) c a) (1 / 0)
       (groupBy eligiblePs (psPart s) `compose` supplycost s)
  >>= \minPerPart ->
  let costIsMin = Q.filterBy (Q.onPair (Q..==.))
        (supplycost s `prod` compose (psPart s) minPerPart)
      ranked = restrict eligiblePs costIsMin `prod` sortFields s
  in (\best -> Q.collect $
        Q.mapValues Q.first best `compose` payload s)
     <$> Q.topK 100 rank2 ranked
  where
    sortFields s =
      compose (psSupplier s) (supplierAcctbal s)
        `prod` compose (compose (psSupplier s) (supplierNation s)) (nationName s)
        `prod` compose (psSupplier s) (supplierName s)
        `prod` psPart s
    payload s =
      compose (psSupplier s) (suppCols s)
        `prod` psPart s
        `prod` compose (psPart s) (mfgr s)
    suppCols s =
      supplierAcctbal s `prod` supplierName s
        `prod` compose (supplierNation s) (nationName s)
        `prod` supplierAddress s `prod` supplierPhone s `prod` supplierComment s

-- | Nested result payload assembled for one Q2 supplier offer.
type Q2Row = (((((((Double, ByteString), ByteString), ByteString), ByteString)
              , ByteString), Id Part), ByteString)

-- | Compare Q2 candidates by balance, nation, supplier name, and part key.
rank2
  :: Q.Scalar (Id PartSupp)
  -> Q.Scalar (Id PartSupp, (((Double, ByteString), ByteString), Id Part))
  -> Q.Scalar (Id PartSupp)
  -> Q.Scalar (Id PartSupp, (((Double, ByteString), ByteString), Id Part))
  -> Q.Scalar Ordering
rank2 _ left _ right =
  Q.onTuple4 (\leftAccount leftNation leftName leftPart ->
    Q.onTuple4 (\rightAccount rightNation rightName rightPart ->
      Q.compare rightAccount leftAccount `Q.thenCompare`
      Q.compare leftNation rightNation `Q.thenCompare`
      Q.compare leftName rightName `Q.thenCompare`
      Q.compare leftPart rightPart)
      (Q.second right))
    (Q.second left)

-- | Flatten a Q2 payload into reporting order.
--
-- The comparator and the formatter can then read
-- named components instead of unpicking a seven-deep tuple.
flat2 :: Q2Row -> (Double, ByteString, ByteString, Id Part, ByteString
                  , ByteString, ByteString, ByteString)
flat2 (((((((acct, sname), nat), addr), phone), comm), pk), mfg) =
  (acct, sname, nat, pk, mfg, addr, phone, comm)

-- | Format one flattened Q2 result row.
fmt2 :: (Double, ByteString, ByteString, Id Part, ByteString, ByteString, ByteString, ByteString)
     -> String
fmt2 (acct, sname, nat, pk, mfg, addr, phone, comm) =
  row [f2 acct, bs sname, bs nat, key1 pk, bs mfg, bs addr, bs phone, bs comm]

-- | Render the already-ranked Q2 result.
render2 :: [(Id PartSupp, Q2Row)] -> String
render2 = joinLines . map fmt2 . map (flat2 . snd)

--------------------------------------------------------------------------------
-- Q3 — shipping priority
--------------------------------------------------------------------------------

-- | Find the ten highest-revenue unshipped orders placed before 15 March 1995 by
-- customers in the BUILDING segment, using lines shipped after that date.
-- SQL groups by (l_orderkey, o_orderdate, o_shippriority). The orderkey fixes
-- the other two, so all three ride in the group key and the output reads them
-- straight off it, with no second lookup.
q3 :: Q.Query TPCHS [(((Id Order, Int), Int), Double)]
q3 = Q.query $ \s ->
  Q.collect <$> (Q.groupFold
    (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 (grouped s)
    >>= Q.topK 10 rank3)
  where
    grouped s =
      groupBy
        (restrict (lineitem s)
          ( Q.gt 19950315 (shipdate s)
            `prod` Q.lt 19950315 (compose (liOrder s) (date s))
            `prod` Q.eq "BUILDING"
              (compose (compose (liOrder s) (orderCustomer s)) (mktsegment s)) ))
        (liOrder s
          `prod` compose (liOrder s) (date s)
          `prod` compose (liOrder s) (shippriority s))
      `compose` (extendedprice s `prod` discount s)

-- | Compare Q3 groups by descending revenue and ascending order date.
rank3
  :: Q.Scalar ((Id Order, Int), Int) -> Q.Scalar Double
  -> Q.Scalar ((Id Order, Int), Int) -> Q.Scalar Double
  -> Q.Scalar Ordering
rank3 leftKey leftRevenue rightKey rightRevenue =
  Q.compare rightRevenue leftRevenue `Q.thenCompare`
  Q.compare (Q.second (Q.first leftKey)) (Q.second (Q.first rightKey))

-- | Format one Q3 order-revenue row.
fmt3 :: (((Id Order, Int), Int), Double) -> String
fmt3 (((o, d), sp), r) = row [key1 o, f2 r, fmtDate d, show sp]

-- | Render the already-ranked Q3 result.
render3 :: [(((Id Order, Int), Int), Double)] -> String
render3 = joinLines . map fmt3

--------------------------------------------------------------------------------
-- Q4 — order priority checking
--------------------------------------------------------------------------------

-- | Count third-quarter 1993 orders by priority when at least one line was
-- received after its committed delivery date.
q4 :: Q.Query TPCHS [(ByteString, Int)]
q4 = Q.query $ \s ->
  -- Orders with at least one late line, precomputed as a bitset so the EXISTS is
  -- a bit test per order rather than a re-scan.
  Q.bitset (orderExtent s)
    ( restrict (lineitem s)
        (Q.filterBy (Q.onPair (Q..<.)) (commitdate s `prod` receiptdate s))
      `compose` liOrder s )
  >>= \lateOrder -> Q.collect <$> Q.groupFold (\a _ -> a + 1) 0
    (groupBy
      (restrict (restrict (orders s) (Q.range 19930701 19931001 (date s))) lateOrder)
      (priority s))

-- | Format one Q4 priority count.
fmt4 :: (ByteString, Int) -> String
fmt4 (p, n) = row [bs p, show n]

-- | Sort and render the complete Q4 result.
render4 :: [(ByteString, Int)] -> String
render4 = joinLines . map fmt4 . sortOn fst

--------------------------------------------------------------------------------
-- Q5 — local supplier volume
--------------------------------------------------------------------------------

-- | Rank Asian nations by 1994 revenue from orders where the customer and
-- supplier belong to the same nation.
-- The group key carries the work: a lineitem maps to its supplier's nation name,
-- with the ASIA restriction and the customer-nation equality pushed into the key
-- itself. A row whose keyed lookup yields nothing drops out, so the key doubles
-- as the filter and only the date window rides on the receiver.
q5 :: Q.Query TPCHS [(Id Nation, (Double, ByteString))]
q5 = Q.query $ \s ->
  Q.collect . (`prod` nationName s)
    <$> Q.denseFold (nationExtent s)
          (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 (grouped s)
  where
    grouped s =
      groupBy
        (restrict (lineitem s)
          (Q.range 19940101 19950101 (compose (liOrder s) (date s))))
        (sameNation s)
      `compose` (extendedprice s `prod` discount s)
    sameNation s = Q.mapValues Q.first . Q.filterBy (Q.onPair (Q..==.)) $
      restrict (compose (liSupplier s) (supplierNation s))
               (Q.eq "ASIA" (compose (nationRegion s) (regionName s)))
      `prod` compose (compose (liOrder s) (orderCustomer s)) (customerNation s)

-- | Format one Q5 nation-revenue row.
fmt5 :: (Id Nation, (Double, ByteString)) -> String
fmt5 (_, (v, n)) = row [bs n, f2 v]

-- | Rank and render the Q5 result by descending revenue.
render5 :: [(Id Nation, (Double, ByteString))] -> String
render5 = joinLines . map fmt5 . sortBy (\a b -> comparing (fst . snd) b a)

--------------------------------------------------------------------------------
-- Q6 — forecasting revenue change
--------------------------------------------------------------------------------

-- | Estimate the revenue from 1994 lines discounted by 5–7 percent whose ordered
-- quantity was below 24 units.
q6 :: Q.Query TPCHS Double
q6 = Q.query $ \s -> pure $ Q.foldAll
  (\a -> Q.onPair (\e dc -> a + e * dc)) 0 $
  restrict (lineitem s)
    ( Q.range 19940101 19950101 (shipdate s)
      `prod` Q.between 0.05 0.07 (discount s)
      `prod` Q.lt 24.0 (quantity s) )
  `compose` (extendedprice s `prod` discount s)

-- | Render the scalar Q6 revenue.
render6 :: Double -> String
render6 = f2

--------------------------------------------------------------------------------
-- Q7 — volume shipping between nation pairs
--------------------------------------------------------------------------------

-- | Report discounted shipping revenue by year and direction for trade between
-- France and Germany during 1995 and 1996.
q7 :: Q.Query TPCHS [((Int, (ByteString, ByteString)), Double)]
q7 = Q.query $ \s ->
  nationMembers (supplierExtent s) (supplier s) (supplierNation s) (nationName s) "FRANCE"
  >>= \frenchSuppliers ->
  nationMembers (supplierExtent s) (supplier s) (supplierNation s) (nationName s) "GERMANY"
  >>= \germanSuppliers ->
  nationMembers (customerExtent s) (customer s) (customerNation s) (nationName s) "FRANCE"
  >>= \frenchCustomers ->
  nationMembers (customerExtent s) (customer s) (customerNation s) (nationName s) "GERMANY"
  >>= \germanCustomers ->
  let supplierIn members = compose (liSupplier s) members
      customerIn members = compose (compose (liOrder s) (orderCustomer s)) members
      directionOk :: Lookup (Id Lineitem) ()
      directionOk = disj
        (supplierIn frenchSuppliers `prod` customerIn germanCustomers)
        (supplierIn germanSuppliers `prod` customerIn frenchCustomers)
      live = restrict
        (restrict (lineitem s) (Q.between 19950101 19961231 (shipdate s)))
        directionOk
      supNation = compose (compose (liSupplier s) (supplierNation s)) (nationName s)
      custNation = compose
        (compose (compose (liOrder s) (orderCustomer s)) (customerNation s))
        (nationName s)
      grouped =
        groupBy live
          ( Q.mapValues (`Q.div` 10000) (shipdate s)
            `prod` (supNation `prod` custNation) )
        `compose` (extendedprice s `prod` discount s)
  in Q.collect <$> Q.groupFold
       (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 grouped
  where
    nationMembers extent entities entityNation names wanted =
      Q.bitset extent $ restrict entities
        (Q.eq wanted (compose entityNation names))

-- | Order Q7 rows by supplier nation, customer nation, then year.
cmp7 :: ((Int, (ByteString, ByteString)), Double)
     -> ((Int, (ByteString, ByteString)), Double) -> Ordering
cmp7 ((y1, (s1, c1)), _) ((y2, (s2, c2)), _) =
  compare s1 s2 <> compare c1 c2 <> compare y1 y2

-- | Format one Q7 directional revenue row.
fmt7 :: ((Int, (ByteString, ByteString)), Double) -> String
fmt7 ((y, (n1, n2)), v) = row [bs n1, bs n2, show y, f2 v]

-- | Sort and render the complete Q7 result.
render7 :: [((Int, (ByteString, ByteString)), Double)] -> String
render7 = joinLines . map fmt7 . sortBy cmp7

--------------------------------------------------------------------------------
-- Q8 — market share for BRAZIL
--------------------------------------------------------------------------------

-- | Compute Brazil's yearly share of revenue for economy anodized steel parts
-- bought by customers in the America region during 1995 and 1996.
-- Per year, the BRAZIL share of the volume on ECONOMY ANODIZED STEEL parts sold
-- to customers in AMERICA. The group key navigates the order once — restricting
-- it and taking its year in one hop — and rows failing that navigation drop out.
q8 :: Q.Query TPCHS [(Int, Double)]
q8 = Q.query $ \s ->
  Q.collect . Q.mapValues (Q.onPair (/))
    <$> Q.groupFold step (Q.pair 0 0) (grouped s)
  where
    grouped s =
      groupBy
        (restrict (lineitem s)
          (Q.eq "ECONOMY ANODIZED STEEL" (compose (liPart s) (ty s))))
        (Q.mapValues (`Q.div` 10000) $ compose
          (restrict (liOrder s)
            (Q.eq "AMERICA" $ compose
              (compose (orderCustomer s) (customerNation s) `compose` nationRegion s)
              (regionName s)))
          (Q.between 19950101 19961231 (date s)))
      `compose`
        ( extendedprice s `prod` discount s
          `prod` compose (compose (liSupplier s) (supplierNation s)) (nationName s) )
    step acc = Q.onTuple3 $ \e dc nm ->
      Q.onPair (\b t -> Q.letScalar (e * (1 - dc)) $ \vol ->
        Q.pair (b + Q.ifThenElse (nm Q..==. "BRAZIL") vol 0) (t + vol)) acc

-- | Format one Q8 year and market-share row.
fmt8 :: (Int, Double) -> String
fmt8 (y, v) = row [show y, f2 v]

-- | Sort and render the complete Q8 result.
render8 :: [(Int, Double)] -> String
render8 = joinLines . map fmt8 . sortOn fst

--------------------------------------------------------------------------------
-- Q9 — product type profit measure
--------------------------------------------------------------------------------

-- | Calculate yearly profit by supplier nation for parts whose names contain
-- "green", subtracting supply cost from discounted sales revenue.
-- Hoist the selective @name contains green@ predicate to the part domain. The
-- six-million-row lineitem scan then pays one bit test before any compound-key
-- lookup. Supply cost uses the staged hash fold rather than the general-purpose
-- ordered @Map@ materializer, and is probed only once for each surviving row.
q9 :: Q.Query TPCHS [((Id Nation, Int), (Double, ByteString))]
q9 = Q.query $ \s -> Q.regex "green" >>= \green -> Q.bitset
    (partExtent s)
    (restrict (part s) (Q.rx green (partName s)))
  >>= \greenParts ->
  let sc :: Stream (Id Part, Id Supplier) Double
      sc = compose (groupBy (restrict (partsupp s)
                               (compose (psPart s) greenParts))
                            (prod (psPart s) (psSupplier s)))
                   (supplycost s)
  in Q.groupFold (\_ cost -> cost) 0 sc >>= \scM ->
  let costPerLi :: Relation (Id Lineitem) Double
      costPerLi = compose (prod (liPart s) (liSupplier s)) scM
      greenLine :: Relation (Id Lineitem) (Id Part)
      greenLine = compose (liPart s) greenParts

      grouped :: Stream (Id Nation, Int) (((Double, Double), Double), Double)
      grouped =
        compose (groupBy (restrict (lineitem s) greenLine)
                         (prod (compose (liSupplier s) (supplierNation s))
                               (Q.mapValues (`Q.div` 10000)
                                  (compose (liOrder s) (date s)))))
                (prod (prod (prod costPerLi (extendedprice s)) (discount s)) (quantity s))
      step a = Q.onTuple4 $ \cost e dc q ->
        a + e * (1 - dc) - cost * q
      groupNationName = Q.mapKeys Q.first (nationName s)
  in Q.collect . (`prod` groupNationName) <$> Q.groupFold step 0 grouped

-- | Order Q9 rows by nation and descending year.
cmp9 :: ((Id Nation, Int), (Double, ByteString))
     -> ((Id Nation, Int), (Double, ByteString)) -> Ordering
cmp9 ((_, y1), (_, n1)) ((_, y2), (_, n2)) = compare n1 n2 <> compare y2 y1

-- | Format one Q9 nation-year profit row.
fmt9 :: ((Id Nation, Int), (Double, ByteString)) -> String
fmt9 ((_, y), (v, n)) = row [bs n, show y, f2 v]

-- | Sort and render the complete Q9 result.
render9 :: [((Id Nation, Int), (Double, ByteString))] -> String
render9 = joinLines . map fmt9 . sortBy cmp9

--------------------------------------------------------------------------------
-- Q10 — returned-item reporting
--------------------------------------------------------------------------------

-- | Find the 20 customers responsible for the most revenue from returned items
-- on orders placed in the final quarter of 1993, including contact details.
q10 :: Q.Query TPCHS [(Id Customer, Q10Row)]
q10 = Q.query $ \s ->
  Q.collect . (`prod` custCols s)
    <$> (Q.denseFold (customerExtent s)
          (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 (grouped s)
         >>= Q.topK 20 (\_ leftRevenue _ rightRevenue ->
                          Q.compare rightRevenue leftRevenue))
  where
    grouped s =
      groupBy
        (restrict (lineitem s)
          ( Q.eq "R" (returnflag s)
            `prod` Q.range 19931001 19940101 (compose (liOrder s) (date s)) ))
        (compose (liOrder s) (orderCustomer s))
      `compose` (extendedprice s `prod` discount s)
    custCols s =
      customerName s `prod` customerAcctbal s
        `prod` compose (customerNation s) (nationName s)
        `prod` customerAddress s `prod` customerPhone s `prod` customerComment s

-- | Revenue and contact payload reported for one Q10 customer.
type Q10Row = (Double, (((((ByteString, Double), ByteString), ByteString), ByteString)
              , ByteString))

-- | Format one Q10 customer-revenue row.
fmt10 :: (Id Customer, Q10Row) -> String
fmt10 (c, (r, (((((nm, ab), nat), addr), phone), comm)))=
  row [key1 c, bs nm, f2 r, f2 ab, bs nat, bs addr, bs phone, bs comm]

-- | Render the already-ranked Q10 result.
render10 :: [(Id Customer, Q10Row)] -> String
render10 = joinLines . map fmt10

--------------------------------------------------------------------------------
-- Q11 — important stock
--------------------------------------------------------------------------------

-- | Find parts stocked by German suppliers whose total on-hand value exceeds
-- 0.01 percent of the value of all German-held stock.
-- The threshold is a scalar read off the same table the answer is filtered
-- against. `Q.share` emits one strict runtime binding and returns a reusable
-- scalar reference to it, without exposing that generated binding here.
q11 :: Q.Query TPCHS [(Id Part, Double)]
q11 = Q.query $ \s ->
  let grouped :: Stream (Id Part) (Double, Int)
      grouped =
        compose (groupBy (restrict (partsupp s)
                           (Q.eq "GERMANY"
                                 (compose (compose (psSupplier s) (supplierNation s))
                                          (nationName s))))
                         (psPart s))
                (prod (supplycost s) (availqty s))
  in Q.groupFold
      (\a -> Q.onPair (\c q -> a + c * Q.fromIntegral q)) 0 grouped
     >>= \valuePerPart ->
       (\threshold -> Q.collect (Q.gt threshold valuePerPart))
         <$> Q.share (0.0001 * Q.foldAll (+) 0 valuePerPart)

-- | Format one Q11 part-value row.
fmt11 :: (Id Part, Double) -> String
fmt11 (p, v) = row [key1 p, f2 v]

-- | Rank and render the Q11 result by descending value.
render11 :: [(Id Part, Double)] -> String
render11 = joinLines . map fmt11 . sortBy (\a b -> comparing snd b a)

--------------------------------------------------------------------------------
-- Q12 — shipping modes and order priority
--------------------------------------------------------------------------------

-- | For MAIL and SHIP deliveries received in 1994, count high-priority versus
-- lower-priority orders whose ship, commit, and receipt dates are in order.
q12 :: Q.Query TPCHS [(ByteString, (Int, Int))]
q12 = Q.query $ \s ->
  Q.collect <$> Q.groupFold step (Q.pair 0 0) (grouped s)
  where
    grouped s =
      groupBy
        (restrict (lineitem s)
          ( Q.range 19940101 19950101 (receiptdate s)
            `prod` Q.oneOf ["MAIL", "SHIP"] (shipmode s)
            `prod` Q.filterBy (Q.onTuple3 $ \sh c r ->
                     (sh Q..<. c) Q..&&. (c Q..<. r))
                   (shipdate s `prod` commitdate s `prod` receiptdate s) ))
        (shipmode s)
      `compose` compose (liOrder s) (priority s)
    step acc pr = Q.onPair (\h l ->
      Q.ifThenElse
        ((pr Q..==. "1-URGENT") Q..||. (pr Q..==. "2-HIGH"))
        (Q.pair (h + 1) l)
        (Q.pair h (l + 1))) acc

-- | Format one Q12 ship-mode priority count.
fmt12 :: (ByteString, (Int, Int)) -> String
fmt12 (m, (h, l)) = row [bs m, show h, show l]

-- | Sort and render the complete Q12 result.
render12 :: [(ByteString, (Int, Int))] -> String
render12 = joinLines . map fmt12 . sortOn fst

--------------------------------------------------------------------------------
-- Q13 — customer distribution
--------------------------------------------------------------------------------

-- | Show how many customers placed each possible number of orders, excluding
-- orders whose comments contain "special" followed later by "requests".
-- SQL's LEFT JOIN: customers with no qualifying order still count, at zero. That
-- is what @denseFoldOuter@ is for — it emits every key in the customer id space,
-- seeded with the initial value. `orders` being a sparse universe means driving
-- it already skips the orderkey gaps, so the group key is the bare foreign key
-- with no validity guard of its own.
--
-- The second fold uses `invStream`, which flips the pairs without building an
-- index. Nothing here needs keyed access, so the enumeration-only form is the
-- honest choice.
q13 :: Q.Query TPCHS [(Int, Int)]
q13 = Q.query $ \s -> Q.regex "special.*requests" >>= \excluded -> Q.collect <$>
  (Q.denseFoldOuter
    (customerExtent s)
    (\a _ -> a + 1) 0
    (groupBy (restrict (orders s)
               (Q.nrx excluded (orderComment s)))
             (orderCustomer s))
   >>= Q.groupFold (\a _ -> a + 1) 0 . invStream)

-- | Order Q13 buckets by descending customer count then descending order count.
cmp13 :: (Int, Int) -> (Int, Int) -> Ordering
cmp13 (k1, v1) (k2, v2) = compare v2 v1 <> compare k2 k1

-- | Format one Q13 order-count distribution bucket.
fmt13 :: (Int, Int) -> String
fmt13 (c, n) = row [show c, show n]

-- | Sort and render the complete Q13 result.
render13 :: [(Int, Int)] -> String
render13 = joinLines . map fmt13 . sortBy cmp13

--------------------------------------------------------------------------------
-- Q14 — promo revenue ratio
--------------------------------------------------------------------------------

-- | Calculate what percentage of September 1995 discounted revenue came from
-- promotional parts.
q14 :: Q.Query TPCHS Double
q14 = Q.query $ \s -> pure . Q.onPair (\promo total -> 100 * promo / total) $
  Q.foldAll step (Q.pair 0 0) $
    restrict (lineitem s) (Q.range 19950901 19951001 (shipdate s))
      `compose`
        ( extendedprice s `prod` discount s
          `prod` compose (liPart s) (ty s) )
  where
    step acc = Q.onTuple3 $ \e dc typ -> Q.onPair (\promo total ->
      Q.letScalar (e * (1 - dc)) $ \price ->
        Q.pair (promo + Q.ifThenElse (Q.isPrefixOf "PROMO" typ) price 0)
               (total + price)) acc

-- | Render the scalar Q14 percentage.
render14 :: Double -> String
render14 = f2

--------------------------------------------------------------------------------
-- Q15 — top supplier
--------------------------------------------------------------------------------

-- | Find every supplier tied for the greatest discounted revenue during the
-- first quarter of 1996, and report their contact details.
q15 :: Q.Query TPCHS
         [(Id Supplier, (Double, ((ByteString, ByteString), ByteString)))]
q15 = Q.query $ \s ->
  Q.denseFold (supplierExtent s)
    (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 (grouped s)
  >>= \revenue ->
    (\maxRev -> Q.collect $
      Q.eq maxRev revenue
        `prod` (supplierName s `prod` supplierAddress s `prod` supplierPhone s))
    <$> Q.share (Q.foldAll (\a v -> Q.ifThenElse (v Q..>. a) v a) 0 revenue)
  where
    grouped s =
      groupBy (restrict (lineitem s) (Q.range 19960101 19960401 (shipdate s)))
              (liSupplier s)
      `compose` (extendedprice s `prod` discount s)

-- | Format one Q15 supplier-revenue row.
fmt15 :: (Id Supplier, (Double, ((ByteString, ByteString), ByteString))) -> String
fmt15 (k, (rev, ((nm, addr), phone))) =
  row [key1 k, bs nm, bs addr, bs phone, f2 rev]

-- | Sort and render the complete Q15 result.
render15
  :: [(Id Supplier, (Double, ((ByteString, ByteString), ByteString)))] -> String
render15 = joinLines . map fmt15 . sortOn fst

--------------------------------------------------------------------------------
-- Q16 — distinct supplier count
--------------------------------------------------------------------------------

-- | Count distinct acceptable suppliers for each qualifying brand, type, and
-- size combination, excluding Brand#45, medium-polished types, and suppliers
-- whose comments identify customer complaints.
-- | Dictionary-coded brand, type, and size grouping key for Q16.
type Q16Key = ((ByteString, ByteString), Int)

-- | Compute distinct acceptable supplier counts for each qualifying part key.
q16 :: Q.Query TPCHS (BV.Vector Q16Key, [(Int, Int)])
q16 = Q.query $ \s -> Q.regex "Customer.*Complaints" >>= \complaint ->
  -- Part attributes repeat across four partsupp rows and supplier comments
  -- repeat across many more. Assign the qualifying attribute triples compact
  -- integer codes once, then keep strings out of the 800K-row partsupp pass.
  Q.dictionary
    (partExtent s)
    (compose
      (restrict (part s)
        (prod (prod (Q.ne "Brand#45" (brand s))
                    (Q.filterBy
                      (Q.notS . Q.isPrefixOf "MEDIUM POLISHED") (ty s)))
              (Q.oneOf [49, 14, 23, 45, 19, 3, 36, 9] (size s))))
      (prod (prod (brand s) (ty s)) (size s)))
  >>= \(partCodes, labels) -> Q.bitset
    (supplierExtent s)
    (restrict (supplier s)
      (Q.nrx complaint (supplierComment s)))
  >>= \acceptableSuppliers ->
  let partCode :: Relation (Id PartSupp) Int
      partCode = compose (psPart s) partCodes
      supplierOk :: Relation (Id PartSupp) (Id Supplier)
      supplierOk = compose (psSupplier s) acceptableSuppliers
      grouped :: Stream Int (Id Supplier)
      grouped =
        compose (groupBy (restrict (partsupp s) (prod partCode supplierOk))
                         partCode)
                (psSupplier s)
  in (Q.pair labels . Q.collect)
       <$> Q.denseDistinctCount (partExtent s) (supplierExtent s) grouped

-- | Order Q16 rows by descending count and then attribute tuple.
cmp16 :: (((ByteString, ByteString), Int), Int)
      -> (((ByteString, ByteString), Int), Int) -> Ordering
cmp16 (k1, c1) (k2, c2) = compare c2 c1 <> compare k1 k2

-- | Format one decoded Q16 grouping row.
fmt16 :: (((ByteString, ByteString), Int), Int) -> String
fmt16 (((b, t), sz), c) = row [bs b, bs t, show sz, show c]

-- | Decode dictionary keys, sort them, and render the Q16 result.
render16 :: (BV.Vector Q16Key, [(Int, Int)]) -> String
render16 (labels, counts) =
  joinLines . map fmt16 . sortBy cmp16 $ map decode counts
  where
    decode (code, supplierCount) =
      case labels BV.!? code of
        Just key -> (key, supplierCount)
        Nothing  -> error "Q16 dictionary code out of bounds"

--------------------------------------------------------------------------------
-- Q17 — small-quantity order revenue
--------------------------------------------------------------------------------

-- | Estimate average yearly revenue from Brand#23 medium-box parts sold in
-- quantities below 20 percent of that part's average ordered quantity.
q17 :: Q.Query TPCHS Double
q17 = Q.query $ \s ->
  -- Only a few hundred parts satisfy the brand/container predicate. Build that
  -- set first and semijoin both lineitem passes against it, rather than folding
  -- quantities for every part and discarding almost all groups afterwards.
  Q.bitset
    (partExtent s)
    (restrict (part s)
      (prod (Q.eq "Brand#23" (brand s))
            (Q.eq "MED BOX" (container s))))
  >>= \qualifyingParts ->
  let qualifyingLine :: Relation (Id Lineitem) (Id Part)
      qualifyingLine = compose (liPart s) qualifyingParts
  in Q.groupFold
    (\acc q -> Q.onPair (\sm n -> Q.pair (sm + q) (n + Q.int 1)) acc)
    (Q.pair 0 (Q.int 0))
    (compose (groupBy (restrict (lineitem s) qualifyingLine) (liPart s))
             (quantity s))
  >>= \avgTbl ->
  let tpp :: Relation (Id Part) Double
      tpp = Q.mapValues
            (Q.onPair (\sm n -> 0.2 * sm / Q.fromIntegral n))
            avgTbl
      qtyOk = Q.filterBy (Q.onPair (Q..<.))
                (prod (quantity s) (compose (liPart s) tpp))
      total = Q.foldAll (+) 0
        (compose (restrict (lineitem s) (prod qualifyingLine qtyOk))
                 (extendedprice s))
  in pure (total / 7)

-- | Render the scalar Q17 revenue.
render17 :: Double -> String
render17 = f2

--------------------------------------------------------------------------------
-- Q18 — large volume customer
--------------------------------------------------------------------------------

-- | Find the top 100 orders whose lines total more than 300 units, reporting the
-- customer, order date, total price, and summed quantity.
-- Every output column is functionally determined by the order, so they are all
-- attached after the fold rather than dragged through it.
q18 :: Q.Query TPCHS [(Id Order, Q18Row)]
q18 = Q.query $ \s ->
  -- Order ids provide the grouping slot directly. Avoid hashing 1.5M order
  -- groups and accumulating a large open-addressed table.
  Q.denseFold (orderExtent s) (+) 0
    (compose (groupBy (lineitem s) (liOrder s)) (quantity s))
  >>= \sumQ ->
  let ranked = prod (Q.gt 300 sumQ)
                    (prod (totalprice s) (date s))
      rank18 _ left _ right =
        let leftSort = Q.second left
            rightSort = Q.second right
        in Q.compare (Q.first rightSort) (Q.first leftSort) `Q.thenCompare`
           Q.compare (Q.second leftSort) (Q.second rightSort)
  in Q.topK 100 rank18 ranked >>= \best ->
    pure . Q.collect $
      Q.mapValues Q.first best
        `prod` ( totalprice s `prod` date s
                 `prod` ( compose (orderCustomer s) (customerName s)
                          `prod` orderCustomer s ) )

-- | Quantity, order attributes, and customer payload reported by Q18.
type Q18Row = (Double, ((Double, Int), (ByteString, Id Customer)))

-- | Format one Q18 large-order row.
fmt18 :: (Id Order, Q18Row) -> String
fmt18 (o, (sumQ, ((tp, d), (nm, c)))) =
  row [bs nm, key1 c, key1 o, fmtDate d, f2 tp, f2 sumQ]

-- | Render the already-ranked Q18 result.
render18 :: [(Id Order, Q18Row)] -> String
render18 = joinLines . map fmt18

--------------------------------------------------------------------------------
-- Q19 — discounted revenue
--------------------------------------------------------------------------------

-- | Sum discounted revenue for three specified brand, container, size, and
-- quantity bands shipped by air with in-person delivery instructions.
q19 :: Q.Query TPCHS Double
q19 = Q.query $ \s -> pure $ Q.foldAll
  (\a -> Q.onPair (\e dc -> a + e * (1 - dc))) 0 $
  restrict (lineitem s)
    ( Q.oneOf ["AIR", "AIR REG"] (shipmode s)
      `prod` Q.eq "DELIVER IN PERSON" (shipinstruct s)
      `prod` Q.filterBy test
        ( compose (liPart s) (brand s `prod` container s `prod` size s)
          `prod` quantity s ) )
  `compose` (extendedprice s `prod` discount s)
  where
    -- The three-way disjunction cannot be split across conjuncts, so it is one
    -- test over (brand, container, size, quantity).
    test = Q.onTuple4 $ \br ct sz q ->
      let branch wantedBrand containers lowQ highQ highSize =
            (br Q..==. wantedBrand) Q..&&. Q.member ct containers Q..&&.
            (q Q..>=. lowQ) Q..&&. (q Q..<=. highQ) Q..&&.
            (sz Q..>=. 1) Q..&&. (sz Q..<=. highSize)
      in branch "Brand#12" ["SM CASE", "SM BOX", "SM PACK", "SM PKG"] 1 11 5
         Q..||. branch "Brand#23" ["MED BAG", "MED BOX", "MED PKG", "MED PACK"] 10 20 10
         Q..||. branch "Brand#34" ["LG CASE", "LG BOX", "LG PACK", "LG PKG"] 20 30 15

-- | Render the scalar Q19 revenue.
render19 :: Double -> String
render19 = f2

--------------------------------------------------------------------------------
-- Q20 — potential part promotion
--------------------------------------------------------------------------------

-- | Find Canadian suppliers with a forest-named part whose available stock is
-- greater than half the quantity of that part shipped in 1994.
q20 :: Q.Query TPCHS [(Id Supplier, (ByteString, ByteString))]
q20 = Q.query $ \s ->
  -- Forest-named parts are a tiny dimension subset. Hoist them before the
  -- six-million-row shipment fold, so the compound-key aggregate never creates
  -- entries for parts that the final partsupp predicate will reject.
  Q.bitset
    (partExtent s)
    (restrict (part s) (Q.filterBy (Q.isPrefixOf "forest") (partName s)))
  >>= \forestParts ->
  -- Per (part, supplier), the quantity shipped in 1994.
  Q.groupFold (+) 0
    (compose (groupBy (restrict (lineitem s)
                        (prod (Q.range 19940101 19950101 (shipdate s))
                              (compose (liPart s) forestParts)))
                      (prod (liPart s) (liSupplier s)))
             (quantity s))
  >>= \sumQty ->
  let threshold = Q.mapValues (0.5 *) $ compose
        (prod (psPart s) (psSupplier s))
        sumQty
      candidates = compose
        (restrict (partsupp s)
          (prod (compose (psPart s) forestParts)
                (Q.filterBy (Q.onPair (Q..>.))
                  (prod (Q.mapValues Q.fromIntegral (availqty s))
                        threshold))))
        (psSupplier s)
  -- The qualifying suppliers as a driveable set, so the CANADA filter runs over
  -- those rather than over the whole supplier universe.
  in Q.bitset
    (supplierExtent s) candidates
  >>= \qualSupps -> pure . Q.collect $
    restrict qualSupps
      (Q.eq "CANADA" (compose (supplierNation s) (nationName s)))
    `compose` (supplierName s `prod` supplierAddress s)

-- | Format one Q20 supplier contact row.
fmt20 :: (ByteString, ByteString) -> String
fmt20 (nm, addr) = row [bs nm, bs addr]

-- | Sort and render the complete Q20 result.
render20 :: [(Id Supplier, (ByteString, ByteString))] -> String
render20 = joinLines . map fmt20 . sortOn fst . map snd

--------------------------------------------------------------------------------
-- Q21 — suppliers who kept orders waiting
--------------------------------------------------------------------------------

-- | First supplier seen and whether a different supplier was later observed.
type SupplierSeen = (Int, Bool)

-- | Overall-supplier and late-supplier cardinality states for one order.
type OrderSupplierState = (SupplierSeen, SupplierSeen)

-- | Rank Saudi Arabian suppliers by late lines on completed orders that involved
-- another supplier but had no other supplier responsible for a late line.
-- A distinct count is more information than either SQL existence test needs.
-- For both all lines and late lines, remember the first supplier and whether a
-- different supplier was subsequently observed. Because orders are an
-- id-indexed domain, one lineitem pass performs both classifications without
-- global @(order, supplier)@ hash sets.
q21 :: Q.Query TPCHS [(Id Supplier, (Int, ByteString))]
q21 = Q.query $ \s ->
  Q.denseFold (orderExtent s) updateState emptyOrderSupplierState (observations s)
  >>= \supplierState ->
  Q.bitset (supplierExtent s)
    (restrict (supplier s)
      (Q.eq "SAUDI ARABIA" (compose (supplierNation s) (nationName s))))
  >>= \saudiSuppliers ->
  let -- The order has more than one supplier across all its lines …
      multiSupp :: Relation (Id Order) OrderSupplierState
      multiSupp = Q.filterBy (Q.second . Q.first) supplierState
      -- … but exactly one across its late ones.
      onlyLate :: Relation (Id Order) OrderSupplierState
      onlyLate = Q.filterBy
        (\state -> Q.onPair (\firstLate multipleLate ->
          (firstLate Q..>=. 0) Q..&&. Q.notS multipleLate) (Q.second state))
        supplierState
      orderOk = restrict (liOrder s)
        (Q.eq "F" (orderStatus s) `prod` multiSupp `prod` onlyLate)
      saudiLine = compose (liSupplier s) saudiSuppliers
  in Q.denseFold (supplierExtent s) (\a _ -> a + 1) 0
       (groupBy (restrict (late s) (saudiLine `prod` orderOk)) (liSupplier s))
  >>= \tally -> Q.collect <$> Q.topK 100 rank21 (tally `prod` supplierName s)
  where
    observations s =
      groupBy (lineitem s) (liOrder s)
        `compose` (liSupplier s `prod` (commitdate s `prod` receiptdate s))

    late s = restrict (lineitem s) $
      Q.filterBy (Q.onPair (Q..<.)) (commitdate s `prod` receiptdate s)

    rank21 _ left _ right =
      Q.onPair (\leftCount leftName ->
        Q.onPair (\rightCount rightName ->
          Q.compare rightCount leftCount `Q.thenCompare`
          Q.compare leftName rightName) right) left

    -- No supplier has yet been observed in either classification.
    -- The first component is -1 until a supplier is observed. This is the
    -- unboxed equivalent of Rust's @(Option<Id<Supplier>>, bool)@ state.
    emptyOrderSupplierState :: Q.Scalar OrderSupplierState
    emptyOrderSupplierState =
      Q.pair (Q.pair (-1) (Q.lit False)) (Q.pair (-1) (Q.lit False))

    -- Incorporate one supplier into a zero/one/multiple cardinality state.
    observe
      :: Q.Scalar SupplierSeen
      -> Q.Scalar (Id Supplier)
      -> Q.Scalar SupplierSeen
    observe seen supplierId = Q.onPair update seen
      where
        update firstSupplier multiple =
          Q.letScalar (Q.idIndex supplierId) $ \candidate ->
            Q.ifThenElse
              (firstSupplier Q..<. 0)
              (Q.pair candidate (Q.lit False))
              (Q.pair firstSupplier
                (multiple Q..||. (firstSupplier Q../=. candidate)))

    -- Update the structured all-lines and late-lines states for one row.
    updateState
      :: Q.Scalar OrderSupplierState
      -> Q.Scalar (Id Supplier, (Int, Int))
      -> Q.Scalar OrderSupplierState
    updateState state observation = Q.onPair updateOrder state
      where
        updateOrder allSeen lateSeen =
          Q.onPair (\supplierId dates ->
            Q.onPair (\committed received ->
              Q.pair
                (observe allSeen supplierId)
                (Q.ifThenElse
                  (committed Q..<. received)
                  (observe lateSeen supplierId)
                  lateSeen)) dates) observation

-- | Format one Q21 supplier waiting-count row.
fmt21 :: (Id Supplier, (Int, ByteString)) -> String
fmt21 (_, (c, nm)) = row [bs nm, show c]

-- | Render the already-ranked Q21 result.
render21 :: [(Id Supplier, (Int, ByteString))] -> String
render21 = joinLines . map fmt21

--------------------------------------------------------------------------------
-- Q22 — global sales opportunity
--------------------------------------------------------------------------------

-- | For selected phone-country prefixes, count customers with no orders and an
-- above-average positive balance, and sum those balances by prefix.
-- Two runtime values are shared across later work: the scalar average and the
-- bitset of customers who have orders. The monadic binds below are generated
-- build-once scopes; all relational mode selection remains implicit.
q22 :: Q.Query TPCHS [(ByteString, (Int, Double))]
q22 = Q.query $ \s ->
  Q.bitset (customerExtent s) (orderCustomer s) >>= \hasOrders ->
  let totals = Q.foldAll
        (\acc v -> Q.onPair (\a n -> Q.pair (a + v) (n + Q.int 1)) acc)
        (Q.pair 0 (Q.int 0))
        (restrict (prefixOk s) (Q.gt 0 (customerAcctbal s))
          `compose` customerAcctbal s)
  in Q.share (Q.onPair (\sm n -> sm / Q.fromIntegral n) totals) >>= \avg ->
    Q.collect <$> Q.groupFold
      (\acc ab -> Q.onPair (\n sm -> Q.pair (n + Q.int 1) (sm + ab)) acc)
      (Q.pair (Q.int 0) 0)
      (groupBy
        (diff (restrict (prefixOk s) (Q.gt avg (customerAcctbal s))) hasOrders)
        (prefix s)
       `compose` customerAcctbal s)
  where
    prefix s = Q.mapValues (Q.take 2) (customerPhone s)
    prefixOk s = restrict (customer s)
      (Q.oneOf ["13", "31", "23", "29", "30", "18", "17"] (prefix s))

-- | Format one Q22 country-prefix opportunity row.
fmt22 :: (ByteString, (Int, Double)) -> String
fmt22 (c, (n, sm)) = row [bs c, show n, f2 sm]

-- | Sort and render the complete Q22 result.
render22 :: [(ByteString, (Int, Double))] -> String
render22 = joinLines . map fmt22 . sortOn fst
