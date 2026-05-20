println("\n=== Bisecting 22a's company branch ===")

# Just `Company.type == "production companies"`
let t = time(); r = company ∘ (Company.type == "production companies")
    println("  company ∘ type='production' -> $(length(r.pairs)) pairs, $(length(unique(p.first for p in r.pairs))) movies ($(round(time()-t; digits=2))s)")
end

# Add country
let t = time(); r = company ∘ ((Company.type == "production companies") & (Company.country != "[us]"))
    println("  + country != '[us]' -> $(length(r.pairs)) pairs, $(length(unique(p.first for p in r.pairs))) movies ($(round(time()-t; digits=2))s)")
end

# Add note ~ \(200.*\)
let t = time(); r = company ∘ ((Company.type == "production companies") & (Company.country != "[us]") & (Company.note ~ r"\(200.*\)"))
    println("  + note ~ \\(200.*\\) -> $(length(r.pairs)) pairs, $(length(unique(p.first for p in r.pairs))) movies ($(round(time()-t; digits=2))s)")
end

# Add note ≁ \(USA\)
let t = time(); r = company ∘ ((Company.type == "production companies") & (Company.country != "[us]") & (Company.note ~ r"\(200.*\)") & (Company.note ≁ r"\(USA\)"))
    println("  + note ≁ \\(USA\\) -> $(length(r.pairs)) pairs, $(length(unique(p.first for p in r.pairs))) movies ($(round(time()-t; digits=2))s)")
end

# Now check intersection with the movie LHS at each step
println()
println("=== Movie LHS ∩ company filter ===")
movie_set = (info ∘ ((Info.type == "countries") & (Info.info in ("Germany", "German", "USA", "American")))) &
            (keyword in ("murder", "murder-in-title", "blood", "violence")) &
            (production_year > 2008) &
            (kind in ("movie", "episode"))
movie_keys = Set(movie_set.values)
println("  movie LHS: $(length(movie_keys))")
for (label, cf) in [
        ("type='production'", company ∘ (Company.type == "production companies")),
        ("+country!=us", company ∘ ((Company.type == "production companies") & (Company.country != "[us]"))),
        ("+note~200", company ∘ ((Company.type == "production companies") & (Company.country != "[us]") & (Company.note ~ r"\(200.*\)"))),
        ("+note≁USA", company ∘ ((Company.type == "production companies") & (Company.country != "[us]") & (Company.note ~ r"\(200.*\)") & (Company.note ≁ r"\(USA\)"))),
    ]
    movies_in_cf = Set(p.first for p in cf.pairs)
    println("  $label: $(length(movies_in_cf)) cf movies, ∩ LHS = $(length(intersect(movie_keys, movies_in_cf)))")
end

# Also look at the SQL version: each movie can have MANY mc rows. The filter
# is on the mc row. As long as ONE mc row passes for a movie, the movie joins.
# Our company filter returns Companies (=mc rows), and then `company ∘ Unary{Company}`
# returns Movies via the company. So movies are those that have at least one
# such mc row. That matches SQL.
