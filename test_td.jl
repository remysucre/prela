# Top-down engine test — runs all 113 JOB queries single-threaded.
ENV["PRELA_SKIP_RUNALL"] = "1"
include("JOB.jl")
include("queries.jl")

function runall_st()
    pass = 0
    fails = String[]
    for (name, oracle, f) in _Q
        t = time()
        got = try
            _vals(f())
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
    end
    println("\n$pass / $(length(_Q)) queries pass")
    isempty(fails) || println("FAILED: ", join(fails, " "))
end

runall_st()
