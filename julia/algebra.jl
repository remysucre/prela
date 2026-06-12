# The language: entity IDs, leaf storage, query node types (logical and
# physical), constructors, and the surface operator syntax. Pure vocabulary —
# execution lives in interp.jl/staged.jl, lowering and state in plan.jl.

export Staging, MapRel, VecRel, SparseRel, MultiRel, Multi, Query, Unary,
       UnaryVec, Universe, Entity, ID,
       primary, lookup_field, →, ∧, ∨, ≁,
       drive, probe, member, materialize

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

# ===== query-tree type hierarchy ========================================
# Every node is a `Query{D, R}` — a lazy binary relation D → R. `Unary{D}`
# is the abstract marker for *identity* relations `D → D`, the home of
# leaf set-shaped things (Universe, UnaryVec) and of Booleanesque nodes
# whose value side is just the key (Disj, MatSet, Bitset). The
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
struct Staging{D, R}
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

# (No `Base.length`/`isempty`/`_pairs` on the leaf or result types: relations
# are consumed via drive/probe, never as collections, and a `length`/`isempty`
# on a sparse/multi leaf would be a silent O(n) scan behind an O(1)-looking
# name. Inspect a leaf's storage fields directly if you need a count.)
# ===== predicates ========================================================
# A Filter's predicate is any callable `y -> Bool`. Named concrete callables
# (not closures) keep the predicate type stable across query constructions;
# `Base.Fix1`/`Fix2` cover the comparison operators, and a raw closure is fine
# where the call site is unique anyway (cross-column compares).

struct EqP{V};  v::V;  end           # == val
struct InP{T};  vs::T;  end          # in (tuple of vals)
@inline (p::EqP)(y) = isequal(y, p.v)
@inline (p::InP)(y) = y in p.vs

# Interval types — used as the rhs of `q in iv`, and callable so they ARE the
# Filter predicate. `a..b` is closed [a, b] (matches IntervalSets convention);
# `during(a, b)` is half-open [a, b) (the common date-range pattern).
struct ClosedInterval{T};      lo::T; hi::T; end
struct ClosedOpenInterval{T};  lo::T; hi::T; end
@inline (iv::ClosedInterval)(v)     = (iv.lo <= v <= iv.hi)
@inline (iv::ClosedOpenInterval)(v) = (iv.lo <= v <  iv.hi)

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
# `prepare` lowers it to `MapRel` (driven → stored pairs) or `Indexed`
# (probed → concrete forward index), so `Materialized` itself is never run.
struct Materialized{D, R, A} <: Query{D, R}
    a::A
end

# `Indexed{D,R,IDX}` — THE probe-side index node: a concrete per-key index,
# dense `Vector{Vector{R}}` when entity-keyed, else `Dict{D, Vector{R}}` (no
# Union). Every probed stream-or-index node (`Materialized`, `Inv`,
# `LeftCompose`) lowers to it.
struct Indexed{D, R, IDX} <: Query{D, R}
    idx::IDX
end

# `materialize` on a set-query. AST-only: `prepare` lowers to `UnaryVec`
# (driven → stored keys) or `MatSetProbed` (probed → concrete membership Set).
struct MatSet{D, A} <: Unary{D}
    a::A
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
struct Bitset{D} <: Unary{D}
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
# (driven → streaming flip) or `Indexed` (probed → eager concrete index), so
# `Inv` itself is never driven/probed.
struct Inv{B, A, Q} <: Query{B, A}
    q::Q
end

# ===== access mode, made type-level by `prepare` ========================
# The drive-vs-probe mode is a top-down property of the plan shape (the root is
# always driven). `prepare` rewrites the plan so each node sits in its mode;
# where a probed node needs an index it becomes a distinct, concrete-typed
# physical node holding eagerly-built state — no lazy `Union{Nothing,…}`.
abstract type Mode end
struct Driven <: Mode end
struct Probed <: Mode end

# `Inv` splits by mode at `prepare` time: driven → streaming flip (no index);
# probed → an eagerly-built, concrete inverse `Indexed`. Each supports exactly
# one access, so the mode is type-enforced.
struct InvStream{B, A, Q} <: Query{B, A}
    q::Q
end

