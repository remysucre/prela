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

export Rel, MapRel, VecRel, SparseRel, MultiRel, Multi, Relation, Query, Unary,
       UnaryVec, Universe, Entity, ID,
       primary, lookup_field, →, ∧, ∨, ×, ≁, seal_entities!,
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
# (entity, field) pairs declared `Multi{…}` in @entity — sealed to MultiRel.
const _MULTI_FIELDS = Set{Tuple{Symbol, Symbol}}()

function _declare_if_needed(mod::Module, sym::Symbol)
    isdefined(mod, sym) && return
    Core.eval(mod, Expr(:abstract, Expr(:(<:), sym, GlobalRef(@__MODULE__, :Entity))))
end

# ===== query-tree type hierarchy ========================================
# Every node is a `Query{D, R}` — a lazy binary relation D → R. `Unary{D}`
# is the abstract marker for *identity* relations `D → D`, the home of
# leaf set-shaped things (Universe, UnaryVec) and of Booleanesque nodes
# whose value side is just the key (Disj, MatSet, Bitset, LeftConj). The
# old `() → T` encoding is gone: a "unary" emits `(x, x)` not `(x, ())`,
# so it composes with `←` and `→` without a special unary-on-right path.
# Restriction never projects a value-bearing query to a "keyset" node —
# downstream `member`/`probe_any` already test domain membership directly.

abstract type Query{D, R} end
abstract type Unary{D} <: Query{D, D} end

_domof(::Query{D, R}) where {D, R} = D
_rangeof(::Query{D, R}) where {D, R} = R

# ===== leaf storage (also Query nodes) ==================================

# Vector-backed unary set — the concrete leaf for `Unary{D}` literals.
struct UnaryVec{D} <: Unary{D}
    values::Vector{D}
end
UnaryVec(vs::Vector{D}) where D = UnaryVec{D}(vs)

# A dense primary-key universe ID{E}(1)..ID{E}(n) — stored as just `n`. The
# entity tables have contiguous PKs, so "scanning the universe" is iterating a
# range, with no N-element vector to hold or chase.
struct Universe{E} <: Unary{ID{E}}
    n::Int
end

# `Staging{D,R}` — the load-time container for a leaf relation: just a flat pair
# list, filled at load and consumed by `seal_entities!`. Deliberately NOT a
# `Query` — it has no drive/probe, so it *cannot* appear in a query plan (the
# node constructors require `Query` arguments). Every leaf starts as a `Staging`
# and, after load, is *sealed* (see `seal_entities!`) into one of the static,
# immutable leaf types below, which is what queries actually run on.
mutable struct Staging{D, R}
    pairs::Vector{Pair{D, R}}
end
Staging{D, R}() where {D, R} = Staging{D, R}(Pair{D, R}[])

# `MapRel{D,R}` — a drive-only materialized relation: a flat pair list wrapped
# as a `Query`. Produced by `collect` (the REPL terminal) and by query code that
# precomputes a `Vector{Pair}` and feeds it back into the algebra (e.g. TPC-H
# Q13's LEFT-JOIN post-processing). It supports `drive` only — a collected /
# inlined result is meant to be scanned, not probed; wrap it in `materialize` if
# you need probe-many.
struct MapRel{D, R} <: Query{D, R}
    pairs::Vector{Pair{D, R}}
end

# ===== static leaf storage (sealed from a Staging at load) ===============
# Three immutable shapes, one per physical layout. drive/probe carry no
# per-row format branch — the type *is* the layout.

# Dense 1:1 entity function: a column store. drive iterates 1..n; probe is a
# bounds-checked array load. (Sealed from a 1:1 leaf whose keys fill 1..n.)
struct VecRel{E, R} <: Query{ID{E}, R}
    values::Vector{R}
end
VecRel(::Type{E}, vs::Vector{R}) where {E, R} = VecRel{E, R}(vs)

# Sparse 1:1 entity function: dense `values` plus a `seen` presence map (for
# entities whose PK has gaps, e.g. TPC-H Order: 1.5M rows over a 6M id range).
# drive skips unseen slots; probe checks `seen` before loading.
struct SparseRel{E, R} <: Query{ID{E}, R}
    values::Vector{R}
    seen::BitVector
end

# Multi-valued entity relation: a dense forward index `fwd[i]` = the values at
# id `i` (e.g. `movie → cast`). drive iterates the nest; probe indexes `fwd`.
struct MultiRel{E, R} <: Query{ID{E}, R}
    fwd::Vector{Vector{R}}
end

# `Multi{T}` — schema-only marker. In `@entity`, `f :: Multi{T}` declares a
# multi-valued field (sealed to `MultiRel`); plain `f :: T` is a 1:1 function
# (sealed to `VecRel`/`SparseRel` by density). Never instantiated.
struct Multi{T} end

const Rel = Staging              # cache.jl serializes the staging leaves
const Relation = Query

# (No `Base.length`/`isempty`/`_pairs` on the leaf or result types: relations
# are consumed via drive/probe, never as collections, and a `length`/`isempty`
# on a sparse/multi leaf would be a silent O(n) scan behind an O(1)-looking
# name. Inspect a leaf's storage fields directly if you need a count.)

