# bench.jl's JOB warm pass, interp engine, with recursion_relation relaxed
# on all Prela methods BEFORE any query compiles.
ENV["PRELA_SKIP_RUNALL"] = "1"
using Printf
include("JOB.jl")
include("queries.jl")

function relax_recursion_limit!(mod::Module)
    world = Base.get_world_counter()
    n = 0
    relax(m::Method) = (m.recursion_relation = (a...) -> true; n += 1)
    for nm in Base.unsorted_names(mod; all = true)
        s = String(nm)
        t = try getglobal(mod, nm) catch; continue end
        if t isa Type && t <: Function && startswith(s, "#")
            ms = Base._methods_by_ftype(Tuple{t, Vararg{Any}}, -1, world)
            ms === nothing && continue
            foreach(mtch -> relax(mtch.method), ms)
        elseif nm in (:drive, :probe, :probe_any, :member) ||
               startswith(s, "_prod_") || startswith(s, "_idx_probe")
            t isa Function || continue
            foreach(relax, methods(t))
        end
    end
    n
end
println(stderr, "patched ", relax_recursion_limit!(Prela), " methods")

const ENG = Prela.Interp()
qs = [(name, () -> Main._vals(f(); eng = ENG)) for (name, _oracle, f) in Main._Q]
for (_, run) in qs
    try run() catch end
end
GC.gc(true)
for (name, run) in qs
    t = time_ns()
    _ = try run() catch e e end
    @printf "%s\t%.6f\n" name ((time_ns() - t) / 1e9)
end
