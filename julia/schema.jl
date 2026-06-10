# Schema machinery: `@entity`/`@declare`/`@expose` macros and the load-time
# sealing of `Staging` leaves into their static storage layouts.

# Registry of declared entities and their fields, filled by `@entity`.
const _ENTITY_FIELDS = Dict{Symbol, Vector{Symbol}}()
# (entity, field) pairs declared `Multi{…}` in @entity — sealed to MultiRel.
const _MULTI_FIELDS = Set{Tuple{Symbol, Symbol}}()

function _declare_if_needed(mod::Module, sym::Symbol)
    isdefined(mod, sym) && return
    Core.eval(mod, Expr(:abstract, Expr(:(<:), sym, GlobalRef(@__MODULE__, :Entity))))
end
# ===== sealing: Staging → static leaf storage ===========================
# After load, each entity leaf is sealed once from its `pairs` into the
# concrete layout dictated by its declared multiplicity + the loaded data:
#   declared 1:1  →  VecRel    (keys fill 1..n)
#                 →  SparseRel (keys have gaps)
#   declared Multi →  MultiRel
# Sealing replaces the per-leaf `const` binding (see `seal_entities!`), so
# `lookup_field` and the bare-name exposures resolve to the sealed object.

function seal(r::Staging{ID{E}, R}, n::Int, multi::Bool, label) where {E, R}
    # multi-valued → dense forward index sized to the entity universe `n` (so
    # every valid id is directly indexable; see `_dense_fwd` in plan.jl).
    multi && return MultiRel{E, R}(_dense_fwd(r.pairs, n))
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
