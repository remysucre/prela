module Prela

# Core algebraic-relational library.
#
# Two value types:
#   Unary{R}    — a set of R values.
#   Rel{D, R}   — a binary relation D -> R (stored as Pair{D,R} vector).
#
# Operators (low to high precedence, so RHS of → can be unparenthesized):
#   →   sequential composition (arrow precedence — looser than ∧/∨/== etc.)
#   ∨   union (lazy-or precedence — looser than ==, tighter than →)
#   ∧   intersection (lazy-and precedence — looser than ==, tighter than ∨)
#   ==, !=, <, >, <=, >=, in, ~, ≁   range predicates (comparison precedence)
#   ×   parallel composition / product (times precedence — tightest binary op here)
#   -   set difference (additions precedence)
#   .field   navigation (via getproperty; chains scalar/entity fields)

export Rel, MapRel, VecRel, Relation, Unary, Entity, ID, primary, lookup_field, →, ∧, ∨, ×, ≁, vectorize

abstract type Entity end

# Phantom-typed entity ID. `ID{Movie}(7)` is an opaque reference to the 7th
# Movie. The type parameter lets Julia dispatch on the entity type — that's how
# predicate elision and per-entity `lookup_field` methods know which entity
# they're dealing with.
struct ID{E <: Entity}
    id::Int
end
Base.:(==)(a::ID{E}, b::ID{E}) where E = a.id == b.id
Base.hash(a::ID, h::UInt) = hash(a.id, h)
Base.isless(a::ID{E}, b::ID{E}) where E = a.id < b.id
Base.show(io::IO, a::ID{E}) where E = print(io, nameof(E), "(", a.id, ")")

function primary end
function lookup_field end

# Macro-time registry of entity → field names. Populated by `@entity` (in its
# emitted top-level block) and read by `@expose` at its macro-expansion time.
const _ENTITY_FIELDS = Dict{Symbol, Vector{Symbol}}()

# Idempotent abstract-type declaration. Lets @entity be called after a forward
# `@declare`, and lets cyclic schemas (Movie ↔ MovieLink) work.
function _declare_if_needed(mod::Module, sym::Symbol)
    isdefined(mod, sym) && return
    Core.eval(mod, Expr(:abstract, Expr(:(<:), sym, GlobalRef(@__MODULE__, :Entity))))
end

struct Unary{R}
    values::Vector{R}
end
Unary(vs::Vector{R}) where R = Unary{R}(vs)

# ===== Relation hierarchy ===============================================
# A `Relation{D, R}` is a finite multi-valued function D → R, accessed
# through a tiny interface (`_pairs`, `_get`, `_inv_get`, `_keys`).
# Two physical representations:
#
#   MapRel{D, R}    Vector{Pair{D, R}}                  — general case
#   VecRel{E, R}    Vector{R}, dense over ID{E}(1..n)   — column-store path
#
# `vectorize(maprel, n)` promotes a MapRel that's a single-valued function
# on a dense domain `ID{E}(1..n)` into a VecRel. Operators are written
# polymorphically so the algebra is unchanged.

abstract type Relation{D, R} end

struct MapRel{D, R} <: Relation{D, R}
    pairs::Vector{Pair{D, R}}
end
MapRel(ps::Vector{Pair{D, R}}) where {D, R} = MapRel{D, R}(ps)

struct VecRel{E, R} <: Relation{ID{E}, R}
    values::Vector{R}
end
VecRel(::Type{E}, vs::Vector{R}) where {E, R} = VecRel{E, R}(vs)

# Backward-compatible alias: most call sites still write `Rel{D, R}(...)`.
const Rel = MapRel

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
    all(seen) || error("MapRel is sparse over 1..$n: only $(count(seen))/$n positions filled")
    VecRel{E, R}(vals)
end

# ===== access interface ================================================
# All operator code goes through these — never touch `.pairs`/`.values`
# directly outside this section.

Base.length(r::MapRel) = length(r.pairs)
Base.length(r::VecRel) = length(r.values)
Base.isempty(r::MapRel) = isempty(r.pairs)
Base.isempty(r::VecRel) = isempty(r.values)

