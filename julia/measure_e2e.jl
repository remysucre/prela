# End-to-end (prepare+drive via the engine) per query, counting sink. Run with
# CG_BUILDS=0 (interpreted builds) vs CG_BUILDS=1 (codegen builds) to price the
# build-codegen lever. `minimum` over reps to cut GC noise.
using Printf
import Main.Prela

mutable struct Ctr; n::Int; end
@noinline function e2e(f, c::Ctr)
    c.n = 0
    Main.execute_drive(f(), (x, y) -> (c.n += 1))
    c.n
end

function run_suite(items, reps)
    c = Ctr(0)
    tot = 0.0
    for (name, f) in items
        local cnt
        try
            cnt = e2e(f, c)
        catch
            @printf "%-5s        ERR\n" name; continue
        end
        ts = Float64[]
        for _ in 1:reps; push!(ts, @elapsed e2e(f, c)); end
        m = minimum(ts) * 1e3
        tot += m
        @printf "%-5s %9.3f ms  rows=%d\n" name m cnt
    end
    @printf "TOTAL %9.3f ms\n" tot
end

items = isdefined(Main, :_QT) ?
    [(string(n), f) for (n, _o, f, _sb, _l, _r) in Main._QT] :
    [(string(n), f) for (n, _o, f) in Main._Q]
@printf "CG_BUILDS=%s\n" get(ENV, "CG_BUILDS", "1")
run_suite(items, parse(Int, get(ENV, "REPS", "8")))
