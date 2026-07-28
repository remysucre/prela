{-# LANGUAGE RankNTypes #-}
{-# LANGUAGE TemplateHaskell #-}

-- | The TPC-H schema, the same eight tables the Rust port declares in
-- src/tpch_schema.rs. Each column is a separate file in the shared binary cache
-- (`prela/cache/Lineitem_quantity.bin` and so on), written by the Rust regen
-- tool out of DuckDB parquet; see benchmarking.md for how to make one.
--
-- Two things about the declaration are worth reading before the queries.
--
-- `Order` is a SPARSE entity. TPC-H leaves gaps in the orderkey sequence, so the
-- id space runs 0 .. 5999999 while only 1.5M of those are real orders. The gaps
-- show up as holes in `Order_customer`, the first declared field, and
-- `sparseEntity` builds the validity mask from exactly that column, so driving
-- `orders` walks the live ids only. Every other entity's keys are contiguous.
--
-- Every field name that appears on more than one entity is renamed with `as`,
-- because the generator puts all accessors at the top level of this module and
-- Haskell has no `Supplier::name` spelling. The rule used below is entity prefix
-- then field: `supplierName`, `customerName`, `liPart`, `psPart`. Names that are
-- unique across the schema keep their bare spelling, which is most of the
-- interesting ones — `quantity`, `discount`, `shipdate`, `mktsegment`.
module TPCH.Schema where

import Prela.Schema

declareSchema "TPCH"
  [ entity "Region" "region"
      [ one "name"    str `as` "regionName"
      , one "comment" str `as` "regionComment"
      ]
  , entity "Nation" "nation"
      [ one "name"    str          `as` "nationName"
      , one "region"  (ref "Region") `as` "nationRegion"
      , one "comment" str          `as` "nationComment"
      ]
  , entity "Supplier" "supplier"
      [ one "name"    str            `as` "supplierName"
      , one "address" str            `as` "supplierAddress"
      , one "nation"  (ref "Nation") `as` "supplierNation"
      , one "phone"   str            `as` "supplierPhone"
      , one "acctbal" dbl            `as` "supplierAcctbal"
      , one "comment" str            `as` "supplierComment"
      ]
  , entity "Customer" "customer"
      [ one "name"       str            `as` "customerName"
      , one "address"    str            `as` "customerAddress"
      , one "nation"     (ref "Nation") `as` "customerNation"
      , one "phone"      str            `as` "customerPhone"
      , one "acctbal"    dbl            `as` "customerAcctbal"
      , one "mktsegment" str
      , one "comment"    str            `as` "customerComment"
      ]
  , entity "Part" "part"
      [ one "name"         str `as` "partName"
      , one "mfgr"         str
      , one "brand"        str
      , one "ty"           str
      , one "size"         int
      , one "container"    str
      , one "retailprice"  dbl
      , one "comment"      str `as` "partComment"
      ]
  , entity "PartSupp" "partsupp"
      [ one "part"       (ref "Part")     `as` "psPart"
      , one "supplier"   (ref "Supplier") `as` "psSupplier"
      , one "availqty"   int
      , one "supplycost" dbl
      , one "comment"    str              `as` "psComment"
      ]
  -- Sparse: the mask comes from `Order_customer`, which must therefore stay the
  -- first field. Same arrangement as Rust's `Order(orders sparse)`.
  , sparseEntity "Order" "orders"
      [ one "customer"     (ref "Customer") `as` "orderCustomer"
      , one "status"       str              `as` "orderStatus"
      , one "totalprice"   dbl
      , one "date"         int
      , one "priority"     str
      , one "clerk"        str
      , one "shippriority" int
      , one "comment"      str              `as` "orderComment"
      ]
  , entity "Lineitem" "lineitem"
      [ one "order"         (ref "Order")    `as` "liOrder"
      , one "part"          (ref "Part")     `as` "liPart"
      , one "supplier"      (ref "Supplier") `as` "liSupplier"
      , one "number"        int
      , one "quantity"      dbl
      , one "extendedprice" dbl
      , one "discount"      dbl
      , one "tax"           dbl
      , one "returnflag"    str
      , one "status"        str `as` "lineStatus"
      , one "shipdate"      int
      , one "commitdate"    int
      , one "receiptdate"   int
      , one "shipinstruct"  str
      , one "shipmode"      str
      , one "comment"       str `as` "lineComment"
      ]
  ]
