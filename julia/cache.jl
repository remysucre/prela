# Persistent on-disk cache using raw binary + Mmap for every Rel: ID-only Rels
# (the fast path, the bulk of JOB data) and String-valued Rels alike.
#
# - Vector{Pair{ID{E1}, ID{E2}}}: 16-byte fixed records, mmapped and
#   reinterpreted as Vector{Pair{Int, Int}} → constant-time "load".
# - Vector{Pair{ID{E}, Int}}: same.
# - Vector{Pair{ID{E}, String}}: one Int-offset file + one byte file. mmap
#   both, build Pair vector via O(n) copy. Still single-digit-second total.

using Mmap

const CACHE_DIR = joinpath(@__DIR__, "..", "cache")

# ---------------- bits-only Rels (ID×ID, ID×Int) ----------------
# Stored as: raw Vector{Pair{Int, Int}} bytes. Just `write(io, pairs)`.

function _bits_save(path::String, rel::Prela.Rel{D, R}) where {D, R}
    n = length(rel.pairs)
    # Reinterpret as Pair{Int, Int} for raw write (D/R wrap Int internally).
    raw = reinterpret(Pair{Int, Int}, rel.pairs)
    open(path, "w") do io
        write(io, UInt64(n))
        write(io, raw)
    end
end

function _bits_load!(rel::Prela.Rel{D, R}, path::String) where {D, R}
    io = open(path, "r")
    n = Int(read(io, UInt64))
    raw = Mmap.mmap(io, Vector{Pair{Int, Int}}, n, 8)  # offset past UInt64 header
    pairs_view = reinterpret(Pair{D, R}, raw)
    resize!(rel.pairs, n)
    copyto!(rel.pairs, pairs_view)
    close(io)
end

# ---------------- String-valued Rels (ID×String) ----------------
# Stored as: header (n, total_bytes) + n offsets (UInt32) + n keys (Int) + bytes.

function _str_save(path::String, rel::Prela.Rel{D, String}) where D
    n = length(rel.pairs)
    keys = Vector{Int}(undef, n)
    offsets = Vector{UInt32}(undef, n + 1)
    offsets[1] = 0
    bytes_total = 0
    for (i, p) in enumerate(rel.pairs)
        keys[i] = p.first.id
        bytes_total += sizeof(p.second)
        offsets[i + 1] = UInt32(bytes_total)
    end
    open(path, "w") do io
        write(io, UInt64(n))
        write(io, keys)
        write(io, offsets)
        for p in rel.pairs
            write(io, p.second)
        end
    end
end

function _str_load!(rel::Prela.Rel{D, String}, path::String) where D
    io = open(path, "r")
    n = Int(read(io, UInt64))
    keys_off    = 8
    offsets_off = keys_off + n * sizeof(Int)
    bytes_off   = offsets_off + (n + 1) * sizeof(UInt32)
    keys    = Mmap.mmap(io, Vector{Int},    n,     keys_off)
    offsets = Mmap.mmap(io, Vector{UInt32}, n + 1, offsets_off)
    bytes   = Mmap.mmap(io, Vector{UInt8},  filesize(io) - bytes_off, bytes_off)
    resize!(rel.pairs, n)
    @inbounds for i in 1:n
        s = unsafe_string(pointer(bytes, offsets[i] + 1), offsets[i + 1] - offsets[i])
        rel.pairs[i] = D(keys[i]) => s
    end
    close(io)
end

# ---------------- dispatch ----------------

_is_bits_pair(::Type{Pair{D, R}}) where {D, R} = isbitstype(Pair{D, R})

function _save_rel(path::String, rel::Prela.Rel{D, R}) where {D, R}
    if _is_bits_pair(Pair{D, R})
        _bits_save(path, rel)
    elseif R === String
        _str_save(path, rel)
    else
        error("unsupported Rel{$D, $R}")
    end
end

function _load_rel!(rel::Prela.Rel{D, R}, path::String) where {D, R}
    if _is_bits_pair(Pair{D, R})
        _bits_load!(rel, path)
    elseif R === String
        _str_load!(rel, path)
    else
        error("unsupported Rel{$D, $R}")
    end
end

function _each_rel()
    out = Tuple{Type, Symbol, Prela.Relation}[]
    for (entity_sym, fields) in Prela._ENTITY_FIELDS
        E = getfield(Main, entity_sym)
        for f in fields
            rel = Prela.lookup_field(Prela.ID{E}, Val(f))
            push!(out, (E, f, rel))
        end
    end
    out
end

_cache_path(E, f) = joinpath(CACHE_DIR, "$(nameof(E))_$(f).bin")

function save_cache!()
    mkpath(CACHE_DIR)
    t = time()
    total = 0
    for (E, f, rel) in _each_rel()
        _save_rel(_cache_path(E, f), rel)
        total += length(rel.pairs)
    end
    println("  cache: saved $total pairs to $CACHE_DIR ($(round(time()-t; digits=1))s)")
end

function load_cache!()
    isdir(CACHE_DIR) || return false
    # Pre-check every rel before loading any — a partial cache (e.g. after a
    # schema change adds new rels) must be a clean full miss, otherwise the
    # parquet fallback would append on top of cache-loaded rels.
    for (E, f, _) in _each_rel()
        path = _cache_path(E, f)
        isfile(path) || (println("  cache miss: $path"); return false)
    end
    t = time()
    total = 0
    for (E, f, rel) in _each_rel()
        _load_rel!(rel, _cache_path(E, f))
        total += length(rel.pairs)
    end
    println("  cache: loaded $total pairs from $CACHE_DIR ($(round(time()-t; digits=1))s)")
    true
end
