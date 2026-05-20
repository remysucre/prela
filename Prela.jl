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

export Rel, Unary, Entity, ID, primary, lookup_field

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
Base.show(io::IO, a::ID{E}) where E = print(io, nameof(E), "(", a.id, ")")

function primary end
function lookup_field end

# Macro-time registry of entity → field names. Populated by `@entity` (in its
# emitted top-level block) and read by `@expose` at its macro-expansion time.
const _ENTITY_FIELDS = Dict{Symbol, Vector{Symbol}}()

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
    # Predicate elision when range is an entity ID: auto-traverse to primary.
    @eval Base.$op(r::Rel{X, ID{E}}, val) where {X, E <: Entity} =
        $op(compose(r, primary(E)), val)
end

# in (membership in a tuple)
function Base.in(r::Rel{X, Y}, vals::Tuple) where {X, Y}
    s = Set(vals)
    Rel{X, Y}([p for p in r.pairs if p.second in s])
end
Base.in(r::Rel{X, ID{E}}, vals::Tuple) where {X, E <: Entity} =
    in(compose(r, primary(E)), vals)

# regex
Base.:~(r::Rel{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    Rel{X, Y}([p for p in r.pairs if occursin(re, p.second)])
Base.:~(r::Rel{X, ID{E}}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E)) ~ re

# Julia doesn't parse `!~` as a binary operator; use `≁` (input via `\nsim<TAB>`).
≁(r::Rel{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    Rel{X, Y}([p for p in r.pairs if !occursin(re, p.second)])
≁(r::Rel{X, ID{E}}, re::Regex) where {X, E <: Entity} =
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
    push!(out.args, :(abstract type $(esc(entity_sym)) <: $(GlobalRef(@__MODULE__, :Entity)) end))

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
    end

    out
end

export @entity

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