# ===== sealing: Staging → static leaf storage ===========================
# After load, each entity leaf is sealed once from its `pairs` into the
# concrete layout dictated by its declared multiplicity + the loaded data:
#   declared 1:1  →  VecRel    (keys fill 1..n)
#                 →  SparseRel (keys have gaps)
#   declared Multi →  MultiRel
# Sealing replaces the per-leaf `const` binding (see `seal_entities!`), so
# `lookup_field` and the bare-name exposures resolve to the sealed object.

# dense forward index sized to the entity universe `n` (so every valid id is
# directly indexable). Junk pairs (id < 1) are skipped.
function _multi_fwd(pairs::Vector{Pair{ID{E}, R}}, n::Int) where {E, R}
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

function seal(r::Staging{ID{E}, R}, n::Int, multi::Bool, label) where {E, R}
    multi && return MultiRel{E, R}(_multi_fwd(r.pairs, n))
    vals = Vector{R}(undef, n)
    seen = falses(n)
    for p in r.pairs
        i = p.first.id
        i < 1 && continue                       # junk pair (nonexistent entity)
        seen[i] && error("$label: duplicate key $i — field declared 1:1 but " *
                         "data is multi-valued (annotate it `Multi{…}`)")
        @inbounds vals[i] = p.second
        @inbounds seen[i] = true
    end
    all(seen) ? VecRel{E, R}(vals) : SparseRel{E, R}(vals, seen)
end

# entity universe = max key id across all of E's (still-staging) leaves.
# `_maxid` is a function barrier: `lookup_field` returns an abstract `Staging`
# (R varies by field), so the pair scan must happen behind a dispatch on the
# concrete element type, else it boxes every pair.
function _maxid(r::Staging{ID{E}, R}) where {E, R}
    n = 0
    for p in r.pairs
        p.first.id > n && (n = p.first.id)
    end
    n
end
function _entity_universe(E, fields)
    n = 0
    for f in fields
        r = lookup_field(ID{E}, Val(f))
        r isa Staging && (n = max(n, _maxid(r)))
    end
    n
end

# Seal every @entity leaf in place by rebinding its `const`. Idempotent:
# already-sealed leaves are skipped. Callers re-run `@expose` afterwards so
# bare names pick up the sealed bindings.
function seal_entities!()
    M = parentmodule(@__MODULE__).Main   # caller's Main, where the consts live
    # Build all sealed objects, then rebind every `const` in a single
    # `Core.eval` — one world-age bump / invalidation wave instead of one per
    # leaf (which is quadratic-ish across a wide schema).
    block = Expr(:block)
    for (E_sym, fields) in _ENTITY_FIELDS
        E = getfield(M, E_sym)
        n = _entity_universe(E, fields)
        for f in fields
            old = lookup_field(ID{E}, Val(f))
            old isa Staging || continue          # already sealed → skip
            sealed = seal(old, n, (E_sym, f) in _MULTI_FIELDS, "$E_sym.$f")
            push!(block.args, :(const $(Symbol("_", E_sym, "_", f)) = $sealed))
        end
    end
    Core.eval(M, block)
    nothing
end
export seal_entities!

# ===== Materialized index builder + readers =============================
# Leaves no longer carry indexes — they are sealed into VecRel/SparseRel/
# MultiRel with their layout baked in. The helpers below build / read a forward
# index for the one node that still needs one on demand: `Materialized` (the
# `materialize`-once / probe-many node). Dense `Vector{Vector{R}}` when entity-
# keyed, a `Dict` otherwise; `_idx_probe`/`_idx_probe_any` read either.

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

# ===== predicate payloads (typed so codegen branches statically) ========

struct EqP{V};  v::V;  end          # == val
struct InP{T};  vs::T;  end          # in (tuple of vals)
struct FnP{F};  f::F;  end          # any unary y -> Bool  (< > <= >= != ~ ≁)

# Interval types — used as the rhs of `q in iv`. `a..b` is closed [a, b]
# (matches IntervalSets convention); `during(a, b)` is half-open [a, b)
# (the common date-range pattern). Concrete callable structs (not closures)
# so the predicate type is stable across query constructions.
struct ClosedInterval{T};      lo::T; hi::T; end
struct ClosedOpenInterval{T};  lo::T; hi::T; end
struct InClosed{T};      lo::T; hi::T; end
struct InClosedOpen{T};  lo::T; hi::T; end
@inline (p::InClosed{T})(v) where {T}     = (p.lo <= v <= p.hi)
@inline (p::InClosedOpen{T})(v) where {T} = (p.lo <= v <  p.hi)

# ===== query nodes ======================================================

struct Compose{D, M, R, A, B} <: Query{D, R};  a::A;  b::B;  end
struct Filter{D, R, A, P}     <: Query{D, R};  a::A;  pred::P;  end
struct Diff{D, R, A, B}       <: Query{D, R};  a::A;  b::B;  end   # value-bearing minus
struct Prod{D, R, T<:Tuple}   <: Query{D, R};  ops::T;  end

struct Disj{D, A, B} <: Unary{D};  a::A;  b::B;  end
# `Restrict(a, b)` — restriction `a : b`. Drives `a` and keeps each row whose
# value is a `member` of `b` (b's keyset), ignoring b's values. Replaces the old
# `Compose(a, askeys(b))` lowering: the per-row `member(b, ·)` check is what
# actually executes, so there is no fictional "keyset unary" node in between.
struct Restrict{D, R, A, B} <: Query{D, R};  a::A;  b::B;  end

