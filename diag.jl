# CPS diagnostic — @time a few cast queries (after warmup) to see per-row
# allocation. High alloc count ⇒ boxing / non-inlined closures on the hot path.
ENV["PRELA_SKIP_RUNALL"] = "1"
include("JOB.jl")
include("queries.jl")

function diag(name)
    for (n, _, f) in _Q
        n == name || continue
        _vals(f())          # warmup: compile + materialize
        GC.gc()
        print(rpad(name, 6), " ")
        @time _vals(f())
        flush(stdout)
        return
    end
end

println("\n=== cast-query @time (post-warmup) ===")
for nm in ("6a", "30a", "8a", "25b", "9a", "17e")
    diag(nm)
end