# `Fold(q, op, init)` — per-key foldl aggregation. `q : D → R`, the inner
# is grouped by D on the fly (it emits (key, value) pairs many-to-one);
# per key we foldl `op` over the values starting from `init`. AST-only:
# `prepare` builds the concrete per-key cache (`FoldP`). Non-materialized by
# default, so a Fold used twice is re-aggregated unless wrapped in `materialize`.
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
# to `LCStream` (driven → walk `s`, probe `r` per row) or `Indexed` (probed →
# concrete `Dict{RK, Vector{SV}}`). Same stream-vs-index split as `Inv`.
struct LeftCompose{D, RK, SV, QR, QS} <: Query{RK, SV}
    r::QR
    s::QS
end
struct LCStream{D, RK, SV, QR, QS} <: Query{RK, SV}
    r::QR
    s::QS
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
# group-dict build.
function ⊵(q::Query{D, R}, opinit::Tuple{OP, S}) where {D, R, OP, S}
    Scalar{S, typeof(q), OP}(q, opinit[1], opinit[2])
end
export ⊵

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

# `⩘` — left-driving wedge (\bigslopedwedge). `l ⩘ r` restricts `r` by the
# *value-set* of `l` (auto-invert, mirroring `←`): pure sugar for `r : l'`.
# No explicit materialize — the `Inv` sits in Probed position, so `prepare`
# builds its eager index through the mode split anyway. (That index is a hash;
# an entity-keyed `l'` could use a dense array instead — that physical choice
# belongs to the planned physical-type annotations, not to this operator.)
⩘(l::Query{D, R}, r) where {D, R} = Restrict(r, Base.adjoint(l))
export ⩘

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
Base.in(q::Query{D, R}, iv::Union{ClosedInterval, ClosedOpenInterval}) where {D, R} =
    Filter(q, iv)
for op in (:(<), :(>), :(<=), :(>=), :(!=))
    @eval Base.$op(q::Query{D, R}, val) where {D, R} = Filter(q, Base.Fix2($op, val))
end

# `a..b` — closed interval [a, b]; pair with `q in (a..b)`.
# `during(a, b)` — half-open [a, b); idiomatic for date ranges.
..(a, b) = ClosedInterval{promote_type(typeof(a), typeof(b))}(promote(a, b)...)
during(a, b) = ClosedOpenInterval{promote_type(typeof(a), typeof(b))}(promote(a, b)...)
export .., during

# predicates — cross-column (Query-vs-Query, same domain). Comparing two
# leaves of the same row is `Filter(a × b, ((x, y),) -> op(x, y))`;
# this overload makes that the natural spelling, e.g.
#   Lineitem.commitdate < Lineitem.receiptdate
#   Customer.nation == Supplier.nation       (when composed onto the same domain)
for op in (:(<), :(>), :(<=), :(>=), :(==), :(!=))
    @eval Base.$op(a::Query{D, X}, b::Query{D, Y}) where {D, X, Y} =
        Filter(Prod((a, b)), ((x, y),) -> $op(x, y))
    # Specific override for entity-typed columns on both sides: compares the
    # entity IDs directly (no primary-field elision). Resolves the ambiguity
    # between the cross-column overload above and the scalar entity-elision
    # overload below.
    @eval Base.$op(a::Query{D, ID{E}}, b::Query{D, ID{E}}) where {D, E} =
        Filter(Prod((a, b)), ((x, y),) -> $op(x, y))
end
Base.:~(q::Query{D, R}, re::Regex) where {D, R <: AbstractString} =
    Filter(q, Base.Fix1(occursin, re))
≁(q::Query{D, R}, re::Regex) where {D, R <: AbstractString} =
    Filter(q, s -> !occursin(re, s))

# predicates — entity range: elide through the primary field. Every shape is
# the same delegation `op(Compose(q, primary(E)), rhs)`.
for (op, RHS) in ((:(==), Any), (:(<), Any), (:(>), Any), (:(<=), Any),
                  (:(>=), Any), (:(!=), Any), (:in, Tuple),
                  (:in, Union{ClosedInterval, ClosedOpenInterval}), (:~, Regex))
    @eval Base.$op(q::Query{D, ID{E}}, rhs::$RHS) where {D, E} =
        Base.$op(Compose(q, primary(E)), rhs)
end
≁(q::Query{D, ID{E}}, re::Regex) where {D, E} = ≁(Compose(q, primary(E)), re)
