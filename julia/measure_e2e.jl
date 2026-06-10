# End-to-end (prepare + scan) per query, counting sink. ENGINE=interp|staged
# selects the engine for every scan (builds + final). `minimum` over reps to
# cut GC noise.
using Printf
import Main.Prela

const ENG = get(ENV, "ENGINE", "staged") == "interp" ? Prela.Interp() : Prela.Staged()

mutable struct Ctr; n::Int; end
@noinline function e2e(f, c::Ctr)
    c.n = 0
    Prela.scan(ENG, Prela.prepare(ENG, f()), (x, y) -> (c.n += 1))
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
@printf "ENGINE=%s\n" get(ENV, "ENGINE", "staged")
run_suite(items, parse(Int, get(ENV, "REPS", "8")))
