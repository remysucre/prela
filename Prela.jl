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

export Rel, Unary, Entity, ID, primary, lookup_field, →, ∧, ∨, ×, ≁

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

struct Rel{D, R}
    pairs::Vector{Pair{D, R}}
end
Rel(ps::Vector{Pair{D, R}}) where {D, R} = Rel{D, R}(ps)

# ===== composition (→ at arrow precedence) =====

function compose(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.first in s])
end

# Per-Rel forward-index cache (D → list of Rs). Lazily built; first compose
# call against a Rel pays the index-build cost, subsequent calls reuse it.
const _FWD_INDEX_CACHE = IdDict{Any, Any}()

function fwd_index(s::Rel{Y, Z}) where {Y, Z}
    get!(_FWD_INDEX_CACHE, s) do
        d = Dict{Y, Vector{Z}}()
        sizehint!(d, length(s.pairs))
        for p in s.pairs
            push!(get!(d, p.first, Z[]), p.second)
        end
        d::Dict{Y, Vector{Z}}
    end::Dict{Y, Vector{Z}}
end

# Per-Rel inverse-index cache (R → list of Ds). Used by `==` and `in` against
# scalar literals so repeated value-filter queries don't re-scan.
const _INV_INDEX_CACHE = IdDict{Any, Any}()

function inv_index(s::Rel{Y, Z}) where {Y, Z}
    get!(_INV_INDEX_CACHE, s) do
        d = Dict{Z, Vector{Y}}()
        sizehint!(d, length(s.pairs))
        for p in s.pairs
            push!(get!(d, p.second, Y[]), p.first)
        end
        d::Dict{Z, Vector{Y}}
    end::Dict{Z, Vector{Y}}
end

function compose(r::Rel{X, Y}, s::Rel{Y, Z}) where {X, Y, Z}
    s_by = fwd_index(s)
    out = Pair{X, Z}[]
    for p in r.pairs
        zs = get(s_by, p.second, nothing)
        zs === nothing && continue
        for z in zs
            push!(out, p.first => z)
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

# Infix → at arrow precedence (below ∧/∨/comparisons). RHS parses as a unit
# without parens: `info → Info.type == "X" ∧ Info.info in vals` is well-formed.
→(r::Rel{X, Y}, s::Rel{Y, Z}) where {X, Y, Z} = compose(r, s)
→(u::Unary{X}, r::Rel{X, Y}) where {X, Y} = compose(u, r)
→(r::Rel{X, Y}, u::Unary{Y}) where {X, Y} = compose(r, u)
→(u::Unary{X}, v::Unary{X}) where X = compose(u, v)

# Navigation: r.field looks up `field` on R via multiple dispatch. Fall through
# to getfield for any name not registered as a Prela field (so internal Julia
# accesses like .singletonname don't trip us during serialization).
function Base.getproperty(r::Rel{X, R}, name::Symbol) where {X, R}
    name === :pairs && return getfield(r, name)
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

_keys(r::Rel) = Set(p.first for p in r.pairs)

∧(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z} =
    Unary{X}(collect(intersect(_keys(r), _keys(s))))
