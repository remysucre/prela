// TPC-H data loader — reads the binary cache produced by Julia's TPCH.jl /
// cache.jl via the shared loaders in cache.rs. f64 columns are bits-pair
// files (f64 reinterpreted from i64 bits); strings are mmap'd and leaked as
// &'static str.

use crate::cache::{ids, ids_fk, load_bits, load_strs, max_key};
use crate::engine::{Universe, VecRel, NO_ID};

/// f64 fields are saved by Julia as Pair{ID, Float64} but reinterpreted as
/// (i64, i64) at write time. Read back: same bytes, just bit-cast.
#[inline] fn f64_val((k, bits): (usize, i64)) -> (usize, f64) {
    (k, f64::from_bits(bits as u64))
}

/// "YYYY-MM-DD" string value → packed i64 date; key passes through.
#[inline] fn date_val((k, s): (usize, &str)) -> (usize, i64) {
    (k, parse_yyyymmdd(s))
}

/// "YYYY-MM-DD" → packed i64 YYYYMMDD (numeric compare preserves lexical
/// order). Cheap inline digit pull; loader use only.
#[inline] pub fn parse_yyyymmdd(s: &str) -> i64 {
    let b = s.as_bytes();
    debug_assert_eq!(b.len(), 10);
    let d = |i: usize| (b[i] - b'0') as i64;
    d(0)*10_000_000 + d(1)*1_000_000 + d(2)*100_000 + d(3)*10_000
        + d(5)*1000 + d(6)*100
        + d(8)*10  + d(9)
}

/// Inverse of parse_yyyymmdd — used for output formatting (Q3, Q10, Q18).
#[inline] pub fn fmt_yyyymmdd(d: i64) -> String {
    format!("{:04}-{:02}-{:02}", d / 10000, (d / 100) % 100, d % 100)
}

pub struct TpchData {
    // Universes
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub region:   Universe,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub nation:   Universe,
    pub supplier: Universe,
    pub customer: Universe,
    pub part:     Universe,
    pub partsupp: Universe,
    pub orders:   Universe,
    pub lineitem: Universe,

    // Region.{name, comment}
    pub re_name: VecRel<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub re_comment: VecRel<&'static str>,

    // Nation.{name, region, comment}
    pub na_name: VecRel<&'static str>,
    pub na_region: VecRel<usize>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub na_comment: VecRel<&'static str>,

    // Supplier.{name, address, nation, phone, acctbal, comment}
    pub su_name: VecRel<&'static str>,
    pub su_address: VecRel<&'static str>,
    pub su_nation: VecRel<usize>,
    pub su_phone: VecRel<&'static str>,
    pub su_acctbal: VecRel<f64>,
    pub su_comment: VecRel<&'static str>,

    // Customer.{name, address, nation, phone, acctbal, mktsegment, comment}
    pub cu_name: VecRel<&'static str>,
    pub cu_address: VecRel<&'static str>,
    pub cu_nation: VecRel<usize>,
    pub cu_phone: VecRel<&'static str>,
    pub cu_acctbal: VecRel<f64>,
    pub cu_mktsegment: VecRel<&'static str>,
    pub cu_comment: VecRel<&'static str>,

    // Part.{name, mfgr, brand, type, size, container, retailprice, comment}
    pub pa_name: VecRel<&'static str>,
    pub pa_mfgr: VecRel<&'static str>,
    pub pa_brand: VecRel<&'static str>,
    pub pa_type: VecRel<&'static str>,
    pub pa_size: VecRel<i64>,
    pub pa_container: VecRel<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub pa_retailprice: VecRel<f64>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub pa_comment: VecRel<&'static str>,

    // PartSupp.{part, supplier, availqty, supplycost, comment}
    pub ps_part: VecRel<usize>,
    pub ps_supplier: VecRel<usize>,
    pub ps_availqty: VecRel<i64>,
    pub ps_supplycost: VecRel<f64>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub ps_comment: VecRel<&'static str>,

