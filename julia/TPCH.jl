# One-shot TPC-H data loader. include("TPCH.jl") once — afterwards every TPC-H
# entity is declared, every relation is populated from `../cache/tpch/*.parquet`,
# and Lineitem's fields are bare-name accessible (`shipdate`, `quantity`, ...).

include("Prela.jl")
using .Prela
using Parquet2, DataFrames, Dates

# === Forward-declare every entity so cyclic refs work ===

@declare Region Nation Supplier Customer Part PartSupp Order Lineitem

# === Entity declarations ===

@entity Region   begin name :: String;  comment :: String  end
@entity Nation   begin name :: String;  region :: ID{Region};  comment :: String  end
@entity Supplier begin name :: String;  address :: String;  nation :: ID{Nation};
                       phone :: String;  acctbal :: Float64;  comment :: String  end
@entity Customer begin name :: String;  address :: String;  nation :: ID{Nation};
                       phone :: String;  acctbal :: Float64;  mktsegment :: String;
                       comment :: String  end
@entity Part     begin name :: String;  mfgr :: String;  brand :: String;
                       type :: String;  size :: Int;  container :: String;
                       retailprice :: Float64;  comment :: String  end
@entity PartSupp begin part :: ID{Part};  supplier :: ID{Supplier};
                       availqty :: Int;  supplycost :: Float64;  comment :: String  end
@entity Order    begin customer :: ID{Customer};  status :: String;
                       totalprice :: Float64;  date :: String;
                       priority :: String;  clerk :: String;
                       shippriority :: Int;  comment :: String  end
@entity Lineitem begin order :: ID{Order};  part :: ID{Part};  supplier :: ID{Supplier};
                       number :: Int;  quantity :: Float64;
                       extendedprice :: Float64;  discount :: Float64;
                       tax :: Float64;  returnflag :: String;  status :: String;
                       shipdate :: String;  commitdate :: String;
                       receiptdate :: String;  shipinstruct :: String;
                       shipmode :: String;  comment :: String  end

@expose Lineitem
# Selective expose: bring entity-unique field names into bare scope, leaving
# genuinely-shared names (name, comment, nation, phone, …) qualified.
@expose Order    : totalprice, date, priority, shippriority, clerk
@expose Part     : mfgr, brand, type, size, container, retailprice
@expose PartSupp : availqty, supplycost
@expose Customer : mktsegment

# Short aliases for entity types — reduce clutter in field accesses.
const Li = Lineitem
const Ord = Order
const Cu = Customer
const Su = Supplier
const PS = PartSupp
const Na = Nation
const Re = Region

# === Loader helpers ===

const TPCH_BASE = "../cache/tpch"

load_df_tpch(name) = DataFrame(Parquet2.Dataset(joinpath(TPCH_BASE, name * ".parquet")); copycols=false)

# Convert a possibly-missing value safely.
@inline _str(x) = x === missing ? "" : String(x)
@inline _f64(x) = x === missing ? 0.0 : Float64(x)
@inline _int(x) = x === missing ? 0 : Int(x)
# Dates come back as Date or DateTime; render ISO yyyy-mm-dd for lexicographic order.
@inline _date(x) = x === missing ? "" : string(Date(x))

# TPC-H natural keys are 1-indexed already (region keys 0..4, but most tables
# use 1..N). For Region/Nation we shift +1. For everything else, the natural
# PK is used directly as ID; FKs are passed through unchanged.

# Push (id, val) into a Rel{D, R}, with id-shift.
function _push_pair!(rel::Prela.Staging{ID{E}, R}, id::Int, val) where {E, R}
    push!(rel.pairs, ID{E}(id) => val)
end

