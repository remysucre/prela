# Does Julia's inference recursion limit explain the interpreted engine's
# slowdown — and does disabling it (Method.recursion_relation) close the gap
# to the staged engine?
#
# Hypothesis (verified on toy CPS chains in ../why_no_inline.jl): the value-
# level CPS protocol re-enters the same drive/probe methods with a
# continuation type that grows one wrapper per plan level; at +3 levels
# inference widens the signature (limit_type_size), poisons the call stack,
# and inlining stops — so deep plans pay boxed calls + closure allocations
# per row. The staged engine is immune (its continuation grows at codegen
# time). `recursion_relation` is the per-method escape hatch: returning true
# tells inference the recursion is fine.
#
# Run each mode in a FRESH process (inference results are cached; the patch
# must precede first inference):
#
#   julia --project=. experiment_recursion.jl interp
#   julia --project=. experiment_recursion.jl interp-rr   # patched
#   julia --project=. experiment_recursion.jl staged
#
# Output: TSV  query \t seconds(min of 3, warm) \t bytes-allocated-per-run

include("Prela.jl")
using .Prela
using .Prela: prepare, scan, Interp, Staged, ID

const MODE = get(ARGS, 1, "interp")

# ===== the patch ========================================================
# Relax the recursion limit on every method that participates in the CPS
# tower: the protocol methods (drive/probe/probe_any/member, and the
# generated _prod_* helpers) plus every closure defined in the module (the
# continuation lambdas inside those methods).
function relax_recursion_limit!(mod::Module)
    world = Base.get_world_counter()
    n = 0
    relax(m::Method) = (m.recursion_relation = (a...) -> true; n += 1)
    for nm in Base.unsorted_names(mod; all = true)
        s = String(nm)
        t = try getglobal(mod, nm) catch; continue end
        if t isa Type && t <: Function && startswith(s, "#")
            ms = Base._methods_by_ftype(Tuple{t, Vararg{Any}}, -1, world)
            ms === nothing && continue
            foreach(mtch -> relax(mtch.method), ms)
        elseif nm in (:drive, :probe, :probe_any, :member) ||
               startswith(s, "_prod_") || startswith(s, "_idx_probe")
            t isa Function || continue
            foreach(relax, methods(t))
        end
    end
    n
end

const ENG = MODE == "staged" ? Staged() : Interp()
if MODE == "interp-rr"
    println(stderr, "patched $(relax_recursion_limit!(Prela)) methods")
end

# ===== synthetic data: a 2M-node linked structure =======================
# `next` hops stay in-universe, `val` is the payload. A depth-d query is a
# d-hop pointer chase per node — the plan type (and the interpreted
# continuation tower) grows linearly with d.

@entity Node begin
    val  :: Int
    next :: ID{Node}
end

const N = 2_000_000
let state = UInt64(42)   # deterministic LCG; no Random dep
    nextr() = (state = state * 0x5851f42d4c957f2d + 1; Int(state >> 33))
    append!(Node.val.pairs,  [ID{Node}(i) => nextr() % 1000 for i in 1:N])
    append!(Node.next.pairs, [ID{Node}(i) => ID{Node}(1 + nextr() % N) for i in 1:N])
end
seal_entities!()
@expose Node

const nodes = Universe{Node}(N)

# ===== queries ==========================================================
# chain(d): nodes → next → next → … (d hops) → val, then filtered — a pure
# Compose tower of depth d+2. wide(w): a w-leg product (Prod arity tower).
chain(d) = (q = nodes; for _ in 1:d; q = q → next; end; q → val)
wide(w) = (p = (val > 0); for i in 1:(w-1); p = p ⊗ (val > i); end; nodes : p → val)

const QUERIES = vcat(
    [("chain$d", chain(d)) for d in (1, 2, 3, 4, 6, 8)],
    [("wide$w", wide(w)) for w in (2, 4, 6)],
    [("chain4+fold", (next ← (chain(4))) ▷ ((a, _) -> a + 1, 0))],
)

# ===== harness ==========================================================
# Time prepare (the index/cache builds — all scans inside go through ENG)
# and the final scan separately; both are where the inlining question lives.
function run_one(q)
    prepare(ENG, q)                          # warm / compile the builds
    GC.gc()
    tprep = @elapsed pq = prepare(ENG, q)
    cnt = Ref(0)
    sink = (x, y) -> (cnt[] += 1; nothing)
    scan(ENG, pq, sink)                      # warm / compile the scan
    tscan = Inf; bytes = 0; rows = 0
    for _ in 1:3
        cnt[] = 0
        GC.gc()
        t = @elapsed scan(ENG, pq, sink)
        b = @allocated scan(ENG, pq, sink)
        tscan = min(tscan, t); bytes = b; rows = cnt[] ÷ 2
    end
    (tprep, tscan, bytes, rows)
end

ms(t) = round(t * 1e3; digits = 2)
println(stderr, "mode = $MODE")
println("query\tprep_ms\tscan_ms\tscan_bytes\trows")
for (name, q) in QUERIES
    tp, ts, b, n = run_one(q)
    println("$name\t$(ms(tp))\t$(ms(ts))\t$b\t$n")
    flush(stdout)
end
