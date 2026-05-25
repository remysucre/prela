module Prela

# Core algebraic-relational library — TOP-DOWN (lazy, CPS-compiled) edition.
#
# Operators build a typed query tree (the whole plan lives in the type);
# `drive`/`probe`/`drivekeys`/`member` form a CPS protocol that fuses the tree
# into a loop nest via Julia's monomorphization + inlining. Nothing executes
# until a folding terminal (`drive`/`drivekeys`) supplies the outermost
# continuation.
#
#   drive(q, k)        — call k(x, y) for every pair q produces
#   probe(q, x, k)     — call k(y) for every y related to key x
#   drivekeys(s, k)    — call k(x) per member of a set-query
#   member(s, x)::Bool — domain/membership test
#
# Operators (low→high precedence):
#   →  composition  | ∨ union | ∧ intersection | ==,<,~,…  predicates
#   ×  product (tightest) | -  difference | .field navigation

export Rel, MapRel, VecRel, Relation, Query, SetQ, Unary, Universe, Entity, ID,
       primary, lookup_field, →, ∧, ∨, ×, ≁, vectorize,
       drive, probe, drivekeys, member, materialize

abstract type Entity end

# Phantom-typed entity ID.
struct ID{E <: Entity}
    id::Int
end
Base.:(==)(a::ID{E}, b::ID{E}) where E = a.id == b.id
Base.hash(a::ID, h::UInt) = hash(a.id, h)
Base.isless(a::ID{E}, b::ID{E}) where E = a.id < b.id
Base.show(io::IO, a::ID{E}) where E = print(io, nameof(E), "(", a.id, ")")

function primary end
function lookup_field end

const _ENTITY_FIELDS = Dict{Symbol, Vector{Symbol}}()

function _declare_if_needed(mod::Module, sym::Symbol)
    isdefined(mod, sym) && return
    Core.eval(mod, Expr(:abstract, Expr(:(<:), sym, GlobalRef(@__MODULE__, :Entity))))
end

# ===== query-tree type hierarchy ========================================
# `Query{D, R}` — a lazy binary relation D → R.
# `SetQ{D}`     — a lazy unary set over D.

abstract type Query{D, R} end
abstract type SetQ{D} end

_domof(::Query{D, R}) where {D, R} = D
_domof(::SetQ{D}) where {D} = D
_rangeof(::Query{D, R}) where {D, R} = R

# ===== leaf storage (also Query nodes) ==================================

struct Unary{D} <: SetQ{D}
    values::Vector{D}
end
Unary(vs::Vector{D}) where D = Unary{D}(vs)

# A dense primary-key universe ID{E}(1)..ID{E}(n) — stored as just `n`. The
# entity tables have contiguous PKs, so "scanning the universe" is iterating a
# range, with no N-element vector to hold or chase.
struct Universe{E} <: SetQ{ID{E}}
    n::Int
end

mutable struct MapRel{D, R} <: Query{D, R}
    pairs::Vector{Pair{D, R}}
    # forward index: dense Vector{Vector{R}} keyed by .id when D is ID{E}
    # (entity PKs are contiguous → array access, not a hash), else a Dict.
    fwd::Union{Nothing, Vector{Vector{R}}, Dict{D, Vector{R}}}
    inv::Union{Nothing, Dict{R, Vector{D}}}   # inverse index, built lazily
end
MapRel{D, R}(ps::Vector{Pair{D, R}}) where {D, R} = MapRel{D, R}(ps, nothing, nothing)
MapRel(ps::Vector{Pair{D, R}}) where {D, R} = MapRel{D, R}(ps, nothing, nothing)

struct VecRel{E, R} <: Query{ID{E}, R}
    values::Vector{R}
end
VecRel(::Type{E}, vs::Vector{R}) where {E, R} = VecRel{E, R}(vs)

const Rel = MapRel
const Relation = Query           # cache.jl refers to `Prela.Relation`

Base.length(r::MapRel) = length(r.pairs)
Base.length(r::VecRel) = length(r.values)
Base.isempty(r::MapRel) = isempty(r.pairs)
Base.isempty(r::VecRel) = isempty(r.values)

_pairs(r::MapRel) = r.pairs
_pairs(r::VecRel{E}) where E = (ID{E}(i) => r.values[i] for i in eachindex(r.values))