# `materialize(q)` — the one explicit "bang". Prela is top-down / non-
# materialized by default: a shared subexpression is re-driven on every use.
# Wrapping it in `materialize(...)` evaluates it once and serves it many — the
# bushy-plan building block (wrap each selective non-driving leg). AST-only:
# `prepare` lowers it to `MatStream` (driven → stored pairs) or `MatProbed`
# (probed → concrete forward index), so `Materialized` itself is never run.
struct Materialized{D, R, A} <: Query{D, R}
    a::A
end

# Prepared forms: drive-only stored pairs vs probe-only concrete index — dense
# `Vector{Vector{R}}` when entity-keyed, else `Dict{D, Vector{R}}` (no Union).
struct MatStream{D, R} <: Query{D, R}
    mat::Vector{Pair{D, R}}
end
struct MatProbed{D, R, IDX} <: Query{D, R}
    idx::IDX
end

# `materialize` on a set-query. AST-only: `prepare` lowers to `MatSetStream`
# (driven → stored keys) or `MatSetProbed` (probed → concrete membership Set).
struct MatSet{D, A} <: Unary{D}
    a::A
end
struct MatSetStream{D} <: Unary{D}
    keys::Vector{D}
end
struct MatSetProbed{D} <: Unary{D}
    set::Set{D}
end

# `Bitset(n)` — dense `BitVector`-backed `Unary{D}`. Drop-in replacement
# for `MatSet` when D coerces to ints `0..n` (the only TPCH shapes are
# `Int` and `ID{E}`): `member` becomes one bit-test, `drive` is a bit scan.
# Use to hoist a per-row predicate (regex, multi-hop nav, expensive
# compare) out of the big-side scan: precompute it once into a `Bitset`
# over the small domain, then `Li.part in green_parts` per lineitem is
# `O(1)` bit-test instead of re-evaluating the predicate.
mutable struct Bitset{D} <: Unary{D}
    bits::BitVector  # length n+1; bit i means member at int slot (i-1)
    n::Int
end
Bitset{D}(n::Int) where {D} = Bitset{D}(falses(n + 1), n)

# `bitset(s/q, n)` — a *lazy* dense-membership materialize. `BitsetMat` is an AST
# node that `prepare` lowers to a `Bitset` (driving the inner once, one bit per
# dense-int member). So the index is built at prepare — part of the plan, timed
# as real work — not eagerly during query construction. A `Unary` input bit-sets
# its keys; a value-bearing `Query` bit-sets its values (both flow through the
# value slot of `drive`, since a Unary emits `(x, x)`).
struct BitsetMat{MEM, A} <: Unary{MEM}
    q::A
    n::Int
end
bitset(s::Unary{D}, n::Int) where {D} = BitsetMat{D, typeof(s)}(s, n)
bitset(q::Query{D, R}, n::Int) where {D, R} = BitsetMat{R, typeof(q)}(q, n)
export bitset, Bitset

# `Inv(q)` — invert a relation. `q : A → B` becomes `Inv(q) : B → A`. Surface
# syntax is postfix adjoint `q'`. AST-only: `prepare` lowers it to `InvStream`
# (driven → streaming flip) or `InvIndexed` (probed → eager concrete index), so
# `Inv` itself is never driven/probed.
struct Inv{B, A, Q} <: Query{B, A}
    q::Q
end

# ===== access mode, made type-level by `prepare` ========================
# The drive-vs-probe mode is a top-down, build-time property (the root is always
# driven). `prepare(plan, Driven())` rewrites the plan so each node sits in its
# mode; where a probed node needs an index it becomes a distinct, concrete-typed
# prepared node — no lazy `Union{Nothing,…}`. (See the plan; this slice splits
# only `Inv`.)
abstract type Mode end
struct Driven <: Mode end
struct Probed <: Mode end

# `Inv` splits by mode at `prepare` time: driven → streaming flip (no index);
# probed → an eagerly-built, concrete inverse index. Each supports exactly one
# access, so the mode is type-enforced.
struct InvStream{B, A, Q} <: Query{B, A}
    q::Q
end
struct InvIndexed{B, A} <: Query{B, A}
    idx::Dict{B, Vector{A}}
end

# `Fold(q, op, init)` — per-key foldl aggregation. `q : D → R`, the inner
# is grouped by D on the fly (it emits (key, value) pairs many-to-one);
# per key we foldl `op` over the values starting from `init`. Mutable +
# lazy-cached so the same Fold can be referenced multiple times (e.g. by
# both a sum and the mean built from sum/count) without re-aggregating.
struct Fold{D, R, S, Q, OP} <: Query{D, S}
    q::Q
    op::OP
    init::S
end

# `DenseFold(q, op, init, n)` — `Fold` variant that caches into a
# `Vector{S}` of length `n+1` (plus a parallel `BitVector` presence map)
# instead of a `Dict{D, S}`. Use when D coerces to `0..n` ints (entity
# IDs, or a packed-byte index like Q1's `(rf, ls)`). Avoids hash + entry
# alloc per reduce step. Surface syntax: `q ▷ (op, init, n)` — adding a
# trailing `n::Int` to the existing 2-tuple opts in to the dense form.
struct DenseFold{D, R, S, Q, OP} <: Query{D, S}
    q::Q
    op::OP
    init::S
    n::Int
end

# coerce/unbox between a DenseFold's D type and its int slot index. D must
# be `Int` or `ID{E}` — the only two domain shapes used by TPC-H.
@inline _denseidx(d::Int)   = d
@inline _denseidx(d::ID)    = d.id
@inline _densebox(::Type{Int}, i::Int) = i
@inline _densebox(::Type{ID{E}}, i::Int) where E = ID{E}(i)

