# Proof of concept: `@generated execute(q)` — walks the query type at codegen
# time and emits a flat loop. The op closure is a runtime value with concrete
# type, so Julia/LLVM inlines its body alongside the rest. Goal: match
# hand-rolled Q6 (0.05s) while preserving the algebra surface.

using Main: Prela
import Main.Prela: Query, Unary, Universe, Diff, Compose,
                   Filter, Disj, Keys, Prod, MapRel,
                   VecRel, UnaryVec, Scalar, FnP, EqP, InP, Map,
                   LeftCompose, LeftConj, DenseFold, Fold, BufFold, Inv,
                   Bitset, Materialized, MatSet, _denseidx, _densebox,
                   member, drive, probe, probe_any, askeys

# =====================================================================
# Emission helpers (run at codegen time inside @generated; return Expr).
# Convention: per-row code uses `_x` (key) and `_v` (value). Inner probes
# use gensym'd names to avoid collisions in nested chains.
# =====================================================================

# _drive_body(QType, q_expr, body_expr) -> Expr
#   Generates a loop that walks Q's type and, per row, binds `_x` to the key
#   (and, where applicable, `_v` to the value) and runs body_expr.

function _drive_body(::Type{Universe{E}}, q_expr, body) where E
    # Identity-Unary: value equals the key.
    quote
        @inbounds for _i in 1:($q_expr).n
            let _x = Prela.ID{$E}(_i), _v = _x
                $body
            end
        end
    end
end

# Restrict / URestrict types are gone (tarski) — both reduce to Compose with
# `askeys(b)` on the rhs. Handled by Compose.drive below.

# MapRel drive (entity-keyed dense values fast path). Locals prefixed
# `_mr_` to avoid collision with DenseFold's `_seen`/`_vals` when the
# MapRel.drive is the cache-build inner of a DenseFold (Q18 shape).
function _drive_body(::Type{<:MapRel{ID_TYPE, R}}, q_expr, body) where {ID_TYPE, R}
    quote
        let _mr_vals = ($q_expr).values, _mr_seen = ($q_expr).seen, _mr_pairs = ($q_expr).pairs
            if !isempty(_mr_vals) && _mr_seen === nothing
                @inbounds for _mr_i in eachindex(_mr_vals)
                    let _x = $ID_TYPE(_mr_i), _v = _mr_vals[_mr_i]
                        $body
                    end
                end
            elseif !isempty(_mr_vals)
                @inbounds for _mr_i in eachindex(_mr_vals)
                    if _mr_seen[_mr_i]
                        let _x = $ID_TYPE(_mr_i), _v = _mr_vals[_mr_i]
                            $body
                        end
                    end
                end
            else
                for _mr_p in _mr_pairs
                    let _x = _mr_p.first, _v = _mr_p.second
                        $body
                    end
                end
            end
        end
    end
end

# VecRel drive
function _drive_body(::Type{<:VecRel{E, R}}, q_expr, body) where {E, R}
    quote
        let _vals = ($q_expr).values
            @inbounds for _i in eachindex(_vals)
                let _x = Prela.ID{$E}(_i), _v = _vals[_i]
                    $body
                end
            end
        end
    end
end

# UnaryVec drive: iterate the values vector (identity-Unary: _v = _x)
function _drive_body(::Type{<:UnaryVec{D}}, q_expr, body) where {D}
    quote
        for _x in ($q_expr).values
            let _v = _x
                $body
            end
        end
    end
end

# InlineRel handler — Main.InlineRel is a custom type in `tpch_queries_*.jl`
# that wraps a Vector{Pair}. Direct field access; no lambda needed (which
# would trip the @generated purity check).
if isdefined(Main, :InlineRel)
    function _drive_body(::Type{<:Main.InlineRel{D, R}}, q_expr, body) where {D, R}
        quote
            for _ir_p in ($q_expr).pairs
                let _x = _ir_p.first, _v = _ir_p.second
                    $body
                end
            end
        end
    end
end

# Runtime fallback for any unhandled Query/Unary type. Emits a
# `Prela.drive` call wrapping a lambda. NOTE: the lambda in the returned
# AST trips Julia's @generated purity check, so this fallback isn't
# actually usable — adding an explicit emitter is required for any type
# that hits it. We keep it as a method (instead of an `error`) so
# experimenters can read the error message Julia prints.
function _drive_body(::Type{Q}, q_expr, body) where {Q}
    quote
        Prela.drive($q_expr, (_x, _v) -> $body)
    end