function vectorize(r::MapRel{ID{E}, R}, n::Int) where {E, R}
    vals = Vector{R}(undef, n)
    seen = falses(n)
    for p in r.pairs
        i = p.first.id
        (1 <= i <= n) || error("ID $i out of dense range 1..$n")
        seen[i] && error("duplicate ID $i — not a function, can't vectorize")
        vals[i] = p.second
        seen[i] = true
    end
    all(seen) || error("MapRel is sparse over 1..$n: only $(count(seen))/$n filled")
    VecRel{E, R}(vals)
end

# ===== leaf indexes =====================================================
# Each leaf carries its own forward/inverse index, built lazily on first use
# and then read as a plain field — so a top-down probe, which calls fwd_index
# once per row, never allocates or locks on the hot path. (For a parallel
# `runall` these fields would become `@atomic` with a double-checked lock;
# single-threaded they need neither.)

const _LEAF_RELS = Base.IdSet{Any}()      # populated by @entity; kept for compat
const _UNARY_SETS = IdDict{Any, Any}()

# Dense forward index: for an entity-keyed relation (contiguous PK 1..n) the
# index is a Vector{Vector{R}} addressed by `.id` — an array access per probe,
# no hashing. Unfilled slots share one empty vector.
function _dense_fwd(pairs::Vector{Pair{ID{E}, R}}) where {E, R}
    n = 0
    for p in pairs
        i = p.first.id
        i > n && (n = i)
    end
    empty = R[]
    v = fill(empty, n)
    for p in pairs
        i = p.first.id
        i < 1 && continue          # junk pair → nonexistent entity (id ≤ 0)
        @inbounds vi = v[i]
        vi === empty && (vi = R[]; @inbounds v[i] = vi)
        push!(vi, p.second)
    end
    v
end

function _build_fwd(s::Query{Y, Z}) where {Y, Z}
    d = Dict{Y, Vector{Z}}()
    sizehint!(d, length(s))
    for p in _pairs(s)
        push!(get!(() -> Z[], d, p.first), p.second)
    end
    d
end

function _build_inv(s::Query{Y, Z}) where {Y, Z}
    d = Dict{Z, Vector{Y}}()
    sizehint!(d, length(s))
    for p in _pairs(s)
        push!(get!(() -> Y[], d, p.second), p.first)
    end
    d
end

# entity-keyed leaf → dense array index; other domains → Dict.
function fwd_index(r::MapRel{ID{E}, R}) where {E, R}
    f = r.fwd
    f === nothing || return f::Vector{Vector{R}}
    d = _dense_fwd(r.pairs)
    r.fwd = d
    d
end
function fwd_index(r::MapRel{D, R}) where {D, R}
    f = r.fwd
    f === nothing || return f::Dict{D, Vector{R}}
    d = _build_fwd(r)
    r.fwd = d
    d
end

function inv_index(r::MapRel)
    v = r.inv
    v === nothing || return v
    r.inv = _build_inv(r)
end
inv_index(s::Query) = _build_inv(s)       # VecRel etc. — rare, uncached

# Uniform per-key access over either index representation.
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

_unary_set(u::Unary{D}) where D = get!(() -> Set(u.values), _UNARY_SETS, u)::Set{D}

# ===== predicate payloads (typed so codegen branches statically) ========

struct EqP{V};  v::V;  end          # == val
struct InP{T};  vs::T;  end          # in (tuple of vals)
struct FnP{F};  f::F;  end          # any unary y -> Bool  (< > <= >= != ~ ≁)
struct InSetP{S};  s::S;  end        # value ∈ a SetQ

# ===== query nodes ======================================================

struct Compose{D, M, R, A, B} <: Query{D, R};  a::A;  b::B;  end
struct Filter{D, R, A, P}     <: Query{D, R};  a::A;  pred::P;  end
struct Restrict{D, R, A, B}   <: Query{D, R};  a::A;  b::B;  end   # a:SetQ, b:Query
struct Diff{D, R, A, B}       <: Query{D, R};  a::A;  b::B;  end   # a:Query, b:SetQ
struct Prod{D, R, T<:Tuple}   <: Query{D, R};  ops::T;  end

struct Keys{D, A}    <: SetQ{D};  a::A;  end                       # Query → SetQ
struct Conj{D, A, B} <: SetQ{D};  a::A;  b::B;  end
struct Disj{D, A, B} <: SetQ{D};  a::A;  b::B;  end
struct SetDiff{D, A, B} <: SetQ{D};  a::A;  b::B;  end