# `BufFold(q, f)` — per-key buffered reduce. Per key, collect all values
# into a `Vector{R}` then call `f(vs) → S`. Use when the reducer needs
# the whole multiset (count-distinct, set construction, median, etc.) —
# anything that doesn't fit foldl's `(S, R) → S` shape.
struct BufFold{D, R, S, Q, F} <: Query{D, S}
    q::Q
    f::F
end

# `Map(q, f)` — generalized projection (per-row lambda). `q : D → R` with
# `f : R → S` becomes `Map(q, f) : D → S`. The function `f` runs per emitted
# row; no aggregation, no caching needed.
struct Map{D, R, S, Q, F} <: Query{D, S}
    q::Q
    f::F
end

# `Scalar(q, op, init)` — no-group foldl. Folds every value emitted by `q`
# into a single scalar (keys ignored). Result is `Query{Nothing, S}` with
# one row keyed by `nothing`, so it still composes uniformly with `↦`.
# Surface syntax `q ▶ (op, init)`.
struct Scalar{S, Q, OP} <: Query{Nothing, S}
    q::Q
    op::OP
    init::S
end

# Prepared fold results (concrete caches, built at `prepare`; no Union, no lazy
# check). `FoldP` serves both `Fold` and `BufFold` (both are `Dict{D,S}` groups).
struct FoldP{D, S} <: Query{D, S}
    cache::Dict{D, S}
end
struct DenseFoldP{D, S} <: Query{D, S}
    vals::Vector{S}
    seen::BitVector
end
struct ScalarP{S} <: Query{Nothing, S}
    value::S
end

# `LeftCompose(r, s)` — for `r : D → R` and `s : D → S` (same domain),
# produces `Query{R, S}`. Surface syntax `r ← s`. AST-only: `prepare` lowers it
# to `LCStream` (driven → walk `s`, probe `r` per row) or `LCIndexed` (probed →
# concrete `Dict{RK, Vector{SV}}`). Same stream-vs-index split as `Inv`.
struct LeftCompose{D, RK, SV, QR, QS} <: Query{RK, SV}
    r::QR
    s::QS
end
struct LCStream{D, RK, SV, QR, QS} <: Query{RK, SV}
    r::QR
    s::QS
end
struct LCIndexed{RK, SV} <: Query{RK, SV}
    idx::Dict{RK, Vector{SV}}
end

# `LeftConj(l, r)` — left-driving conjunction. `l ⩓ r` materializes `l`
# so its `member` is O(1), then drives `r` (ignoring its value) and
# member-checks `l` per row. Lets a user-written `∧`-style expression
# put a Query-shaped predicate (like an `Inv` for EXISTS) on the left
# without needing an explicit `!` — the operator does the materialization.
struct LeftConj{D, ML, R} <: Unary{D}
    l::ML  # already materialized predicate (MatSet) — fast probe_any
    r::R   # predicate to drive
end

# constructors — extract D/M/R via dispatch
Compose(a::Query{D, M}, b::Query{M, R}) where {D, M, R} =
    Compose{D, M, R, typeof(a), typeof(b)}(a, b)
Filter(a::Query{D, R}, p::P) where {D, R, P} =
    Filter{D, R, typeof(a), P}(a, p)
Diff(a::Query{D, R}, b) where {D, R} =
    Diff{D, R, typeof(a), typeof(b)}(a, b)
Restrict(a::Query{D, R}, b) where {D, R} =
    Restrict{D, R, typeof(a), typeof(b)}(a, b)
Disj(a::Query{D, Ra}, b::Query{D, Rb}) where {D, Ra, Rb} =
    Disj{D, typeof(a), typeof(b)}(a, b)
function Prod(ops::Tuple)
    D = _domof(ops[1])
    R = Tuple{map(_rangeof, ops)...}
    Prod{D, R, typeof(ops)}(ops)
end
materialize(s::Unary{D}) where {D} = MatSet{D, typeof(s)}(s)
materialize(q::Query{D, R}) where {D, R} = Materialized{D, R, typeof(q)}(q)

# Adjoint = inverse: `q'` on a Query{A, B} returns Inv : Query{B, A}.
Base.adjoint(q::Query{A, B}) where {A, B} = Inv{B, A, typeof(q)}(q)

# `▷` — per-key foldl. Pass `(op, init)` as a 2-tuple on the rhs.
# `q ▷ (+, 0.0)` is sum; `q ▷ ((a, _) -> a + 1, 0)` is count; arbitrary
# `(S, R) → S` reductions supported. Free function, no getproperty overload.
function ▷(q::Query{D, R}, opinit::Tuple{OP, S}) where {D, R, OP, S}
    Fold{D, R, S, typeof(q), OP}(q, opinit[1], opinit[2])
end

# `▷` with a 3-tuple `(op, init, n)` opts in to `DenseFold` — `Vector{S}`-
# backed group cache over the dense int domain `0..n`. The user explicitly
# states the bound; no heuristic dense-vs-hash selection.
function ▷(q::Query{D, R}, opinitn::Tuple{OP, S, Int}) where {D, R, OP, S}
    DenseFold{D, R, S, typeof(q), OP}(q, opinitn[1], opinitn[2], opinitn[3])
end
export ▷

