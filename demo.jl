include("Prela.jl")
using .Prela
import .Prela: lookup_field, primary

# === A tiny "JOB-shaped" schema, with String-valued range relations to keep
# the demo focused on the core algebra. Entity / predicate elision is wired
# up by a separate pass (see entity_demo.jl, TODO).

# Movies are represented as Ints. Field relations:
const movie = Unary{Int}([1, 2, 3, 4, 5])

const _title = Rel{Int, String}([
    1 => "Shrek 2", 2 => "Iron Man", 3 => "Iron Man 2",
    4 => "Inception", 5 => "The Departed",
])
const _production_year = Rel{Int, Int}([
    1 => 2004, 2 => 2008, 3 => 2010, 4 => 2010, 5 => 2006,
])
const _keyword = Rel{Int, String}([
    1 => "animation", 2 => "marvel", 2 => "action",
    3 => "marvel", 3 => "action",
    4 => "thriller", 4 => "heist",
    5 => "crime",
])
const _country = Rel{Int, String}([
    1 => "[us]", 2 => "[us]", 3 => "[us]", 4 => "[us]", 5 => "[us]",
])

# Wire up navigation for `m.title`, `m.keyword`, etc.
lookup_field(::Type{Int}, ::Val{:title})           = _title
lookup_field(::Type{Int}, ::Val{:production_year}) = _production_year
lookup_field(::Type{Int}, ::Val{:keyword})         = _keyword
lookup_field(::Type{Int}, ::Val{:country})         = _country

print_pairs(r::Rel) = (for p in r.pairs; println("  $(p.first) -> $(p.second)"); end)
print_unary(u::Unary) = (for v in u.values; println("  $v"); end)

# === Q1: titles of movies made after 2005 ===
println("\nQ1: movie.((production_year > 2005) : title)")
q1 = movie.((_production_year > 2005) : _title)
print_pairs(q1)

# === Q2: titles of movies with the "marvel" keyword ===
println("\nQ2: movie.((keyword == \"marvel\") : title)")
q2 = movie.((_keyword == "marvel") : _title)
print_pairs(q2)

# === Q3: movies with 'action' keyword AND year >= 2010, return (title, year) ===
println("\nQ3: movie.(((keyword == \"action\") & (production_year >= 2010)) : title, production_year)")
q3 = movie.(((_keyword == "action") & (_production_year >= 2010)) : _title, _production_year)
print_pairs(q3)

# === Q4: navigation — movie.keyword (all movie/keyword pairs) ===
println("\nQ4: movie.keyword")
q4 = movie.keyword
print_pairs(q4)

# === Q5: regex match — movies whose title contains "Iron" ===
println("\nQ5: movie.((title ~ r\"Iron\") : title)")
q5 = movie.((_title ~ r"Iron") : _title)
print_pairs(q5)

# === Q6: set difference — movies WITHOUT the "marvel" keyword ===
println("\nQ6: movie - (keyword == \"marvel\")  [movies without marvel keyword]")
q6 = movie - (_keyword == "marvel")
print_unary(q6)

# === Q7: union | — movies with year > 2009 OR title contains "Shrek" ===
println("\nQ7: movie.(((production_year > 2009) | (title ~ r\"Shrek\")) : title)")
q7 = movie.(((_production_year > 2009) | (_title ~ r"Shrek")) : _title)
print_pairs(q7)