# `materialize(q)` — the one explicit "bang". Prela is top-down / non-
# materialized by default: a shared subexpression is re-driven on every use.
# Wrapping it in `materialize(...)` evaluates it once into a stored vector +
# hash index — materialize-once / probe-many. The bushy-plan building block:
# wrap each selective non-driving leg in `materialize(...)` and the author gets
# a hand-picked bushy hash-join plan (cf. ../ttj-rs, which materializes every
# leg). Lazy: the materialization fires on first drive/probe, in demand order.
mutable struct Materialized{D, R, A} <: Query{D, R}
    a::A
    mat::Union{Nothing, Vector{Pair{D, R}}}
    idx::Union{Nothing, Vector{Vector{R}}, Dict{D, Vector{R}}}
end

# `materialize` on a set-query: evaluate once into a vector + membership set.
mutable struct MatSet{D, A} <: SetQ{D}
    a::A
    keys::Union{Nothing, Vector{D}}
    set::Union{Nothing, Set{D}}
end

# `Inv(q)` — invert a relation. `q : A → B` becomes `Inv(q) : B → A`.
# Surface syntax is postfix adjoint `q'`. Streaming-only: `drive(Inv, k)` flips
# pairs from `drive(q, …)` without any materialization. For fast probe/member
# access, wrap explicitly via `materialize(q')` (or `!q'`).
struct Inv{B, A, Q} <: Query{B, A}
    q::Q
end

# `Fold(q, op, init)` — per-key foldl aggregation. `q : D → R`, the inner
# is grouped by D on the fly (it emits (key, value) pairs many-to-one);
# per key we foldl `op` over the values starting from `init`. Mutable +
# lazy-cached so the same Fold can be referenced multiple times (e.g. by
# both a sum and the mean built from sum/count) without re-aggregating.
mutable struct Fold{D, R, S, Q, OP} <: Query{D, S}
    q::Q
    op::OP
    init::S
    cache::Union{Nothing, Dict{D, S}}
end

# `Map(q, f)` — generalized projection (per-row lambda). `q : D → R` with
# `f : R → S` becomes `Map(q, f) : D → S`. The function `f` runs per emitted
# row; no aggregation, no caching needed.
struct Map{D, R, S, Q, F} <: Query{D, S}
    q::Q
    f::F
end

# `LeftCompose(r, s)` — for `r : D → R` and `s : D → S` (same domain),
# produces `Query{R, S}`. Surface syntax `r ← s`. Driven by walking `s`
# and probing `r` per row — distinct from `r' → s` which walks `r` and
# probes `s`. Use `←` when `s` is the "natural" thing to scan (e.g. a
# filtered universe + measure projection in SQL-table-scan style) and
# the group-key extractor `r` is cheap to probe per row.
struct LeftCompose{D, RK, SV, QR, QS} <: Query{RK, SV}
    r::QR
    s::QS
end

# `LeftConj(l, r)` — left-driving conjunction. `l ⩓ r` materializes `l`
# (via `materialize(askeys(l))`) so its `member` is O(1), then drives `r`
# and member-checks `l` per row. Lets a user-written `∧`-style expression
# put a Query-shaped predicate (like an `Inv` for EXISTS) on the left
# without needing an explicit `!` — the operator does the materialization.
struct LeftConj{D, ML, R} <: SetQ{D}
    l::ML  # already materialized SetQ (MatSet) — fast member
    r::R   # SetQ to drive
end

# constructors — extract D/M/R via dispatch
Compose(a::Query{D, M}, b::Query{M, R}) where {D, M, R} =
    Compose{D, M, R, typeof(a), typeof(b)}(a, b)
Filter(a::Query{D, R}, p::P) where {D, R, P} =
    Filter{D, R, typeof(a), P}(a, p)
Restrict(a::SetQ{D}, b::Query{D, R}) where {D, R} =
    Restrict{D, R, typeof(a), typeof(b)}(a, b)
Diff(a::Query{D, R}, b::SetQ{D}) where {D, R} =
    Diff{D, R, typeof(a), typeof(b)}(a, b)
Keys(a::Query{D, R}) where {D, R} = Keys{D, typeof(a)}(a)
Conj(a::SetQ{D}, b::SetQ{D}) where D = Conj{D, typeof(a), typeof(b)}(a, b)
Disj(a::SetQ{D}, b::SetQ{D}) where D = Disj{D, typeof(a), typeof(b)}(a, b)
SetDiff(a::SetQ{D}, b::SetQ{D}) where D = SetDiff{D, typeof(a), typeof(b)}(a, b)
function Prod(ops::Tuple)
    D = _domof(ops[1])
    R = Tuple{map(_rangeof, ops)...}
    Prod{D, R, typeof(ops)}(ops)
