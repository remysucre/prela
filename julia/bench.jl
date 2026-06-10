# Single-threaded warm benchmark used by the comparison plots in
# rust/bench. Loads JOB or TPC-H, runs each query twice in registration
# order, and writes `name <tab> warm_seconds` lines to stdout.
#
# Run from the julia/ dir:
#   julia --project=. -t1 bench.jl job   > ../rust/bench/data/julia_job.txt
#   QS=idiomatic julia --project=. -t1 bench.jl tpch > ../rust/bench/data/julia_tpch_idiomatic.txt
#   QS=optimized julia --project=. -t1 bench.jl tpch > ../rust/bench/data/julia_tpch_optimized.txt
#
# ENGINE=interp|interp-rr|staged selects the engine (staged default);
# interp-rr is the interpreter with the inference recursion limit relaxed
# (see relax_recursion.jl).

using Printf

suite = get(ARGS, 1, "job")
ENV["PRELA_SKIP_RUNALL"] = "1"

# `run()` builds the plan (`f()` — which eagerly materializes any `bitset`
# indexes), lowers its access mode (`prepare`, now ~free since it's type-stable),
# and drives it to the result. Everything here is real query work — index/cache/
# bitset building included — so it's all timed; only JIT is excluded (cold pass).
# `ENG` is resolved lazily inside the thunks, after Prela is loaded.
ENG() = get(ENV, "ENGINE", "staged") in ("interp", "interp-rr") ? Main.Prela.Interp() :
                                                                  Main.Prela.Staged()
if suite == "job"
    include("JOB.jl")
    include("queries.jl")
    # JOB entries: (name, oracle, f)  — f() returns a Query.
    qs = [(name, () -> Main._vals(f(); eng = ENG())) for (name, _oracle, f) in Main._Q]
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
            () -> Main._vals_tpch(f(), sb, lim, r; eng = ENG())
        end
        push!(qs, (name, run))
    end
else
    error("usage: julia bench.jl {job|tpch}")
end

# interp-rr: patch the recursion limit BEFORE the cold pass compiles anything.
if get(ENV, "ENGINE", "staged") == "interp-rr"
    include("relax_recursion.jl")
    println(stderr, "patched ", relax_recursion_limit!(Main.Prela), " methods")
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
