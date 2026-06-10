# Single-threaded warm benchmark used by the comparison plots in
# rust/bench. Loads JOB or TPC-H, runs each query twice in registration
# order, and writes `name <tab> warm_seconds` lines to stdout.
#
# Run from the julia/ dir:
#   julia --project=. -t1 bench.jl job   > ../rust/bench/data/julia_job.txt
#   julia --project=. -t1 bench.jl tpch  > ../rust/bench/data/julia_tpch.txt

using Printf

suite = get(ARGS, 1, "job")
ENV["PRELA_SKIP_RUNALL"] = "1"

# ENGINE=interp|staged selects the engine for every scan (builds + final).
const ENG = get(ENV, "ENGINE", "staged") == "interp" ? Main.Prela.Interp() :
                                                       Main.Prela.Staged()

# `run()` builds the plan (`f()` — which eagerly materializes any `bitset`
# indexes), lowers its access mode (`prepare`, now ~free since it's type-stable),
# and drives it to the result. Everything here is real query work — index/cache/
# bitset building included — so it's all timed; only JIT is excluded (cold pass).
if suite == "job"
    include("JOB.jl")
    include("queries.jl")
    # JOB entries: (name, oracle, f)  — f() returns a Query.
    qs = [(name, () -> Main._vals(f(); eng = ENG)) for (name, _oracle, f) in Main._Q]
elseif suite == "tpch"
    include("TPCH.jl")
    variant = get(ENV, "QS", "idiomatic")
    variant in ("idiomatic", "optimized") ||
        error("QS must be \"idiomatic\" or \"optimized\", got $(repr(variant))")
    include("tpch_queries_$(variant).jl")
    # TPC-H entries: (name, oracle, f, sort_by, limit, row).
    qs = Tuple{String, Function}[]
    for (name, _oracle, f, sort_by, limit, row) in Main._QT
        run = let f=f, sb=sort_by, lim=limit, r=row
            () -> Main._vals_tpch(f(), sb, lim, r; eng = ENG)
        end
        push!(qs, (name, run))
    end
else
    error("usage: julia bench.jl {job|tpch}")
end

# Cold pass — triggers JIT/specialization; results discarded.
for (_, run) in qs
    try run() catch end
end
GC.gc(true)  # clear cold-pass garbage so the first warm query isn't penalized by it

# Warm pass — wall-clock per query (build + prepare + drive), written to stdout.
for (name, run) in qs
    t = time_ns()
    _ = try run() catch e e end
    dt = (time_ns() - t) / 1e9
    @printf "%s\t%.6f\n" name dt
end
