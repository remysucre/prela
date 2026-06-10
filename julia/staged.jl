# The staged engine: type-level CPS.
#
# `_exec_drive` is `@generated` over the *physical* plan's type: the emitters
# below walk that type at codegen time and emit one flat, fused loop nest.
# Mode is already resolved into the types (`prepare` lowered every index/cache
# node to a concrete physical struct — `InvStream`/`Indexed`,
# `FoldP`/`DenseFoldP`, `MapRel`/`Indexed`, …), so the emitters read
# concrete fields and never build anything: all state construction happened in
# `prepare`, which itself scans through this same engine (see plan.jl).
#
# The emitters are the type-level mirror of the value-level `drive`/`probe`
# spec in interp.jl: the `body` Expr is the continuation, threaded over the
# plan *type* the way `k` is threaded over the *value*. This is why deep plans
# compile flat here while the interpreted engine trips the inference recursion
# limit — the continuation grows at codegen time, where growth is free.
#
# Structure: leaves describe their probe-side access ONCE via `_access` /
# `_access_multi` descriptors; the three probe protocols (`_probe_body`,
# `_probe_body_test`, `_probe_any_body`) are derived from the descriptor by
# generic templates. Only structural nodes (Compose, Filter, …) and the
# drive protocol (where iteration shapes genuinely differ) have bespoke
# emitters.

# =====================================================================
# _drive_body(QType, q_expr, body) -> Expr
#   Per row, bind `_x` (key) and `_v` (value), then run `body`.
# =====================================================================

# ---- leaves ----
function _drive_body(::Type{Universe{E}}, q_expr, body) where E
    quote
        @inbounds for _i in 1:($q_expr).n
            let _x = ID{$E}(_i), _v = _x
                $body
            end
        end
    end
end

function _drive_body(::Type{<:VecRel{E, R}}, q_expr, body) where {E, R}
    quote
        let _vals = ($q_expr).values
            @inbounds for _i in eachindex(_vals)
                let _x = ID{$E}(_i), _v = _vals[_i]
                    $body
                end
            end
        end
    end
end

function _drive_body(::Type{<:SparseRel{E, R}}, q_expr, body) where {E, R}
    quote
        let _vals = ($q_expr).values, _seen = ($q_expr).seen
            @inbounds for _i in eachindex(_vals)
                if _seen[_i]
                    let _x = ID{$E}(_i), _v = _vals[_i]
                        $body
                    end
                end
            end
        end
    end
end

function _drive_body(::Type{<:MultiRel{E, R}}, q_expr, body) where {E, R}
    quote
        let _fwd = ($q_expr).fwd
            @inbounds for _i in eachindex(_fwd)
                for _y in _fwd[_i]
                    let _x = ID{$E}(_i), _v = _y
                        $body
                    end
                end
            end
        end
    end
end

# `MapRel` — drive-only materialized result (collect / materialize / inlined
# pairs).
function _drive_body(::Type{<:MapRel{D, R}}, q_expr, body) where {D, R}
    quote
        for _p in ($q_expr).pairs
            let _x = _p.first, _v = _p.second
                $body
            end
        end
    end
end

# `UnaryVec` — stored key set, driven as identity (`_v = _x`).
function _drive_body(::Type{<:UnaryVec{D}}, q_expr, body) where {D}
    quote
        for _x in ($q_expr).values
            let _v = _x
                $body
            end
        end
    end
end

# `Bitset` — iterate set bits; box the dense int back to D.
function _drive_body(::Type{<:Bitset{D}}, q_expr, body) where {D}
    quote
        let _b = $q_expr
            @inbounds for _i in eachindex(_b.bits)
                if _b.bits[_i]
                    let _x = _densebox($D, _i - 1), _v = _x
                        $body
                    end
                end
            end
        end
    end
end

# ---- structural ----
# Compose drive: drive(a, (x, m) -> probe(b, m, r -> body))
function _drive_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, body) where {D, M, R, A, B}
    m_sym = gensym(:_m)
    inner = _probe_body(B, :(($q_expr).b), m_sym, :_v, body)
    inner_with_m = quote
        let $m_sym = _v
            $inner
        end
    end
    _drive_body(A, :(($q_expr).a), inner_with_m)
end

# Filter drive — streaming filtered scan; the predicate is any callable.
function _drive_body(::Type{<:Filter{D, R, A}}, q_expr, body) where {D, R, A}
    wrap = quote
        if ($q_expr).pred(_v)
            $body
        end
    end
    _drive_body(A, :(($q_expr).a), wrap)
