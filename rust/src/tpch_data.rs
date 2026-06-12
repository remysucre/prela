// TPC-H data loader — reads the v2 binary cache produced by `regen tpch`
// (see src/format.rs). Every transformation happened at regen time: ids
// are 0-based with NO_ID holes (the orderkey space is sparse), dates are
// pre-parsed yyyymmdd i64, f64 columns are plain dense doubles. Strings
// are mmap'd and leaked as &'static str.

use crate::cache::{load_f64, load_i64, load_ids, load_strs};
use crate::engine::{Universe, VecRel};

/// Packed-i64-date (yyyymmdd) → "YYYY-MM-DD" — used for output formatting
/// (Q3, Q10, Q18). The parse direction lives in regen, which bakes dates
/// into the cache as i64.
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
        let d = TpchData {
            // patched below from column lengths
            region:   Universe { n: 0 },
            nation:   Universe { n: 0 },
            supplier: Universe { n: 0 },
            customer: Universe { n: 0 },
            part:     Universe { n: 0 },
            partsupp: Universe { n: 0 },
            orders:   Universe { n: 0 },
            lineitem: Universe { n: 0 },

            re_name: load_strs("Region_name"),
            re_comment: load_strs("Region_comment"),

            na_name: load_strs("Nation_name"),
            na_region: load_ids("Nation_region"),
            na_comment: load_strs("Nation_comment"),

            su_name: load_strs("Supplier_name"),
            su_address: load_strs("Supplier_address"),
            su_nation: load_ids("Supplier_nation"),
            su_phone: load_strs("Supplier_phone"),
            su_acctbal: load_f64("Supplier_acctbal"),
            su_comment: load_strs("Supplier_comment"),

            cu_name: load_strs("Customer_name"),
            cu_address: load_strs("Customer_address"),
            cu_nation: load_ids("Customer_nation"),
            cu_phone: load_strs("Customer_phone"),
            cu_acctbal: load_f64("Customer_acctbal"),
            cu_mktsegment: load_strs("Customer_mktsegment"),
            cu_comment: load_strs("Customer_comment"),

            pa_name: load_strs("Part_name"),
            pa_mfgr: load_strs("Part_mfgr"),
            pa_brand: load_strs("Part_brand"),
            pa_type: load_strs("Part_type"),
            pa_size: load_i64("Part_size"),
            pa_container: load_strs("Part_container"),
            pa_retailprice: load_f64("Part_retailprice"),
            pa_comment: load_strs("Part_comment"),

            ps_part: load_ids("PartSupp_part"),
            ps_supplier: load_ids("PartSupp_supplier"),
            ps_availqty: load_i64("PartSupp_availqty"),
            ps_supplycost: load_f64("PartSupp_supplycost"),
            ps_comment: load_strs("PartSupp_comment"),

            // The orderkey space is sparse: dense order columns have
            // NO_ID/default holes baked in by regen.
            ord_customer: load_ids("Order_customer"),
            ord_status: load_strs("Order_status"),
            ord_totalprice: load_f64("Order_totalprice"),
            ord_date: load_i64("Order_date"),
            ord_priority: load_strs("Order_priority"),
            ord_clerk: load_strs("Order_clerk"),
            ord_shippriority: load_i64("Order_shippriority"),
            ord_comment: load_strs("Order_comment"),

            li_order: load_ids("Lineitem_order"),
            li_part: load_ids("Lineitem_part"),
            li_supplier: load_ids("Lineitem_supplier"),
            li_number: load_i64("Lineitem_number"),
            li_quantity: load_f64("Lineitem_quantity"),
            li_extendedprice: load_f64("Lineitem_extendedprice"),
            li_discount: load_f64("Lineitem_discount"),
            li_tax: load_f64("Lineitem_tax"),
            li_returnflag: load_strs("Lineitem_returnflag"),
            li_status: load_strs("Lineitem_status"),
            li_shipdate: load_i64("Lineitem_shipdate"),
            li_commitdate: load_i64("Lineitem_commitdate"),
            li_receiptdate: load_i64("Lineitem_receiptdate"),
            li_shipinstruct: load_strs("Lineitem_shipinstruct"),
            li_shipmode: load_strs("Lineitem_shipmode"),
            li_comment: load_strs("Lineitem_comment"),
        };

        // Universe sizes ARE the dense column lengths; cross-check a few.
        assert_eq!(d.li_order.values.len(), d.li_shipdate.values.len());
        assert_eq!(d.ord_customer.values.len(), d.ord_date.values.len());
        TpchData {
            region:   Universe { n: d.re_name.values.len() },
            nation:   Universe { n: d.na_name.values.len() },
            supplier: Universe { n: d.su_name.values.len() },
            customer: Universe { n: d.cu_name.values.len() },
            part:     Universe { n: d.pa_name.values.len() },
            partsupp: Universe { n: d.ps_part.values.len() },
            orders:   Universe { n: d.ord_customer.values.len() },
            lineitem: Universe { n: d.li_order.values.len() },
            ..d
        }
    }
}