# `▷` with a single callable: buffered per-key reduce — collect values
# into `Vector{R}` per key, apply `f`. Tuple-rhs (foldl) dispatch above
# is preferred when the reduction fits a `(S, R) → S` shape.
function ▷(q::Query{D, R}, f::Base.Callable) where {D, R}
    S = Core.Compiler.return_type(f, Tuple{Vector{R}})
    S === Union{} && (S = Any)
    BufFold{D, R, S, typeof(q), typeof(f)}(q, f)
end

# `⊵` — no-group foldl. Folds every value of `q` into one scalar; result
# is `Query{Nothing, S}` with a single row, so it still chains with `↦`.
# Equivalent of synthesizing a singleton group key, but cheaper: skips the
# group-dict build. `▶` is a prefix-only alias (Julia parses `▶` as an
# identifier, not as a binary operator).
function ⊵(q::Query{D, R}, opinit::Tuple{OP, S}) where {D, R, OP, S}
    Scalar{S, typeof(q), OP}(q, opinit[1], opinit[2])
end
const ▶ = ⊵
export ⊵, ▶

# `unwrap(q::Query{Nothing, S}) → S` — eliminator for the one-row container
# `⊵` (and `↦` on it) produces. Drives once, returns the single value as a
# plain Julia scalar. Useful for scalar-subquery escapes: e.g.
# `threshold = 0.0001 * unwrap(value_per_part ⊵ (+, 0.0))`.
function unwrap(q::Query{Nothing, S}) where {S}
    r = Ref{S}()
    drive(prepare(q, Driven()), (_, v) -> r[] = v)
    r[]
end
export unwrap

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
# with measures), and `r' → s` when the source is the left side. With
# Unary now identity-shaped, `r ← (set)` is just the general Query/Query
# form — no special unary-on-right path is needed.
function ←(r::Query{D, RK}, s::Query{D, SV}) where {D, RK, SV}
    LeftCompose{D, RK, SV, typeof(r), typeof(s)}(r, s)
end
export ←

# `⩘` — left-driving wedge (\bigslopedwedge). `l ⩘ r` materializes the
# *value-set* of `l` (auto-invert, mirroring `←`), then drives `r` and
# member-checks per row. For an identity `l` (`Unary{D}`), invert is a
# no-op so we materialize directly. `⩓` kept as a back-compat alias.
function ⩘(l::Unary{D}, r) where {D}
    ml = materialize(l)
    LeftConj{_domof(r), typeof(ml), typeof(r)}(ml, r)   # drive r ignoring its value
end
function ⩘(l::Query{D, R}, r) where {D, R}
    # Materialize the adjoint: for an entity-keyed value type this gives `ml` a
    # dense-array membership index (vs the `Inv`'s `Dict`), ~13% faster on q20's
    # per-row member-check. (The `Inv` self-caches too, but only as a hash.)
    ml = materialize(Base.adjoint(l))
    LeftConj{_domof(r), typeof(ml), typeof(r)}(ml, r)
end
const ⩓ = ⩘
export ⩘, ⩓

# Prefix `!` is the terse spelling of `materialize` — `!(q)` ≡ `materialize(q)`.
# Borrowed from Haskell's strictness bang; a query has no boolean-not, so `!`
# is free to mean "force this leg".
Base.:!(q::Query) = materialize(q)

# ===== operators (build nodes) ==========================================
# Navigation is `→` only — `q.field` overloads on Query/Unary were removed
# (use `q → Type.field` instead). `Entity.field` (e.g. `Company.country`)
# still works via the `@entity`-generated `Base.getproperty(::Type{E}, ...)`.

# `→` is just Compose — Unary is `Query{Y, Y}` so `Restrict`/Filter-by-Unary
# both reduce to Compose with identity on one side.
→(a::Query{X, Y}, b::Query{Y, Z}) where {X, Y, Z} = Compose(a, b)

# ∧ ∨ : - ⊗
# `∧` aliases `⊗` — under the specialized `probe_any(::Prod)`, the conj-use
# of Prod short-circuits identically to the old dedicated `Conj` node, so the
# separate type is no longer pulling weight.
∧(a, b) = ⊗(a, b)
∨(a, b) = Disj(a, b)
# `:` restriction — keep rows of `a` whose value is a `member` of `b`. The rhs
# `b` is consumed only via `member` (b's keyset), so any value-bearing predicate
# works directly with no keyset projection.
Base.:(:)(a::Query{X, Y}, b) where {X, Y} = Restrict(a, b)
# `-`: value-bearing difference. Identity lhs falls through here too — Diff
# emits `(x, x)` when `x` is not a `member` of `b`, same shape as the old SetDiff.
Base.:-(a::Query{D, R}, b) where {D, R} = Diff(a, b)
# Product — `⊗` is the canonical spelling (tensor-product convention from math).
# `×` is a legacy alias; both build flat `Prod` nodes.
⊗(a::Query, b::Query) = Prod((a, b))
⊗(a::Prod,  b::Query) = Prod((a.ops..., b))
const × = ⊗
export ⊗, ×

# predicates — scalar range (value-vs-constant)
Base.:(==)(q::Query{D, R}, val) where {D, R} = Filter(q, EqP(val))
Base.in(q::Query{D, R}, vals::Tuple) where {D, R} = Filter(q, InP(vals))
Base.in(q::Query{D, R}, iv::ClosedInterval) where {D, R} =
    Filter(q, FnP(InClosed{typeof(iv.lo)}(iv.lo, iv.hi)))
