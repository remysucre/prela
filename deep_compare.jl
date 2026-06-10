using InteractiveUtils

# Compare CPS (exp.jl) vs staged (stage.jl) for a DEEP right-nested product:
#   product(probe(c1), product(probe(c2), ... product(probe(c_{D-1}), probe(cD))))
# at increasing depth D. Everything is concrete-typed (cols::Vector{Vector{Int}},
# row index Int) so the pipeline stays type-stable; the only question is whether
# @inline keeps collapsing the closure tree as it gets deeper.

# ---------- staged combinators (stage.jl) ----------
let counter = Ref(0)
    global fresh(prefix = :v) = Symbol(prefix, counter[] += 1)
end
istrivial(e) = e isa Symbol || e isa Number || e isa String || e isa Bool
letbind(e, k) = istrivial(e) ? k(e) : (v = fresh(); :(let $v = $e; $(k(v)); end))
S_compose(lhs, rhs) = k -> lhs(xy -> letbind(:($xy.second), s -> rhs(s, z -> letbind(:($xy.first => $z), k))))
S_product(lhs, rhs) = (x, k) -> lhs(x, y -> rhs(x, z -> letbind(:($y => $z), k)))
S_scan_id(n) = k -> (i = fresh(:i); :(for $i in 1:$n; $(letbind(:($i => $i), k)); end))
S_probe(col) = (x, k) -> letbind(:($col[$x]), k)
emit_collect(xs) = :(push!(res, $xs))

# ---------- generators ----------
# literal nested product expression for the CPS function body, depth D
function cps_build(D)
    e = :(probe(cols[$D]))
    for j in (D-1):-1:1
        e = :(product(probe(cols[$j]), $e))
    end
    e
end

function make_cps(D)
    name = Symbol("run_cps_$D")
    @eval function $name(cols, res)
        @inline compose(lhs, rhs) = k -> lhs(xy -> rhs(xy.second, (z -> k(xy.first => z))))
        @inline product(lhs, rhs) = (x, k) -> lhs(x, y -> rhs(x, z -> k(y => z)))
        @inline scan_id(n) = k -> for i in 1:n; k(i => i); end
        @inline probe(col) = (i, k) -> k(col[i])
        collect = xs -> push!(res, xs)
        exp = compose(scan_id(2), $(cps_build(D)))
        exp(collect)
        res
    end
    @eval $name
end

function make_staged(D)
    cs = [:(cols[$j]) for j in 1:D]
    rel = S_probe(cs[D])
    for j in (D-1):-1:1
        rel = S_product(S_probe(cs[j]), rel)
    end
    body = S_compose(S_scan_id(2), rel)(emit_collect)
    name = Symbol("run_staged_$D")
    @eval function $name(cols, res)
        $body
        res
    end
    @eval $name
end

# ---------- normalization for IR comparison ----------
function norm(s)
    s = replace(s, r"run_(cps|staged)_[0-9]+" => "F")  # function name
    s = replace(s, r"_[0-9]{3,}" => "_N")              # throwaway symbol IDs
    s
end

ircount(f, sig) = count(==('\n'), sprint(io -> code_llvm(io, f, sig; debuginfo = :none)))

sig = Tuple{Vector{Vector{Int}}, Vector{Any}}

println("depth |  result-match | IR-match | cps-IR-lines | staged-IR-lines")
println("------+---------------+----------+--------------+----------------")
for D in (2, 4, 8, 12, 16, 24, 32)
    cps = make_cps(D); stg = make_staged(D)
    cols = [[10j + 1, 10j + 2, 10j + 3] for j in 1:D]
    r1 = cps(cols, []); r2 = stg(cols, [])
    same_result = r1 == r2
    ir_cps = sprint(io -> code_llvm(io, cps, sig; debuginfo = :none))
    ir_stg = sprint(io -> code_llvm(io, stg, sig; debuginfo = :none))
    same_ir = norm(ir_cps) == norm(ir_stg)
    nc = count(==('\n'), ir_cps); ns = count(==('\n'), ir_stg)
    println(lpad(D, 5), " | ", lpad(same_result, 13), " | ", lpad(same_ir, 8),
            " | ", lpad(nc, 12), " | ", lpad(ns, 15))
end
