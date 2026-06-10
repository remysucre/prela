# The staged engine: type-level CPS.
#
# `_exec_drive` is `@generated` over the *physical* plan's type: the emitters
# below walk that type at codegen time and emit one flat, fused loop nest.
# Mode is already resolved into the types (`prepare` lowered every index/cache
# node to a concrete physical struct — `InvStream`/`InvIndexed`,
# `FoldP`/`DenseFoldP`, `MatStream`/`MatProbed`, …), so the emitters read
# concrete fields and never build anything: all state construction happened in
# `prepare`, which itself scans through this same engine (see plan.jl).
#
# The emitters are the type-level mirror of the value-level `drive`/`probe`
# spec in interp.jl: the `body` Expr is the continuation, threaded over the
# plan *type* the way `k` is threaded over the *value*. This is why deep plans
# compile flat here while the interpreted engine trips the inference recursion
# limit — the continuation grows at codegen time, where growth is free.

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

# `MapRel` — drive-only materialized result (collect / inlined pairs).
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

# Filter drive — streaming filtered scan, one emit per predicate kind.
function _drive_body(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, body) where {D, R, A, F}
    wrap = quote
        if ($q_expr).pred.f(_v)
            $body
        end
    end
    _drive_body(A, :(($q_expr).a), wrap)
end
function _drive_body(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, body) where {D, R, A, V}
    wrap = quote
        if isequal(_v, ($q_expr).pred.v)
            $body
        end
    end
    _drive_body(A, :(($q_expr).a), wrap)
end
function _drive_body(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, body) where {D, R, A, T}
    wrap = quote
        if _v in ($q_expr).pred.vs
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

# MatStream drive: iterate the stored pairs.
function _drive_body(::Type{<:MatStream{D, R}}, q_expr, body) where {D, R}
    quote
        for _p in ($q_expr).mat
            let _x = _p.first, _v = _p.second
                $body
            end
        end
    end
end

# MatSetStream drive: iterate the stored keys (identity).
function _drive_body(::Type{<:MatSetStream{D}}, q_expr, body) where {D}
    quote
        for _x in ($q_expr).keys
            let _v = _x
                $body
            end
        end
    end
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
# _probe_body(QType, q_expr, x_sym, y_sym, body) -> Expr
#   Probe Q at x_sym, bind the value to y_sym, run body (a statement).
# =====================================================================