# Pair iterator (Pair{D, R}).
_pairs(r::MapRel) = r.pairs
_pairs(r::VecRel{E}) where E = (ID{E}(i) => r.values[i] for i in eachindex(r.values))

# Keys iterator (lazy).
_keys_iter(r::MapRel) = (p.first for p in r.pairs)
_keys_iter(r::VecRel{E}) where E = (ID{E}(i) for i in eachindex(r.values))

# ===== composition (→ at arrow precedence) =====

function compose(u::Unary{X}, r::Relation{X, Y}) where {X, Y}
    s = Set(u.values)
    MapRel{X, Y}([p for p in _pairs(r) if p.first in s])
end

# Index caches are shared mutable state; this lock makes `fwd_index`/`inv_index`
# safe to call from a parallel (multi-threaded) query run.
const _CACHE_LOCK = ReentrantLock()

# Only *leaf* relations (the per-field stores created by `@entity`) are cached:
# they're reused across every query. Derived relations are single-use, so their
# indexes are built fresh and discarded — caching them would grow unboundedly.
const _LEAF_RELS = Base.IdSet{Any}()

const _FWD_INDEX_CACHE = IdDict{Any, Any}()
const _INV_INDEX_CACHE = IdDict{Any, Any}()

function _build_fwd(s::Relation{Y, Z}) where {Y, Z}
    d = Dict{Y, Vector{Z}}()
    sizehint!(d, length(s))
    for p in _pairs(s)
        push!(get!(d, p.first, Z[]), p.second)
    end
    d
end

# Forward index D → list of Rs. Cached for leaf relations, rebuilt for derived.
function fwd_index(s::Relation{Y, Z}) where {Y, Z}
    s in _LEAF_RELS || return _build_fwd(s)
    lock(_CACHE_LOCK) do
        get!(() -> _build_fwd(s), _FWD_INDEX_CACHE, s)::Dict{Y, Vector{Z}}
    end
end

function _build_inv(s::Relation{Y, Z}) where {Y, Z}
    d = Dict{Z, Vector{Y}}()
    sizehint!(d, length(s))
    for p in _pairs(s)
        push!(get!(d, p.second, Y[]), p.first)
    end
    d
end

# Inverse index R → list of Ds, used by `==`/`in`. Cached for leaf relations.
function inv_index(s::Relation{Y, Z}) where {Y, Z}
    s in _LEAF_RELS || return _build_inv(s)
    lock(_CACHE_LOCK) do
        get!(() -> _build_inv(s), _INV_INDEX_CACHE, s)::Dict{Z, Vector{Y}}
    end
end

# Point access — VecRel bypasses the Dict entirely. Returns iterable of Rs.
_get(s::MapRel{Y, Z}, k::Y) where {Y, Z} = get(fwd_index(s), k, EMPTY_Z(Z))
_get(s::VecRel{E, Z}, k::ID{E}) where {E, Z} = (s.values[k.id],)
@inline EMPTY_Z(::Type{Z}) where Z = Z[]

# VecRel → VecRel: pure index-chasing, no Dict, no Pair allocation per item.
function compose(r::VecRel{E, ID{F}}, s::VecRel{F, Z}) where {E, F, Z}
    rv, sv = r.values, s.values
    out = Vector{Z}(undef, length(rv))
    @inbounds for i in eachindex(rv)
        out[i] = sv[rv[i].id]
    end
    VecRel{E, Z}(out)
end

# VecRel → MapRel: one Dict lookup per row, but no front-pair iteration.
function compose(r::VecRel{E, Y}, s::MapRel{Y, Z}) where {E, Y, Z}
    s_by = fwd_index(s)
    out = Pair{ID{E}, Z}[]
    @inbounds for i in eachindex(r.values)
        zs = get(s_by, r.values[i], nothing)
        zs === nothing && continue
        key = ID{E}(i)
        for z in zs
            push!(out, key => z)
        end
    end
    MapRel{ID{E}, Z}(out)
end

# MapRel → VecRel: each (x, y) does a single array access on s.
function compose(r::MapRel{X, ID{F}}, s::VecRel{F, Z}) where {X, F, Z}
    sv = s.values
    out = Pair{X, Z}[]
    sizehint!(out, length(r.pairs))
    @inbounds for p in r.pairs
        push!(out, p.first => sv[p.second.id])
    end
    MapRel{X, Z}(out)
