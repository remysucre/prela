# The interpreted engine: value-level CPS.
#
# `drive`/`probe`/`probe_any`/`member` fuse a physical plan into a loop nest
# through closure continuations and Julia's inlining. This is the executable
# spec the staged engine (staged.jl) mirrors expression-by-expression — and
# the subject of the compiler-inlining experiments: deep plans exceed the
# inference recursion limit (the continuation type grows per level), at which
# point calls stop inlining. See experiment_recursion.jl.

# Uniform per-key access over either index representation (dense fwd / Dict).
@inline function _idx_probe(idx::Vector{Vector{R}}, x::ID, k) where {R}
    i = x.id
    (1 <= i <= length(idx)) || return
    @inbounds vs = idx[i]
    for y in vs
        k(y)
    end
end
@inline function _idx_probe(idx::Dict, x, k)
    vs = get(idx, x, nothing)
    vs === nothing && return
    for y in vs
        k(y)
    end
end
@inline function _idx_probe_any(idx::Vector{Vector{R}}, x::ID, k) where {R}
    i = x.id
    (1 <= i <= length(idx)) || return false
    @inbounds vs = idx[i]
    for y in vs
        k(y) && return true
    end
    false
end
@inline function _idx_probe_any(idx::Dict, x, k)
    vs = get(idx, x, nothing)
    vs === nothing && return false
    for y in vs
        k(y) && return true
    end
    false
end
# ===== CPS execution protocol ===========================================
# drive(q,k): k(x,y) per pair    probe(q,x,k): k(y) per value at x
# member(s,x)::Bool — domain/membership test

# ---- leaves ----
# `MapRel` — drive-only materialized result (collect / inlined pairs). No probe:
# a scanned result is never probed; `materialize` it if you need probe-many.
@inline function drive(r::MapRel, k)
    for p in r.pairs
        k(p.first, p.second)
    end
end

# `VecRel` — dense 1:1 column store. drive iterates 1..n; probe is a
# bounds-checked array load (an id outside 1..n simply emits nothing — a leaf
# may be probed at an id from another table that doesn't exist here).
@inline function drive(r::VecRel{E, R}, k) where {E, R}
    v = r.values
    @inbounds for i in eachindex(v)
        k(ID{E}(i), v[i])
    end
end
@inline function probe(r::VecRel{E, R}, x::ID{E}, k) where {E, R}
    v = r.values; i = x.id
    @inbounds (1 <= i <= length(v)) && (k(v[i]); nothing)
end

# `SparseRel` — dense values + presence map. drive skips unseen; probe checks.
@inline function drive(r::SparseRel{E, R}, k) where {E, R}
    v = r.values; s = r.seen
    @inbounds for i in eachindex(v)
        s[i] && k(ID{E}(i), v[i])
    end
end
@inline function probe(r::SparseRel{E, R}, x::ID{E}, k) where {E, R}
    v = r.values; i = x.id
    @inbounds (1 <= i <= length(v) && r.seen[i]) && (k(v[i]); nothing)
end

# `MultiRel` — dense forward index. drive iterates the nest; probe indexes it.
@inline function drive(r::MultiRel{E, R}, k) where {E, R}
    f = r.fwd
    @inbounds for i in eachindex(f)
        for y in f[i]
            k(ID{E}(i), y)
        end
    end
end
@inline function probe(r::MultiRel{E, R}, x::ID{E}, k) where {E, R}
    f = r.fwd; i = x.id
    (1 <= i <= length(f)) || return
    @inbounds for y in f[i]
        k(y)
    end
end

# ---- Compose: the loop nest ----
@inline drive(n::Compose, k) = drive(n.a, (x, m) -> probe(n.b, m, r -> k(x, r)))
@inline probe(n::Compose, x, k) = probe(n.a, x, m -> probe(n.b, m, r -> k(r)))