end

# Restrict drive: drive a, keep rows whose value is a member of b.
function _drive_body(::Type{<:Restrict{D, R, A, B}}, q_expr, body) where {D, R, A, B}
    chk = _probe_any_body(B, :(($q_expr).b), :_v)
    wrap = quote
        if $chk
            $body
        end
    end
    _drive_body(A, :(($q_expr).a), wrap)
end

# Diff drive (value-bearing): drive a, exclude keys in b.
function _drive_body(::Type{<:Diff{D, R, A, B}}, q_expr, body) where {D, R, A, B}
    bchk = _probe_any_body(B, :(($q_expr).b), :_x)
    wrap = quote
        if !($bchk)
            $body
        end
    end
    _drive_body(A, :(($q_expr).a), wrap)
end

# Prod drive: drive ops[1], probe ops[2..N], emit (x, tuple).
function _drive_body(::Type{<:Prod{D, R, OPS}}, q_expr, body) where {D, R, OPS}
    op_types = OPS.parameters
    N = length(op_types)
    sub_ys = Symbol[gensym(:_yp) for _ in 1:N]
    inner = quote
        let _v = ($(sub_ys...),)
            $body
        end
    end
    for i in N:-1:2
        inner = _probe_body(op_types[i], :(($q_expr).ops[$i]), :_x, sub_ys[i], inner)
    end
    outer = quote
        let $(sub_ys[1]) = _v
            $inner
        end
    end
    _drive_body(op_types[1], :(($q_expr).ops[1]), outer)
end

# Map drive: drive(inner, (d, v) -> body with _v = f(v)).
function _drive_body(::Type{<:Map{D, R, S, Q, F}}, q_expr, body) where {D, R, S, Q, F}
    inner_v = gensym(:_vm)
    wrapped = quote
        let $inner_v = _v
            let _v = ($q_expr).f($inner_v)
                $body
            end
        end
    end
    _drive_body(Q, :(($q_expr).q), wrapped)
end

# LeftConj drive (identity Unary): drive r, member-check l per row.
function _drive_body(::Type{<:LeftConj{D, ML, R}}, q_expr, body) where {D, ML, R}
    pred = _probe_any_body(ML, :(($q_expr).l), :_x)
    wrap = quote
        if $pred
            $body
        end
    end
    _drive_body(R, :(($q_expr).r), wrap)
end

# ---- prepared streams (driven side; pure wrappers / stored streams) ----
# InvStream drive: flip the inner's pairs.
function _drive_body(::Type{<:InvStream{B, A, Q}}, q_expr, body) where {B, A, Q}
    swap = gensym(:_inv)
    flip = quote
        let $swap = _x, _x = _v, _v = $swap
            $body
        end
    end
    _drive_body(Q, :(($q_expr).q), flip)
end

# LCStream drive: drive s, probe r per row → emit (rk, sv).
function _drive_body(::Type{<:LCStream{D, RK, SV, QR, QS}}, q_expr, body) where {D, RK, SV, QR, QS}
    sv_save = gensym(:_sv)
    orig_x  = gensym(:_xs)
    rk_sym  = gensym(:_rk)
    body_with_rk = quote
        let _x = $rk_sym, _v = $sv_save
            $body
        end
    end
    probe_expr = _probe_body(QR, :(($q_expr).r), orig_x, rk_sym, body_with_rk)
    inner = quote
        let $orig_x = _x, $sv_save = _v
            $probe_expr
        end
    end
    _drive_body(QS, :(($q_expr).s), inner)
end

# ---- prepared caches (pipeline-breakers; built by `prepare`) ----
# FoldP drive: iterate the per-key cache.
function _drive_body(::Type{<:FoldP{D, S}}, q_expr, body) where {D, S}
    quote
        for (_x, _v) in ($q_expr).cache
            $body
        end
    end
end

# DenseFoldP drive: iterate present slots, boxing the dense int back to D.
function _drive_body(::Type{<:DenseFoldP{D, S}}, q_expr, body) where {D, S}
    quote
        let _vals = ($q_expr).vals, _seen = ($q_expr).seen
            @inbounds for _i in eachindex(_vals)
                if _seen[_i]
                    let _x = _densebox($D, _i - 1), _v = _vals[_i]
                        $body
                    end
                end
            end
        end
    end
end

