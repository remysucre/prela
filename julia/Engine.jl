# Proof of concept: `@generated execute(q)` — walks the query type at codegen
# time and emits a flat loop. The op closure is a runtime value with concrete
# type, so Julia/LLVM inlines its body alongside the rest. Goal: match
# hand-rolled Q6 (0.05s) while preserving the algebra surface.

using Main: Prela
import Main.Prela: Query, Unary, Universe, URestrict, Restrict, Compose, Filter,
                   Conj, Disj, SetDiff, Keys, Prod, MapRel, VecRel, Scalar,
                   FnP, EqP, InP, InSetP, Map, LeftCompose, LeftConj,
                   DenseFold, Fold, Bitset, Materialized, MatSet, _denseidx,
                   member, drive, probe, probe_any

# =====================================================================
# Emission helpers (run at codegen time inside @generated; return Expr).
# Convention: per-row code uses `_x` (key) and `_v` (value). Inner probes
# use gensym'd names to avoid collisions in nested chains.
# =====================================================================

# _drive_body(QType, q_expr, body_expr) -> Expr
#   Generates a loop that walks Q's type and, per row, binds `_x` to the key
#   (and, where applicable, `_v` to the value) and runs body_expr.

function _drive_body(::Type{Universe{E}}, q_expr, body) where E
    quote
        @inbounds for _i in 1:($q_expr).n
            let _x = Prela.ID{$E}(_i)
                $body
            end
        end
    end
end

function _drive_body(::Type{<:URestrict{D, A, B}}, q_expr, body) where {D, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    pred = _probe_any_body(B, b_expr, :_x)
    wrapped = quote
        if $pred
            $body
        end
    end
    _drive_body(A, a_expr, wrapped)
end

function _drive_body(::Type{<:Restrict{D, R, A, B}}, q_expr, body) where {D, R, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    inner = _probe_body(B, b_expr, :_x, :_v, body)
    _drive_body(A, a_expr, inner)
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

# DenseFold drive: build the cache, then iterate it
function _drive_body(::Type{<:DenseFold{D, R, S, Q, OP}}, q_expr, body) where {D, R, S, Q, OP}
    q_inner = :(($q_expr).q)
    # Per-row body: i = _denseidx(_x) + 1; if in range; vals[i] = op(vals[i], _v); seen[i] = true
    inner = quote
        let _i = _denseidx(_x) + 1
            if 1 <= _i <= _sz
                @inbounds _vals[_i] = _op(_vals[_i], _v)
                @inbounds _seen[_i] = true
            end
        end
    end
    cache_build = _drive_body(Q, q_inner, inner)
    iter_body = quote
        @inbounds for _idx in eachindex(_vals)
            if _seen[_idx]
                let _x = _idx - 1, _v = _vals[_idx]
                    $body
                end
            end
        end
    end
    quote
        _sz   = ($q_expr).n + 1
        _vals = fill(($q_expr).init, _sz)
        _seen = falses(_sz)
        _op   = ($q_expr).op
        $cache_build
        $iter_body
    end
end

# =====================================================================
# _probe_body(QType, q_expr, x_sym, y_sym, body) -> Expr
#   Probes Q at x_sym, binds the value to y_sym, runs body.
# =====================================================================

function _probe_body(::Type{MapRel{ID, R}}, q_expr, x_sym, y_sym, body) where {ID, R}
    quote
        let $y_sym = @inbounds ($q_expr).values[($x_sym).id]
            $body
        end
    end
end

function _probe_body(::Type{VecRel{E, R}}, q_expr, x_sym, y_sym, body) where {E, R}
    quote
        let $y_sym = @inbounds ($q_expr).values[($x_sym).id]
            $body
        end
    end
end

# Map probe: probe(q, x, k) = probe(inner, x, v -> k(f(v)))
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
    sub_ys = [gensym(:_yp) for _ in 1:N]
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

function _probe_any_body(::Type{<:Conj{D, A, B}}, q_expr, x_sym) where {D, A, B}
    a_expr = :(($q_expr).a)
    b_expr = :(($q_expr).b)
    a_check = _probe_any_body(A, a_expr, x_sym)
    b_check = _probe_any_body(B, b_expr, x_sym)
    :($a_check && $b_check)
end

# Filter with FnP{F} closure predicate — covers <, >, <=, >=, in interval, etc.
function _probe_any_body(::Type{<:Filter{D, R, A, FnP{F}}}, q_expr, x_sym) where {D, R, A, F}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_yf)
    val_test = :(($q_expr).pred.f($y_sym))
    _probe_body(A, a_expr, x_sym, y_sym, val_test)
end

function _probe_any_body(::Type{<:Filter{D, R, A, EqP{V}}}, q_expr, x_sym) where {D, R, A, V}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_ye)
    test = :(isequal($y_sym, ($q_expr).pred.v))
    _probe_body(A, a_expr, x_sym, y_sym, test)
end

function _probe_any_body(::Type{<:Filter{D, R, A, InP{T}}}, q_expr, x_sym) where {D, R, A, T}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_yi)
    test = :($y_sym in ($q_expr).pred.vs)
    _probe_body(A, a_expr, x_sym, y_sym, test)
end

# Filter with InSetP{S} — value must be a member of the set S.
function _probe_any_body(::Type{<:Filter{D, R, A, InSetP{S}}}, q_expr, x_sym) where {D, R, A, S}
    a_expr = :(($q_expr).a)
    y_sym = gensym(:_yis)
    # member(set, y) is the engine API for set-membership; for Bitset/Keys/etc.
    # we could emit specialized code, but the fallback is fine because `member`
    # itself is @inline and goes to probe_any.
    test = :(member(($q_expr).pred.s, $y_sym))
    _probe_body(A, a_expr, x_sym, y_sym, test)
end

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

# SetDiff — in A but not in B.
function _probe_any_body(::Type{<:SetDiff{D, A, B}}, q_expr, x_sym) where {D, A, B}
    a = _probe_any_body(A, :(($q_expr).a), x_sym)
    b = _probe_any_body(B, :(($q_expr).b), x_sym)
    :($a && !$b)
end

# MatSet.probe_any — fall back to the runtime set lookup. MatSet builds its
# internal `Set{D}` lazily on first call, so it's fine to call `member` here.
function _probe_any_body(::Type{<:MatSet{D, A}}, q_expr, x_sym) where {D, A}
    :(member($q_expr, $x_sym))
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
