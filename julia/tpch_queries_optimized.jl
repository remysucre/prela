# TPC-H, hand-optimized variants — an overlay on the idiomatic baseline.
#
# Run via `include("TPCH.jl"); include("tpch_queries_optimized.jl")`.
#
# This file includes `tpch_queries_idiomatic.jl` (all 22 queries) and then
# redefines just the queries it tunes — Q1, Q2, Q9, Q13, Q17, Q18, Q21 —
# so it reads as exactly the set of optimizations applied. Redefining `_qN`
# replaces the method on the generic function already held by the registry;
# a query is re-registered only when its registration itself changes (Q1's
# packed-key `row` formatter).

include("tpch_harness.jl")
_TPCH_DEFER[] = true
include("tpch_queries_idiomatic.jl")
_TPCH_DEFER[] = false

# ============================================================================
# Q1 — pricing summary report
# ============================================================================

function _q1()
    function cmb((qty, ext, di, dp, chg, n), (q, e, d, x))
        (qty + q, ext + e, di + d,
         dp  + e * (1 - d),
         chg + e * (1 - d) * (1 + x),
         n + 1)
    end
    function out((qty, ext, di, dp, chg, n))
        (qty, ext, dp, chg, qty/n, ext/n, di/n, n)
    end
    # Pack (rf, ls) into a small Int so the group fold becomes `DenseFold`
    # over `0..288` (a `[Acc; 288]`-equivalent array cache) instead of a
    # HashMap with `(String, String)` keys. The packed encoding preserves
    # (rf, ls) ascii sort order under Int comparison.
    pack(rfls) =
        (Int(UInt8(rfls[1][1]) - UInt8('A')) << 4) |
         Int(UInt8(rfls[2][1]) - UInt8('F'))
    (((returnflag ⊗ Li.status) ↦ pack)
        ← (lineitem : (shipdate <= "1998-09-02")
            → quantity ⊗ extendedprice ⊗ discount ⊗ tax)) ▷
        (cmb, (0.0, 0.0, 0.0, 0.0, 0.0, 0), 288) ↦ out
end
_q_tpch("1", _ORACLE_Q1, _q1;
        row = (k, (qty, ext, dp, chg, q_avg, e_avg, di_avg, n)) ->
              [string(Char(UInt8('A') + (k >> 4))),
               string(Char(UInt8('F') + (k & 0xF))),
               _fmt(qty), _fmt(ext), _fmt(dp), _fmt(chg),
               _fmt(q_avg), _fmt(e_avg), _fmt(di_avg), _fmt(n)])

# ============================================================================
# Q17 — small-quantity order revenue
# ============================================================================

function _q17()
    # Inner agg: 0.2 * avg(quantity) per part across ALL lineitems —
    # fused single fold producing (sum, count) in one pass, dense over part.n.
    threshold_per_part = ((Li.part ← quantity) ▷
        (((s, n), q) -> (s + q, n + 1), (0.0, 0), part.n)) ↦ (((s, n),) -> 0.2 * s / n)
    let live = lineitem : (Li.part → (brand == "Brand#23") ∧ (container == "MED BOX")) ∧
                          (quantity < (Li.part → threshold_per_part))
        (live → extendedprice) ⊵ (+, 0.0) ↦ (s -> s / 7.0)
    end
end

# ============================================================================
# Q13 — customer distribution
# ============================================================================

function _q13(eng = Prela.Staged())
    # Hoist `Ord.comment ≁ r"special.*requests"` into a Bitset of qualifying
    # orderkeys (~1.5M scan, applied once) so `live_orders` becomes an
    # `O(1)`-membership restriction rather than re-running the regex per
    # order during the count fold.
    qual_orders = bitset(orders : (Ord.comment ≁ r"special.*requests"), orders.n)
    let live_orders = qual_orders,
        # Per-customer order count, dense-folded over customer.n
        count_per_cust = (Ord.customer ← live_orders → date) ▷ ((a, _) -> a + 1, 0, customer.n)
        # Build the c_count → custdist distribution. Customers with no matching
        # orders get c_count = 0 (LEFT JOIN semantic).
        dist = Dict{Int, Int}()
        n_with = 0
        Prela.scan(eng, Prela.prepare(eng, count_per_cust), (_, c) -> begin
            dist[c] = get(dist, c, 0) + 1
            n_with += 1
        end)
        dist[0] = customer.n - n_with
        Prela.MapRel{Int, Int}([k => v for (k, v) in dist])
    end