# ScalarP drive: emit the single (nothing, value) row.
function _drive_body(::Type{<:ScalarP{S}}, q_expr, body) where {S}
    quote
        let _x = nothing, _v = ($q_expr).value
            $body
        end
    end
end

# Fallback: any unhandled type. The lambda in the emitted AST trips the
# @generated purity check, so this is effectively a loud error path — add an
# explicit emitter for any type that reaches it.
function _drive_body(::Type{Q}, q_expr, body) where {Q}
    quote
        drive($q_expr, (_x, _v) -> $body)
    end
end

# =====================================================================
# Leaf access descriptors. Each probe-able physical node describes its
# access exactly once:
#
#   _access(T, q_expr, x_sym)       -> (setup, cond, val)    single-valued
#   _access_multi(T, q_expr, x_sym) -> (setup, cond, iter)   vector-valued
#
# `setup` is a vector of `lhs = rhs` binding Exprs (wrapped in a `let` when
# nonempty); `cond` is a Bool Expr guarding presence; `val` / `iter` is the
# value / iterable Expr. The generic templates below derive all three probe
# protocols from the descriptor, so a new physical node needs one method
# here instead of three hand-written emitters.
# =====================================================================

const SingleValued = Union{VecRel, SparseRel, Universe, Bitset, MatSetProbed,
                           FoldP, DenseFoldP, ScalarP, Disj, LeftConj}
const MultiValued  = Union{MultiRel, Indexed}

_let(setup, body) = isempty(setup) ? body : Expr(:let, Expr(:block, setup...), body)

_access(::Type{<:VecRel}, q, x) = (
    [:(_i = ($x).id), :(_vv = ($q).values)],
    :(1 <= _i <= length(_vv)),
    :(@inbounds _vv[_i]))
_access(::Type{<:SparseRel}, q, x) = (
    [:(_i = ($x).id), :(_vv = ($q).values)],
    :(1 <= _i <= length(_vv) && @inbounds(($q).seen[_i])),
    :(@inbounds _vv[_i]))
_access(::Type{<:Universe}, q, x) = (
    Expr[], :(1 <= ($x).id <= ($q).n), x)
_access(::Type{<:Bitset}, q, x) = (
    [:(_b = $q), :(_i = _denseidx($x) + 1)],
    :(@inbounds(1 <= _i <= length(_b.bits) && _b.bits[_i])),
    x)
_access(::Type{<:MatSetProbed}, q, x) = (
    Expr[], :($x in ($q).set), x)
_access(::Type{<:FoldP}, q, x) = (
    [:(_g = get(($q).cache, $x, nothing))], :(_g !== nothing), :_g)
_access(::Type{<:DenseFoldP}, q, x) = (
    [:(_vals = ($q).vals), :(_i = _denseidx($x) + 1)],
    :(1 <= _i <= length(_vals) && @inbounds(($q).seen[_i])),
    :(@inbounds _vals[_i]))
_access(::Type{<:ScalarP}, q, x) = (Expr[], true, :(($q).value))

# Conditional identity nodes: yield x iff member (cond recurses into legs).
_access(::Type{<:Disj{D, A, B}}, q, x) where {D, A, B} = (
    Expr[],
    :($(_probe_any_body(A, :(($q).a), x)) || $(_probe_any_body(B, :(($q).b), x))),
    x)
_access(::Type{<:LeftConj{D, ML, R}}, q, x) where {D, ML, R} = (
    Expr[],
    :($(_probe_any_body(ML, :(($q).l), x)) && $(_probe_any_body(R, :(($q).r), x))),
    x)

_access_multi(::Type{<:MultiRel}, q, x) = (
    [:(_i = ($x).id), :(_f = ($q).fwd)],
    :(1 <= _i <= length(_f)),
    :(@inbounds _f[_i]))
# `Indexed` — dense `Vector{Vector}` (entity-keyed) or `Dict`; branch on IDX.
function _access_multi(::Type{<:Indexed{D, R, IDX}}, q, x) where {D, R, IDX}
    if IDX <: AbstractVector
        ([:(_i = ($x).id), :(_idx = ($q).idx)],
         :(1 <= _i <= length(_idx)),
         :(@inbounds _idx[_i]))
    else
        ([:(_vs = get(($q).idx, $x, nothing))], :(_vs !== nothing), :_vs)
    end
end

# =====================================================================
# _probe_body(QType, q_expr, x_sym, y_sym, body) -> Expr
#   Probe Q at x_sym, bind the value to y_sym, run body (a statement).
# =====================================================================

