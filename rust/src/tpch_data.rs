// TPC-H data loader — reads the binary cache produced by Julia's TPCH.jl /
// cache.jl via the shared loaders in cache.rs. f64 columns are bits-pair
// files (f64 reinterpreted from i64 bits); strings are mmap'd and leaked as
// &'static str.

use crate::cache::{load_bits, load_strs, max_key};
use crate::engine::{Universe, Vec1};

/// f64 fields are saved by Julia as Pair{ID, Float64} but reinterpreted as
/// (i64, i64) at write time. Read back: same bytes, just bit-cast.
#[inline] fn f64_from_bits_pair(&(k, bits): &(i64, i64)) -> (i64, f64) {
    (k, f64::from_bits(bits as u64))
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
    pub re_name: Vec1<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub re_comment: Vec1<&'static str>,

    // Nation.{name, region, comment}
    pub na_name: Vec1<&'static str>,
    pub na_region: Vec1<i64>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub na_comment: Vec1<&'static str>,

    // Supplier.{name, address, nation, phone, acctbal, comment}
    pub su_name: Vec1<&'static str>,
    pub su_address: Vec1<&'static str>,
    pub su_nation: Vec1<i64>,
    pub su_phone: Vec1<&'static str>,
    pub su_acctbal: Vec1<f64>,
    pub su_comment: Vec1<&'static str>,

    // Customer.{name, address, nation, phone, acctbal, mktsegment, comment}
    pub cu_name: Vec1<&'static str>,
    pub cu_address: Vec1<&'static str>,
    pub cu_nation: Vec1<i64>,
    pub cu_phone: Vec1<&'static str>,
    pub cu_acctbal: Vec1<f64>,
    pub cu_mktsegment: Vec1<&'static str>,
    pub cu_comment: Vec1<&'static str>,

    // Part.{name, mfgr, brand, type, size, container, retailprice, comment}
    pub pa_name: Vec1<&'static str>,
    pub pa_mfgr: Vec1<&'static str>,
    pub pa_brand: Vec1<&'static str>,
    pub pa_type: Vec1<&'static str>,
    pub pa_size: Vec1<i64>,
    pub pa_container: Vec1<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub pa_retailprice: Vec1<f64>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub pa_comment: Vec1<&'static str>,

    // PartSupp.{part, supplier, availqty, supplycost, comment}
    pub ps_part: Vec1<i64>,
    pub ps_supplier: Vec1<i64>,
    pub ps_availqty: Vec1<i64>,
    pub ps_supplycost: Vec1<f64>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub ps_comment: Vec1<&'static str>,

    // Order.{customer, status, totalprice, date, priority, clerk, shippriority, comment}
    pub ord_customer: Vec1<i64>,
    pub ord_status: Vec1<&'static str>,
    pub ord_totalprice: Vec1<f64>,
    pub ord_date: Vec1<i64>,                      // YYYYMMDD
    pub ord_priority: Vec1<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub ord_clerk: Vec1<&'static str>,
    pub ord_shippriority: Vec1<i64>,
    pub ord_comment: Vec1<&'static str>,

    // Lineitem.{order, part, supplier, number, quantity, extendedprice, discount,
    //           tax, returnflag, status, shipdate, commitdate, receiptdate,
    //           shipinstruct, shipmode, comment}
    pub li_order: Vec1<i64>,
    pub li_part: Vec1<i64>,
    pub li_supplier: Vec1<i64>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub li_number: Vec1<i64>,
    pub li_quantity: Vec1<f64>,
    pub li_extendedprice: Vec1<f64>,
    pub li_discount: Vec1<f64>,
    pub li_tax: Vec1<f64>,
    pub li_returnflag: Vec1<&'static str>,
    pub li_status: Vec1<&'static str>,
    pub li_shipdate: Vec1<i64>,                   // YYYYMMDD
    pub li_commitdate: Vec1<i64>,                 // YYYYMMDD
    pub li_receiptdate: Vec1<i64>,                // YYYYMMDD
    pub li_shipinstruct: Vec1<&'static str>,
    pub li_shipmode: Vec1<&'static str>,
    #[allow(dead_code)] // loaded for full-schema parity; no query reads it
    pub li_comment: Vec1<&'static str>,
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
            region:   Universe { n: n_region as i64 },
            nation:   Universe { n: n_nation as i64 },
            supplier: Universe { n: n_supplier as i64 },
            customer: Universe { n: n_customer as i64 },
            part:     Universe { n: n_part as i64 },
            partsupp: Universe { n: n_partsupp as i64 },
            orders:   Universe { n: n_orders as i64 },
            lineitem: Universe { n: n_lineitem as i64 },

            re_name: Vec1::from_pairs(n_region, re_name_p.iter().copied()),
            re_comment: Vec1::from_pairs(n_region, re_comment_p.iter().copied()),

            na_name: Vec1::from_pairs(n_nation, na_name_p.iter().copied()),
            na_region: Vec1::from_pairs(n_nation, na_region_p.iter().copied()),
            na_comment: Vec1::from_pairs(n_nation, na_comment_p.iter().copied()),

            su_name: Vec1::from_pairs(n_supplier, su_name_p.iter().copied()),
            su_address: Vec1::from_pairs(n_supplier, su_address_p.iter().copied()),
            su_nation: Vec1::from_pairs(n_supplier, su_nation_p.iter().copied()),
            su_phone: Vec1::from_pairs(n_supplier, su_phone_p.iter().copied()),
            su_acctbal: Vec1::from_pairs(n_supplier, su_acctbal_p.iter().map(f64_from_bits_pair)),
            su_comment: Vec1::from_pairs(n_supplier, su_comment_p.iter().copied()),

            cu_name: Vec1::from_pairs(n_customer, cu_name_p.iter().copied()),
            cu_address: Vec1::from_pairs(n_customer, cu_address_p.iter().copied()),
            cu_nation: Vec1::from_pairs(n_customer, cu_nation_p.iter().copied()),
            cu_phone: Vec1::from_pairs(n_customer, cu_phone_p.iter().copied()),
            cu_acctbal: Vec1::from_pairs(n_customer, cu_acctbal_p.iter().map(f64_from_bits_pair)),
            cu_mktsegment: Vec1::from_pairs(n_customer, cu_mktsegment_p.iter().copied()),
            cu_comment: Vec1::from_pairs(n_customer, cu_comment_p.iter().copied()),

            pa_name: Vec1::from_pairs(n_part, pa_name_p.iter().copied()),
            pa_mfgr: Vec1::from_pairs(n_part, pa_mfgr_p.iter().copied()),
            pa_brand: Vec1::from_pairs(n_part, pa_brand_p.iter().copied()),
            pa_type: Vec1::from_pairs(n_part, pa_type_p.iter().copied()),
            pa_size: Vec1::from_pairs(n_part, pa_size_p.iter().copied()),
            pa_container: Vec1::from_pairs(n_part, pa_container_p.iter().copied()),
            pa_retailprice: Vec1::from_pairs(n_part, pa_retailprice_p.iter().map(f64_from_bits_pair)),
            pa_comment: Vec1::from_pairs(n_part, pa_comment_p.iter().copied()),

            ps_part: Vec1::from_pairs(n_partsupp, ps_part_p.iter().copied()),
            ps_supplier: Vec1::from_pairs(n_partsupp, ps_supplier_p.iter().copied()),
            ps_availqty: Vec1::from_pairs(n_partsupp, ps_availqty_p.iter().copied()),
            ps_supplycost: Vec1::from_pairs(n_partsupp, ps_supplycost_p.iter().map(f64_from_bits_pair)),
            ps_comment: Vec1::from_pairs(n_partsupp, ps_comment_p.iter().copied()),

            ord_customer: Vec1::from_pairs(n_orders, ord_customer_p.iter().copied()),
            ord_status: Vec1::from_pairs(n_orders, ord_status_p.iter().copied()),
            ord_totalprice: Vec1::from_pairs(n_orders, ord_totalprice_p.iter().map(f64_from_bits_pair)),
            ord_date: Vec1::from_pairs(n_orders, ord_date_p.iter().map(|(k, s)| (*k, parse_yyyymmdd(s)))),
            ord_priority: Vec1::from_pairs(n_orders, ord_priority_p.iter().copied()),
            ord_clerk: Vec1::from_pairs(n_orders, ord_clerk_p.iter().copied()),
            ord_shippriority: Vec1::from_pairs(n_orders, ord_shippriority_p.iter().copied()),
            ord_comment: Vec1::from_pairs(n_orders, ord_comment_p.iter().copied()),

            li_order: Vec1::from_pairs(n_lineitem, li_order_p.iter().copied()),
            li_part: Vec1::from_pairs(n_lineitem, li_part_p.iter().copied()),
            li_supplier: Vec1::from_pairs(n_lineitem, li_supplier_p.iter().copied()),
            li_number: Vec1::from_pairs(n_lineitem, li_number_p.iter().copied()),
            li_quantity: Vec1::from_pairs(n_lineitem, li_quantity_p.iter().map(f64_from_bits_pair)),
            li_extendedprice: Vec1::from_pairs(n_lineitem, li_extendedprice_p.iter().map(f64_from_bits_pair)),
            li_discount: Vec1::from_pairs(n_lineitem, li_discount_p.iter().map(f64_from_bits_pair)),
            li_tax: Vec1::from_pairs(n_lineitem, li_tax_p.iter().map(f64_from_bits_pair)),
            li_returnflag: Vec1::from_pairs(n_lineitem, li_returnflag_p.iter().copied()),
            li_status: Vec1::from_pairs(n_lineitem, li_status_p.iter().copied()),
            li_shipdate:    Vec1::from_pairs(n_lineitem, li_shipdate_p.iter().map(|(k, s)| (*k, parse_yyyymmdd(s)))),
            li_commitdate:  Vec1::from_pairs(n_lineitem, li_commitdate_p.iter().map(|(k, s)| (*k, parse_yyyymmdd(s)))),
            li_receiptdate: Vec1::from_pairs(n_lineitem, li_receiptdate_p.iter().map(|(k, s)| (*k, parse_yyyymmdd(s)))),
            li_shipinstruct: Vec1::from_pairs(n_lineitem, li_shipinstruct_p.iter().copied()),
            li_shipmode: Vec1::from_pairs(n_lineitem, li_shipmode_p.iter().copied()),
            li_comment: Vec1::from_pairs(n_lineitem, li_comment_p.iter().copied()),
        }
    }
}
