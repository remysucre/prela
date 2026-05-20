# Load JOB once, run several queries. After the first include, subsequent
# `julia --project=. queries.jl` invocations re-pay the loading cost; for
# interactive use, include this from the REPL once and re-run queries cheaply.

include("JOB.jl")

println()

# === 2a ===
println("=== 2a: 'character-name-in-title' + German company ===")
let t = time()
    q = movie.(((keyword == "character-name-in-title") & (company.country == "[de]")) : title)
    titles = sort!(unique(p.second for p in q.pairs))
    println("  $(length(unique(p.first for p in q.pairs))) movies, $(length(titles)) distinct titles ($(round(time()-t; digits=2))s)")
    println("  MIN(title) = $(repr(first(titles))) ")
    flush(stdout)
end

# === 2d (same shape, different country) ===
println("\n=== 2d: 'character-name-in-title' + US company ===")
let t = time()
    q = movie.(((keyword == "character-name-in-title") & (company.country == "[us]")) : title)
    titles = sort!(unique(p.second for p in q.pairs))
    println("  $(length(unique(p.first for p in q.pairs))) movies, $(length(titles)) distinct titles ($(round(time()-t; digits=2))s)")
    println("  MIN(title) = $(repr(first(titles))) ")
    flush(stdout)
end

# === 3b: keyword like '%sequel%' + Bulgarian movie_info + year > 2010 ===
# Note the `info ∘ (Info.info == "Bulgaria")` form: predicate pushdown.
# The slow form `info.info == "Bulgaria"` would materialize all 14.8M movie-info
# pairs first; the `∘` form filters Info.info to "Bulgaria" first (a few rows),
# THEN composes movie.info with the small result.
println("\n=== 3b: '%sequel%' keyword + Bulgarian info + year > 2010 ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (info ∘ (Info.info == "Bulgaria"))
           & (production_year > 2010))
          : title)
    titles = sort!(unique(p.second for p in q.pairs))
    println("  $(length(unique(p.first for p in q.pairs))) movies, $(length(titles)) distinct titles ($(round(time()-t; digits=2))s)")
    flush(stdout)
    isempty(titles) || println("  MIN(title) = $(repr(first(titles)))")
end

# === Sanity: movies with at least one info of type 'countries' ===
println("\n=== Sanity (instrumented) ===")
let
    t = time(); a = Info.type == "countries"
    println("  step 1: Info.type == \"countries\" -> $(length(a.pairs)) pairs ($(round(time()-t; digits=2))s)"); flush(stdout)
    t = time(); b = info ∘ a
    println("  step 2: info ∘ a -> $(length(b.pairs)) pairs ($(round(time()-t; digits=2))s)"); flush(stdout)
    t = time(); q = movie & b
    println("  step 3: movie & b -> $(length(q.values)) values ($(round(time()-t; digits=2))s)"); flush(stdout)
end