# ---- leaves, derived from the access descriptors ----
function _probe_body(::Type{Q}, q_expr, x_sym, y_sym, body) where {Q <: SingleValued}
    setup, cond, val = _access(Q, q_expr, x_sym)
    _let(setup, quote
        if $cond
            let $y_sym = $val
                $body
            end
        end
    end)
end

function _probe_body(::Type{Q}, q_expr, x_sym, y_sym, body) where {Q <: MultiValued}
    setup, cond, iter = _access_multi(Q, q_expr, x_sym)
    _let(setup, quote
        if $cond
            for $y_sym in $iter
                $body
            end
        end
    end)
end

# ---- structural ----
function _probe_body(::Type{<:Filter{D, R, A}}, q_expr, x_sym, y_sym, body) where {D, R, A}
    wrap = quote
        if ($q_expr).pred($y_sym)
            $body
        end
    end
    _probe_body(A, :(($q_expr).a), x_sym, y_sym, wrap)
end

# Restrict probe: probe a, keep value if member of b.
function _probe_body(::Type{<:Restrict{D, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, R, A, B}
    chk = _probe_any_body(B, :(($q_expr).b), y_sym)
    wrap = quote
        if $chk
            $body
        end
    end
    _probe_body(A, :(($q_expr).a), x_sym, y_sym, wrap)
end

# Diff probe: skip if key in b, else probe a.
function _probe_body(::Type{<:Diff{D, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, R, A, B}
    bchk = _probe_any_body(B, :(($q_expr).b), x_sym)
    inner = _probe_body(A, :(($q_expr).a), x_sym, y_sym, body)
    quote
        if !($bchk)
            $inner
        end
    end
end

# Compose probe: probe(a, x, m -> probe(b, m, y -> body)).
function _probe_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, M, R, A, B}
    m_sym = gensym(:_mp)
    inner = _probe_body(B, :(($q_expr).b), m_sym, y_sym, body)
    _probe_body(A, :(($q_expr).a), x_sym, m_sym, inner)
end

function _probe_body(::Type{<:Map{D, R, S, Q, F}}, q_expr, x_sym, y_sym, body) where {D, R, S, Q, F}
    inner_y = gensym(:_ym)
    inner_body = quote
        let $y_sym = ($q_expr).f($inner_y)
            $body
        end
    end
    _probe_body(Q, :(($q_expr).q), x_sym, inner_y, inner_body)
end

function _probe_body(::Type{<:Prod{D, R, OPS}}, q_expr, x_sym, y_sym, body) where {D, R, OPS}
    op_types = OPS.parameters
    N = length(op_types)
    sub_ys = Symbol[gensym(:_yp) for _ in 1:N]
    inner = quote
        let $y_sym = ($(sub_ys...),)
            $body
        end
    end
    for i in N:-1:1
        inner = _probe_body(op_types[i], :(($q_expr).ops[$i]), x_sym, sub_ys[i], inner)
    end
    inner
end

function _probe_body(::Type{Q}, q_expr, x_sym, y_sym, body) where {Q}
    quote
        probe($q_expr, $x_sym, $y_sym -> $body)
    end
end

# =====================================================================
# _probe_body_test(QType, q_expr, x_sym, y_sym, body) -> Expr (Bool)
#   Like _probe_body but `body` is a Bool; returns true iff some matched y
#   makes body true. Used by probe_any chains carrying a value predicate.
#   No body duplication for multi-valued probes.
# =====================================================================

# ---- leaves, derived from the access descriptors ----
function _probe_body_test(::Type{Q}, q_expr, x_sym, y_sym, body) where {Q <: SingleValued}
    setup, cond, val = _access(Q, q_expr, x_sym)
    _let(setup, quote
        ($cond) ? (let $y_sym = $val; $body; end) : false
    end)
end

function _probe_body_test(::Type{Q}, q_expr, x_sym, y_sym, body) where {Q <: MultiValued}
    setup, cond, iter = _access_multi(Q, q_expr, x_sym)
    _let(setup, quote
        let _acc = false
            if $cond
                for $y_sym in $iter
                    _acc = ($body) || _acc
                end
            end
            _acc
        end
    end)
end

# ---- structural ----
function _probe_body_test(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, M, R, A, B}
    m_sym = gensym(:_mpt)
    inner = _probe_body_test(B, :(($q_expr).b), m_sym, y_sym, body)
    _probe_body_test(A, :(($q_expr).a), x_sym, m_sym, inner)
end
function _probe_body_test(::Type{<:Map{D, R, S, Q, F}}, q_expr, x_sym, y_sym, body) where {D, R, S, Q, F}
    inner_y = gensym(:_ymt)
    inner_body = quote
        let $y_sym = ($q_expr).f($inner_y)
            $body
        end
    end
    _probe_body_test(Q, :(($q_expr).q), x_sym, inner_y, inner_body)
end
function _probe_body_test(::Type{<:Filter{D, R, A}}, q_expr, x_sym, y_sym, body) where {D, R, A}
    wrap = quote
        ($q_expr).pred($y_sym) ? ($body) : false
    end
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Restrict{D, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, R, A, B}
    chk = _probe_any_body(B, :(($q_expr).b), y_sym)
    wrap = quote
        ($chk) ? ($body) : false
    end
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Prod{D, R, OPS}}, q_expr, x_sym, y_sym, body) where {D, R, OPS}
    op_types = OPS.parameters
    N = length(op_types)
    sub_ys = Symbol[gensym(:_ypt) for _ in 1:N]
    inner = quote
        let $y_sym = ($(sub_ys...),)
            $body
        end
    end
    for i in N:-1:1
        inner = _probe_body_test(op_types[i], :(($q_expr).ops[$i]), x_sym, sub_ys[i], inner)
    end
    inner
end

# Fallback: types whose probe form runs body at most once unconditionally.
_probe_body_test(T::Type, q_expr, x_sym, y_sym, body) = _probe_body(T, q_expr, x_sym, y_sym, body)

# =====================================================================
# _probe_any_body(QType, q_expr, x_sym) -> Expr (a Bool)
#   True iff x is in the domain / has a matching value.
# =====================================================================

# ---- leaves, derived from the access descriptors ----
function _probe_any_body(::Type{Q}, q_expr, x_sym) where {Q <: SingleValued}
    setup, cond, _ = _access(Q, q_expr, x_sym)
    _let(setup, cond)
end

function _probe_any_body(::Type{Q}, q_expr, x_sym) where {Q <: MultiValued}
    setup, cond, iter = _access_multi(Q, q_expr, x_sym)
    _let(setup, :($cond && !isempty($iter)))
end

# ---- structural ----
# Compose probe_any: any m from probe(a, x) with probe_any(b, m).
function _probe_any_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym) where {D, M, R, A, B}
    m_sym = gensym(:_mc)
    b_check = _probe_any_body(B, :(($q_expr).b), m_sym)
    _probe_body_test(A, :(($q_expr).a), x_sym, m_sym, b_check)
