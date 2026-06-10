# Why the CPS combinators stop inlining at product-chain depth >= 3.
#
# TL;DR: it is Julia's *inference recursion-limiting heuristic*, not the inliner.
# Every product() in the chain instantiates the SAME lambda method, only with
# deeper captured types. When inference sees the same method on its own call
# stack again with a "grown" signature, it widens the signature with
# limit_type_size (stripping closure type parameters -> non-concrete) and
# poisons the surrounding call stack (LimitedAccuracy), which forbids caching
# and inlining. See Compiler/src/abstractinterpretation.jl, abstract_call_method,
# around the `edgecycle` / `limit_type_size` / `poison_callstack!` lines.

include(joinpath(@__DIR__, "deep_compare.jl"))   # make_cps, make_staged, norm

const SIG = Tuple{Vector{Vector{Int}}, Vector{Any}}
dyncalls(ir) = count(l -> occursin("apply_generic", l) || occursin("jl_invoke", l),
                     split(ir, "\n"))
llvm(f) = sprint(io -> code_llvm(io, f, SIG; debuginfo = :none))

# ---------------------------------------------------------------------------
println("== 1. The failure threshold is exactly depth 3 ==")
# scan_id(2) unrolls to 2 iterations, so >= 3 means 2 opaque ijl_invoke calls.
for D in 1:4
    println("  D=$D  opaque calls: ", dyncalls(llvm(make_cps(D))))
end

# ---------------------------------------------------------------------------
println("\n== 2. The widened signatures inference leaves in the method cache ==")
# After inferring run_cps_3, the closure methods have specializations whose
# continuation type lost ALL its parameters (bare UnionAll = non-concrete).
code_typed(make_cps(3), SIG)  # ensure inference ran
let world = Base.get_world_counter()
    for n in Base.unsorted_names(Main; all = true)
        s = string(n)
        (startswith(s, "#") && occursin("run_cps_3#", s)) || continue
        t = getglobal(Main, n)
        (t isa Type && t <: Function) || continue
        ms = Base._methods_by_ftype(Tuple{t, Vararg{Any}}, -1, world)
        ms === nothing && continue
        for mtch in ms, mi in Base.specializations(mtch.method)
            ps = Base.unwrap_unionall(mi.specTypes).parameters
            if !all(p -> isconcretetype(p) || p isa Core.TypeofVararg, ps)
                println("  WIDENED  ", first(string(mi.specTypes), 120))
            end
        end
    end
end

# ---------------------------------------------------------------------------
println("\n== 3. Proof: disable the recursion heuristic per-method, failure gone ==")
# Method.recursion_relation is the official escape hatch: returning true tells
# inference "this recursion is fine, keep going". Patch it on every closure
# method generated for run_cps_16, BEFORE first inference of run_cps_16.
const D = 16
f, stg = make_cps(D), make_staged(D)
let world = Base.get_world_counter(), npatched = 0
    for n in Base.unsorted_names(Main; all = true)
        s = string(n)
        (startswith(s, "#") && occursin("run_cps_$(D)#", s)) || continue
        t = getglobal(Main, n)
        (t isa Type && t <: Function) || continue   # lambdas w/ captures are UnionAll
        ms = Base._methods_by_ftype(Tuple{t, Vararg{Any}}, -1, world)
        ms === nothing && continue
        for mtch in ms
            mtch.method.recursion_relation = (args...) -> true
            npatched += 1
        end
    end
    println("  patched recursion_relation on $npatched closure methods")
end
cols = [[10j + 1, 10j + 2, 10j + 3] for j in 1:D]
f(cols, Any[]); stg(cols, Any[])   # warm up
println("  depth-$D cps: opaque calls = ", dyncalls(llvm(f)),
        ", alloc = ", @allocated(f(cols, Any[])))
println("  depth-$D stg: opaque calls = ", dyncalls(llvm(stg)),
        ", alloc = ", @allocated(stg(cols, Any[])))
