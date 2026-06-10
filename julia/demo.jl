# Tiny self-contained demo — no datasets needed.
#
#   julia --project=. demo.jl
#
# Shows the full pipeline: @entity schema → staged leaves → seal → queries
# through `→ : ∧ ∨ ⊗ ' ▷` → results via both engines.

include("Prela.jl")
using .Prela

# === Schema with polymorphic names: `info` and `name` appear on multiple entities.

@entity InfoType begin
    info :: String   # primary (the info type's name, e.g. "countries")
end

@entity Keyword begin
    keyword :: String
end

@entity Company begin
    name    :: String
    country :: String
end

@entity Info begin
    info :: String              # primary — the info text
    type :: ID{InfoType}
end

@entity Movie begin
    title           :: String
    production_year :: Int
    keyword         :: Multi{ID{Keyword}}
    company         :: ID{Company}
    info            :: Multi{ID{Info}}
end

M(i) = ID{Movie}(i); K(i) = ID{Keyword}(i); C(i) = ID{Company}(i)
I(i) = ID{Info}(i);  IT(i) = ID{InfoType}(i)

append!(Movie.title.pairs, [
    M(1) => "Shrek 2", M(2) => "Iron Man", M(3) => "Iron Man 2",
    M(4) => "Inception", M(5) => "The Departed",
])
append!(Movie.production_year.pairs, [
    M(1) => 2004, M(2) => 2008, M(3) => 2010, M(4) => 2010, M(5) => 2006,
])
append!(Movie.keyword.pairs, [
    M(1) => K(1),
    M(2) => K(2), M(2) => K(3),
    M(3) => K(2), M(3) => K(3),
    M(4) => K(4), M(5) => K(5),
])
append!(Movie.company.pairs, [
    M(1) => C(1), M(2) => C(2), M(3) => C(2), M(4) => C(3), M(5) => C(4),
])
append!(Movie.info.pairs, [
    M(1) => I(1), M(2) => I(2), M(3) => I(2), M(4) => I(3), M(5) => I(2),
])
append!(Keyword.keyword.pairs, [
    K(1) => "animation", K(2) => "marvel", K(3) => "action",
    K(4) => "heist", K(5) => "crime",
])
append!(Company.name.pairs, [
    C(1) => "DreamWorks", C(2) => "Marvel Studios", C(3) => "Warner Bros", C(4) => "Plan B",
])
append!(Company.country.pairs, [
    C(1) => "[us]", C(2) => "[us]", C(3) => "[us]", C(4) => "[us]",
])
append!(Info.info.pairs, [I(1) => "USA", I(2) => "USA", I(3) => "UK"])
append!(Info.type.pairs, [I(1) => IT(1), I(2) => IT(1), I(3) => IT(1)])
append!(InfoType.info.pairs, [IT(1) => "countries"])

seal_entities!()
@expose Movie : title, production_year

const movie = UnaryVec{ID{Movie}}(M.(1:5))

show_pairs(q) = (r = collect(q); for p in r.pairs; println("  ", p.first, " -> ", p.second); end)

println("Q1: titles of movies after 2005")
show_pairs(movie : (production_year > 2005) → title)

println("\nQ2: marvel movies (navigate keyword, filter, back to title)")
show_pairs(movie : (Movie.keyword → Keyword.keyword == "marvel") → title)

println("\nQ3: title ⊗ year pairs for USA movies")
show_pairs(movie : (Movie.info → Info.info == "USA") → title ⊗ production_year)

println("\nQ4: movies per company (inverse navigation + fold)")
show_pairs((Movie.company ← title) ▷ ((a, _) -> a + 1, 0))

println("\nQ5: same query, interpreted engine — identical result")
let q = (Movie.company ← title) ▷ ((a, _) -> a + 1, 0)
    r = collect(q, Interp())
    for p in r.pairs; println("  ", p.first, " -> ", p.second); end
end