end
function _probe_any_body(::Type{<:Filter{D, R, A}}, q_expr, x_sym) where {D, R, A}
    y_sym = gensym(:_yf)
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, :(($q_expr).pred($y_sym)))
end
function _probe_any_body(::Type{<:Restrict{D, R, A, B}}, q_expr, x_sym) where {D, R, A, B}
    y_sym = gensym(:_yr)
    chk = _probe_any_body(B, :(($q_expr).b), y_sym)
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, chk)
end
function _probe_any_body(::Type{<:Diff{D, R, A, B}}, q_expr, x_sym) where {D, R, A, B}
    a_chk = _probe_any_body(A, :(($q_expr).a), x_sym)
    b_chk = _probe_any_body(B, :(($q_expr).b), x_sym)
    :(!($b_chk) && $a_chk)
end
function _probe_any_body(::Type{<:Map{D, R, S, Q, F}}, q_expr, x_sym) where {D, R, S, Q, F}
    _probe_any_body(Q, :(($q_expr).q), x_sym)
end
# Prod member: flat short-circuit AND over the legs.
function _probe_any_body(::Type{<:Prod{D, R, OPS}}, q_expr, x_sym) where {D, R, OPS}
    N = length(OPS.parameters)
    body = :true
    for i in N:-1:1
        body = :(member(($q_expr).ops[$i], $x_sym) && $body)
    end
    body
end

# Fallback.
function _probe_any_body(::Type{Q}, q_expr, x_sym) where {Q}
    :(probe_any($q_expr, $x_sym, _ -> true))
end


# =====================================================================
# Entry point: scan a prepared plan. `scan(Staged(), pq, sink)` lands here.
# =====================================================================

@generated function _exec_drive(q::Q, sink::F) where {Q, F}
    drv = _drive_body(Q, :q, :(sink(_x, _v)))
    quote
        $drv
        nothing
    end
end