function load_tpch!()
    t_total = time()

    # ---- region (key shifted +1 to avoid Prela's id ≤ 0 sentinel) ----
    t = time()
    df = load_df_tpch("region")
    let rk = df.r_regionkey, nm = df.r_name, cm = df.r_comment
        for i in 1:length(rk)
            id = Int(rk[i]) + 1
            _push_pair!(Prela.lookup_field(ID{Region}, Val(:name)),    id, _str(nm[i]))
            _push_pair!(Prela.lookup_field(ID{Region}, Val(:comment)), id, _str(cm[i]))
        end
    end
    println("  region: $(length(df.r_regionkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- nation (key shifted +1; region FK also +1) ----
    t = time()
    df = load_df_tpch("nation")
    for i in 1:length(df.n_nationkey)
        id = Int(df.n_nationkey[i]) + 1
        _push_pair!(Prela.lookup_field(ID{Nation}, Val(:name)),    id, _str(df.n_name[i]))
        _push_pair!(Prela.lookup_field(ID{Nation}, Val(:region)),  id, ID{Region}(Int(df.n_regionkey[i]) + 1))
        _push_pair!(Prela.lookup_field(ID{Nation}, Val(:comment)), id, _str(df.n_comment[i]))
    end
    println("  nation: $(length(df.n_nationkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- supplier (PK already 1..N) ----
    t = time()
    df = load_df_tpch("supplier")
    for i in 1:length(df.s_suppkey)
        id = Int(df.s_suppkey[i])
        _push_pair!(Prela.lookup_field(ID{Supplier}, Val(:name)),    id, _str(df.s_name[i]))
        _push_pair!(Prela.lookup_field(ID{Supplier}, Val(:address)), id, _str(df.s_address[i]))
        _push_pair!(Prela.lookup_field(ID{Supplier}, Val(:nation)),  id, ID{Nation}(Int(df.s_nationkey[i]) + 1))
        _push_pair!(Prela.lookup_field(ID{Supplier}, Val(:phone)),   id, _str(df.s_phone[i]))
        _push_pair!(Prela.lookup_field(ID{Supplier}, Val(:acctbal)), id, _f64(df.s_acctbal[i]))
        _push_pair!(Prela.lookup_field(ID{Supplier}, Val(:comment)), id, _str(df.s_comment[i]))
    end
    println("  supplier: $(length(df.s_suppkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- customer (PK already 1..N) ----
    t = time()
    df = load_df_tpch("customer")
    for i in 1:length(df.c_custkey)
        id = Int(df.c_custkey[i])
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:name)),       id, _str(df.c_name[i]))
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:address)),    id, _str(df.c_address[i]))
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:nation)),     id, ID{Nation}(Int(df.c_nationkey[i]) + 1))
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:phone)),      id, _str(df.c_phone[i]))
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:acctbal)),    id, _f64(df.c_acctbal[i]))
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:mktsegment)), id, _str(df.c_mktsegment[i]))
        _push_pair!(Prela.lookup_field(ID{Customer}, Val(:comment)),    id, _str(df.c_comment[i]))
    end
    println("  customer: $(length(df.c_custkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- part (PK already 1..N) ----
    t = time()
    df = load_df_tpch("part")
    for i in 1:length(df.p_partkey)
        id = Int(df.p_partkey[i])
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:name)),        id, _str(df.p_name[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:mfgr)),        id, _str(df.p_mfgr[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:brand)),       id, _str(df.p_brand[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:type)),        id, _str(df.p_type[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:size)),        id, _int(df.p_size[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:container)),   id, _str(df.p_container[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:retailprice)), id, _f64(df.p_retailprice[i]))
        _push_pair!(Prela.lookup_field(ID{Part}, Val(:comment)),     id, _str(df.p_comment[i]))
    end
    println("  part: $(length(df.p_partkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- partsupp (composite key → synthetic 1..N) ----
    t = time()
    df = load_df_tpch("partsupp")
    for i in 1:length(df.ps_partkey)
        id = i   # synthetic 1..N
        _push_pair!(Prela.lookup_field(ID{PartSupp}, Val(:part)),       id, ID{Part}(Int(df.ps_partkey[i])))
        _push_pair!(Prela.lookup_field(ID{PartSupp}, Val(:supplier)),   id, ID{Supplier}(Int(df.ps_suppkey[i])))
        _push_pair!(Prela.lookup_field(ID{PartSupp}, Val(:availqty)),   id, _int(df.ps_availqty[i]))
        _push_pair!(Prela.lookup_field(ID{PartSupp}, Val(:supplycost)), id, _f64(df.ps_supplycost[i]))
        _push_pair!(Prela.lookup_field(ID{PartSupp}, Val(:comment)),    id, _str(df.ps_comment[i]))
    end
    println("  partsupp: $(length(df.ps_partkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- orders (PK already 1..N — though TPC-H makes orderkey sparse, see note) ----
    # NOTE: TPC-H orderkey is sparse (not 1..N — skips some IDs). For Prela's
    # dense forward index, we just trust the PK as-is; the dense-fwd grows to
    # the max key (1.5M × 4 ≈ 6M), most slots empty. Memory: 6M * 8B = 48MB
    # per orders.* leaf. Acceptable for SF=1.
    t = time()
    df = load_df_tpch("orders")
    for i in 1:length(df.o_orderkey)
        id = Int(df.o_orderkey[i])
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:customer)),      id, ID{Customer}(Int(df.o_custkey[i])))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:status)),        id, _str(df.o_orderstatus[i]))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:totalprice)),    id, _f64(df.o_totalprice[i]))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:date)),          id, _date(df.o_orderdate[i]))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:priority)),      id, _str(df.o_orderpriority[i]))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:clerk)),         id, _str(df.o_clerk[i]))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:shippriority)),  id, _int(df.o_shippriority[i]))
        _push_pair!(Prela.lookup_field(ID{Order}, Val(:comment)),       id, _str(df.o_comment[i]))
    end
    println("  orders: $(length(df.o_orderkey)) rows ($(round(time()-t; digits=2))s)")

    # ---- lineitem (composite key → synthetic 1..N) ----
    t = time()
    df = load_df_tpch("lineitem")
    n_li = length(df.l_orderkey)
    sizehint!(Prela.lookup_field(ID{Lineitem}, Val(:order)).pairs, n_li)
    for i in 1:n_li
        id = i   # synthetic 1..N
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:order)),         id, ID{Order}(Int(df.l_orderkey[i])))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:part)),          id, ID{Part}(Int(df.l_partkey[i])))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:supplier)),      id, ID{Supplier}(Int(df.l_suppkey[i])))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:number)),        id, _int(df.l_linenumber[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:quantity)),      id, _f64(df.l_quantity[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:extendedprice)), id, _f64(df.l_extendedprice[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:discount)),      id, _f64(df.l_discount[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:tax)),           id, _f64(df.l_tax[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:returnflag)),    id, _str(df.l_returnflag[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:status)),        id, _str(df.l_linestatus[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:shipdate)),      id, _date(df.l_shipdate[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:commitdate)),    id, _date(df.l_commitdate[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:receiptdate)),   id, _date(df.l_receiptdate[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:shipinstruct)),  id, _str(df.l_shipinstruct[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:shipmode)),      id, _str(df.l_shipmode[i]))
        _push_pair!(Prela.lookup_field(ID{Lineitem}, Val(:comment)),       id, _str(df.l_comment[i]))
    end
    println("  lineitem: $n_li rows ($(round(time()-t; digits=2))s)")

    println("TPC-H load total: $(round(time()-t_total; digits=1))s")