end
materialize(q::Query{D, R}) where {D, R} = Materialized{D, R, typeof(q)}(q, nothing, nothing)
materialize(s::SetQ{D}) where {D} = MatSet{D, typeof(s)}(s, nothing, nothing)

# Adjoint = inverse: `q'` on a Query{A, B} returns Inv : Query{B, A}.
Base.adjoint(q::Query{A, B}) where {A, B} = Inv{B, A, typeof(q)}(q)

# `▷` — per-key foldl. Pass `(op, init)` as a 2-tuple on the rhs.
# `q ▷ (+, 0.0)` is sum; `q ▷ ((a, _) -> a + 1, 0)` is count; arbitrary
# `(S, R) → S` reductions supported. Free function, no getproperty overload.
function ▷(q::Query{D, R}, opinit::Tuple{OP, S}) where {D, R, OP, S}
    Fold{D, R, S, typeof(q), OP}(q, opinit[1], opinit[2], nothing)
end
export ▷

# `↦` — per-row Map (apply a Julia function to the value, key unchanged).
# `q ↦ (v -> f(v))` produces `Map(q, f) : Query{D, S}` where `S` is the
# inferred return type. Used for post-aggregation arithmetic (mean = sum / cnt,
# ratios, etc.) without leaving the algebra.
function ↦(q::Query{D, R}, f::F) where {D, R, F<:Function}
    S = Core.Compiler.return_type(f, Tuple{R})
    S === Union{} && (S = Any)
    Map{D, R, S, typeof(q), F}(q, f)
end
export ↦

# `←` — left compose. `r ← s` builds `LeftCompose(r, s) : Query{R, S}`
# where both r and s have the same domain D. Drives `s`, probes `r` per
# row. Distinct from `r' → s` (which drives r, probes s) — use `←` when
# the source you want to scan is on the right (e.g. a filtered universe
# with measures), and `r' → s` when the source is the left side.
function ←(r::Query{D, RK}, s::Query{D, SV}) where {D, RK, SV}
    LeftCompose{D, RK, SV, typeof(r), typeof(s)}(r, s)
end
export ←

# `⩓` — left-driving conjunction. `l ⩓ r` (where both reduce to a SetQ
# via askeys) materializes `l` for fast member-check, then at drive time
# drives `r` and member-checks `l` per row. Use when the natural driver
# of an intersection is on the right (e.g. a Universe-rooted filter chain)
# and the left side is a derived Query (e.g. an inverted relation) that
# you want to use as a fast EXISTS check.
function ⩓(l, r)
    ml = materialize(askeys(l))   # → MatSet, O(1) member via Set
    rs = askeys(r)
    LeftConj{_domof(rs), typeof(ml), typeof(rs)}(ml, rs)
end
export ⩓

# Prefix `!` is the terse spelling of `materialize` — `!(q)` ≡ `materialize(q)`.
# Borrowed from Haskell's strictness bang; a query has no boolean-not, so `!`
# is free to mean "force this leg".
Base.:!(q::Query) = materialize(q)
Base.:!(s::SetQ) = materialize(s)

askeys(q::Query) = Keys(q)
askeys(s::SetQ) = s

# ===== operators (build nodes) ==========================================
# Navigation is `→` only — `q.field` overloads on Query/Unary were removed
# (use `q → Type.field` instead). `Entity.field` (e.g. `Company.country`)
# still works via the `@entity`-generated `Base.getproperty(::Type{E}, ...)`.

# → composition / restriction / intersection
→(a::Query{X, Y}, b::Query{Y, Z}) where {X, Y, Z} = Compose(a, b)
→(a::SetQ{X},     b::Query{X, Z}) where {X, Z}    = Restrict(a, b)
→(a::SetQ{X},     b::SetQ{X})     where {X}       = Conj(a, b)
→(a::Query{X, Y}, b::SetQ{Y})     where {X, Y}    = Filter(a, InSetP(b))

# ∧ ∨ : - ×
∧(a, b) = Conj(askeys(a), askeys(b))
∨(a, b) = Disj(askeys(a), askeys(b))
Base.:(:)(a, b::Query) = Restrict(askeys(a), b)
Base.:-(a::Query{D, R}, b) where {D, R} = Diff(a, askeys(b))
Base.:-(a::SetQ{D},     b) where {D}    = SetDiff(a, askeys(b))
×(a::Query, b::Query) = Prod((a, b))
×(a::Prod,  b::Query) = Prod((a.ops..., b))

