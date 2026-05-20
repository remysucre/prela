include("Prela.jl")
using .Prela

# === Schema: declare each entity once. `@entity` emits the abstract type,
# qualified internal storage (`_Movie_title`, etc.), `lookup_field` methods,
# and `primary` (defaulting to the first field).

@entity Keyword begin
    keyword :: String
end

@entity Company begin
    name    :: String
    note    :: String
    country :: String
end

@entity Movie begin
    title           :: String
    production_year :: Int
    keyword         :: ID{Keyword}
    company         :: ID{Company}
end

# Short aliases for Movie's fields (so queries can write bare `title`, etc.).
# Other entities' fields are reached via navigation (e.g. `m.keyword.keyword`).
const title           = Prela.lookup_field(ID{Movie}, Val(:title))
const production_year = Prela.lookup_field(ID{Movie}, Val(:production_year))
const keyword         = Prela.lookup_field(ID{Movie}, Val(:keyword))
const company         = Prela.lookup_field(ID{Movie}, Val(:company))

# === Data ===

M(i) = ID{Movie}(i); K(i) = ID{Keyword}(i); C(i) = ID{Company}(i)

const movie = Unary{ID{Movie}}(M.(1:5))

append!(title.pairs, [
    M(1) => "Shrek 2", M(2) => "Iron Man", M(3) => "Iron Man 2",
    M(4) => "Inception", M(5) => "The Departed",
])
append!(production_year.pairs, [
    M(1) => 2004, M(2) => 2008, M(3) => 2010, M(4) => 2010, M(5) => 2006,
])
append!(keyword.pairs, [
    M(1) => K(10),
    M(2) => K(11), M(2) => K(12),
    M(3) => K(11), M(3) => K(12),
    M(4) => K(13), M(4) => K(14),
    M(5) => K(15),
])
append!(company.pairs, [
    M(1) => C(100), M(2) => C(101), M(3) => C(101),
    M(4) => C(102), M(5) => C(103),
])
append!(Prela.lookup_field(ID{Keyword}, Val(:keyword)).pairs, [
    K(10) => "animation", K(11) => "marvel", K(12) => "action",
    K(13) => "thriller", K(14) => "heist",  K(15) => "crime",
])
append!(Prela.lookup_field(ID{Company}, Val(:name)).pairs, [
    C(100) => "DreamWorks", C(101) => "Marvel Studios",
    C(102) => "Warner Bros", C(103) => "Plan B",
])
append!(Prela.lookup_field(ID{Company}, Val(:country)).pairs, [
    C(100) => "[us]", C(101) => "[us]", C(102) => "[us]", C(103) => "[us]",
])

# === Helpers ===
print_pairs(r::Rel) = (for p in r.pairs; println("  $(p.first) -> $(p.second)"); end)

# === Queries ===

println("\nQ1: movie.((production_year > 2005) : title)")
print_pairs(movie.((production_year > 2005) : title))

println("\nQ2: movie.((keyword == \"marvel\") : title)")
print_pairs(movie.((keyword == "marvel") : title))

println("\nQ3: movie.(((keyword == \"action\") & (production_year >= 2010)) : title, production_year)")
print_pairs(movie.(((keyword == "action") & (production_year >= 2010)) : title, production_year))

println("\nQ4: movie.keyword.keyword")
print_pairs(movie.keyword.keyword)

println("\nQ5: movie.((keyword ~ r\"^a\") : title)")
print_pairs(movie.((keyword ~ r"^a") : title))

println("\nQ6: movie.((company.country == \"[us]\") : title)")
print_pairs(movie.((company.country == "[us]") : title))

println("\nQ7: movie.((keyword in (\"marvel\", \"crime\")) : title)")
print_pairs(movie.((keyword in ("marvel", "crime")) : title))

println("\nQ8: movie.(((production_year > 2009) | (title ~ r\"Shrek\")) : title)")
print_pairs(movie.(((production_year > 2009) | (title ~ r"Shrek")) : title))