function ∧(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    k = _keys(r)
    Unary{X}([x for x in u.values if x in k])
end
∧(r::Rel{X, Y}, u::Unary{X}) where {X, Y} = u ∧ r
∧(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(collect(intersect(Set(u.values), Set(v.values))))

# ===== union (∨ at lazy-or precedence) =====

∨(r::Rel{X, Y}, s::Rel{X, Y}) where {X, Y} =
    Rel{X, Y}(unique(vcat(r.pairs, s.pairs)))
# Mixed-range case: union over keys (predicate OR with differing value types).
∨(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z} =
    Unary{X}(collect(union(_keys(r), _keys(s))))
∨(u::Unary{X}, r::Rel{X, Y}) where {X, Y} =
    Unary{X}(collect(union(Set(u.values), _keys(r))))
∨(r::Rel{X, Y}, u::Unary{X}) where {X, Y} = u ∨ r
∨(u::Unary{X}, v::Unary{X}) where X =
    Unary{X}(unique(vcat(u.values, v.values)))

# ===== set difference (-) =====

function Base.:-(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    k = _keys(r)
    Unary{X}([x for x in u.values if !(x in k)])
end
function Base.:-(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z}
    k = _keys(s)
    Rel{X, Y}([p for p in r.pairs if !(p.first in k)])
end

# ===== Rel→Rel restrict (`:`) =====
#
# `(r::Rel{X, Y}) : (s::Rel{X, Z})` filters `s` by `r`'s keys, keeping `s`'s
# values. Used to project a different field after a predicate that yields a
# Rel: `(Info.type == "release dates") : Info.info` returns the actual info
# text for Infos whose type matches. `→` can't express this because both sides
# have the same first column (compose requires LHS's second = RHS's first).
function Base.:(:)(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z}
    k = _keys(r)
    Rel{X, Z}([p for p in s.pairs if p.first in k])
end
# Unary on the left: a conjunction of predicates reduces to a `Unary` of keys,
# so `(pred₁ ∧ pred₂) : field` projects `field` for the matching domain.
function Base.:(:)(u::Unary{X}, s::Rel{X, Z}) where {X, Z}
    k = Set(u.values)
    Rel{X, Z}([p for p in s.pairs if p.first in k])
end

# ===== range predicates =====

# Equality and `in`: indexed via inv_index — O(1) per probed value after
# the first call (which builds the index).
function Base.:(==)(r::Rel{X, Y}, val) where {X, Y}
    inv = inv_index(r)
    keys = get(inv, val, X[])
    Rel{X, Y}([k => val for k in keys])
end
function Base.in(r::Rel{X, Y}, vals::Tuple) where {X, Y}
    inv = inv_index(r)
    out = Pair{X, Y}[]
    for v in vals
        for k in get(inv, v, X[])
            push!(out, k => v)
        end
    end
    Rel{X, Y}(out)
end

# Order predicates: linear scan (can't use value index for ranges).
for op in (:(!=), :<, :>, :(<=), :(>=))
    @eval Base.$op(r::Rel{X, Y}, val) where {X, Y} =
        Rel{X, Y}([p for p in r.pairs if $op(p.second, val)])
end

# Predicate elision: filter the primary FIRST (small), then compose with r.
for op in (:(==), :(!=), :<, :>, :(<=), :(>=))
    @eval Base.$op(r::Rel{X, ID{E}}, val) where {X, E <: Entity} =
        compose(r, $op(primary(E), val))
end
Base.in(r::Rel{X, ID{E}}, vals::Tuple) where {X, E <: Entity} =
    compose(r, in(primary(E), vals))

# regex
Base.:~(r::Rel{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    Rel{X, Y}([p for p in r.pairs if occursin(re, p.second)])
Base.:~(r::Rel{X, ID{E}}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E) ~ re)

# Julia doesn't parse `!~` as a binary operator; use `≁` (input via `\nsim<TAB>`).
≁(r::Rel{X, Y}, re::Regex) where {X, Y <: AbstractString} =
    Rel{X, Y}([p for p in r.pairs if !occursin(re, p.second)])
≁(r::Rel{X, ID{E}}, re::Regex) where {X, E <: Entity} =
    compose(r, primary(E)) ≁ re

# ===== product (× at times precedence — tightest binary op in queries) =====

# Hash-join on the shared key. Build the hash on the smaller side so an
# unfiltered (wide) output column is streamed, not materialized into a hash.
function ×(r::Rel{X, Y}, s::Rel{X, Z}) where {X, Y, Z}
    out = Pair{X, Tuple{Y, Z}}[]
    if length(s.pairs) <= length(r.pairs)
        s_by = Dict{X, Vector{Z}}()
        for p in s.pairs
            push!(get!(s_by, p.first, Z[]), p.second)
        end
        for p in r.pairs
            for z in get(s_by, p.first, Z[])
                push!(out, p.first => (p.second, z))
            end
        end
    else
        r_by = Dict{X, Vector{Y}}()
        for p in r.pairs
            push!(get!(r_by, p.first, Y[]), p.second)
        end
        for p in s.pairs
            for y in get(r_by, p.first, Y[])
                push!(out, p.first => (y, p.second))
            end
        end
    end
    Rel{X, Tuple{Y, Z}}(out)
end

# Unary as a "constraint" in product: restricts the domain but contributes no column.
function ×(u::Unary{X}, r::Rel{X, Y}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.first in s])
end
function ×(r::Rel{X, Y}, u::Unary{X}) where {X, Y}
    s = Set(u.values)
    Rel{X, Y}([p for p in r.pairs if p.first in s])
end
×(u::Unary{X}, v::Unary{X}) where X = u ∧ v

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