end

# MapRel → MapRel: the original general path.
function compose(r::MapRel{X, Y}, s::MapRel{Y, Z}) where {X, Y, Z}
    s_by = fwd_index(s)
    out = Pair{X, Z}[]
    for p in r.pairs
        zs = get(s_by, p.second, nothing)
        zs === nothing && continue
        for z in zs
            push!(out, p.first => z)
        end
    end
    MapRel{X, Z}(out)
end

function compose(r::Relation{X, Y}, u::Unary{Y}) where {X, Y}
    s = Set(u.values)
    MapRel{X, Y}([p for p in _pairs(r) if p.second in s])
end

compose(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(collect(intersect(Set(u.values), Set(v.values))))

# Infix → at arrow precedence (below ∧/∨/comparisons). RHS parses as a unit
# without parens: `info → Info.type == "X" ∧ Info.info in vals` is well-formed.
→(r::Relation{X, Y}, s::Relation{Y, Z}) where {X, Y, Z} = compose(r, s)
→(u::Unary{X}, r::Relation{X, Y}) where {X, Y} = compose(u, r)
→(r::Relation{X, Y}, u::Unary{Y}) where {X, Y} = compose(r, u)
→(u::Unary{X}, v::Unary{X}) where X = compose(u, v)

# Navigation: r.field looks up `field` on R via multiple dispatch. Fall through
# to getfield for any name not registered as a Prela field (so internal Julia
# accesses like .singletonname don't trip us during serialization).
function Base.getproperty(r::MapRel{X, R}, name::Symbol) where {X, R}
    name === :pairs && return getfield(r, name)
    if hasmethod(lookup_field, Tuple{Type{R}, Val{name}})
        return compose(r, lookup_field(R, Val(name)))
    end
    return getfield(r, name)
end
function Base.getproperty(r::VecRel{E, R}, name::Symbol) where {E, R}
    name === :values && return getfield(r, name)
    if hasmethod(lookup_field, Tuple{Type{R}, Val{name}})
        return compose(r, lookup_field(R, Val(name)))
    end
    return getfield(r, name)
end
function Base.getproperty(u::Unary{R}, name::Symbol) where R
    name === :values && return getfield(u, name)
    if hasmethod(lookup_field, Tuple{Type{R}, Val{name}})
        return compose(u, lookup_field(R, Val(name)))
    end
    return getfield(u, name)
end

# ===== intersection (∧ at lazy-and precedence) =====

_keys(r::Relation) = Set(_keys_iter(r))
_keys(u::Unary{X}) where X = Set(u.values)

∧(r::Relation{X, Y}, s::Relation{X, Z}) where {X, Y, Z} =
    Unary{X}(collect(intersect(_keys(r), _keys(s))))
function ∧(u::Unary{X}, r::Relation{X, Y}) where {X, Y}
    k = _keys(r)
    Unary{X}([x for x in u.values if x in k])
end
∧(r::Relation{X, Y}, u::Unary{X}) where {X, Y} = u ∧ r
∧(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(collect(intersect(Set(u.values), Set(v.values))))

# ===== union (∨ at lazy-or precedence) =====

∨(r::Relation{X, Y}, s::Relation{X, Y}) where {X, Y} =
    MapRel{X, Y}(unique(vcat(collect(_pairs(r)), collect(_pairs(s)))))
# Mixed-range case: union over keys (predicate OR with differing value types).
∨(r::Relation{X, Y}, s::Relation{X, Z}) where {X, Y, Z} =
    Unary{X}(collect(union(_keys(r), _keys(s))))
∨(u::Unary{X}, r::Relation{X, Y}) where {X, Y} =
    Unary{X}(collect(union(Set(u.values), _keys(r))))
∨(r::Relation{X, Y}, u::Unary{X}) where {X, Y} = u ∨ r
∨(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(unique(vcat(u.values, v.values)))

# ===== set difference (-) =====

function Base.:-(u::Unary{X}, r::Relation{X, Y}) where {X, Y}
    k = _keys(r)
    Unary{X}([x for x in u.values if !(x in k)])
end
function Base.:-(r::Relation{X, Y}, s::Relation{X, Z}) where {X, Y, Z}
    k = _keys(s)
    MapRel{X, Y}([p for p in _pairs(r) if !(p.first in k)])
end

# ===== Rel→Rel restrict (`:`) =====
#
# `(r::Rel{X, Y}) : (s::Rel{X, Z})` filters `s` by `r`'s keys, keeping `s`'s
# values. Used to project a different field after a predicate that yields a
# Rel: `(Info.type == "release dates") : Info.info` returns the actual info
# text for Infos whose type matches. `→` can't express this because both sides
# have the same first column (compose requires LHS's second = RHS's first).
function Base.:(:)(r::Relation{X, Y}, s::Relation{X, Z}) where {X, Y, Z}
    k = _keys(r)
    MapRel{X, Z}([p for p in _pairs(s) if p.first in k])
end
# Unary on the left: a conjunction of predicates reduces to a `Unary` of keys,
# so `(pred₁ ∧ pred₂) : field` projects `field` for the matching domain.
function Base.:(:)(u::Unary{X}, s::Relation{X, Z}) where {X, Z}
    k = Set(u.values)
    MapRel{X, Z}([p for p in _pairs(s) if p.first in k])
end

# ===== range predicates =====

# Equality and `in`: indexed via inv_index — O(1) per probed value after
# the first call (which builds the index).
function Base.:(==)(r::Relation{X, Y}, val) where {X, Y}
    inv = inv_index(r)
    keys = get(inv, val, X[])
    MapRel{X, Y}([k => val for k in keys])
end
function Base.in(r::Relation{X, Y}, vals::Tuple) where {X, Y}
    inv = inv_index(r)
    out = Pair{X, Y}[]
    for v in vals
        for k in get(inv, v, X[])
            push!(out, k => v)
        end
    end
    MapRel{X, Y}(out)
end

# Order predicates: linear scan (can't use value index for ranges).
for op in (:(!=), :<, :>, :(<=), :(>=))
    @eval Base.$op(r::Relation{X, Y}, val) where {X, Y} =
        MapRel{X, Y}([p for p in _pairs(r) if $op(p.second, val)])
end

# Predicate elision: filter the primary FIRST (small), then compose with r.
for op in (:(==), :(!=), :<, :>, :(<=), :(>=))
    @eval Base.$op(r::Relation{X, ID{E}}, val) where {X, E <: Entity} =
        compose(r, $op(primary(E), val))
end
Base.in(r::Relation{X, ID{E}}, vals::Tuple) where {X, E <: Entity} =
    compose(r, in(primary(E), vals))

# regex
Base.:~(r::Relation{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    MapRel{X, Y}([p for p in _pairs(r) if occursin(re, p.second)])
Base.:~(r::Relation{X, ID{E}}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E) ~ re)

# Julia doesn't parse `!~` as a binary operator; use `≁` (input via `\nsim<TAB>`).
≁(r::Relation{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    MapRel{X, Y}([p for p in _pairs(r) if !occursin(re, p.second)])
≁(r::Relation{X, ID{E}}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E)) ≁ re

# ===== product (× at times precedence — tightest binary op in queries) =====

# VecRel × VecRel: dense + dense, zero hash needed.
function ×(r::VecRel{E, Y}, s::VecRel{E, Z}) where {E, Y, Z}
    n = length(r.values)
    @assert n == length(s.values)
    out = Vector{Tuple{Y, Z}}(undef, n)
    @inbounds for i in 1:n
        out[i] = (r.values[i], s.values[i])
    end
    VecRel{E, Tuple{Y, Z}}(out)
end

# Hash-join on the shared key. Build the hash on the smaller side so an
# unfiltered (wide) output column is streamed, not materialized into a hash.
function ×(r::Relation{X, Y}, s::Relation{X, Z}) where {X, Y, Z}
    out = Pair{X, Tuple{Y, Z}}[]
    if length(s) <= length(r)
        s_by = Dict{X, Vector{Z}}()
        for p in _pairs(s)
            push!(get!(s_by, p.first, Z[]), p.second)
        end
        for p in _pairs(r)
            for z in get(s_by, p.first, Z[])
                push!(out, p.first => (p.second, z))
            end
        end
    else
        r_by = Dict{X, Vector{Y}}()
        for p in _pairs(r)
            push!(get!(r_by, p.first, Y[]), p.second)
        end
        for p in _pairs(s)
            for y in get(r_by, p.first, Y[])
                push!(out, p.first => (y, p.second))
            end
        end
    end
    MapRel{X, Tuple{Y, Z}}(out)
end

# Unary as a "constraint" in product: restricts the domain but contributes no column.
function ×(u::Unary{X}, r::Relation{X, Y}) where {X, Y}
    s = Set(u.values)
    MapRel{X, Y}([p for p in _pairs(r) if p.first in s])
end
function ×(r::Relation{X, Y}, u::Unary{X}) where {X, Y}
    s = Set(u.values)
    MapRel{X, Y}([p for p in _pairs(r) if p.first in s])
end
×(u::Unary{X}, v::Unary{X}) where X = u ∧ v

# ===== universe-rooted broadcast ========================================
# `cast.(o₁, o₂, …; p₁, p₂, …)` iterates the Cast universe once. For each
# `i ∈ universe`:
#   - short-circuit if any pred is unsatisfied (i not in its domain);
#   - look up each projection at i and ×-combine results into output rows.
#
# Each `oₖ`/`pₖ` is still a fully eager `Relation`; what's saved is the
# per-operator intermediate (no chain of `(cast ∧ p₁) ∧ p₂` Set/Unary
# allocations, no `: prod` filter pass — one loop instead).

_range_type(::Relation{D, R}) where {D, R} = R

# If a relation's range is `ID{E}` for some entity E, traverse it through
# E's primary field so values are scalars (per Prela's "entities render as
# primary" rule). Used to auto-resolve `keyword` prods/preds in broadcast
# to their string values without an explicit `.<primary>` chain.
function _maybe_traverse(rel::Relation{D, R}) where {D, R}
    if R isa Type && R <: ID
        E = R.parameters[1]
        return compose(rel, primary(E))
    end
    rel
end
_maybe_traverse(x) = x   # fallback for non-Relation values

# Split preds into set-style (Relation/Unary, materialized as keyset) and
# column-style (any other value, interpreted as a predicate function applied
# to the entity's named field). Returns (set_keysets, col_preds).
function _split_preds(preds, ::Type{Y}) where Y
    set_keysets = Set{Y}[]
    col_preds = Tuple{Any, Any}[]
    for (k, v) in pairs(preds)
        if v isa Union{Relation, Unary}
            push!(set_keysets, _keys(v))
        elseif hasmethod(lookup_field, Tuple{Type{Y}, Val{k}})
            push!(col_preds, (_maybe_traverse(lookup_field(Y, Val(k))), v))
        else
            error("pred `$k` is neither a Relation/Unary nor a field-predicate of $Y")
        end
    end
    set_keysets, col_preds
end

@inline function _y_passes(y, rest_sets, col_preds_ix)
    for ks in rest_sets
        y in ks || return false
    end
    for (frel, fix, fn) in col_preds_ix
        any(fn, _idx_lookup(frel, fix, y)) || return false
    end
    return true
end

# Lookup wrapper: VecRel → direct array access; MapRel → use pre-built fwd_index.
@inline _idx_lookup(frel::VecRel{E, R}, _fix, k::ID{E}) where {E, R} = (frel.values[k.id],)
@inline _idx_lookup(frel::MapRel{D, R}, fix::Dict{D, Vector{R}}, k::D) where {D, R} = get(fix, k, EMPTY_Z(R))

# When a column-pred kwarg name matches the field-rel of a prod, the col-pred
# fn becomes a value filter on that prod: only matching values are emitted.
# E.g., `Cast.movie.(keyword; keyword = in(_KW8))` emits only _KW8 keywords.
function _prod_value_filters(prods, preds, ::Type{Y}) where Y
    fns = Vector{Any}(undef, length(prods))
    fill!(fns, nothing)
    for (k, v) in pairs(preds)
        (v isa Union{Relation, Unary}) && continue
        hasmethod(lookup_field, Tuple{Type{Y}, Val{k}}) || continue
        frel = lookup_field(Y, Val(k))
        for i in eachindex(prods)
            prods[i] === frel && (fns[i] = v)
        end
    end
    fns
end

# Filter prod-lookup output by an optional value-pred fn.
@inline _apply_value_fn(vs, ::Nothing) = vs
@inline _apply_value_fn(vs, fn) = Iterators.filter(fn, vs)

# Build a tuple of (field_rel, fwd_idx_or_nothing, fn) for each column pred.
function _precompute_col_preds(col_preds)
    Tuple{Any, Any, Any}[ (frel, frel isa MapRel ? fwd_index(frel) : nothing, fn) for (frel, fn) in col_preds ]
end

# Build prod indexes: VecRel → nothing (direct); MapRel → fwd_index.
function _precompute_prod_idx(prods)
    Any[ p isa MapRel ? fwd_index(p) : nothing for p in prods ]
end

@inline _prod_lookup(p::VecRel{E, R}, _idx, k::ID{E}) where {E, R} = (p.values[k.id],)
@inline _prod_lookup(p::MapRel{D, R}, idx::Dict{D, Vector{R}}, k::D) where {D, R} = get(idx, k, EMPTY_Z(R))

function (u::Unary{X})(prods::Vararg{Any}; preds...) where X
    set_keysets, col_preds = _split_preds(preds, X)
    col_preds_ix = _precompute_col_preds(col_preds)
    prod_value_fns = _prod_value_filters(prods, preds, X)
    prods = map(_maybe_traverse, prods)

    # Pick the smallest set-pred's keyset as the outer iteration domain.
    # Column preds are checked per-element after iter is chosen.
    if isempty(set_keysets)
        iter_keys = u.values
        rest_sets = set_keysets
    else
        sizes = map(length, set_keysets)
        smallest_idx = argmin(sizes)
        iter_keys = set_keysets[smallest_idx]
        rest_sets = Set{X}[ks for (i, ks) in enumerate(set_keysets) if i != smallest_idx]
    end

    if isempty(prods)
        result = X[]
        for x in iter_keys
            _y_passes(x, rest_sets, col_preds_ix) && push!(result, x)
        end
        return Unary{X}(result)
    end

    nprods = length(prods)
    range_types = map(_range_type, prods)
    R = nprods == 1 ? range_types[1] : Tuple{range_types...}
    prod_idx = _precompute_prod_idx(prods)
    out = Pair{X, R}[]

    for x in iter_keys
        _y_passes(x, rest_sets, col_preds_ix) || continue
        if nprods == 1
            vs = _apply_value_fn(_prod_lookup(prods[1], prod_idx[1], x), prod_value_fns[1])
            for v in vs
                push!(out, x => v)
            end
        else
            vals = Vector{Any}(undef, nprods)
            skip = false
            for i in 1:nprods
                vs = collect(_apply_value_fn(_prod_lookup(prods[i], prod_idx[i], x), prod_value_fns[i]))
                if isempty(vs); skip = true; break; end
                vals[i] = vs
            end
            skip && continue
            for combo in Iterators.product(vals...)
                push!(out, x => combo)
            end
        end
    end

    return MapRel{X, R}(out)
end

# Mark Relations and Unaries as broadcast-scalar so `cast.(args; kws)` resolves
# to a single call into the method above (no per-element broadcast iteration).
Base.broadcastable(r::Relation) = Ref(r)
Base.broadcastable(u::Unary) = Ref(u)

# Nested broadcast on a Relation: `r.(prods…; preds…)` where r::Rel{X, Y}.
# Single scan over r's pairs, per-y short-circuit on preds (Y-space), per-y
# prod lookups, emit (x, prod-combo) rows. Lets us write
# `person.(Person.aka.name; gender == "f")` to push filters into the leaf
# entity without paying for an inverse index on r.
function (r::Relation{X, Y})(prods::Vararg{Any}; preds...) where {X, Y}
    set_keysets, col_preds = _split_preds(preds, Y)
    col_preds_ix = _precompute_col_preds(col_preds)
    prod_value_fns = _prod_value_filters(prods, preds, Y)
    prods = map(_maybe_traverse, prods)
    nprods = length(prods)
    range_types = map(_range_type, prods)
    R = nprods == 0 ? Nothing : (nprods == 1 ? range_types[1] : Tuple{range_types...})
    prod_idx = _precompute_prod_idx(prods)
    out = Pair{X, R}[]

    for p in _pairs(r)
        x, y = p.first, p.second
        _y_passes(y, set_keysets, col_preds_ix) || continue
        if nprods == 1
            vs = _apply_value_fn(_prod_lookup(prods[1], prod_idx[1], y), prod_value_fns[1])
            for v in vs
                push!(out, x => v)
            end
        else
            vals = Vector{Any}(undef, nprods)
            skip = false
            for i in 1:nprods
                vs = collect(_apply_value_fn(_prod_lookup(prods[i], prod_idx[i], y), prod_value_fns[i]))
                if isempty(vs); skip = true; break; end
                vals[i] = vs
            end
            skip && continue
            for combo in Iterators.product(vals...)
                push!(out, x => combo)
            end
        end
    end

    return MapRel{X, R}(out)
end

# ===== schema sugar =====
#
# Declare an entity type + its field relations in one place:
#
#   @entity Movie begin
#       title           :: String
#       production_year :: Int
#       keyword         :: ID{Keyword}    # entity-typed range
#       company         :: ID{Company}
#   end
#
# Emits:
#   - `abstract type Movie <: Entity end`
#   - one `const <field>` per field, initialized to an empty Rel{ID{Movie}, T}
#   - one `lookup_field(::Type{ID{Movie}}, ::Val{:<field>})` method per field
#   - `primary(::Type{Movie}) = <first_field>`  (convention)
#
# Data is loaded later by appending to the `.pairs` vector of each relation.

macro entity(entity_sym, block)
    entity_sym isa Symbol || error("@entity expects a symbol entity name")
    (block isa Expr && block.head === :block) || error("@entity expects `begin ... end`")

    out = Expr(:block)
    push!(out.args, :($(GlobalRef(@__MODULE__, :_declare_if_needed))(@__MODULE__, $(QuoteNode(entity_sym)))))

    id_type    = :($(GlobalRef(@__MODULE__, :ID)){$(esc(entity_sym))})
    rel_type   = GlobalRef(@__MODULE__, :Rel)
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
        # primary = first field
        push!(out.args, quote
            $primary_fn(::Type{$(esc(entity_sym))}) = $(esc(field_consts[1]))
        end)

        # Register fields for @expose to read.
        push!(out.args, :(
            $(GlobalRef(@__MODULE__, :_ENTITY_FIELDS))[$(QuoteNode(entity_sym))] =
                $(field_names)
        ))

        # Per-entity Base.getproperty: name === :field && return _Entity_field;
        # falls through to default for non-matching (DataType internals).
        gp_body = Expr(:block)
        for (fname, fconst) in zip(field_names, field_consts)
            push!(gp_body.args, :(name === $(QuoteNode(fname)) && return $(esc(fconst))))
        end
        push!(gp_body.args, :(return getfield($(esc(entity_sym)), name)))
        push!(out.args, :(
            Base.getproperty(::Type{$(esc(entity_sym))}, name::Symbol) = $gp_body
        ))
        # Bypass our Type{E}.name override for Julia internals: when a Prela
        # field is also a DataType property (notably `:name`), `nameof(E)` and
        # `show_datatype(E)` would otherwise hit our override (returning the
        # Prela Rel) and crash. Provide entity-specific overrides for those.
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

# Forward declare entity types — needed when schemas have cyclic references
# (e.g. Movie references MovieLink which references Movie back).
macro declare(syms...)
    out = Expr(:block)
    for s in syms
        s isa Symbol || error("@declare expects symbols")
        push!(out.args, :($(GlobalRef(@__MODULE__, :_declare_if_needed))(@__MODULE__, $(QuoteNode(s)))))
    end
    out
end
export @declare

# `@expose Movie` declares short-name `const`s for each of Movie's fields,
# bound to the qualified internal storage (`_Movie_title`, etc.). Use it on
# the "root" entity whose fields you want to use bare in queries.
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