    // Order.{customer, status, totalprice, date, priority, clerk, shippriority, comment}
    pub ord_customer: VecRel<usize>,
    pub ord_status: VecRel<&'static str>,
    pub ord_totalprice: VecRel<f64>,
    pub ord_date: VecRel<i64>,                      // YYYYMMDD
    pub ord_priority: VecRel<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub ord_clerk: VecRel<&'static str>,
    pub ord_shippriority: VecRel<i64>,
    pub ord_comment: VecRel<&'static str>,

    // Lineitem.{order, part, supplier, number, quantity, extendedprice, discount,
    //           tax, returnflag, status, shipdate, commitdate, receiptdate,
    //           shipinstruct, shipmode, comment}
    pub li_order: VecRel<usize>,
    pub li_part: VecRel<usize>,
    pub li_supplier: VecRel<usize>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub li_number: VecRel<i64>,
    pub li_quantity: VecRel<f64>,
    pub li_extendedprice: VecRel<f64>,
    pub li_discount: VecRel<f64>,
    pub li_tax: VecRel<f64>,
    pub li_returnflag: VecRel<&'static str>,
    pub li_status: VecRel<&'static str>,
    pub li_shipdate: VecRel<i64>,                   // YYYYMMDD
    pub li_commitdate: VecRel<i64>,                 // YYYYMMDD
    pub li_receiptdate: VecRel<i64>,                // YYYYMMDD
    pub li_shipinstruct: VecRel<&'static str>,
    pub li_shipmode: VecRel<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub li_comment: VecRel<&'static str>,
}

