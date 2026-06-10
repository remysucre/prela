# @time a few cast queries (post-warmup) — allocation count is the tell.
# Standalone: julia --project=. diag.jl  (loads JOB itself).
ENV["PRELA_SKIP_RUNALL"] = "1"
include("JOB.jl")
include("queries.jl")

function diag(name)
    for (n, _, f) in _Q
        n == name || continue
        _vals(f())          # warmup
        GC.gc()
        print(rpad(name, 6), " ")
        @time _vals(f())
        flush(stdout)
        return
    end
end

println("\n=== @time (post-warmup) ===")
for nm in ("6a", "30a", "8a", "25b", "9a", "17e", "15a")
    diag(nm)
end