# ---- Filter ----
# Driving a Filter is a streaming filtered scan: drive the inner, keep rows
# whose value passes the predicate. Used for top-level result filtering /
# HAVING (e.g. `value_per_part > threshold`, `revenue == max_rev`), where the
# inner is itself a driven source (a Fold) with no leaf to probe. There is no
# value-jump / inverse-index path: a predicate is always either probed into
# (the common case) or streamed over — never seek-by-value.
@inline drive(n::Filter{D,R,A,<:FnP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> n.pred.f(y) && k(x, y))
@inline drive(n::Filter{D,R,A,<:EqP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> isequal(y, n.pred.v) && k(x, y))
@inline drive(n::Filter{D,R,A,<:InP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> (y in n.pred.vs) && k(x, y))
@inline probe(n::Filter{D,R,A,<:FnP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> n.pred.f(y) && k(y))
@inline probe(n::Filter{D,R,A,<:EqP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> isequal(y, n.pred.v) && k(y))
@inline probe(n::Filter{D,R,A,<:InP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> (y in n.pred.vs) && k(y))

# ---- Restrict (a : b) — drive a, keep rows whose value is a member of b ----
@inline drive(n::Restrict, k) =
    drive(n.a, (x, m) -> member(n.b, m) && k(x, m))
@inline probe(n::Restrict, x, k) =
    probe(n.a, x, m -> member(n.b, m) && k(m))
@inline probe_any(n::Restrict, x, k) =
    probe_any(n.a, x, m -> member(n.b, m) && k(m))

# ---- Diff (a:Query - b:predicate) ----
@inline drive(n::Diff, k) =
    drive(n.a, (x, y) -> member(n.b, x) || k(x, y))
@inline probe(n::Diff, x, k) =
    member(n.b, x) || probe(n.a, x, k)

# ---- Prod (n-ary ×) ----
# Generated drive/probe — per-arity unroll. The previous recursive `_pp`
# (`probe(ops[1], x, y -> _pp(tail(ops), x, (acc..., y), k))`) wouldn't
# unroll at compile time, so each level built a closure capture on the
# growing `acc` tuple. The result was ~3 heap allocations per produced
# row (visible in `Profile.Allocs` as the `_pp` closure). A `@generated`
# function emits a flat nest specialized to the concrete tuple length,
# so the closure chain is just N straight-line `probe(..., y -> probe(...))`
# calls — fully inlinable, no recursion.
# `@generated` bodies must be pure — Julia checks for allocations/closures
# in the generator itself, not the returned AST. We build the per-arity AST
# in a helper called from a normal function, then `@eval` per-arity
# specializations at module-load time. The same effect as @generated.
_prod_yvar(i::Int) = Symbol("y_", i)
function _prod_probe_body(N::Int)
    yvars = ntuple(_prod_yvar, N)
    body = Expr(:call, :k, Expr(:tuple, yvars...))
    for i in N:-1:1
        body = Expr(:call, :probe, :(ops[$i]), :x, Expr(:->, yvars[i], body))
    end
    body
end
function _prod_drive_body(N::Int)
    yvars = ntuple(_prod_yvar, N)
    body = Expr(:call, :k, :x, Expr(:tuple, yvars...))
    for i in N:-1:2
        body = Expr(:call, :probe, :(ops[$i]), :x, Expr(:->, yvars[i], body))
    end
    body = Expr(:call, :drive, :(ops[1]),
                Expr(:->, Expr(:tuple, :x, yvars[1]), body))
    body
end
# `probe_any` for Prod — nested probe_any chain that short-circuits each leg
# and threads the real tuple `(y_1, …, y_N)` to `k` at the bottom. Needed by
# tuple-bearing callers like `Filter(Prod, FnP)` (cross-column compares like
# `commitdate < receiptdate`).
function _prod_probe_any_body(N::Int)
    yvars = ntuple(_prod_yvar, N)
    body = Expr(:call, :k, Expr(:tuple, yvars...))
    for i in N:-1:1
        body = Expr(:call, :probe_any, :(ops[$i]), :x, Expr(:->, yvars[i], body))
    end
    body
end
# `member` for Prod — flat short-circuit AND of per-leg `member` calls.
# This is the conj-use fast path: `lineitem : (f1 ∧ f2 ∧ f3)` calls
# `member(Prod, x)` per row (from `Restrict`), which routes here. No tuple is
# built and the closures are stateless, matching the flat shape of the old
# Conj's probe_any. The tuple-bearing `probe_any(::Prod)` above stays available
# for the non-trivial-k cases (FnP destructuring etc.).
function _prod_member_body(N::Int)
    body = true
    for i in N:-1:1
        body = Expr(:&&, Expr(:call, :member, :(ops[$i]), :x), body)
    end
    body
end
# Emit per-arity methods up to N=8 (Q1 has 4, Q2 has 6, no TPCH query is wider).
for N in 1:8
    @eval @inline _prod_probe(ops::NTuple{$N, Any}, x, k) = $(_prod_probe_body(N))
    @eval @inline _prod_drive(ops::NTuple{$N, Any}, k)    = $(_prod_drive_body(N))
    @eval @inline _prod_probe_any(ops::NTuple{$N, Any}, x, k) = $(_prod_probe_any_body(N))
    @eval @inline _prod_member(ops::NTuple{$N, Any}, x) = $(_prod_member_body(N))
end
@inline probe(n::Prod, x, k) = _prod_probe(n.ops, x, k)
@inline drive(n::Prod, k)    = _prod_drive(n.ops, k)
@inline probe_any(n::Prod, x, k) = _prod_probe_any(n.ops, x, k)

# ---- Materialized: `prepare` lowers to MatStream (driven) / MatProbed (probed) ----
# `_mat_idx` builds a materialized result's forward index: dense `Vector{Vector}`
# when entity-keyed (array access, no hashing — reuses `_dense_fwd`), else a Dict.
_mat_idx(pairs::Vector{Pair{ID{E}, R}}) where {E, R} = _dense_fwd(pairs)
function _mat_idx(pairs::Vector{Pair{D, R}}) where {D, R}
    d = Dict{D, Vector{R}}()
    for p in pairs
        push!(get!(() -> R[], d, p.first), p.second)
    end
    d
end
@inline function drive(n::MatStream, k)
    for p in n.mat
        k(p.first, p.second)
    end
end
@inline probe(n::MatProbed, x, k) = _idx_probe(n.idx, x, k)
@inline probe_any(n::MatProbed, x, k) = _idx_probe_any(n.idx, x, k)

# ---- Inv: `prepare` lowers to InvStream (driven) / InvIndexed (probed) ----
# drive-only stream vs probe-only concrete index — `Inv` itself is never run.
@inline drive(n::InvStream, k) = drive(n.q, (a, b) -> k(b, a))
@inline function probe(n::InvIndexed{B, A}, b, k) where {B, A}
    vs = get(n.idx, b, nothing)
    vs === nothing && return
    for a in vs; k(a); end
end
@inline probe_any(n::InvIndexed, b, k) = _idx_probe_any(n.idx, b, k)

# ---- LeftCompose: `prepare` lowers to LCStream (driven) / LCIndexed (probed) ----
# `r ← s` drives `s` (the natural source) and probes `r` per row to compute the
# group key. Driven → streaming; probed → a concrete `Dict{RK, Vector{SV}}`.
@inline drive(n::LCStream, k) = drive(n.s, (d, v) -> probe(n.r, d, rk -> k(rk, v)))
@inline function probe(n::LCIndexed, rk, k)
    vs = get(n.idx, rk, nothing)
    vs === nothing && return
    for v in vs; k(v); end
end
@inline probe_any(n::LCIndexed, rk, k) = _idx_probe_any(n.idx, rk, k)
# ---- LeftConj: drive r (ignoring its value), member-check materialized l ----
@inline drive(n::LeftConj, k) =
    drive(n.r, (x, _) -> member(n.l, x) && k(x, x))
@inline probe(n::LeftConj, x, k) =
    member(n.l, x) && member(n.r, x) && (k(x); nothing)
@inline probe_any(n::LeftConj, x, k) =
    member(n.l, x) && member(n.r, x) && k(x)

# ---- FoldP: prepared per-key group cache (Fold + BufFold). drive iterates,
# probe looks up. The cache is built eagerly in `prepare`.
@inline function drive(n::FoldP, k)
    for (d, s) in n.cache
        k(d, s)
    end
end
@inline function probe(n::FoldP, d, k)
    s = get(n.cache, d, nothing)
    s === nothing && return
    k(s)
end
@inline probe_any(n::FoldP, d, k) =
    (s = get(n.cache, d, nothing); s === nothing ? false : k(s))

# ---- DenseFoldP: prepared dense per-key fold over `0..n` ----
@inline function drive(n::DenseFoldP{D, S}, k) where {D, S}
    vals, seen = n.vals, n.seen
    @inbounds for i in eachindex(vals)
        seen[i] && k(_densebox(D, i - 1), vals[i])
    end
end
@inline function probe(n::DenseFoldP{D, S}, d, k) where {D, S}
    vals, seen = n.vals, n.seen
    i = _denseidx(d) + 1
    @inbounds if 1 <= i <= length(vals) && seen[i]
        k(vals[i])
    end
end
@inline function probe_any(n::DenseFoldP{D, S}, d, k) where {D, S}
    vals, seen = n.vals, n.seen
    i = _denseidx(d) + 1
    @inbounds (1 <= i <= length(vals) && seen[i]) ? k(vals[i]) : false
end

# ---- Map: per-row lambda ----
@inline drive(n::Map, k) = drive(n.q, (d, v) -> k(d, n.f(v)))
@inline probe(n::Map, d, k) = probe(n.q, d, v -> k(n.f(v)))

# ---- ScalarP: prepared no-group fold result (a single value) ----
@inline drive(n::ScalarP, k) = k(nothing, n.value)
@inline probe(n::ScalarP, ::Nothing, k) = k(n.value)

# MatSet prepared forms: drive-only stored keys vs probe-only membership Set.
@inline drive(n::MatSetStream, k) = (for x in n.keys; k(x, x); end)
@inline probe(n::MatSetProbed, x, k) = (x in n.set) && (k(x); nothing)
@inline probe_any(n::MatSetProbed, x, k) = (x in n.set) && k(x)

# ---- identity-leaf relations: drive emits `(x, x)`, probe yields `x`. ----
# These are the binary identity form of the former Unary leaves — the value
# side equals the key, and `member` is just `probe_any`.

# `UnaryVec` is a stored key set. Driven → iterate (kept as-is); probed → a
# concrete membership `Set` (lowered to `MatSetProbed` by `prepare`), so there's
# no global lazy membership cache.
@inline drive(u::UnaryVec{D}, k) where {D} = (for v in u.values; k(v, v); end)

@inline drive(u::Universe{E}, k) where {E} =
    (for i in 1:u.n; let id = ID{E}(i); k(id, id); end; end)
@inline probe(u::Universe{E}, x::ID{E}, k) where {E} =
    (1 <= x.id <= u.n) && (k(x); nothing)
@inline probe_any(u::Universe{E}, x::ID{E}, k) where {E} =
    (1 <= x.id <= u.n) && k(x)

# ---- Bitset: BitVector-backed dense identity Unary{D}; member is one bit-test ----
@inline function drive(b::Bitset{D}, k) where {D}
    @inbounds for i in eachindex(b.bits)
        if b.bits[i]
            d = _densebox(D, i - 1)
            k(d, d)
        end
    end
end
@inline function probe(b::Bitset{D}, x, k) where {D}
    i = _denseidx(x) + 1
    @inbounds (1 <= i <= length(b.bits) && b.bits[i]) && (k(x); nothing)
end
@inline function probe_any(b::Bitset{D}, x, k) where {D}
    i = _denseidx(x) + 1
    @inbounds (1 <= i <= length(b.bits) && b.bits[i]) && k(x)
end

# `∨`/`Disj` is a *membership* union (probe-only): `member(l ∨ r, x)` is just
# `member(l, x) || member(r, x)` — order-free, no dedup, and the two sides need
# not share an element type (it's a predicate over the coproduct domain). It is
# never enumerated: driving a union (dedup-while-emitting) is the one operation
# that would need its lhs both driven and probed, so it lives elsewhere — use
# `Union` (bag-concat of same-typed rels) to enumerate.
drive(::Disj, k) = error("∨ (Disj) is a probe-only membership union; " *
                         "use `Union` to enumerate a union")
@inline probe(s::Disj{D}, x, k) where {D} =
    (member(s.a, x) || member(s.b, x)) && (k(x); nothing)
@inline probe_any(s::Disj{D}, x, k) where {D} =
    (member(s.a, x) || member(s.b, x)) && k(x)

# `probe_any(q, x, k)` — like `probe`, but the continuation returns a Bool and
# `probe_any` stops, returning true, as soon as `k` does. The Bool is threaded
# through return values (no mutable cell) so the whole chain is allocation-free
# when inlined — this is the hot path for `member` on a driven stream.
@inline function probe_any(r::VecRel{E, R}, x::ID{E}, k) where {E, R}
    v = r.values; i = x.id
    @inbounds (1 <= i <= length(v)) ? k(v[i]) : false
end
@inline function probe_any(r::SparseRel{E, R}, x::ID{E}, k) where {E, R}
    v = r.values; i = x.id
    @inbounds (1 <= i <= length(v) && r.seen[i]) ? k(v[i]) : false
end
@inline function probe_any(r::MultiRel{E, R}, x::ID{E}, k) where {E, R}
    f = r.fwd; i = x.id
    (1 <= i <= length(f)) || return false
    @inbounds for y in f[i]
        k(y) && return true
    end
    false
end
@inline probe_any(n::Compose, x, k) = probe_any(n.a, x, m -> probe_any(n.b, m, k))
@inline probe_any(n::Filter{D,R,A,<:FnP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> n.pred.f(y) && k(y))
@inline probe_any(n::Filter{D,R,A,<:EqP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> isequal(y, n.pred.v) && k(y))
@inline probe_any(n::Filter{D,R,A,<:InP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> (y in n.pred.vs) && k(y))
@inline probe_any(n::Diff, x, k) =
    (!member(n.b, x)) && probe_any(n.a, x, k)
# generic fallback (Prod and other shapes) — no early exit, rarely on hot paths
function probe_any(q::Query, x, k)
    found = Ref(false)
    probe(q, x, y -> (k(y) && (found[] = true)))
    found[]
end

# member of a Query = "is x in its domain".
@inline member(q::Query, x) = probe_any(q, x, _ -> true)
# Fast path: `Prod` short-circuits flat across its legs without ever building
# the tuple value (vs the generic `probe_any` which threads the tuple).
@inline member(n::Prod, x) = _prod_member(n.ops, x)
