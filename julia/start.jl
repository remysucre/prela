# Persistent REPL workflow:
#
#   julia --project=. -i -e 'include("start.jl")'
#
# Loads Revise (so edits to Prela.jl pick up automatically). Pick a dataset
# yourself afterwards — JOB or TPC-H — then run queries.

using Revise

# Load Prela module so Revise can track it without forcing a dataset load.
include("Prela.jl")
Revise.track(Prela, joinpath(@__DIR__, "Prela.jl"))

println()
println("Engines: every runner takes eng = Prela.Staged() (default) or")
println("Prela.Interp() — e.g. _vals(q; eng = Prela.Interp()), collect(q, Interp()).")
println()
println("Ready. Pick a dataset:")
println("  julia> include(\"JOB.jl\")            # load JOB tables (~30s one-time)")
println("  julia> include(\"queries.jl\")        # then run all JOB queries")
println("  julia> includet(\"queries.jl\")       # or run + auto-rerun on edit")
println()
println("  julia> include(\"TPCH.jl\")           # load TPC-H tables (one-time)")
println("  julia> include(\"tpch_queries_idiomatic.jl\")   # then run all TPC-H queries")
println("  julia> includet(\"tpch_queries_idiomatic.jl\")  # or run + auto-rerun on edit")
println()
