module Prela

# Core algebraic-relational library.
#
# Two value types:
#   Unary{R}    — a set of R values.
#   Rel{D, R}   — a binary relation D -> R (stored as Pair{D,R} vector).
#
# Operators (`Base.X`):
#   .       sequential composition (via getproperty when navigating fields)
#   &       intersection (keys)
#   |       union (compatible relations)
#   -       set difference (keys not in rhs)
#   :       select (lhs is unary; rhs is binary; restrict rhs's domain)
#   ==/!=/  range predicates (filter binary by range value)
#   </>/<=/>=
#   in      `r in (a, b, c)` filter by range membership
#   ~/!~    regex match/non-match against String range
#   r.(args...)   broadcast: compose r with parallel-composed args
#
# Entity types subtype `Entity`; each declares `primary(::Type{E})` returning
# the canonical scalar relation. Predicate elision then auto-traverses when a
# Rel{X, E} is compared against a scalar.

import Base.Broadcast: BroadcastStyle, broadcasted, materialize

export Rel, Unary, Entity, primary, lookup_field

abstract type Entity end

function primary end
function lookup_field end

struct Unary{R}
    values::Vector{R}
end
Unary(vs::Vector{R}) where R = Unary{R}(vs)

struct Rel{D, R}
    pairs::Vector{Pair{D, R}}
end
Rel(ps::Vector{Pair{D, R}}) where {D, R} = Rel{D, R}(ps)

# ===== composition =====

function compose(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.first in s])
end

function compose(r::Rel{X, Y}, s::Rel{Y, Z}) where {X, Y, Z}
    s_by = Dict{Y, Vector{Z}}()
    for p in s.pairs
        push!(get!(s_by, p.first, Z[]), p.second)
    end
    out = Pair{X, Z}[]
    for p in r.pairs
        if haskey(s_by, p.second)
            for z in s_by[p.second]
                push!(out, p.first => z)
            end
        end
    end
    Rel{X, Z}(out)
end

function compose(r::Rel{X, Y}, u::Unary{Y}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.second in s])
end

compose(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(collect(intersect(Set(u.values), Set(v.values))))

# Navigation: r.field looks up `field` on R via multiple dispatch.
function Base.getproperty(r::Rel{X, R}, name::Symbol) where {X, R}
    name === :pairs && return getfield(r, name)
    compose(r, lookup_field(R, Val(name)))
end
function Base.getproperty(u::Unary{R}, name::Symbol) where R
    name === :values && return getfield(u, name)
    compose(u, lookup_field(R, Val(name)))
end

# ===== intersection & =====

_keys(r::Rel) = Set(p.first for p in r.pairs)

Base.:&(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z} =
    Unary{X}(collect(intersect(_keys(r), _keys(s))))
Base.:&(u::Unary{X}, r::Rel{X, Y}) where {X, Y} =
    Unary{X}([x for x in u.values if x in _keys(r)])
Base.:&(r::Rel{X, Y}, u::Unary{X}) where {X, Y} = u & r
Base.:&(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(collect(intersect(Set(u.values), Set(v.values))))

# ===== union | =====

Base.:|(r::Rel{X, Y}, s::Rel{X, Y}) where {X, Y} =
    Rel{X, Y}(unique(vcat(r.pairs, s.pairs)))
# Mixed-range case: union over keys (predicate OR).
Base.:|(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z} =
    Unary{X}(collect(union(_keys(r), _keys(s))))
Base.:|(u::Unary{X}, r::Rel{X, Y}) where {X, Y} =
    Unary{X}(collect(union(Set(u.values), _keys(r))))
Base.:|(r::Rel{X, Y}, u::Unary{X}) where {X, Y} = u | r
Base.:|(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(unique(vcat(u.values, v.values)))

# ===== set difference - =====

function Base.:-(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    k = _keys(r)
    Unary{X}([x for x in u.values if !(x in k)])
end
function Base.:-(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z}
    k = _keys(s)
    Rel{X, Y}([p for p in r.pairs if !(p.first in k)])
end

# ===== select : =====

Base.:(:)(u::Unary{X}, r::Rel{X, Y}) where {X, Y} = compose(u, r)
function Base.:(:)(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z}
    k = _keys(r)
    Rel{X, Z}([p for p in s.pairs if p.first in k])
end

# ===== range predicates =====

for op in (:(==), :(!=), :<, :>, :(<=), :(>=))
    @eval Base.$op(r::Rel{X, Y}, val) where {X, Y} =
        Rel{X, Y}([p for p in r.pairs if $op(p.second, val)])
    # Predicate elision when range is an Entity
    @eval Base.$op(r::Rel{X, E}, val) where {X, E <: Entity} =
        $op(compose(r, primary(E)), val)
end

# in (membership in a tuple)
function Base.in(r::Rel{X, Y}, vals::Tuple) where {X, Y}
    s = Set(vals)
    Rel{X, Y}([p for p in r.pairs if p.second in s])
end
Base.in(r::Rel{X, E}, vals::Tuple) where {X, E <: Entity} = in(compose(r, primary(E)), vals)

# regex
Base.:~(r::Rel{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    Rel{X, Y}([p for p in r.pairs if occursin(re, p.second)])
Base.:~(r::Rel{X, E}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E)) ~ re

# Julia doesn't parse `!~` as a binary operator; use `≁` (input via `\nsim<TAB>`).
≁(r::Rel{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    Rel{X, Y}([p for p in r.pairs if !occursin(re, p.second)])
≁(r::Rel{X, E}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E)) ≁ re
export ≁

# ===== product (parallel composition) =====

function product(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z}
    s_by = Dict{X, Vector{Z}}()
    for p in s.pairs
        push!(get!(s_by, p.first, Z[]), p.second)
    end
    out = Pair{X, Tuple{Y, Z}}[]
    for p in r.pairs
        for z in get(s_by, p.first, Z[])
            push!(out, p.first => (p.second, z))
        end
    end
    Rel{X, Tuple{Y, Z}}(out)
end

# Unary as a "constraint" in parallel composition: restricts the domain but
# doesn't contribute a column.
function product(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.first in s])
end
function product(r::Rel{X, Y}, u::Unary{X}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.first in s])
end
product(u::Unary{X}, v::Unary{X}) where X = u & v

# ===== broadcast (r.(args...)) =====

struct PrelaStyle <: BroadcastStyle end
BroadcastStyle(::Type{<:Rel}) = PrelaStyle()
BroadcastStyle(::Type{<:Unary}) = PrelaStyle()

# Don't try to iterate our types; treat them as scalars for broadcasting.
Base.Broadcast.broadcastable(r::Rel) = r
Base.Broadcast.broadcastable(u::Unary) = u

function Base.Broadcast.broadcasted(::PrelaStyle, r, args...)
    isempty(args) && return r
    prod = args[1]
    for a in Base.tail(args)
        prod = product(prod, a)
    end
    compose(r, prod)
end

Base.Broadcast.materialize(x::Rel) = x
Base.Broadcast.materialize(x::Unary) = x

end # module