Base.in(q::Query{D, R}, iv::ClosedOpenInterval) where {D, R} =
    Filter(q, FnP(InClosedOpen{typeof(iv.lo)}(iv.lo, iv.hi)))
for op in (:(<), :(>), :(<=), :(>=), :(!=))
    @eval Base.$op(q::Query{D, R}, val) where {D, R} = Filter(q, FnP(Base.Fix2($op, val)))
end

# `a..b` — closed interval [a, b]; pair with `q in (a..b)`.
# `during(a, b)` — half-open [a, b); idiomatic for date ranges.
..(a, b) = ClosedInterval{promote_type(typeof(a), typeof(b))}(promote(a, b)...)
during(a, b) = ClosedOpenInterval{promote_type(typeof(a), typeof(b))}(promote(a, b)...)
export .., during

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
Base.in(q::Query{D, ID{E}}, iv::ClosedInterval) where {D, E} = in(Compose(q, primary(E)), iv)
Base.in(q::Query{D, ID{E}}, iv::ClosedOpenInterval) where {D, E} = in(Compose(q, primary(E)), iv)
for op in (:(<), :(>), :(<=), :(>=), :(!=))
    @eval Base.$op(q::Query{D, ID{E}}, val) where {D, E} = $op(Compose(q, primary(E)), val)
end
Base.:~(q::Query{D, ID{E}}, re::Regex) where {D, E} = Compose(q, primary(E)) ~ re
≁(q::Query{D, ID{E}}, re::Regex) where {D, E} = ≁(Compose(q, primary(E)), re)

# ===== CPS execution protocol ===========================================
# drive(q,k): k(x,y) per pair    probe(q,x,k): k(y) per value at x
# drivekeys(s,k): k(x) per member    member(s,x)::Bool

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

# `drivekeys(q, k)` — emit each domain key. Back-compat alias over `drive`.
@inline drivekeys(q::Query, k) = drive(q, (x, _) -> k(x))

# ===== build_*: the data-building half of `prepare` =====================
# `prepare` (below) is pure mode-propagation: it threads access modes and
# rebuilds structural nodes, choosing each node's prepared *type* by a rule that
# depends only on (node type, mode) — never on data. The actual data
# construction — driving an inner once to fill an index/cache — lives here, in
# named `build_*` functions over already-prepared (driven) inners. This is the
# lower/build split: `prepare` is the "lower" recursion (a pure, type-level
# rule); these drives are the "build". Keeping them apart means the `@generated`
# Engine can reuse this exact build logic while reproducing the mode rule at
# codegen time (the type-level CPS).

# Inv probed: eager inverse index — drive the inner, bucket keys by value.
function build_inv_index(pq::Query{A, B}) where {A, B}
    d = Dict{B, Vector{A}}()
    drive(pq, (a, b) -> push!(get!(() -> A[], d, b), a))
    InvIndexed{B, A}(d)
end

# LeftCompose probed: concrete Dict{RK, Vector{SV}} — drive s, probe r per row.
function build_lc_index(pr::Query{D, RK}, ps::Query{D, SV}) where {D, RK, SV}
    d = Dict{RK, Vector{SV}}()
    drive(ps, (dd, v) -> probe(pr, dd, rk -> push!(get!(() -> SV[], d, rk), v)))
    LCIndexed{RK, SV}(d)
end

# Fold: per-key foldl cache (`S` is the accumulator type, fixed by `init`).
function build_fold(pq::Query{D, R}, op, init::S) where {D, R, S}
    acc = Dict{D, S}()
    drive(pq, (d, v) -> (acc[d] = op(get(acc, d, init), v)))
    FoldP{D, S}(acc)
end

# DenseFold: dense per-key fold over slots `0..n`.
function build_densefold(pq::Query{D, R}, op, init::S, n::Int) where {D, R, S}
    sz = n + 1; vals = fill(init, sz); seen = falses(sz)
    drive(pq, (d, v) -> begin
        i = _denseidx(d) + 1
        if 1 <= i <= sz
            @inbounds vals[i] = op(vals[i], v); @inbounds seen[i] = true
        end
    end)
    DenseFoldP{D, S}(vals, seen)
end

# BufFold: per-key buffered reduce — collect all values, then call `f`. `S` is
# `f`'s result type (not derivable from the inner), so the caller passes it.
function build_buffold(pq::Query{D, R}, f, ::Type{S}) where {D, R, S}
    buf = Dict{D, Vector{R}}()
    drive(pq, (d, v) -> push!(get!(() -> R[], buf, d), v))
    out = Dict{D, S}()
    for (d, vs) in buf; out[d] = f(vs); end
    FoldP{D, S}(out)
end

# Scalar: no-group foldl to a single value.
function build_scalar(pq, op, init::S) where {S}
    acc = Ref{S}(init)
    drive(pq, (_, v) -> (acc[] = op(acc[], v)))
    ScalarP{S}(acc[])
end

# Materialized: drive the inner once into stored pairs (driven) or a concrete
# forward index (probed). `_matpairs`/`_mat_idx` are the underlying builders.
build_mat_stream(pa::Query{D, R}) where {D, R} = MatStream{D, R}(_matpairs(pa, D, R))
function build_mat_probed(pa::Query{D, R}) where {D, R}
    idx = _mat_idx(_matpairs(pa, D, R))
    MatProbed{D, R, typeof(idx)}(idx)
