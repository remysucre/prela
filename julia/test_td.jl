# Full JOB oracle suite, on one engine. Runs everything, or _SUBSET if
# non-empty. ENGINE=interp|staged (default staged).
ENV["PRELA_SKIP_RUNALL"] = "1"
include("JOB.jl")
include("queries.jl")

const ENG = get(ENV, "ENGINE", "staged") == "interp" ? Prela.Interp() : Prela.Staged()
const _SUBSET = Set(String[])

function runall_st()
    pass = 0; tot = 0
    fails = String[]
    for (name, oracle, f) in _Q
        (isempty(_SUBSET) || name in _SUBSET) || continue
        tot += 1
        t = time()
        got = try
            _vals(f(); eng = ENG)
        catch e
            "ERROR: " * sprint(showerror, e)
        end
        dt = round(time() - t; digits=1)
        ok = got == oracle
        ok ? (pass += 1) : push!(fails, name)
        println(rpad(name, 6), ok ? "ok   " : "DIFF ", rpad("$(dt)s", 9),
                length(got) > 80 ? got[1:80] * "…" : got)
        ok || println("       expected: ",
                      length(oracle) > 80 ? oracle[1:80] * "…" : oracle)
        flush(stdout)
    end
    println("\n$pass / $tot pass")
    isempty(fails) || println("FAILED: ", join(fails, " "))
end

runall_st()