end

# ============================================================================
# Q9 — product type profit measure
# ============================================================================

function _q9()
    # Hoist `Part.name ~ "green"` out of the 6M-row lineitem scan by
    # precomputing the matching part ids into a Bitset (~200K Part rows
    # scanned once). Per-lineitem becomes one bit-test.
    green_parts = bitset(part : (Part.name ~ r"green"), part.n)
    let live  = lineitem : (Li.part → green_parts),
        sname = live → Li.supplier → Su.nation → Na.name,
        year  = (live → order → date) ↦ (d -> d[1:4]),
        scan  = live → (extendedprice ⊗ discount ⊗ quantity
                        ⊗ ((Li.part ⊗ Li.supplier) → (PS.part ⊗ PS.supplier)' → supplycost))
        ((sname ⊗ year) ← scan) ▷ (
            (a, (e, d, q, cost)) -> a + e * (1 - d) - cost * q,
            0.0
        )
    end
end

# ============================================================================
# Q18 — large-volume customers
# ============================================================================

function _q18()
    # Subquery: sum(l_quantity) per orderkey, dense-folded over orders.n
    sum_qty_per_order = (order ← quantity) ▷ (+, 0.0, orders.n)
    big_orders = sum_qty_per_order > 300.0   # Filter{Ord, Float64}
    # Decorate with the per-order fields. Value tuple: (sum_qty, c_name, c_custkey, date, totalprice)
    big_orders ⊗ (Ord.customer → Cu.name) ⊗ Ord.customer ⊗ date ⊗ totalprice
end

# ============================================================================
# Q2 — minimum-cost supplier per part
# ============================================================================

function _q2()
    let eu_ps = partsupp : (PS.supplier → Su.nation → Na.region → Re.name == "EUROPE"),
        # min(supplycost) per part over European partsupps, dense over part.n
        min_per_part = (PS.part ← eu_ps → supplycost) ▷ (min, Inf, part.n),
        target = (eu_ps :
                  (PS.part → (size == 15) ∧ (type ~ r"BRASS$")) ∧
                  (supplycost == (PS.part → min_per_part)))
        # Output value tuple — supplier-side and part-side fields each
        # factored under their respective navigation.
        target → ((PS.supplier → Su.acctbal ⊗ Su.name ⊗ (Su.nation → Na.name)
                               ⊗ Su.address ⊗ Su.phone ⊗ Su.comment)
               ⊗ PS.part ⊗ (PS.part → mfgr))
    end
end

# ============================================================================
# Q21 — suppliers who kept orders waiting
# ============================================================================

function _q21()
    late = lineitem : (receiptdate > commitdate)
    # `count_distinct > 1` / `count_distinct == 1` materialize a SVec of
    # suppliers per orderkey just to ask 0/1/many. Replace with a
    # constant-state DenseFold tracking `(first_supplier_seen, multi)`:
    # `multi` ⇔ "more than one distinct", and `first != 0 && !multi` ⇔
    # "exactly one distinct". Same `cmb` for both whole-vs-late variants.
    cmb_first_multi(state, s) = let (first, multi) = state, sid = s.id
        first == 0   ? (sid, multi) :
        first != sid ? (first, true) :
                       (first, multi)
    end
    supp_state      = (order ← Li.supplier)        ▷ (cmb_first_multi, (0, false), orders.n)
    late_supp_state = (order ← late → Li.supplier) ▷ (cmb_first_multi, (0, false), orders.n)
    # Hoist each membership predicate (saudi, F-status, multi, only-late)
    # into a Bitset over its domain — the 4-deep `∧` chain on `qualifying`
    # becomes 4 bit-tests per row instead of probing lazy Dicts/regex chains.
    multi_supp = bitset(orders : ((supp_state      ↦ (((f, m),) -> m))             == true), orders.n)
    only_late  = bitset(orders : ((late_supp_state ↦ (((f, m),) -> f != 0 && !m))  == true), orders.n)
    saudi      = bitset(supplier : (Su.nation → Na.name == "SAUDI ARABIA"), supplier.n)
    f_ords     = bitset(orders   : (Ord.status == "F"),                     orders.n)
    qualifying = late : (Li.supplier → saudi) ∧
                        (order → f_ords ∧ multi_supp ∧ only_late)
    counts = (Li.supplier ← qualifying) ▷ ((a, _) -> a + 1, 0, supplier.n)
    counts ⊗ Su.name
end

_autorun_tpch()