impl TpchData {
    pub fn load() -> Self {
        // Region (str / str)
        let re_name_p = load_strs("Region_name");
        let re_comment_p = load_strs("Region_comment");
        let n_region = max_key(&re_name_p);

        // Nation (str / i64 FK / str)
        let na_name_p = load_strs("Nation_name");
        let na_region_p = load_bits("Nation_region");
        let na_comment_p = load_strs("Nation_comment");
        let n_nation = max_key(&na_name_p);

        // Supplier
        let su_name_p = load_strs("Supplier_name");
        let su_address_p = load_strs("Supplier_address");
        let su_nation_p = load_bits("Supplier_nation");
        let su_phone_p = load_strs("Supplier_phone");
        let su_acctbal_p = load_bits("Supplier_acctbal");
        let su_comment_p = load_strs("Supplier_comment");
        let n_supplier = max_key(&su_name_p);

        // Customer
        let cu_name_p = load_strs("Customer_name");
        let cu_address_p = load_strs("Customer_address");
        let cu_nation_p = load_bits("Customer_nation");
        let cu_phone_p = load_strs("Customer_phone");
        let cu_acctbal_p = load_bits("Customer_acctbal");
        let cu_mktsegment_p = load_strs("Customer_mktsegment");
        let cu_comment_p = load_strs("Customer_comment");
        let n_customer = max_key(&cu_name_p);

        // Part
        let pa_name_p = load_strs("Part_name");
        let pa_mfgr_p = load_strs("Part_mfgr");
        let pa_brand_p = load_strs("Part_brand");
        let pa_type_p = load_strs("Part_type");
        let pa_size_p = load_bits("Part_size");
        let pa_container_p = load_strs("Part_container");
        let pa_retailprice_p = load_bits("Part_retailprice");
        let pa_comment_p = load_strs("Part_comment");
        let n_part = max_key(&pa_name_p);

        // PartSupp (composite-key, synthetic ID 1..N)
        let ps_part_p = load_bits("PartSupp_part");
        let ps_supplier_p = load_bits("PartSupp_supplier");
        let ps_availqty_p = load_bits("PartSupp_availqty");
        let ps_supplycost_p = load_bits("PartSupp_supplycost");
        let ps_comment_p = load_strs("PartSupp_comment");
        let n_partsupp = max_key(&ps_part_p);

        // Orders (sparse keys — n is max orderkey, not row count)
        let ord_customer_p = load_bits("Order_customer");
        let ord_status_p = load_strs("Order_status");
        let ord_totalprice_p = load_bits("Order_totalprice");
        let ord_date_p = load_strs("Order_date");
        let ord_priority_p = load_strs("Order_priority");
        let ord_clerk_p = load_strs("Order_clerk");
        let ord_shippriority_p = load_bits("Order_shippriority");
        let ord_comment_p = load_strs("Order_comment");
        let n_orders = max_key(&ord_customer_p);

        // Lineitem (composite-key, synthetic ID 1..N)
        let li_order_p = load_bits("Lineitem_order");
        let li_part_p = load_bits("Lineitem_part");
        let li_supplier_p = load_bits("Lineitem_supplier");
        let li_number_p = load_bits("Lineitem_number");
        let li_quantity_p = load_bits("Lineitem_quantity");
        let li_extendedprice_p = load_bits("Lineitem_extendedprice");
        let li_discount_p = load_bits("Lineitem_discount");
        let li_tax_p = load_bits("Lineitem_tax");
        let li_returnflag_p = load_strs("Lineitem_returnflag");
        let li_status_p = load_strs("Lineitem_status");
        let li_shipdate_p = load_strs("Lineitem_shipdate");
        let li_commitdate_p = load_strs("Lineitem_commitdate");
        let li_receiptdate_p = load_strs("Lineitem_receiptdate");
        let li_shipinstruct_p = load_strs("Lineitem_shipinstruct");
        let li_shipmode_p = load_strs("Lineitem_shipmode");
        let li_comment_p = load_strs("Lineitem_comment");
        let n_lineitem = max_key(&li_order_p);

        TpchData {
            region:   Universe { n: n_region },
            nation:   Universe { n: n_nation },
            supplier: Universe { n: n_supplier },
            customer: Universe { n: n_customer },
            part:     Universe { n: n_part },
            partsupp: Universe { n: n_partsupp },
            orders:   Universe { n: n_orders },
            lineitem: Universe { n: n_lineitem },

            // `ids` shifts keys to 0-based usize (internal id = cache id
            // − 1); `ids_fk` also shifts the value (FK columns). FK VecRel
            // columns fill holes with NO_ID so a gap key (the sparse
            // orderkey space) never aliases entity 0 — see the VecRel
            // invariant in engine.rs.
            re_name: VecRel::from_pairs(n_region, ids(&re_name_p)),
            re_comment: VecRel::from_pairs(n_region, ids(&re_comment_p)),

            na_name: VecRel::from_pairs(n_nation, ids(&na_name_p)),
            na_region: VecRel::from_pairs_fill(n_nation, NO_ID, ids_fk(&na_region_p)),
            na_comment: VecRel::from_pairs(n_nation, ids(&na_comment_p)),

            su_name: VecRel::from_pairs(n_supplier, ids(&su_name_p)),
            su_address: VecRel::from_pairs(n_supplier, ids(&su_address_p)),
            su_nation: VecRel::from_pairs_fill(n_supplier, NO_ID, ids_fk(&su_nation_p)),
            su_phone: VecRel::from_pairs(n_supplier, ids(&su_phone_p)),
            su_acctbal: VecRel::from_pairs(n_supplier, ids(&su_acctbal_p).map(f64_val)),
            su_comment: VecRel::from_pairs(n_supplier, ids(&su_comment_p)),

            cu_name: VecRel::from_pairs(n_customer, ids(&cu_name_p)),
            cu_address: VecRel::from_pairs(n_customer, ids(&cu_address_p)),
            cu_nation: VecRel::from_pairs_fill(n_customer, NO_ID, ids_fk(&cu_nation_p)),
            cu_phone: VecRel::from_pairs(n_customer, ids(&cu_phone_p)),
            cu_acctbal: VecRel::from_pairs(n_customer, ids(&cu_acctbal_p).map(f64_val)),
            cu_mktsegment: VecRel::from_pairs(n_customer, ids(&cu_mktsegment_p)),
            cu_comment: VecRel::from_pairs(n_customer, ids(&cu_comment_p)),

            pa_name: VecRel::from_pairs(n_part, ids(&pa_name_p)),
            pa_mfgr: VecRel::from_pairs(n_part, ids(&pa_mfgr_p)),
            pa_brand: VecRel::from_pairs(n_part, ids(&pa_brand_p)),
            pa_type: VecRel::from_pairs(n_part, ids(&pa_type_p)),
            pa_size: VecRel::from_pairs(n_part, ids(&pa_size_p)),
            pa_container: VecRel::from_pairs(n_part, ids(&pa_container_p)),
            pa_retailprice: VecRel::from_pairs(n_part, ids(&pa_retailprice_p).map(f64_val)),
            pa_comment: VecRel::from_pairs(n_part, ids(&pa_comment_p)),

            ps_part: VecRel::from_pairs_fill(n_partsupp, NO_ID, ids_fk(&ps_part_p)),
            ps_supplier: VecRel::from_pairs_fill(n_partsupp, NO_ID, ids_fk(&ps_supplier_p)),
            ps_availqty: VecRel::from_pairs(n_partsupp, ids(&ps_availqty_p)),
            ps_supplycost: VecRel::from_pairs(n_partsupp, ids(&ps_supplycost_p).map(f64_val)),
            ps_comment: VecRel::from_pairs(n_partsupp, ids(&ps_comment_p)),

            ord_customer: VecRel::from_pairs_fill(n_orders, NO_ID, ids_fk(&ord_customer_p)),
            ord_status: VecRel::from_pairs(n_orders, ids(&ord_status_p)),
            ord_totalprice: VecRel::from_pairs(n_orders, ids(&ord_totalprice_p).map(f64_val)),
            ord_date: VecRel::from_pairs(n_orders, ids(&ord_date_p).map(date_val)),
            ord_priority: VecRel::from_pairs(n_orders, ids(&ord_priority_p)),
            ord_clerk: VecRel::from_pairs(n_orders, ids(&ord_clerk_p)),
            ord_shippriority: VecRel::from_pairs(n_orders, ids(&ord_shippriority_p)),
            ord_comment: VecRel::from_pairs(n_orders, ids(&ord_comment_p)),

            li_order: VecRel::from_pairs_fill(n_lineitem, NO_ID, ids_fk(&li_order_p)),
            li_part: VecRel::from_pairs_fill(n_lineitem, NO_ID, ids_fk(&li_part_p)),
            li_supplier: VecRel::from_pairs_fill(n_lineitem, NO_ID, ids_fk(&li_supplier_p)),
            li_number: VecRel::from_pairs(n_lineitem, ids(&li_number_p)),
            li_quantity: VecRel::from_pairs(n_lineitem, ids(&li_quantity_p).map(f64_val)),
            li_extendedprice: VecRel::from_pairs(n_lineitem, ids(&li_extendedprice_p).map(f64_val)),
            li_discount: VecRel::from_pairs(n_lineitem, ids(&li_discount_p).map(f64_val)),
            li_tax: VecRel::from_pairs(n_lineitem, ids(&li_tax_p).map(f64_val)),
            li_returnflag: VecRel::from_pairs(n_lineitem, ids(&li_returnflag_p)),
            li_status: VecRel::from_pairs(n_lineitem, ids(&li_status_p)),
            li_shipdate:    VecRel::from_pairs(n_lineitem, ids(&li_shipdate_p).map(date_val)),
            li_commitdate:  VecRel::from_pairs(n_lineitem, ids(&li_commitdate_p).map(date_val)),
            li_receiptdate: VecRel::from_pairs(n_lineitem, ids(&li_receiptdate_p).map(date_val)),
            li_shipinstruct: VecRel::from_pairs(n_lineitem, ids(&li_shipinstruct_p)),
            li_shipmode: VecRel::from_pairs(n_lineitem, ids(&li_shipmode_p)),
            li_comment: VecRel::from_pairs(n_lineitem, ids(&li_comment_p)),
        }
    }
}
