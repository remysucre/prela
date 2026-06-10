# Relax Julia's inference recursion limit on every method that participates
# in the interpreted engine's CPS tower: the protocol methods (drive/probe/
# probe_any/member, and the generated _prod_*/_idx_probe helpers) plus every
# closure defined in the module (the continuation lambdas inside those
# methods). `Method.recursion_relation` returning true tells inference the
# recursion is fine, so deep plans keep inlining instead of widening.
#
# Must run BEFORE any query compiles (inference results are cached; the patch
# has no effect on already-inferred methods). Used by `bench.jl`'s
# ENGINE=interp-rr mode and `experiments/experiment_recursion.jl`.
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
