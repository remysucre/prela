# TPC-H suite machinery: the query registry, result formatting, the
# oracle-file loader, and `runall_tpch()`. Query definitions live in
# `tpch_queries_idiomatic.jl` (the baseline 22) and
# `tpch_queries_optimized.jl` (an overlay of hand-tuned rewrites).

using Printf
using Dates

# --- registry: (name, oracle, thunk, sort_by, limit, row) ---
if !@isdefined(_QT)
    const _QT = Tuple{String, String, Function, Function, Union{Nothing,Int}, Function}[]
    # Set while an overlay file includes its base file, so only the outermost
    # include auto-runs the suite (see `_autorun_tpch`).
    const _TPCH_DEFER = Ref(false)
end

# Clear on every include so leftover duplicates (e.g. from previous includet
# sessions where Revise re-fired do-blocks) don't accumulate.
empty!(_QT)

function _q_tpch(name, oracle, f; sort_by = identity, limit = nothing, row = _default_row)
    entry = (name, oracle, f, sort_by, limit, row)
    idx = findfirst(t -> t[1] == name, _QT)
    idx === nothing ? push!(_QT, entry) : (_QT[idx] = entry)
    nothing
end

# --- oracles ---
# The long oracle strings are checked into the repo; regenerate with
# rust/bench/regen_tpch_oracles.sh (DuckDB, single-threaded, on the same
# SF=1 parquet data).
_oracle(n::Int) = read(joinpath(@__DIR__, "..", "oracles", "tpch", "Q$n.txt"), String)

# --- formatting ---
_fmt(x::Integer) = string(x)
_fmt(x::AbstractString) = String(x)
_fmt(x::Date) = string(x)
_fmt(x::AbstractFloat) = @sprintf("%.2f", x)
_fmt(x::Prela.ID) = string(x.id)
_fmt(x::Tuple) = (Base.Iterators.flatten(_fmt_iter(y) for y in x))
_fmt_iter(x) = (_fmt(x),)
_fmt_iter(x::Tuple) = Base.Iterators.flatten(_fmt_iter(y) for y in x)

# Default row formatter: flatten key then value into pipe-separated cols.
_default_row(k, v) = String[String(c) for c in _fmt_iter((k, v))]
# Value-only formatter (drops the key) — for scalar queries.
_value_only(_, v) = String[c for c in _fmt_iter(v)]

# --- runner ---
# prepare = compilation: bench prepares untimed, then times `_vals_tpch_prepared`.
function _vals_tpch_prepared(pq, sort_by, limit, row; eng = Prela.Staged())
    rows = Tuple{Any, Any}[]
    Prela.scan(eng, pq, (x, y) -> push!(rows, (x, y)))
    isempty(rows) && return "(empty)"
    sort!(rows; by = sort_by)
    limit !== nothing && length(rows) > limit && resize!(rows, limit)
    lines = String[]
    for (k, v) in rows
        push!(lines, join(row(k, v), "|"))
    end
    join(lines, "\n")
end
_vals_tpch(q, sort_by, limit, row; eng = Prela.Staged()) =
    _vals_tpch_prepared(Prela.prepare(eng, q), sort_by, limit, row; eng)

function runall_tpch()
    ok = 0
    for (name, oracle, f, sort_by, limit, row) in _QT
        t = time()
        got = try
            _vals_tpch(f(), sort_by, limit, row)
        catch e
            "ERROR: " * sprint(showerror, e)[1:min(200, end)]
        end
        dt = round(time() - t; digits = 2)
        pass = got == oracle
        pass && (ok += 1)
        status = pass ? "ok  " : "DIFF"
        println(rpad(name, 5), " ", status, "  ", lpad("$(dt)s", 7))
        if !pass
            println("  got    : ", got[1:min(500, end)])
            println("  oracle : ", oracle[1:min(500, end)])
        end
    end
    println("\n$ok / $(length(_QT)) TPC-H queries match reference")
end

# Auto-run on include so the workflow is: edit, `include("tpch_queries_*.jl")`,
# results print. Skippable for the bench scripts via PRELA_SKIP_RUNALL=1;
# suppressed for a base file included by an overlay (only the outermost
# include runs the suite, after its overrides are registered).
_autorun_tpch() =
    (!_TPCH_DEFER[] && get(ENV, "PRELA_SKIP_RUNALL", "0") == "0") ? runall_tpch() : nothing