end

# === Cache (reuse JOB's cache.jl serializers) ===
include("cache.jl")

if isdir(CACHE_DIR) && load_cache!()
    println("Loaded TPC-H tables from cache (binary + mmap).")
else
    println("Cache miss; loading TPC-H tables from parquet...")
    load_tpch!()
    println("Saving cache for next time...")
    save_cache!()
end

# === Universe sizes — captured from staging `.pairs` before sealing (the
# sealed leaves drop `.pairs`). Order's PK is gappy, so its universe is the
# max id, not the row count.
const _UNIV_N = (
    Lineitem = length(_Lineitem_order.pairs),
    Order    = (let n = 0; for x in _Order_customer.pairs; x.first.id > n && (n = x.first.id); end; n end),
    Customer = length(_Customer_name.pairs),
    Supplier = length(_Supplier_name.pairs),
    Part     = length(_Part_name.pairs),
    PartSupp = length(_PartSupp_part.pairs),
    Nation   = length(_Nation_name.pairs),
    Region   = length(_Region_name.pairs),
)

# Seal every entity-leaf relation into static storage. After this, `Li.quantity`
# etc. take the dense `values[i]` fast path (VecRel column store; SparseRel for
# the gappy Order PK) with no per-row format branch. Re-`@expose` so bare names
# pick up the sealed bindings.
let t = time()
    Prela.seal_entities!()
    println("Sealed entity leaves in $(round(time()-t; digits=2))s")
end
@expose Lineitem
@expose Order    : totalprice, date, priority, shippriority, clerk
@expose Part     : mfgr, brand, type, size, container, retailprice
@expose PartSupp : availqty, supplycost
@expose Customer : mktsegment

# === Universes for the root entities — defined LAST so the `part`/`supplier`
# universe bindings override the same-named Lineitem leaf fields exposed above.
const lineitem = Universe{Lineitem}(_UNIV_N.Lineitem)
const orders   = Universe{Order}(_UNIV_N.Order)
const customer = Universe{Customer}(_UNIV_N.Customer)
const supplier = Universe{Supplier}(_UNIV_N.Supplier)
const part     = Universe{Part}(_UNIV_N.Part)
const partsupp = Universe{PartSupp}(_UNIV_N.PartSupp)
const nation   = Universe{Nation}(_UNIV_N.Nation)
const region   = Universe{Region}(_UNIV_N.Region)

println("Universes: lineitem=$(lineitem.n)  orders=$(orders.n)  customer=$(customer.n)  " *
        "supplier=$(supplier.n)  part=$(part.n)  partsupp=$(partsupp.n)  nation=$(nation.n)  " *
        "region=$(region.n)")
