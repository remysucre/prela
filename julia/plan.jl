# Engines and `prepare` — the lowering from logical to physical plan.
#
# The invariant this file enforces: state lives only in physical-plan nodes,
# is built exactly once — at physical-node construction, inside `prepare` —
# from already-prepared children, and is always concretely typed. Execution
# (`scan`) is stateless and re-entrant; a prepared plan is a pure value that
# can be scanned any number of times.
#
# `prepare` itself is pure mode-propagation: it threads access modes top-down
# and picks each node's physical *type* by a rule on (node type, mode) — never
# on data. The data work — driving an inner once to fill an index/cache —
# lives in the `build_*` functions below, and every one of those scans goes
# through `scan(engine, …)`, the same seam the final result scan uses. So the
# engine choice covers ALL per-row work in the system; only `prepare`'s
# selection (type-level, instant) stays interpreted.

# ===== the engine seam ==================================================
# An engine is anything that implements `scan(eng, pq, sink)` — drive a
# *prepared* plan, calling sink(x, y) per pair. Two engines are provided:
#
#   Interp() — value-level CPS: the `drive`/`probe` closure chains in
#              interp.jl. Simple, debuggable, and the subject of the
#              compiler-inlining experiments.
#   Staged() — type-level CPS: `_exec_drive` in staged.jl walks the prepared
#              plan's TYPE at codegen time and emits one fused loop nest.
#              The default engine.

abstract type Engine end
struct Interp <: Engine end
struct Staged <: Engine end

scan(::Interp, pq, sink) = drive(pq, sink)
scan(::Staged, pq, sink) = _exec_drive(pq, sink)   # defined in staged.jl

export Engine, Interp, Staged, scan, prepare

# ===== index-building helpers ===========================================

# Dense forward index: for an entity-keyed relation (contiguous PK 1..n) the
# index is a Vector{Vector{R}} addressed by `.id` — an array access per probe,
# no hashing. Unfilled slots share one empty vector. Out-of-range keys (junk
# id ≤ 0, or beyond a caller-supplied universe `n`) are skipped.
function _dense_fwd(pairs::Vector{Pair{ID{E}, R}}, n::Int = _max_id(pairs)) where {E, R}
    empty = R[]
    v = fill(empty, n)
    for p in pairs
        i = p.first.id
        (1 <= i <= n) || continue
        @inbounds vi = v[i]
        vi === empty && (vi = R[]; @inbounds v[i] = vi)
        push!(vi, p.second)
    end
    v
end
function _max_id(pairs::Vector{<:Pair{<:ID}})
    n = 0
    for p in pairs
        i = p.first.id
        i > n && (n = i)
    end
    n
end

# `_mat_idx` builds a materialized result's forward index: dense
# `Vector{Vector}` when entity-keyed (array access, no hashing), else a Dict.
_mat_idx(pairs::Vector{Pair{ID{E}, R}}) where {E, R} = _dense_fwd(pairs)
function _mat_idx(pairs::Vector{Pair{D, R}}) where {D, R}
    d = Dict{D, Vector{R}}()
    for p in pairs
        push!(get!(() -> R[], d, p.first), p.second)
    end
    d
end

# ===== build_*: physical-node constructors with state ===================
# Each takes the engine plus already-prepared (driven) inners, fills its
# index/cache through `scan(eng, …)`, and returns the finished physical node.

# Inv probed: eager inverse index — drive the inner, bucket keys by value.
function build_inv_index(eng::Engine, pq::Query{A, B}) where {A, B}
    d = Dict{B, Vector{A}}()
    scan(eng, pq, (a, b) -> push!(get!(() -> A[], d, b), a))
    Indexed{B, A, typeof(d)}(d)
end

# LeftCompose probed: concrete Dict{RK, Vector{SV}} — drive s, probe r per row.
function build_lc_index(eng::Engine, pr::Query{D, RK}, ps::Query{D, SV}) where {D, RK, SV}
    d = Dict{RK, Vector{SV}}()
    scan(eng, ps, (dd, v) -> probe(pr, dd, rk -> push!(get!(() -> SV[], d, rk), v)))
    Indexed{RK, SV, typeof(d)}(d)
end

# Fold: per-key foldl cache (`S` is the accumulator type, fixed by `init`).
function build_fold(eng::Engine, pq::Query{D, R}, op, init::S) where {D, R, S}
    acc = Dict{D, S}()
    scan(eng, pq, (d, v) -> (acc[d] = op(get(acc, d, init), v)))
    FoldP{D, S}(acc)
end