# predicates — scalar range (value-vs-constant)
Base.:(==)(q::Query{D, R}, val) where {D, R} = Filter(q, EqP(val))
Base.in(q::Query{D, R}, vals::Tuple) where {D, R} = Filter(q, InP(vals))
for op in (:(<), :(>), :(<=), :(>=), :(!=))
    @eval Base.$op(q::Query{D, R}, val) where {D, R} = Filter(q, FnP(Base.Fix2($op, val)))
end

# predicates — cross-column (Query-vs-Query, same domain). Comparing two
# leaves of the same row is `Filter(a × b, FnP(((x, y),) -> op(x, y)))`;
# this overload makes that the natural spelling, e.g.
#   Lineitem.commitdate < Lineitem.receiptdate
#   Customer.nation == Supplier.nation       (when composed onto the same domain)
for op in (:(<), :(>), :(<=), :(>=), :(==), :(!=))
    @eval Base.$op(a::Query{D, X}, b::Query{D, Y}) where {D, X, Y} =
        Filter(Prod((a, b)), FnP(((x, y),) -> $op(x, y)))
    # Specific override for entity-typed columns on both sides: compares the
    # entity IDs directly (no primary-field elision). Resolves the ambiguity
    # between the cross-column overload above and the scalar entity-elision
    # overload below.
    @eval Base.$op(a::Query{D, ID{E}}, b::Query{D, ID{E}}) where {D, E} =
        Filter(Prod((a, b)), FnP(((x, y),) -> $op(x, y)))
end
Base.:~(q::Query{D, R}, re::Regex) where {D, R <: AbstractString} =
    Filter(q, FnP(Base.Fix1(occursin, re)))
≁(q::Query{D, R}, re::Regex) where {D, R <: AbstractString} =
    Filter(q, FnP(s -> !occursin(re, s)))

# predicates — entity range: elide through the primary field
Base.:(==)(q::Query{D, ID{E}}, val) where {D, E} = Compose(q, primary(E)) == val
Base.in(q::Query{D, ID{E}}, vals::Tuple) where {D, E} = in(Compose(q, primary(E)), vals)
for op in (:(<), :(>), :(<=), :(>=), :(!=))
    @eval Base.$op(q::Query{D, ID{E}}, val) where {D, E} = $op(Compose(q, primary(E)), val)
end
Base.:~(q::Query{D, ID{E}}, re::Regex) where {D, E} = Compose(q, primary(E)) ~ re
≁(q::Query{D, ID{E}}, re::Regex) where {D, E} = ≁(Compose(q, primary(E)), re)

# ===== CPS execution protocol ===========================================
# drive(q,k): k(x,y) per pair    probe(q,x,k): k(y) per value at x
# drivekeys(s,k): k(x) per member    member(s,x)::Bool

# ---- leaves ----
@inline function drive(r::MapRel, k)
    for p in r.pairs
        k(p.first, p.second)
    end
end
@inline probe(r::MapRel, x, k) = _idx_probe(fwd_index(r), x, k)
@inline function drive(r::VecRel{E, R}, k) where {E, R}
    v = r.values
    @inbounds for i in eachindex(v)
        k(ID{E}(i), v[i])
    end
end
@inline probe(r::VecRel{E, R}, x::ID{E}, k) where {E, R} =
    (@inbounds k(r.values[x.id]); nothing)

# ---- Compose: the loop nest ----
@inline drive(n::Compose, k) = drive(n.a, (x, m) -> probe(n.b, m, r -> k(x, r)))
@inline probe(n::Compose, x, k) = probe(n.a, x, m -> probe(n.b, m, r -> k(r)))