end

# MatSet driven/probed: stored keys / membership Set.
function build_matset_keys(pa::Unary{D}) where {D}
    keys = D[]; drive(pa, (x, _) -> push!(keys, x)); MatSetStream{D}(keys)
end
function build_matset_set(pa::Unary{D}) where {D}
    s = Set{D}(); drive(pa, (x, _) -> push!(s, x)); MatSetProbed{D}(s)
end

# BitsetMat → dense `Bitset` membership: one bit per dense-int value (`MEM` is
# the value side — a Unary emits its keys through the value slot).
function build_bitset(pq::Query{D, MEM}, n::Int) where {D, MEM}
    b = Bitset{MEM}(n)
    drive(pq, (_, v) -> begin
        i = _denseidx(v) + 1
        @inbounds (1 <= i <= n + 1) && (b.bits[i] = true)
    end)
    b
end

# ===== prepare: lift the drive/probe mode to the types =================
# `prepare(q, Driven())` rewrites the plan top-down so each node is in its access
# mode (the root is driven). Structural nodes rebuild with children prepared per
# the mode table; `Inv` splits into `InvStream` (driven) / `InvIndexed` (probed,
# concrete index). The executor calls this once before `drive` — think of it as
# compilation. It is **memo-free and type-stable**: for a concrete (node, mode)
# the prepared type is determined, so `prepare(q, Driven())` infers a concrete
# plan and the rebuild inlines. Prela is non-materialized by default — a
# subexpression referenced twice is prepared (and run) twice; wrap it in
# `materialize`/`collect` to share. This slice splits only `Inv`; the other lazy
# nodes are rebuilt but stay lazy.

# Inv: the split (driven → stream, probed → eager concrete index).
prepare(n::Inv{B,A,Q}, ::Driven) where {B,A,Q} =
    (pq = prepare(n.q, Driven()); InvStream{B,A,typeof(pq)}(pq))
prepare(n::Inv, ::Probed) = build_inv_index(prepare(n.q, Driven()))

# Structural nodes: rebuild with children prepared in their modes.
prepare(n::Compose, m::Mode) = Compose(prepare(n.a, m), prepare(n.b, Probed()))
prepare(n::Filter, m::Mode) = Filter(prepare(n.a, m), n.pred)
prepare(n::Restrict, m::Mode) = Restrict(prepare(n.a, m), prepare(n.b, Probed()))
prepare(n::Diff, m::Mode) = Diff(prepare(n.a, m), prepare(n.b, Probed()))
prepare(n::Disj, ::Mode) = Disj(prepare(n.a, Probed()), prepare(n.b, Probed()))
prepare(n::Prod, m::Mode) =
    Prod((prepare(n.ops[1], m), map(o -> prepare(o, Probed()), Base.tail(n.ops))...))
prepare(n::Map{D,R,S,Q,F}, m::Mode) where {D,R,S,Q,F} =
    (pq = prepare(n.q, m); Map{D,R,S,typeof(pq),F}(pq, n.f))
prepare(n::LeftConj{D,ML,R}, m::Mode) where {D,ML,R} =
    (pl = prepare(n.l, Probed()); pr = prepare(n.r, m);
     LeftConj{D, typeof(pl), typeof(pr)}(pl, pr))

# Lazy nodes (not split this slice): rebuild with children driven to build the
# cache; the node itself stays lazy (cache reset to `nothing`).
prepare(n::LeftCompose{D,RK,SV,QR,QS}, ::Driven) where {D,RK,SV,QR,QS} =
    (pr = prepare(n.r, Probed()); ps = prepare(n.s, Driven());
     LCStream{D,RK,SV,typeof(pr),typeof(ps)}(pr, ps))
prepare(n::LeftCompose, ::Probed) =
    build_lc_index(prepare(n.r, Probed()), prepare(n.s, Driven()))

# Pipeline-breakers (Folds/Scalar): the cache is always needed (both modes), so
# `prepare` builds it eagerly into a concrete prepared result — no mode split.
prepare(n::Fold, ::Mode) = build_fold(prepare(n.q, Driven()), n.op, n.init)
prepare(n::DenseFold, ::Mode) = build_densefold(prepare(n.q, Driven()), n.op, n.init, n.n)
prepare(n::BufFold{D,R,S,Q,F}, ::Mode) where {D,R,S,Q,F} =
    build_buffold(prepare(n.q, Driven()), n.f, S)
prepare(n::Scalar, ::Mode) = build_scalar(prepare(n.q, Driven()), n.op, n.init)

# Materialized: split by mode — driven → stored pairs; probed → concrete index.
# `_matpairs` is the function barrier so the materializing drive specializes on
# the concrete prepared inner type.
function _matpairs(pa, ::Type{D}, ::Type{R}) where {D, R}
    out = Pair{D, R}[]
    drive(pa, (x, y) -> push!(out, x => y))
    out
end
prepare(n::Materialized, ::Driven) = build_mat_stream(prepare(n.a, Driven()))
prepare(n::Materialized, ::Probed) = build_mat_probed(prepare(n.a, Driven()))

# MatSet: split by mode — driven → stored keys; probed → membership Set.
prepare(n::MatSet, ::Driven) = build_matset_keys(prepare(n.a, Driven()))
prepare(n::MatSet, ::Probed) = build_matset_set(prepare(n.a, Driven()))