# DenseFold: dense per-key fold over slots `0..n`.
function build_densefold(eng::Engine, pq::Query{D, R}, op, init::S, n::Int) where {D, R, S}
    sz = n + 1; vals = fill(init, sz); seen = falses(sz)
    scan(eng, pq, (d, v) -> begin
        i = _denseidx(d) + 1
        if 1 <= i <= sz
            @inbounds vals[i] = op(vals[i], v); @inbounds seen[i] = true
        end
    end)
    DenseFoldP{D, S}(vals, seen)
end

# BufFold: per-key buffered reduce — collect all values, then call `f`. `S` is
# `f`'s result type (not derivable from the inner), so the caller passes it.
function build_buffold(eng::Engine, pq::Query{D, R}, f, ::Type{S}) where {D, R, S}
    buf = Dict{D, Vector{R}}()
    scan(eng, pq, (d, v) -> push!(get!(() -> R[], buf, d), v))
    out = Dict{D, S}(); for (d, vs) in buf; out[d] = f(vs); end
    FoldP{D, S}(out)
end

# Scalar: no-group foldl to a single value.
function build_scalar(eng::Engine, pq, op, init::S) where {S}
    acc = Ref{S}(init)
    scan(eng, pq, (_, v) -> (acc[] = op(acc[], v)))
    ScalarP{S}(acc[])
end

# Materialized: drive the inner once into stored pairs (driven) or a concrete
# forward index (probed). `_matpairs` is the function barrier so the
# materializing scan specializes on the concrete prepared inner type.
function _matpairs(eng::Engine, pa, ::Type{D}, ::Type{R}) where {D, R}
    out = Pair{D, R}[]
    scan(eng, pa, (x, y) -> push!(out, x => y))
    out
end
build_mat_stream(eng::Engine, pa::Query{D, R}) where {D, R} =
    MapRel{D, R}(_matpairs(eng, pa, D, R))
function build_mat_probed(eng::Engine, pa::Query{D, R}) where {D, R}
    idx = _mat_idx(_matpairs(eng, pa, D, R))
    Indexed{D, R, typeof(idx)}(idx)
end

# MatSet driven/probed: stored keys / membership Set.
function build_matset_keys(eng::Engine, pa::Unary{D}) where {D}
    keys = D[]; scan(eng, pa, (x, _) -> push!(keys, x)); UnaryVec{D}(keys)
end
function build_matset_set(eng::Engine, pa::Unary{D}) where {D}
    s = Set{D}(); scan(eng, pa, (x, _) -> push!(s, x)); MatSetProbed{D}(s)
end

# BitsetMat → dense `Bitset` membership: one bit per dense-int value (`MEM` is
# the value side — a Unary emits its keys through the value slot).
function build_bitset(eng::Engine, pq::Query{D, MEM}, n::Int) where {D, MEM}
    b = Bitset{MEM}(n)
    scan(eng, pq, (_, v) -> begin
        i = _denseidx(v) + 1
        @inbounds (1 <= i <= n + 1) && (b.bits[i] = true)
    end)
    b
end

# ===== prepare: lift the drive/probe mode to the types ==================
# `prepare(eng, q)` rewrites the plan top-down so each node is in its access
# mode (the root is driven). Structural nodes rebuild with children prepared
# per the mode table; stream-or-index nodes (`Inv`, `LeftCompose`,
# `Materialized`, `MatSet`) split by mode; pipeline-breakers (the folds) build
# their cache in either mode. It is **memo-free and type-stable**: for a
# concrete (node, mode) the physical type is determined, so `prepare` infers a
# concrete plan and the rebuild inlines. Prela is non-materialized by default —
# a subexpression referenced twice is prepared (and run) twice; wrap it in
# `materialize`/`collect` to share.

prepare(eng::Engine, q) = prepare(eng, q, Driven())

# Inv: the split (driven → stream, probed → eager concrete index).
prepare(eng::Engine, n::Inv{B,A,Q}, ::Driven) where {B,A,Q} =
    (pq = prepare(eng, n.q, Driven()); InvStream{B,A,typeof(pq)}(pq))
prepare(eng::Engine, n::Inv, ::Probed) =
    build_inv_index(eng, prepare(eng, n.q, Driven()))

# Structural nodes: rebuild with children prepared in their modes.
prepare(eng::Engine, n::Compose, m::Mode) =
    Compose(prepare(eng, n.a, m), prepare(eng, n.b, Probed()))
prepare(eng::Engine, n::Filter, m::Mode) = Filter(prepare(eng, n.a, m), n.pred)
prepare(eng::Engine, n::Restrict, m::Mode) =
    Restrict(prepare(eng, n.a, m), prepare(eng, n.b, Probed()))
prepare(eng::Engine, n::Diff, m::Mode) =
    Diff(prepare(eng, n.a, m), prepare(eng, n.b, Probed()))
prepare(eng::Engine, n::Disj, ::Mode) =
    Disj(prepare(eng, n.a, Probed()), prepare(eng, n.b, Probed()))