# ---- Filter ----
@inline drive(n::Filter{D,R,A,<:FnP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> n.pred.f(y) && k(x, y))
@inline probe(n::Filter{D,R,A,<:FnP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> n.pred.f(y) && k(y))

@inline probe(n::Filter{D,R,A,<:EqP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> isequal(y, n.pred.v) && k(y))
@inline drive(n::Filter{D,R,A,<:EqP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> isequal(y, n.pred.v) && k(x, y))
# driving-mode for `==` on a leaf: jump to matches via inv_index
function drive(n::Filter{D,R,<:MapRel,<:EqP}, k) where {D,R}
    xs = get(inv_index(n.a), n.pred.v, nothing)
    xs === nothing && return
    for x in xs; k(x, n.pred.v); end
end
function drive(n::Filter{D,R,<:VecRel,<:EqP}, k) where {D,R}
    xs = get(inv_index(n.a), n.pred.v, nothing)
    xs === nothing && return
    for x in xs; k(x, n.pred.v); end
end

@inline probe(n::Filter{D,R,A,<:InP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> (y in n.pred.vs) && k(y))
@inline drive(n::Filter{D,R,A,<:InP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> (y in n.pred.vs) && k(x, y))

@inline probe(n::Filter{D,R,A,<:InSetP}, x, k) where {D,R,A} =
    probe(n.a, x, y -> member(n.pred.s, y) && k(y))
@inline drive(n::Filter{D,R,A,<:InSetP}, k) where {D,R,A} =
    drive(n.a, (x, y) -> member(n.pred.s, y) && k(x, y))

# ---- Restrict (a:SetQ : b:Query) — drive the keys, probe the rel ----
@inline drive(n::Restrict, k) = drivekeys(n.a, x -> probe(n.b, x, y -> k(x, y)))
@inline probe(n::Restrict, x, k) = member(n.a, x) && probe(n.b, x, k)

# ---- Diff (a:Query - b:SetQ) ----
@inline drive(n::Diff, k) = drive(n.a, (x, y) -> member(n.b, x) || k(x, y))
@inline probe(n::Diff, x, k) = member(n.b, x) || probe(n.a, x, k)

# ---- Prod (n-ary ×) ----
@inline _pp(::Tuple{}, x, acc, k) = k(acc)
@inline _pp(ops::Tuple, x, acc, k) =
    probe(ops[1], x, y -> _pp(Base.tail(ops), x, (acc..., y), k))
@inline probe(n::Prod, x, k) = _pp(n.ops, x, (), k)
@inline drive(n::Prod, k) =
    drive(n.ops[1], (x, y1) -> _pp(Base.tail(n.ops), x, (y1,), t -> k(x, t)))

# ---- Materialized: materialize once, then serve from vector + hash index ----
# `A` (the inner query type) is named explicitly so the method specializes on
# it — otherwise `n.a` is abstract and the materializing drive boxes per row.
function _cmat(n::Materialized{D, R, A}) where {D, R, A}
    m = n.mat
    m === nothing || return m::Vector{Pair{D, R}}
    out = Pair{D, R}[]
    drive(n.a, (x, y) -> push!(out, x => y))
    n.mat = out
    out
end
function _cidx(n::Materialized{ID{E}, R, A}) where {E, R, A}
    f = n.idx
    f === nothing || return f::Vector{Vector{R}}
    d = _dense_fwd(_cmat(n))
    n.idx = d
    d
end
function _cidx(n::Materialized{D, R, A}) where {D, R, A}
    f = n.idx
    f === nothing || return f::Dict{D, Vector{R}}
    d = Dict{D, Vector{R}}()
    for p in _cmat(n)
        push!(get!(() -> R[], d, p.first), p.second)
    end
    n.idx = d
    d
end
@inline function drive(n::Materialized, k)
    for p in _cmat(n)
        k(p.first, p.second)
    end
end
@inline probe(n::Materialized, x, k) = _idx_probe(_cidx(n), x, k)

# ---- Inv: streaming-only. drive flips pairs; no probe/member without ! ----
# `probe`/`member`/`drivekeys` are not directly supported — wrap the Inv in
# `materialize(...)` (or use `!q'`) to get an indexed copy that supports
# them. The default streaming path keeps Inv allocation-free.
@inline drive(n::Inv, k) = drive(n.q, (a, b) -> k(b, a))

# ---- LeftCompose: drive s, probe r per row ----
# `r ← s` semantically equals `r' ∘ s` but flips which side scans. Drives
# `s` (the natural source — e.g. a filtered table scan) and probes `r` per
# row to compute the would-be group key. Designed to feed `▷`.
@inline function drive(n::LeftCompose, k)
    drive(n.s, (d, v) -> probe(n.r, d, rk -> k(rk, v)))
end
@inline function probe(n::LeftCompose{D, RK, SV}, rk, k) where {D, RK, SV}
    drive(n.s, (d, v) -> probe(n.r, d, x -> isequal(x, rk) && k(v)))
end

# ---- LeftConj: drive r, member-check materialized l ----
@inline drivekeys(n::LeftConj, k) = drivekeys(n.r, x -> member(n.l, x) && k(x))
@inline member(n::LeftConj, x) = member(n.l, x) && member(n.r, x)

# ---- Fold: per-key foldl, lazy-cached ----
function _fold_cache(n::Fold{D, R, S, Q, OP}) where {D, R, S, Q, OP}
    n.cache === nothing || return n.cache::Dict{D, S}
    acc = Dict{D, S}()
    drive(n.q, (d, v) -> (acc[d] = n.op(get(acc, d, n.init), v)))
    n.cache = acc
end
@inline function drive(n::Fold{D, R, S, Q, OP}, k) where {D, R, S, Q, OP}
    for (d, s) in _fold_cache(n)
        k(d, s)
    end
end
@inline function probe(n::Fold{D, R, S, Q, OP}, d, k) where {D, R, S, Q, OP}
    s = get(_fold_cache(n), d, nothing)
    s === nothing && return
    k(s)
end

# ---- Map: per-row lambda ----
@inline drive(n::Map, k) = drive(n.q, (d, v) -> k(d, n.f(v)))
@inline probe(n::Map, d, k) = probe(n.q, d, v -> k(n.f(v)))

function _mkeys(n::MatSet{D}) where {D}
    if n.keys === nothing
        out = D[]
        drivekeys(n.a, x -> push!(out, x))
        n.keys = out
    end
    n.keys
end
function _mset(n::MatSet{D}) where {D}
    n.set === nothing && (n.set = Set(_mkeys(n)))
    n.set
end
@inline drivekeys(n::MatSet, k) = (for x in _mkeys(n); k(x); end)
@inline member(n::MatSet, x) = x in _mset(n)

# ---- SetQ: drivekeys + member ----
@inline drivekeys(u::Unary, k) = (for v in u.values; k(v); end)
@inline member(u::Unary, x) = x in _unary_set(u)

@inline drivekeys(u::Universe{E}, k) where {E} = (for i in 1:u.n; k(ID{E}(i)); end)
@inline member(u::Universe{E}, x::ID{E}) where {E} = 1 <= x.id <= u.n

@inline drivekeys(s::Keys, k) = drive(s.a, (x, _) -> k(x))
@inline member(s::Keys, x) = member(s.a, x)

@inline drivekeys(s::Conj, k) = drivekeys(s.a, x -> member(s.b, x) && k(x))
@inline member(s::Conj, x) = member(s.a, x) && member(s.b, x)

@inline drivekeys(s::Disj, k) = begin
    drivekeys(s.a, k)
    drivekeys(s.b, x -> member(s.a, x) || k(x))
end
@inline member(s::Disj, x) = member(s.a, x) || member(s.b, x)

@inline drivekeys(s::SetDiff, k) = drivekeys(s.a, x -> member(s.b, x) || k(x))
@inline member(s::SetDiff, x) = member(s.a, x) && !member(s.b, x)

# `probe_any(q, x, k)` — like `probe`, but the continuation returns a Bool and
# `probe_any` stops, returning true, as soon as `k` does. The Bool is threaded
# through return values (no mutable cell) so the whole chain is allocation-free
# when inlined — this is the hot path for `member` on a driven stream.
@inline probe_any(r::MapRel, x, k) = _idx_probe_any(fwd_index(r), x, k)
@inline probe_any(r::VecRel{E, R}, x::ID{E}, k) where {E, R} =
    k(@inbounds r.values[x.id])
@inline probe_any(n::Compose, x, k) = probe_any(n.a, x, m -> probe_any(n.b, m, k))
@inline probe_any(n::Filter{D,R,A,<:FnP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> n.pred.f(y) && k(y))
@inline probe_any(n::Filter{D,R,A,<:EqP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> isequal(y, n.pred.v) && k(y))
@inline probe_any(n::Filter{D,R,A,<:InP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> (y in n.pred.vs) && k(y))
@inline probe_any(n::Filter{D,R,A,<:InSetP}, x, k) where {D,R,A} =
    probe_any(n.a, x, y -> member(n.pred.s, y) && k(y))
@inline probe_any(n::Restrict, x, k) = member(n.a, x) && probe_any(n.b, x, k)
@inline probe_any(n::Diff, x, k) = (!member(n.b, x)) && probe_any(n.a, x, k)
@inline probe_any(n::Materialized, x, k) = _idx_probe_any(_cidx(n), x, k)
# generic fallback (Prod and other shapes) — no early exit, rarely on hot paths
function probe_any(q::Query, x, k)
    found = Ref(false)
    probe(q, x, y -> (k(y) && (found[] = true)))
    found[]
end

# member of a Query = "is x in its domain".
@inline member(q::Query, x) = probe_any(q, x, _ -> true)

# ===== terminals ========================================================
# Queries are consumed by `drive`/`drivekeys` with a folding continuation
# (see `_vals` in queries.jl) — no result relation is ever built. `collect`
# is the convenience terminal for the REPL: drive a query into a concrete Rel.

function Base.collect(q::Query{D, R}) where {D, R}
    out = Pair{D, R}[]
    drive(q, (x, y) -> push!(out, x => y))
    MapRel{D, R}(out)
end
function Base.collect(s::SetQ{D}) where D
    out = D[]
    drivekeys(s, x -> push!(out, x))
    Unary{D}(out)
end

# ===== schema sugar (@entity / @declare / @expose) ======================

macro entity(entity_sym, block)
    entity_sym isa Symbol || error("@entity expects a symbol entity name")
    (block isa Expr && block.head === :block) || error("@entity expects `begin ... end`")

    out = Expr(:block)
    push!(out.args, :($(GlobalRef(@__MODULE__, :_declare_if_needed))(@__MODULE__, $(QuoteNode(entity_sym)))))

    id_type    = :($(GlobalRef(@__MODULE__, :ID)){$(esc(entity_sym))})
    rel_type   = GlobalRef(@__MODULE__, :MapRel)
    lookup_fn  = GlobalRef(@__MODULE__, :lookup_field)
    primary_fn = GlobalRef(@__MODULE__, :primary)

    field_names = Symbol[]
    field_consts = Symbol[]
    for stmt in block.args
        stmt isa LineNumberNode && continue
        if stmt isa Expr && stmt.head === :(::)
            field_sym  = stmt.args[1]
            range_expr = stmt.args[2]
            qual_sym = Symbol("_", entity_sym, "_", field_sym)
            push!(field_names, field_sym)
            push!(field_consts, qual_sym)
            push!(out.args, quote
                const $(esc(qual_sym)) = $rel_type{$id_type, $(esc(range_expr))}(
                    Pair{$id_type, $(esc(range_expr))}[]
                )
                $lookup_fn(::Type{$id_type}, ::Val{$(QuoteNode(field_sym))}) = $(esc(qual_sym))
                push!($(GlobalRef(@__MODULE__, :_LEAF_RELS)), $(esc(qual_sym)))
            end)
        else
            error("@entity: unsupported statement $stmt; expected `name :: Type`")
        end
    end

    if !isempty(field_names)
        push!(out.args, quote
            $primary_fn(::Type{$(esc(entity_sym))}) = $(esc(field_consts[1]))
        end)
        push!(out.args, :(
            $(GlobalRef(@__MODULE__, :_ENTITY_FIELDS))[$(QuoteNode(entity_sym))] =
                $(field_names)
        ))
        gp_body = Expr(:block)
        for (fname, fconst) in zip(field_names, field_consts)
            push!(gp_body.args, :(name === $(QuoteNode(fname)) && return $(esc(fconst))))
        end
        push!(gp_body.args, :(return getfield($(esc(entity_sym)), name)))
        push!(out.args, :(
            Base.getproperty(::Type{$(esc(entity_sym))}, name::Symbol) = $gp_body
        ))
        push!(out.args, :(
            Base.nameof(::Type{$(esc(entity_sym))}) = $(QuoteNode(entity_sym))
        ))
        push!(out.args, :(
            Base.show(io::IO, ::Type{$(esc(entity_sym))}) = print(io, $(string(entity_sym)))
        ))
    end
    out
end
export @entity

macro declare(syms...)
    out = Expr(:block)
    for s in syms
        s isa Symbol || error("@declare expects symbols")
        push!(out.args, :($(GlobalRef(@__MODULE__, :_declare_if_needed))(@__MODULE__, $(QuoteNode(s)))))
    end
    out
end
export @declare

macro expose(entity_sym)
    entity_sym isa Symbol || error("@expose expects a symbol entity name")
    haskey(_ENTITY_FIELDS, entity_sym) ||
        error("@expose: no @entity declaration found for `$entity_sym`")
    out = Expr(:block)
    for f in _ENTITY_FIELDS[entity_sym]
        qual_sym = Symbol("_", entity_sym, "_", f)
        push!(out.args, :(const $(esc(f)) = $(esc(qual_sym))))
    end
    out
end
export @expose

end # module