# UnaryVec: driven → iterate keys (identity); probed → concrete membership Set.
prepare(n::UnaryVec, ::Driven) = n
prepare(n::UnaryVec{D}, ::Probed) where {D} = MatSetProbed{D}(Set(n.values))

# BitsetMat → a dense `Bitset` membership, built by driving the inner once.
prepare(n::BitsetMat, ::Mode) = build_bitset(prepare(n.q, Driven()), n.n)

# Leaves / sources (and already-prepared variants): identity.
prepare(n::Union{VecRel,SparseRel,MultiRel,MapRel,Universe,Bitset,
                 InvStream,InvIndexed,MatStream,MatProbed,LCStream,LCIndexed,
                 FoldP,DenseFoldP,ScalarP,MatSetStream,MatSetProbed}, ::Mode) = n

# ===== terminals ========================================================
# Queries are consumed by `drive`/`drivekeys` with a folding continuation
# (see `_vals` in queries.jl) — no result relation is ever built. `collect`
# is the convenience terminal for the REPL: drive a query into a concrete Rel.

function Base.collect(q::Query{D, R}) where {D, R}
    out = Pair{D, R}[]
    drive(prepare(q, Driven()), (x, y) -> push!(out, x => y))
    MapRel{D, R}(out)
end
function Base.collect(s::Unary{D}) where D
    out = D[]
    drive(prepare(s, Driven()), (x, _) -> push!(out, x))
    UnaryVec{D}(out)
end

# ===== schema sugar (@entity / @declare / @expose) ======================

macro entity(entity_sym, block)
    entity_sym isa Symbol || error("@entity expects a symbol entity name")
    (block isa Expr && block.head === :block) || error("@entity expects `begin ... end`")

    out = Expr(:block)
    push!(out.args, :($(GlobalRef(@__MODULE__, :_declare_if_needed))(@__MODULE__, $(QuoteNode(entity_sym)))))

    id_type    = :($(GlobalRef(@__MODULE__, :ID)){$(esc(entity_sym))})
    rel_type   = GlobalRef(@__MODULE__, :Staging)
    lookup_fn  = GlobalRef(@__MODULE__, :lookup_field)
    primary_fn = GlobalRef(@__MODULE__, :primary)

    field_names = Symbol[]
    field_consts = Symbol[]
    for stmt in block.args
        stmt isa LineNumberNode && continue
        if stmt isa Expr && stmt.head === :(::)
            field_sym  = stmt.args[1]
            range_expr = stmt.args[2]
            # `f :: Multi{T}` declares a multi-valued field: record it and store
            # the unwrapped `T` as the leaf's value type (the `Staging` leaf and
            # sealed `MultiRel` both hold `Pair{ID{E}, T}`).
            is_multi = range_expr isa Expr && range_expr.head === :curly &&
                       range_expr.args[1] === :Multi
            is_multi && (range_expr = range_expr.args[2])
            qual_sym = Symbol("_", entity_sym, "_", field_sym)
            push!(field_names, field_sym)
            push!(field_consts, qual_sym)
            is_multi && push!(out.args,
                :(push!($(GlobalRef(@__MODULE__, :_MULTI_FIELDS)),
                        ($(QuoteNode(entity_sym)), $(QuoteNode(field_sym))))))
            push!(out.args, quote
                const $(esc(qual_sym)) = $rel_type{$id_type, $(esc(range_expr))}(
                    Pair{$id_type, $(esc(range_expr))}[]
                )
                $lookup_fn(::Type{$id_type}, ::Val{$(QuoteNode(field_sym))}) = $(esc(qual_sym))
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

macro expose(arg)
    # Two forms:
    #   @expose Entity                       — bare-name all fields
    #   @expose Entity : f1, f2, …           — bare-name only the listed fields
    #
    # `@expose Entity : f` parses as a single `:` call: `(:, Entity, f)`.
    # `@expose Entity : f, g, h` parses as a tuple whose first element is
    # `(:, Entity, f)` and remaining elements are bare symbols.
    entity_sym = nothing
    fields = nothing
    if arg isa Symbol
        entity_sym = arg
    elseif arg isa Expr && arg.head === :call && arg.args[1] === :(:) &&
           length(arg.args) == 3 && arg.args[2] isa Symbol && arg.args[3] isa Symbol
        entity_sym = arg.args[2]
        fields = Symbol[arg.args[3]]
    elseif arg isa Expr && arg.head === :tuple &&
           arg.args[1] isa Expr && arg.args[1].head === :call &&
           arg.args[1].args[1] === :(:)
        entity_sym = arg.args[1].args[2]
        fields = Symbol[arg.args[1].args[3]]
        for r in arg.args[2:end]
            r isa Symbol || error("@expose: field-list entries must be symbols, got $r")
            push!(fields, r)
        end
    else
        error("@expose syntax: `@expose Entity` or `@expose Entity : f1, f2, …`")
    end
    haskey(_ENTITY_FIELDS, entity_sym) ||
        error("@expose: no @entity declaration found for `$entity_sym`")
    all_fields = _ENTITY_FIELDS[entity_sym]
    chosen = fields === nothing ? all_fields : fields
    out = Expr(:block)
    for f in chosen
        f in all_fields || error("@expose: $entity_sym has no field `$f`")
        qual_sym = Symbol("_", entity_sym, "_", f)
        push!(out.args, :(const $(esc(f)) = $(esc(qual_sym))))
    end
    out
end
export @expose

end # module
