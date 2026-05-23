# Persistent REPL workflow:
#
#   julia --project=. -i -e 'include("start.jl")'
#
# Loads Revise (so edits to Prela.jl / queries.jl pick up automatically) and
# JOB data (~30s one-time). Drop into REPL afterwards; re-run queries cheaply
# by including queries.jl or typing them directly.

using Revise

# Load JOB data (one-time, ~30s). Prela is included from inside JOB.jl.
include("JOB.jl")

# Track Prela.jl now that the module exists. Edits will be picked up
# automatically on the next REPL evaluation.
Revise.track(Prela, joinpath(@__DIR__, "Prela.jl"))

println()
println("Ready. To run queries, either:")
println("  julia> include(\"queries.jl\")     # run all queries once")
println("  julia> includet(\"queries.jl\")    # run + auto-rerun on edit")
println("  julia> # or type a query directly, using `movie`, `title`, etc.")
println()
