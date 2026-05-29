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

if suite == "job"
    include("JOB.jl")
    include("queries.jl")
    # JOB entries: (name, oracle, f)  — f() returns a Query.
    eval_fn = (f) -> Main._vals(f())
    qs = [(name, oracle, () -> eval_fn(f)) for (name, oracle, f) in Main._Q]
elseif suite == "tpch"
    include("TPCH.jl")
    variant = get(ENV, "QS", "idiomatic")
    variant in ("idiomatic", "optimized") ||
        error("QS must be \"idiomatic\" or \"optimized\", got $(repr(variant))")
    include("tpch_queries_$(variant).jl")
    # TPC-H entries: (name, oracle, f, sort_by, limit, row).
    qs = Tuple{String, String, Function}[]
    for (name, oracle, f, sort_by, limit, row) in Main._QT
        run = let f=f, sb=sort_by, lim=limit, r=row
            () -> Main._vals_tpch(f(), sb, lim, r)
        end
        push!(qs, (name, oracle, run))
    end
else
    error("usage: julia bench.jl {job|tpch}")
end

# Cold pass — triggers JIT/specialization; results discarded.
for (_, _, run) in qs
    try run() catch end
end
GC.gc(true)  # clear cold-pass garbage so the first warm query isn't penalized by it

# Warm pass — wall-clock per query, written to stdout for the plot scripts.
for (name, _oracle, run) in qs
    t = time_ns()
    _ = try run() catch e e end
    dt = (time_ns() - t) / 1e9
    @printf "%s\t%.6f\n" name dt
end