prepare(eng::Engine, n::Prod, m::Mode) =
    Prod((prepare(eng, n.ops[1], m),
          map(o -> prepare(eng, o, Probed()), Base.tail(n.ops))...))
prepare(eng::Engine, n::Map{D,R,S,Q,F}, m::Mode) where {D,R,S,Q,F} =
    (pq = prepare(eng, n.q, m); Map{D,R,S,typeof(pq),F}(pq, n.f))
prepare(eng::Engine, n::LeftConj{D,ML,R}, m::Mode) where {D,ML,R} =
    (pl = prepare(eng, n.l, Probed()); pr = prepare(eng, n.r, m);
     LeftConj{D, typeof(pl), typeof(pr)}(pl, pr))

# LeftCompose: stream-or-index split, like Inv.
prepare(eng::Engine, n::LeftCompose{D,RK,SV,QR,QS}, ::Driven) where {D,RK,SV,QR,QS} =
    (pr = prepare(eng, n.r, Probed()); ps = prepare(eng, n.s, Driven());
     LCStream{D,RK,SV,typeof(pr),typeof(ps)}(pr, ps))
prepare(eng::Engine, n::LeftCompose, ::Probed) =
    build_lc_index(eng, prepare(eng, n.r, Probed()), prepare(eng, n.s, Driven()))

# Pipeline-breakers (Folds/Scalar): the cache is always needed (both modes), so
# `prepare` builds it eagerly into a concrete physical result — no mode split.
prepare(eng::Engine, n::Fold, ::Mode) =
    build_fold(eng, prepare(eng, n.q, Driven()), n.op, n.init)
prepare(eng::Engine, n::DenseFold, ::Mode) =
    build_densefold(eng, prepare(eng, n.q, Driven()), n.op, n.init, n.n)
prepare(eng::Engine, n::BufFold{D,R,S,Q,F}, ::Mode) where {D,R,S,Q,F} =
    build_buffold(eng, prepare(eng, n.q, Driven()), n.f, S)
prepare(eng::Engine, n::Scalar, ::Mode) =
    build_scalar(eng, prepare(eng, n.q, Driven()), n.op, n.init)

# Materialized / MatSet: split by mode — driven → stored stream; probed →
# concrete index / membership Set.
prepare(eng::Engine, n::Materialized, ::Driven) =
    build_mat_stream(eng, prepare(eng, n.a, Driven()))
prepare(eng::Engine, n::Materialized, ::Probed) =
    build_mat_probed(eng, prepare(eng, n.a, Driven()))
prepare(eng::Engine, n::MatSet, ::Driven) =
    build_matset_keys(eng, prepare(eng, n.a, Driven()))
prepare(eng::Engine, n::MatSet, ::Probed) =
    build_matset_set(eng, prepare(eng, n.a, Driven()))

# UnaryVec: driven → iterate keys (identity); probed → concrete membership Set.
prepare(::Engine, n::UnaryVec, ::Driven) = n
prepare(::Engine, n::UnaryVec{D}, ::Probed) where {D} = MatSetProbed{D}(Set(n.values))

# BitsetMat → a dense `Bitset` membership, built by driving the inner once.
prepare(eng::Engine, n::BitsetMat, ::Mode) =
    build_bitset(eng, prepare(eng, n.q, Driven()), n.n)

# Leaves / sources (and already-physical nodes): identity.
prepare(::Engine, n::Union{VecRel,SparseRel,MultiRel,MapRel,Universe,Bitset,
                 InvStream,LCStream,Indexed,
                 FoldP,DenseFoldP,ScalarP,MatSetProbed}, ::Mode) = n

# ===== terminals ========================================================
# Queries are consumed by `scan` with a folding continuation (see `_vals` in
# queries.jl) — no result relation is ever built. `collect` is the convenience
# terminal for the REPL: run a query into a concrete Rel.

# `unwrap(q::Query{Nothing, S}) → S` — eliminator for the one-row container
# `⊵` (and `↦` on it) produces. Runs once, returns the single value as a
# plain Julia scalar. Useful for scalar-subquery escapes: e.g.
# `threshold = 0.0001 * unwrap(value_per_part ⊵ (+, 0.0))`.
function unwrap(q::Query{Nothing, S}, eng::Engine = Staged()) where {S}
    r = Ref{S}()
    scan(eng, prepare(eng, q), (_, v) -> r[] = v)
    r[]
end
export unwrap

# `collect` is exactly "prepare, then materialize the driven plan": a stored
# pair stream (`MapRel`) for a Query, a stored key set (`UnaryVec`) for a Unary.
Base.collect(q::Query, eng::Engine = Staged()) = build_mat_stream(eng, prepare(eng, q))
Base.collect(s::Unary, eng::Engine = Staged()) = build_matset_keys(eng, prepare(eng, s))