# ---- leaves ----
function _probe_body(::Type{<:VecRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let _i = ($x_sym).id, _vv = ($q_expr).values
            if 1 <= _i <= length(_vv)
                let $y_sym = @inbounds _vv[_i]
                    $body
                end
            end
        end
    end
end
function _probe_body(::Type{<:SparseRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let _i = ($x_sym).id, _vv = ($q_expr).values
            if 1 <= _i <= length(_vv) && @inbounds(($q_expr).seen[_i])
                let $y_sym = @inbounds _vv[_i]
                    $body
                end
            end
        end
    end
end
function _probe_body(::Type{<:MultiRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let _i = ($x_sym).id, _f = ($q_expr).fwd
            if 1 <= _i <= length(_f)
                for $y_sym in @inbounds _f[_i]
                    $body
                end
            end
        end
    end
end

function _probe_body(::Type{Universe{E}}, q_expr, x_sym, y_sym, body) where {E}
    quote
        if 1 <= ($x_sym).id <= ($q_expr).n
            let $y_sym = $x_sym
                $body
            end
        end
    end
end

function _probe_body(::Type{<:Bitset{D}}, q_expr, x_sym, y_sym, body) where {D}
    quote
        let _b = $q_expr, _i = _denseidx($x_sym) + 1
            if @inbounds(1 <= _i <= length(_b.bits) && _b.bits[_i])
                let $y_sym = $x_sym
                    $body
                end
            end
        end
    end
end

# ---- structural ----
function _probe_body(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, x_sym, y_sym, body) where {D, R, A, F}
    wrap = quote
        if ($q_expr).pred.f($y_sym)
            $body
        end
    end
    _probe_body(A, :(($q_expr).a), x_sym, y_sym, wrap)
end
function _probe_body(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, x_sym, y_sym, body) where {D, R, A, V}
    wrap = quote
        if isequal($y_sym, ($q_expr).pred.v)
            $body
        end
    end
    _probe_body(A, :(($q_expr).a), x_sym, y_sym, wrap)
end
function _probe_body(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, x_sym, y_sym, body) where {D, R, A, T}
    wrap = quote
        if $y_sym in ($q_expr).pred.vs
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

# ---- identity-Unary probes: yield x as the value iff member. ----
function _probe_body(::Type{<:Disj{D, A, B}}, q_expr, x_sym, y_sym, body) where {D, A, B}
    a_chk = _probe_any_body(A, :(($q_expr).a), x_sym)
    b_chk = _probe_any_body(B, :(($q_expr).b), x_sym)
    quote
        if $a_chk || $b_chk
            let $y_sym = $x_sym
                $body
            end
        end
    end
end

function _probe_body(::Type{<:LeftConj{D, ML, R}}, q_expr, x_sym, y_sym, body) where {D, ML, R}
    l_chk = _probe_any_body(ML, :(($q_expr).l), x_sym)
    r_chk = _probe_any_body(R, :(($q_expr).r), x_sym)
    quote
        if $l_chk && $r_chk
            let $y_sym = $x_sym
                $body
            end
        end
    end
end

function _probe_body(::Type{<:MatSetProbed{D}}, q_expr, x_sym, y_sym, body) where {D}
    quote
        if $x_sym in ($q_expr).set
            let $y_sym = $x_sym
                $body
            end
        end
    end
end

# ---- prepared probe-side indexes ----
# InvIndexed / LCIndexed — Dict{key, Vector{val}}; iterate matches.
function _probe_body(::Type{<:InvIndexed{B, A}}, q_expr, x_sym, y_sym, body) where {B, A}
    quote
        let _vs = get(($q_expr).idx, $x_sym, nothing)
            if _vs !== nothing
                for $y_sym in _vs
                    $body
                end
            end
        end
    end
end
function _probe_body(::Type{<:LCIndexed{RK, SV}}, q_expr, x_sym, y_sym, body) where {RK, SV}
    quote
        let _vs = get(($q_expr).idx, $x_sym, nothing)
            if _vs !== nothing
                for $y_sym in _vs
                    $body
                end
            end
        end
    end
end

# MatProbed — dense `Vector{Vector}` (entity-keyed) or `Dict`; branch on IDX.
function _probe_body(::Type{<:MatProbed{D, R, IDX}}, q_expr, x_sym, y_sym, body) where {D, R, IDX}
    if IDX <: AbstractVector
        quote
            let _i = ($x_sym).id, _idx = ($q_expr).idx
                if 1 <= _i <= length(_idx)
                    for $y_sym in @inbounds _idx[_i]
                        $body
                    end
                end
            end
        end
    else
        quote
            let _vs = get(($q_expr).idx, $x_sym, nothing)
                if _vs !== nothing
                    for $y_sym in _vs
                        $body
                    end
                end
            end
        end
    end
end

# FoldP probe — single value per key.
function _probe_body(::Type{<:FoldP{D, S}}, q_expr, x_sym, y_sym, body) where {D, S}
    quote
        let _g = get(($q_expr).cache, $x_sym, nothing)
            if _g !== nothing
                let $y_sym = _g
                    $body
                end
            end
        end
    end
end

# DenseFoldP probe — dense slot lookup.
function _probe_body(::Type{<:DenseFoldP{D, S}}, q_expr, x_sym, y_sym, body) where {D, S}
    quote
        let _vals = ($q_expr).vals, _i = _denseidx($x_sym) + 1
            if 1 <= _i <= length(_vals) && @inbounds(($q_expr).seen[_i])
                let $y_sym = @inbounds _vals[_i]
                    $body
                end
            end
        end
    end
end

# ScalarP probe — the single value (key is `nothing`).
function _probe_body(::Type{<:ScalarP{S}}, q_expr, x_sym, y_sym, body) where {S}
    quote
        let $y_sym = ($q_expr).value
            $body
        end
    end
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

function _probe_body_test(::Type{<:VecRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let _i = ($x_sym).id, _vv = ($q_expr).values
            (1 <= _i <= length(_vv)) ? (let $y_sym = @inbounds _vv[_i]; $body; end) : false
        end
    end
end
function _probe_body_test(::Type{<:SparseRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let _i = ($x_sym).id, _vv = ($q_expr).values
            (1 <= _i <= length(_vv) && @inbounds(($q_expr).seen[_i])) ?
                (let $y_sym = @inbounds _vv[_i]; $body; end) : false
        end
    end
end
function _probe_body_test(::Type{<:MultiRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let _i = ($x_sym).id, _f = ($q_expr).fwd, _acc = false
            if 1 <= _i <= length(_f)
                for $y_sym in @inbounds _f[_i]
                    _acc = ($body) || _acc
                end
            end
            _acc
        end
    end
end

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
function _probe_body_test(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, x_sym, y_sym, body) where {D, R, A, F}
    wrap = quote
        ($q_expr).pred.f($y_sym) ? ($body) : false
    end
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, x_sym, y_sym, body) where {D, R, A, V}
    wrap = quote
        isequal($y_sym, ($q_expr).pred.v) ? ($body) : false
    end
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, x_sym, y_sym, body) where {D, R, A, T}
    wrap = quote
        ($y_sym in ($q_expr).pred.vs) ? ($body) : false
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

function _probe_body_test(::Type{<:InvIndexed{B, A}}, q_expr, x_sym, y_sym, body) where {B, A}
    quote
        let _vs = get(($q_expr).idx, $x_sym, nothing), _acc = false
            if _vs !== nothing
                for $y_sym in _vs
                    _acc = ($body) || _acc
                end
            end
            _acc
        end
    end
end
function _probe_body_test(::Type{<:LCIndexed{RK, SV}}, q_expr, x_sym, y_sym, body) where {RK, SV}
    quote
        let _vs = get(($q_expr).idx, $x_sym, nothing), _acc = false
            if _vs !== nothing
                for $y_sym in _vs
                    _acc = ($body) || _acc
                end
            end
            _acc
        end
    end
end
function _probe_body_test(::Type{<:MatProbed{D, R, IDX}}, q_expr, x_sym, y_sym, body) where {D, R, IDX}
    if IDX <: AbstractVector
        quote
            let _i = ($x_sym).id, _idx = ($q_expr).idx, _acc = false
                if 1 <= _i <= length(_idx)
                    for $y_sym in @inbounds _idx[_i]
                        _acc = ($body) || _acc
                    end
                end
                _acc
            end
        end
    else
        quote
            let _vs = get(($q_expr).idx, $x_sym, nothing), _acc = false
                if _vs !== nothing
                    for $y_sym in _vs
                        _acc = ($body) || _acc
                    end
                end
                _acc
            end
        end
    end
end
function _probe_body_test(::Type{<:FoldP{D, S}}, q_expr, x_sym, y_sym, body) where {D, S}
    quote
        let _g = get(($q_expr).cache, $x_sym, nothing)
            _g !== nothing ? (let $y_sym = _g; $body; end) : false
        end
    end
end
function _probe_body_test(::Type{<:DenseFoldP{D, S}}, q_expr, x_sym, y_sym, body) where {D, S}
    quote
        let _vals = ($q_expr).vals, _i = _denseidx($x_sym) + 1
            (1 <= _i <= length(_vals) && @inbounds(($q_expr).seen[_i])) ?
                (let $y_sym = @inbounds _vals[_i]; $body; end) : false
        end
    end
end
function _probe_body_test(::Type{Universe{E}}, q_expr, x_sym, y_sym, body) where {E}
    quote
        (1 <= ($x_sym).id <= ($q_expr).n) ? (let $y_sym = $x_sym; $body; end) : false
    end
end
function _probe_body_test(::Type{<:Bitset{D}}, q_expr, x_sym, y_sym, body) where {D}
    quote
        let _b = $q_expr, _i = _denseidx($x_sym) + 1
            @inbounds(1 <= _i <= length(_b.bits) && _b.bits[_i]) ?
                (let $y_sym = $x_sym; $body; end) : false
        end
    end
end
function _probe_body_test(::Type{<:MatSetProbed{D}}, q_expr, x_sym, y_sym, body) where {D}
    quote
        ($x_sym in ($q_expr).set) ? (let $y_sym = $x_sym; $body; end) : false
    end
end

# Fallback: types whose probe form runs body at most once unconditionally.
_probe_body_test(T::Type, q_expr, x_sym, y_sym, body) = _probe_body(T, q_expr, x_sym, y_sym, body)

# =====================================================================
# _probe_any_body(QType, q_expr, x_sym) -> Expr (a Bool)
#   True iff x is in the domain / has a matching value.
# =====================================================================

function _probe_any_body(::Type{<:VecRel{E, R}}, q_expr, x_sym) where {E, R}
    :(let _i = ($x_sym).id; 1 <= _i <= length(($q_expr).values); end)
end
function _probe_any_body(::Type{<:SparseRel{E, R}}, q_expr, x_sym) where {E, R}
    quote
        let _i = ($x_sym).id, _vv = ($q_expr).values
            1 <= _i <= length(_vv) && @inbounds(($q_expr).seen[_i])
        end
    end
end
function _probe_any_body(::Type{<:MultiRel{E, R}}, q_expr, x_sym) where {E, R}
    quote
        let _i = ($x_sym).id, _f = ($q_expr).fwd
            1 <= _i <= length(_f) && !isempty(@inbounds _f[_i])
        end
    end
end
function _probe_any_body(::Type{<:Universe{E}}, q_expr, x_sym) where {E}
    :(let _i = ($x_sym).id; 1 <= _i <= ($q_expr).n; end)
end
function _probe_any_body(::Type{<:Bitset{D}}, q_expr, x_sym) where {D}
    quote
        let _b = $q_expr, _i = _denseidx($x_sym) + 1
            @inbounds(1 <= _i <= length(_b.bits) && _b.bits[_i])
        end
    end
end

# Compose probe_any: any m from probe(a, x) with probe_any(b, m).
function _probe_any_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym) where {D, M, R, A, B}
    m_sym = gensym(:_mc)
    b_check = _probe_any_body(B, :(($q_expr).b), m_sym)
    _probe_body_test(A, :(($q_expr).a), x_sym, m_sym, b_check)
end
function _probe_any_body(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, x_sym) where {D, R, A, F}
    y_sym = gensym(:_yf)
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, :(($q_expr).pred.f($y_sym)))
end
function _probe_any_body(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, x_sym) where {D, R, A, V}
    y_sym = gensym(:_ye)
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, :(isequal($y_sym, ($q_expr).pred.v)))
end
function _probe_any_body(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, x_sym) where {D, R, A, T}
    y_sym = gensym(:_yi)
    _probe_body_test(A, :(($q_expr).a), x_sym, y_sym, :($y_sym in ($q_expr).pred.vs))
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
function _probe_any_body(::Type{<:Disj{D, A, B}}, q_expr, x_sym) where {D, A, B}
    a = _probe_any_body(A, :(($q_expr).a), x_sym)
    b = _probe_any_body(B, :(($q_expr).b), x_sym)
    :($a || $b)
end
function _probe_any_body(::Type{<:LeftConj{D, ML, R}}, q_expr, x_sym) where {D, ML, R}
    l = _probe_any_body(ML, :(($q_expr).l), x_sym)
    r = _probe_any_body(R, :(($q_expr).r), x_sym)
    :($l && $r)
end
function _probe_any_body(::Type{<:MatSetProbed{D}}, q_expr, x_sym) where {D}
    :($x_sym in ($q_expr).set)
end

# prepared probe-side indexes — membership = key present.
_probe_any_body(::Type{<:InvIndexed{B, A}}, q_expr, x_sym) where {B, A} =
    :(haskey(($q_expr).idx, $x_sym))
_probe_any_body(::Type{<:LCIndexed{RK, SV}}, q_expr, x_sym) where {RK, SV} =
    :(haskey(($q_expr).idx, $x_sym))
function _probe_any_body(::Type{<:MatProbed{D, R, IDX}}, q_expr, x_sym) where {D, R, IDX}
    if IDX <: AbstractVector
        quote
            let _i = ($x_sym).id, _idx = ($q_expr).idx
                1 <= _i <= length(_idx) && !isempty(@inbounds _idx[_i])
            end
        end
    else
        :(haskey(($q_expr).idx, $x_sym))
    end
end
_probe_any_body(::Type{<:FoldP{D, S}}, q_expr, x_sym) where {D, S} =
    :(haskey(($q_expr).cache, $x_sym))
function _probe_any_body(::Type{<:DenseFoldP{D, S}}, q_expr, x_sym) where {D, S}
    quote
        let _i = _denseidx($x_sym) + 1, _seen = ($q_expr).seen
            1 <= _i <= length(_seen) && @inbounds(_seen[_i])
        end
    end
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