end

# Compose drive: drive(a, (x, m) -> probe(b, m, r -> body))
function _drive_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, body) where {D, M, R, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    m_sym = gensym(:_m)
    inner = _probe_body(B, b_expr, m_sym, :_v, body)
    inner_with_m = quote
        let $m_sym = _v
            $inner
        end
    end
    _drive_body(A, a_expr, inner_with_m)
end

# LeftCompose drive: drive(s, (x, sv) -> probe(r, x, rk -> body))
# After this, body sees `_x = rk` (the projected key) and `_v = sv` (the value).
function _drive_body(::Type{<:LeftCompose{D, RK, SV, QR, QS}}, q_expr, body) where {D, RK, SV, QR, QS}
    r_expr = :(($q_expr).r)
    s_expr = :(($q_expr).s)
    # Build inner: probe(r, original_x, rk -> let _x = rk; _v = original_sv; body end)
    sv_save = gensym(:_sv)
    orig_x = gensym(:_xs)
    rk_sym = gensym(:_rk)
    body_with_rk = quote
        let _x = $rk_sym, _v = $sv_save
            $body
        end
    end
    probe_expr = _probe_body(QR, r_expr, orig_x, rk_sym, body_with_rk)
    # _drive_body(QS, ...) emits driving the rhs s, calling body per (_x=x, _v=sv)
    # We want to rebind to use original names but stash them
    inner = quote
        let $orig_x = _x, $sv_save = _v
            $probe_expr
        end
    end
    _drive_body(QS, s_expr, inner)
end

# Map drive: drive(inner, (d, v) -> body with _v = f(v))
function _drive_body(::Type{<:Map{D, R, S, Q, F}}, q_expr, body) where {D, R, S, Q, F}
    q_inner = :(($q_expr).q)
    inner_v = gensym(:_vm)
    wrapped = quote
        let $inner_v = _v
            let _v = ($q_expr).f($inner_v)
                $body
            end
        end
    end
    _drive_body(Q, q_inner, wrapped)
end

# Scalar drive: evaluate the scalar fold and emit (nothing, value) once.
# Lets execute_drive work uniformly even on Scalar tops.
function _drive_body(::Type{<:Scalar{S, Q, OP}}, q_expr, body) where {S, Q, OP}
    q_inner = :(($q_expr).q)
    inner = quote
        _acc = ($q_expr).op(_acc, _v)
    end
    inner_loop = _drive_body(Q, q_inner, inner)
    quote
        _acc = ($q_expr).init
        $inner_loop
        let _x = nothing, _v = _acc
            $body
        end
    end
end

# Prod drive: drive ops[1], probe ops[2..N], emit (x, tuple)
function _drive_body(::Type{<:Prod{D, R, OPS}}, q_expr, body) where {D, R, OPS}
    op_types = OPS.parameters
    N = length(op_types)
    if N == 1
        # degenerate
        sub_y = gensym(:_yp)
        wrap = quote
            let _v = ($sub_y,)
                $body
            end
        end
        return _drive_body(op_types[1], :(($q_expr).ops[1]), quote
            let $sub_y = _v
                $wrap
            end
        end)
    end
    sub_ys = Vector{Symbol}(undef, N)
    for k in 1:N
        sub_ys[k] = gensym(:_yp)
    end
    # Innermost: bind _v = (sub_ys...,); body
    inner = quote
        let _v = ($(sub_ys...),)
            $body
        end
    end
    # Wrap N..2 probes
    for i in N:-1:2
        op_expr = :(($q_expr).ops[$i])
        inner = _probe_body(op_types[i], op_expr, :_x, sub_ys[i], inner)
    end
    # Outermost: drive ops[1], binding _x and sub_ys[1] = _v
    outer = quote
        let $(sub_ys[1]) = _v
            $inner
        end
    end
    _drive_body(op_types[1], :(($q_expr).ops[1]), outer)
end

# Filter drive — each predicate type gets its own emit
function _drive_body(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, body) where {D, R, A, F}
    a_expr = :(($q_expr).a)
    wrap = quote
        if ($q_expr).pred.f(_v)
            $body
        end
    end
    _drive_body(A, a_expr, wrap)
end
function _drive_body(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, body) where {D, R, A, V}
    a_expr = :(($q_expr).a)
    wrap = quote
        if isequal(_v, ($q_expr).pred.v)
            $body
        end
    end
    _drive_body(A, a_expr, wrap)
end
function _drive_body(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, body) where {D, R, A, T}
    a_expr = :(($q_expr).a)
    wrap = quote
        if _v in ($q_expr).pred.vs
            $body
        end
    end
    _drive_body(A, a_expr, wrap)
end
# Filter{InSetP} is gone — under tarski, set membership in filter chains
# uses Compose with `askeys(set)` on the rhs, not a Filter-with-set-pred.
# Conj / SetDiff types are gone (tarski). `∧` aliases `⊗` (Prod); set
# difference goes through Diff with `askeys(b)` on the rhs.

# Disj drive: drive a (emits (x, x)), then drive b excluding a's keys
function _drive_body(::Type{<:Disj{D, A, B}}, q_expr, body) where {D, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    a_check = _probe_any_body(A, a_expr, :_x)
    body_b = quote
        if !($a_check)
            $body
        end
    end
    a_drive = _drive_body(A, a_expr, body)
    b_drive = _drive_body(B, b_expr, body_b)
    quote
        $a_drive
        $b_drive
    end
end

# Diff drive (value-bearing): drive a, exclude keys in b.
function _drive_body(::Type{<:Diff{D, R, A, B}}, q_expr, body) where {D, R, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    b_check = _probe_any_body(B, b_expr, :_x)
    wrap = quote
        if !($b_check)
            $body
        end
    end
    _drive_body(A, a_expr, wrap)
end

# Keys drive (identity view of a Query's keyset): drive a, rebind _v = _x.
function _drive_body(::Type{<:Keys{D, A}}, q_expr, body) where {D, A}
    a_expr = :(($q_expr).a)
    wrap = quote
        let _v = _x
            $body
        end
    end
    _drive_body(A, a_expr, wrap)
end

# Bitset drive: iterate set bits. Box D at codegen; emit (x, x) for identity.
function _drive_body(::Type{<:Bitset{D}}, q_expr, body) where {D}
    box_expr = D === Int ? :(_i - 1) : :($D(_i - 1))
    quote
        let _b = $q_expr
            @inbounds for _i in eachindex(_b.bits)
                if _b.bits[_i]
                    let _x = $box_expr, _v = _x
                        $body
                    end
                end
            end
        end
    end
end

# Pipeline-breaker drive emitters.
#
# Conceptually we wanted `_engine_fold_drive(@nospecialize(q), sink::F)` so
# the wrapper compiles once across all query types, with `Prela._fold_cache(q)`
# inside dynamically dispatching at runtime — sharing the existing engine's
# specialization. BUT: emitting `_engine_fold_drive($q_expr, (_x, _v) -> $body)`
# puts an `Expr(:->, …)` (a lambda) into the returned AST, which trips
# Julia's `@generated` purity check ("function body AST is not pure").
#
# So we inline the for-loop directly, accepting that `Prela._fold_cache(q)`
# specializes per typeof(q) at this call site (same as the algebra path).
# The pipeline-breaker call cost is a single dispatch per query, not per
# row — invisible next to the cache-build work.
function _drive_body(::Type{<:Fold{D, R, S, Q, OP}}, q_expr, body) where {D, R, S, Q, OP}
    quote
        for (_x, _v) in Prela._fold_cache($q_expr)
            $body
        end
    end
end

function _drive_body(::Type{<:BufFold{D, R, S, Q, F}}, q_expr, body) where {D, R, S, Q, F}
    quote
        for (_x, _v) in Prela._buf_cache($q_expr)
            $body
        end
    end
end

# Inv drive: flips pairs. Streaming operator, not a pipeline-breaker —
# keep the per-row specialization by recursing into the inner Q's drive.
function _drive_body(::Type{<:Inv{B, A, Q}}, q_expr, body) where {B, A, Q}
    q_inner = :(($q_expr).q)
    inner_x = gensym(:_xinv)
    flip = quote
        let $inner_x = _x, _x = _v, _v = $inner_x
            $body
        end
    end
    _drive_body(Q, q_inner, flip)
end

function _drive_body(::Type{<:Materialized{D, R, A}}, q_expr, body) where {D, R, A}
    quote
        for _p in Prela._cmat($q_expr)
            let _x = _p.first, _v = _p.second
                $body
            end
        end
    end
end

function _drive_body(::Type{<:MatSet{D, A}}, q_expr, body) where {D, A}
    quote
        for _x in Prela._mkeys($q_expr)
            let _v = _x
                $body
            end
        end
    end
end

# LeftConj drive (identity Unary): drive r, member-check l per row.
function _drive_body(::Type{<:LeftConj{D, ML, R}}, q_expr, body) where {D, ML, R}
    r_expr = :(($q_expr).r)
    l_expr = :(($q_expr).l)
    pred = _probe_any_body(ML, l_expr, :_x)
    wrap = quote
        if $pred
            $body
        end
    end
    _drive_body(R, r_expr, wrap)
end

# DenseFold drive: build the cache, then iterate it.
# Locals are prefixed `_df_` to avoid name collisions with nested emitters
# that need their own `_vals`/`_seen` (e.g. MapRel.drive used in the inner
# query). Iter binds `_x` back to D via `_densebox` so downstream probes
# get the right wrapped key (ID{E} or Int).
function _drive_body(::Type{<:DenseFold{D, R, S, Q, OP}}, q_expr, body) where {D, R, S, Q, OP}
    q_inner = :(($q_expr).q)
    inner = quote
        let _df_i = _denseidx(_x) + 1
            if 1 <= _df_i <= _df_sz
                @inbounds _df_vals[_df_i] = _df_op(_df_vals[_df_i], _v)
                @inbounds _df_seen[_df_i] = true
            end
        end
    end
    cache_build = _drive_body(Q, q_inner, inner)
    iter_body = quote
        @inbounds for _df_idx in eachindex(_df_vals)
            if _df_seen[_df_idx]
                let _x = _densebox($D, _df_idx - 1), _v = _df_vals[_df_idx]
                    $body
                end
            end
        end
    end
    quote
        _df_sz   = ($q_expr).n + 1
        _df_vals = fill(($q_expr).init, _df_sz)
        _df_seen = falses(_df_sz)
        _df_op   = ($q_expr).op
        $cache_build
        $iter_body
    end
end

# =====================================================================
# _probe_body(QType, q_expr, x_sym, y_sym, body) -> Expr
#   Probes Q at x_sym, binds the value to y_sym, runs body.
# =====================================================================

function _probe_body(::Type{<:MapRel{D, R}}, q_expr, x_sym, y_sym, body) where {D, R}
    # Emits ONLY the fwd_index fallback path — `$body` is interpolated
    # exactly once per call site. The earlier dual-path version blew up
    # AST by 2^N for N nested probes (Q33a hit 131k copies). `body` is
    # a statement here (drive context); the for-loop runs it per match
    # and returns nothing. For probe_any (Bool) context, the caller
    # should use `_probe_body_test` (below).
    if D <: Prela.ID
        quote
            let _mrp_idx = Prela.fwd_index($q_expr), _mrp_i = ($x_sym).id
                if 1 <= _mrp_i <= length(_mrp_idx)
                    for $y_sym in @inbounds _mrp_idx[_mrp_i]
                        $body
                    end
                end
            end
        end
    else
        quote
            let _mrp_idx = Prela.fwd_index($q_expr),
                _mrp_vs  = get(_mrp_idx, $x_sym, nothing)
                if _mrp_vs !== nothing
                    for $y_sym in _mrp_vs
                        $body
                    end
                end
            end
        end
    end
end

# `_probe_body_test` — for probe_any chains. `body` must be a Bool.
# Returns true iff any matched y makes body true. No body duplication.
function _probe_body_test(::Type{<:MapRel{D, R}}, q_expr, x_sym, y_sym, body) where {D, R}
    if D <: Prela.ID
        quote
            let _mrp_idx = Prela.fwd_index($q_expr), _mrp_i = ($x_sym).id, _mrp_acc = false
                if 1 <= _mrp_i <= length(_mrp_idx)
                    for $y_sym in @inbounds _mrp_idx[_mrp_i]
                        _mrp_acc = ($body) || _mrp_acc
                    end
                end
                _mrp_acc
            end
        end
    else
        quote
            let _mrp_idx = Prela.fwd_index($q_expr),
                _mrp_vs  = get(_mrp_idx, $x_sym, nothing),
                _mrp_acc = false
                if _mrp_vs !== nothing
                    for $y_sym in _mrp_vs
                        _mrp_acc = ($body) || _mrp_acc
                    end
                end
                _mrp_acc
            end
        end
    end
end
# Test-context recursive helpers — propagate `_probe_body_test` calls so
# the Bool-accumulating MapRel path is used even when reached through
# intermediates (Compose/Filter/Map/Prod) in a probe_any chain.

function _probe_body_test(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, M, R, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    m_sym = gensym(:_mpt)
    inner = _probe_body_test(B, b_expr, m_sym, y_sym, body)
    _probe_body_test(A, a_expr, x_sym, m_sym, inner)
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
    a_expr = :(($q_expr).a)
    wrap = quote
        if ($q_expr).pred.f($y_sym)
            $body
        else
            false
        end
    end
    _probe_body_test(A, a_expr, x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, x_sym, y_sym, body) where {D, R, A, V}
    a_expr = :(($q_expr).a)
    wrap = quote
        if isequal($y_sym, ($q_expr).pred.v)
            $body
        else
            false
        end
    end
    _probe_body_test(A, a_expr, x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, x_sym, y_sym, body) where {D, R, A, T}
    a_expr = :(($q_expr).a)
    wrap = quote
        if $y_sym in ($q_expr).pred.vs
            $body
        else
            false
        end
    end
    _probe_body_test(A, a_expr, x_sym, y_sym, wrap)
end
function _probe_body_test(::Type{<:Prod{D, R, OPS}}, q_expr, x_sym, y_sym, body) where {D, R, OPS}
    op_types = OPS.parameters
    N = length(op_types)
    sub_ys = Vector{Symbol}(undef, N)
    for k in 1:N; sub_ys[k] = gensym(:_ypt); end
    inner = quote
        let $y_sym = ($(sub_ys...),)
            $body
        end
    end
    for i in N:-1:1
        op_expr = :(($q_expr).ops[$i])
        inner = _probe_body_test(op_types[i], op_expr, x_sym, sub_ys[i], inner)
    end
    inner
end

# VecRel and other leaves that always return a single value: body propagates.
function _probe_body_test(::Type{VecRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let $y_sym = @inbounds ($q_expr).values[($x_sym).id]
            $body
        end
    end
end

# Test-context versions for pipeline-breakers. OR-accumulate Bool body.
function _probe_body_test(::Type{<:Materialized{D, R, A}}, q_expr, x_sym, y_sym, body) where {D, R, A}
    quote
        let _mat_idx = Prela._cidx($q_expr), _mat_vs = get(_mat_idx, $x_sym, nothing), _mat_acc = false
            if _mat_vs !== nothing
                for $y_sym in _mat_vs
                    _mat_acc = ($body) || _mat_acc
                end
            end
            _mat_acc
        end
    end
end
function _probe_body_test(::Type{<:Inv{B, A, Q}}, q_expr, x_sym, y_sym, body) where {B, A, Q}
    quote
        let _inv_idx = Prela._inv_index($q_expr), _inv_vs = get(_inv_idx, $x_sym, nothing), _inv_acc = false
            if _inv_vs !== nothing
                for $y_sym in _inv_vs
                    _inv_acc = ($body) || _inv_acc
                end
            end
            _inv_acc
        end
    end
end
function _probe_body_test(::Type{<:Fold{D, R, S, Q, OP}}, q_expr, x_sym, y_sym, body) where {D, R, S, Q, OP}
    quote
        let _c = Prela._fold_cache($q_expr), _g = get(_c, $x_sym, nothing)
            if _g !== nothing
                let $y_sym = _g
                    $body
                end
            else
                false
            end
        end
    end
end
function _probe_body_test(::Type{<:DenseFold{D, R, S, Q, OP}}, q_expr, x_sym, y_sym, body) where {D, R, S, Q, OP}
    quote
        let (_vals, _seen) = Prela._dfold_cache($q_expr), _i = _denseidx($x_sym) + 1
            if 1 <= _i <= length(_vals) && @inbounds(_seen[_i])
                let $y_sym = @inbounds _vals[_i]
                    $body
                end
            else
                false
            end
        end
    end
end

# Fallback: same as drive _probe_body. Safe for types whose drive form
# already evaluates body unconditionally (VecRel, Compose, Map, Prod, etc.).
_probe_body_test(T::Type, q_expr, x_sym, y_sym, body) = _probe_body(T, q_expr, x_sym, y_sym, body)

function _probe_body(::Type{VecRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let $y_sym = @inbounds ($q_expr).values[($x_sym).id]
            $body
        end
    end
end

# Map probe: probe(q, x, k) = probe(inner, x, v -> k(f(v)))
# Filter probe (drive context) — fetch the filtered value at a key.
# Probe inner, test pred, run body. No Bool needed; test-context
# Filter handlers in `_probe_body_test` (above) emit the `else false`.
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
# ---- Identity-Unary probes (tarski) — yield x as the value if member. ----
# In tarski, Unary{D} <: Query{D, D}: probing at x yields x iff member(q, x).

function _probe_body(::Type{<:Keys{D, A}}, q_expr, x_sym, y_sym, body) where {D, A}
    test = _probe_any_body(A, :(($q_expr).a), x_sym)
    quote
        if $test
            let $y_sym = $x_sym
                $body
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

function _probe_body(::Type{<:UnaryVec{D}}, q_expr, x_sym, y_sym, body) where {D}
    quote
        if member($q_expr, $x_sym)
            let $y_sym = $x_sym
                $body
            end
        end
    end
end

function _probe_body(::Type{<:MatSet{D, A}}, q_expr, x_sym, y_sym, body) where {D, A}
    quote
        if member($q_expr, $x_sym)
            let $y_sym = $x_sym
                $body
            end
        end
    end
end

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

# Compose probe (when Compose appears on the rhs of a → ): probe through chain
function _probe_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym, y_sym, body) where {D, M, R, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    m_sym = gensym(:_mp)
    inner = _probe_body(B, b_expr, m_sym, y_sym, body)
    _probe_body(A, a_expr, x_sym, m_sym, inner)
end

# Fold probe (drive context) — dict get, run body if found.
function _probe_body(::Type{<:Fold{D, R, S, Q, OP}}, q_expr, x_sym, y_sym, body) where {D, R, S, Q, OP}
    quote
        let _g = get(Prela._fold_cache($q_expr), $x_sym, nothing)
            if _g !== nothing
                let $y_sym = _g
                    $body
                end
            end
        end
    end
end

# DenseFold probe (drive context) — array lookup, run body if present.
function _probe_body(::Type{<:DenseFold{D, R, S, Q, OP}}, q_expr, x_sym, y_sym, body) where {D, R, S, Q, OP}
    quote
        let (_vals, _seen) = Prela._dfold_cache($q_expr), _i = _denseidx($x_sym) + 1
            if 1 <= _i <= length(_vals) && @inbounds(_seen[_i])
                let $y_sym = @inbounds _vals[_i]
                    $body
                end
            end
        end
    end
end

# Materialized probe — drive context, body is a statement, no Bool needed.
function _probe_body(::Type{<:Materialized{D, R, A}}, q_expr, x_sym, y_sym, body) where {D, R, A}
    quote
        let _mat_idx = Prela._cidx($q_expr), _mat_vs = get(_mat_idx, $x_sym, nothing)
            if _mat_vs !== nothing
                for $y_sym in _mat_vs
                    $body
                end
            end
        end
    end
end

# Inv probe — drive context.
function _probe_body(::Type{<:Inv{B, A, Q}}, q_expr, x_sym, y_sym, body) where {B, A, Q}
    quote
        let _inv_idx = Prela._inv_index($q_expr), _inv_vs = get(_inv_idx, $x_sym, nothing)
            if _inv_vs !== nothing
                for $y_sym in _inv_vs
                    $body
                end
            end
        end
    end
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
    sub_ys = Vector{Symbol}(undef, N)
    for i in 1:N
        sub_ys[i] = gensym(:_yp)
    end
    inner = quote
        let $y_sym = ($(sub_ys...),)
            $body
        end
    end
    for i in N:-1:1
        op_expr = :(($q_expr).ops[$i])
        inner = _probe_body(op_types[i], op_expr, x_sym, sub_ys[i], inner)
    end
    inner
end

# =====================================================================
# _probe_any_body(QType, q_expr, x_sym) -> Expr (a Bool expression)
#   True iff x is in the unary set / has a matching value.
# =====================================================================

function _probe_any_body(::Type{<:Keys{D, A}}, q_expr, x_sym) where {D, A}
    a_expr = :(($q_expr).a)
    _probe_any_body(A, a_expr, x_sym)
end

# Conj is gone (tarski) — `∧` aliases `⊗` (Prod). Prod gets its own
# probe_any via Prela._prod_member (flat short-circuit AND), so we
# delegate at codegen.
function _probe_any_body(::Type{<:Prod{D, R, OPS}}, q_expr, x_sym) where {D, R, OPS}
    op_types = OPS.parameters
    N = length(op_types)
    # Emit a flat short-circuit AND chain — mirrors Prela._prod_member.
    body = :true
    for i in N:-1:1
        body = :(member(($q_expr).ops[$i], $x_sym) && $body)
    end
    body
end

# Filter with FnP{F} closure predicate — covers <, >, <=, >=, in interval, etc.
function _probe_any_body(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, x_sym) where {D, R, A, F}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_yf)
    val_test = :(($q_expr).pred.f($y_sym))
    _probe_body_test(A, a_expr, x_sym, y_sym, val_test)
end

function _probe_any_body(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, x_sym) where {D, R, A, V}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_ye)
    test = :(isequal($y_sym, ($q_expr).pred.v))
    _probe_body_test(A, a_expr, x_sym, y_sym, test)
end

function _probe_any_body(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, x_sym) where {D, R, A, T}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_yi)
    test = :($y_sym in ($q_expr).pred.vs)
    _probe_body_test(A, a_expr, x_sym, y_sym, test)
end

# Filter{InSetP} is gone — set membership in filter chains now goes through
# Compose with `askeys(set)` on the rhs.

# Bitset.probe_any — direct bit-test in the @generated codegen so the bit
# bounds check inlines flat into the surrounding loop.
function _probe_any_body(::Type{Bitset{D}}, q_expr, x_sym) where {D}
    quote
        let _b = $q_expr, _i = _denseidx($x_sym) + 1
            @inbounds (1 <= _i <= length(_b.bits) && _b.bits[_i])
        end
    end
end

# Universe.probe_any — bounds check.
function _probe_any_body(::Type{<:Universe{E}}, q_expr, x_sym) where {E}
    :(let _u = $q_expr, _i = ($x_sym).id
         1 <= _i <= _u.n
      end)
end

# Disj — value in either side.
function _probe_any_body(::Type{<:Disj{D, A, B}}, q_expr, x_sym) where {D, A, B}
    a = _probe_any_body(A, :(($q_expr).a), x_sym)
    b = _probe_any_body(B, :(($q_expr).b), x_sym)
    :($a || $b)
end

# Materialized.probe_any — through cached idx. For ID-keyed (Vector{Vector{R}})
# do bounds + non-empty inner. For Dict-keyed use haskey.
function _probe_any_body(::Type{<:Materialized{D, R, A}}, q_expr, x_sym) where {D, R, A}
    if D <: Prela.ID
        quote
            let _idx = Prela._cidx($q_expr), _i = ($x_sym).id
                1 <= _i <= length(_idx) && !isempty(@inbounds _idx[_i])
            end
        end
    else
        :(haskey(Prela._cidx($q_expr), $x_sym))
    end
end

# SetDiff is gone (tarski). MatSet.probe_any — fall back to the runtime set lookup. MatSet builds its
# internal `Set{D}` lazily on first call, so it's fine to call `member` here.
function _probe_any_body(::Type{<:MatSet{D, A}}, q_expr, x_sym) where {D, A}
    :(member($q_expr, $x_sym))
end

# Compose probe_any: probe through chain, collapsing on final bool.
# probe_any(a → b, x, k) = probe_any(a, x, m -> probe_any(b, m, k))
# For a Compose chain, the value at the end goes to whatever predicate uses it.
# Here the "test" is just emit `member(b, m)` for the rightmost; but we can
# express it as probe_body(a, x, m) wrapping probe_any(b, m).
function _probe_any_body(::Type{<:Compose{D, M, R, A, B}}, q_expr, x_sym) where {D, M, R, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    m_sym = gensym(:_mc)
    b_check = _probe_any_body(B, b_expr, m_sym)
    # probe_body(A, x, m, body): evaluate the body as a Bool expression
    # We want: any m from probe(A, x) such that probe_any(B, m).
    # _probe_body wraps body in let-binding; we need it as boolean.
    # Easiest: emit a let with a Ref-style flag, but that's heavyweight.
    # Alternative: probe_body emits binding then runs body; if A is a 1:1
    # leaf like MapRel/VecRel/Map of a leaf, the probe yields exactly one m,
    # so body runs at most once. We can wrap in a `let` that returns body.
    probe_let = _probe_body_test(A, a_expr, x_sym, m_sym, b_check)
    probe_let
end

# Restrict / URestrict types are gone (tarski). Inv probe_any: lazy dict lookup
function _probe_any_body(::Type{<:Inv{B, A, Q}}, q_expr, x_sym) where {B, A, Q}
    :(haskey(Prela._inv_index($q_expr), $x_sym))
end

# LeftConj probe_any: both
function _probe_any_body(::Type{<:LeftConj{D, ML, R}}, q_expr, x_sym) where {D, ML, R}
    l = _probe_any_body(ML, :(($q_expr).l), x_sym)
    r = _probe_any_body(R, :(($q_expr).r), x_sym)
    :($l && $r)
end

# Materialized/MapRel/VecRel probe_any: bounds check (a 1:1 entity rel has
# every entity id in its domain after vectorize!)
function _probe_any_body(::Type{<:MapRel{ID_TYPE, R}}, q_expr, x_sym) where {ID_TYPE, R}
    quote
        let _r = $q_expr, _i = ($x_sym).id
            if !isempty(_r.values)
                let _s = _r.seen
                    (1 <= _i <= length(_r.values)) && (_s === nothing || @inbounds(_s[_i]))
                end
            else
                # fallback through Prela
                member(_r, $x_sym)
            end
        end
    end
end
function _probe_any_body(::Type{<:VecRel{E, R}}, q_expr, x_sym) where {E, R}
    :(let _r = $q_expr, _i = ($x_sym).id; 1 <= _i <= length(_r.values); end)
end

# Runtime fallbacks — same caveat as _drive_body fallback above.
function _probe_any_body(::Type{Q}, q_expr, x_sym) where {Q}
    :(Prela.probe_any($q_expr, $x_sym, _ -> true))
end
function _probe_body(::Type{Q}, q_expr, x_sym, y_sym, body) where {Q}
    quote
        Prela.probe($q_expr, $x_sym, $y_sym -> $body)
    end
end

# =====================================================================
# Entry point: execute(q::Scalar) — walks the type, emits a flat loop
# that calls q.op as a runtime function value. Because op's TYPE is part
# of typeof(q.op), each unique lambda gets its own specialized codegen.
# =====================================================================

@generated function execute(q::Q) where {Q <: Scalar}
    QInner = Q.parameters[2]
    body = quote
        acc = (q.op)(acc, _v)
    end
    drive = _drive_body(QInner, :(q.q), body)
    quote
        acc = q.init
        $drive
        acc
    end
end

# Count emitted (key, value) pairs — useful for benching the drive cost of a
# query that's not wrapped in Scalar (e.g. a DenseFold returned standalone).
@generated function execute_count(q::Q) where {Q}
    body = :(_count += 1)
    drive = _drive_body(Q, :q, body)
    quote
        _count = 0
        $drive
        _count
    end
end

# Sink-based drive: caller passes a function `sink(x, v)` invoked per row.
# `sink`'s type is part of the @generated dispatch, so each unique sink gets
# its own specialized version with the sink body inlined into the loop.
# This is the @generated equivalent of `Prela.drive(q, k)`.
@generated function execute_drive(q::Q, sink::F) where {Q, F}
    body = :(sink(_x, _v))
    drive = _drive_body(Q, :q, body)
    quote
        $drive
        nothing
    end
end

# Drop-in override: if `_vals_tpch` (or `_vals` for JOB) is already
# defined in Main from a prior include, redefine it to use execute_drive.
# Lets `runall_tpch()` / `runall()` take the engine path without the user
# needing to change `tpch_queries_*.jl` or `queries.jl`.
if isdefined(Main, :_vals_tpch)
    function Main._vals_tpch(q, sort_by, limit, row)
        rows = Tuple{Any, Any}[]
        execute_drive(q, (x, y) -> push!(rows, (x, y)))
        isempty(rows) && return "(empty)"
        sort!(rows; by = sort_by)
        limit !== nothing && length(rows) > limit && resize!(rows, limit)
        lines = String[]
        for (k, v) in rows
            push!(lines, join(row(k, v), "|"))
        end
        join(lines, "\n")
    end
end
if isdefined(Main, :_vals)
    # Mirror queries.jl `_vals`: per-column running min, no materialization.
    # Drives via execute_drive so the engine's flattened loop is used.
    function Main._vals(q)
        cur = Ref{Any}(nothing)
        emit = y -> begin
            fl = Main._flatten(y)
            c = cur[]
            if c === nothing
                cur[] = collect(Any, fl)
            else
                cc = c::Vector{Any}
                @inbounds for i in eachindex(cc)
                    fi = fl[i]
                    isless(fi, cc[i]) && (cc[i] = fi)
                end
            end
        end
        if Prela._rangeof(q) === Tuple{}
            execute_drive(q, (x, _) -> emit(x))
        else
            execute_drive(q, (_, y) -> emit(y))
        end
        c = cur[]
        c === nothing ? "(empty)" : join(string.(c::Vector{Any}), " || ")
    end
end
