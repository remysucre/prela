include("Prela.jl")
using .Prela

# === Schema with polymorphic names: `info` and `type` appear on several
# entities. Prela tells them apart by type, so the same name can name different
# relations. Multi-valued fields (a movie has many keywords / info rows) are
# declared `Multi{...}`.

# Declare all entity types up front (settles the type bindings before use), then
# give each its fields.
@declare InfoType Keyword Company Info Movie

@entity InfoType begin
    info :: String              # primary — the info type's name, e.g. "countries"
end

@entity Keyword begin
    keyword :: String
end

@entity Company begin
    name    :: String
    note    :: String
    country :: String
end

@entity Info begin
    info :: String              # primary — the info text
    type :: ID{InfoType}
    note :: String
end

@entity Movie begin
    title           :: String
    production_year :: Int
    keyword         :: Multi{ID{Keyword}}
    company         :: ID{Company}
    info            :: Multi{ID{Info}}
end

# === Data — appended to the staging leaves (before sealing). `Entity.field`
# resolves to the staging relation, whose `.pairs` we push into. ===

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
    M(1) => K(10),
    M(2) => K(11), M(2) => K(12),
    M(3) => K(11), M(3) => K(12),
    M(4) => K(13), M(4) => K(14),
    M(5) => K(15),
])
append!(Movie.company.pairs, [
    M(1) => C(100), M(2) => C(101), M(3) => C(101),
    M(4) => C(102), M(5) => C(103),
])
# Each movie has multiple Info rows (countries + release dates).
append!(Movie.info.pairs, [
    M(1) => I(201), M(1) => I(202),
    M(2) => I(203), M(2) => I(204),
    M(3) => I(205), M(3) => I(206),
    M(4) => I(207),
    M(5) => I(208),
])
append!(Info.info.pairs, [
    I(201) => "USA",      I(202) => "USA:2004-04-23",
    I(203) => "USA",      I(204) => "USA:2008-04-30",
    I(205) => "USA",      I(206) => "USA:2010-05-07",
    I(207) => "USA",      I(208) => "USA",
])
append!(Info.type.pairs, [
    I(201) => IT(1), I(202) => IT(2),
    I(203) => IT(1), I(204) => IT(2),
    I(205) => IT(1), I(206) => IT(2),
    I(207) => IT(1),
    I(208) => IT(1),
])
append!(InfoType.info.pairs, [
    IT(1) => "countries",
    IT(2) => "release dates",
])
append!(Keyword.keyword.pairs, [
    K(10) => "animation", K(11) => "marvel", K(12) => "action",
    K(13) => "thriller", K(14) => "heist",  K(15) => "crime",
])
append!(Company.name.pairs, [
    C(100) => "DreamWorks", C(101) => "Marvel Studios",
    C(102) => "Warner Bros", C(103) => "Plan B",
])
append!(Company.country.pairs, [
    C(100) => "[us]", C(101) => "[us]", C(102) => "[us]", C(103) => "[us]",
])

# === Seal the staging leaves into static storage, then bind a few bare aliases
# for the unique (non-polymorphic) Movie fields. `movie` is the universe of all
# five movies. ===

Prela.seal_entities!()
const movie           = UnaryVec{ID{Movie}}(M.(1:5))
const title           = Movie.title
const production_year = Movie.production_year

# Drive a query to completion and print its (key → value) pairs.
print_pairs(q) = for p in collect(q).pairs
    println("  $(p.first) -> $(p.second)")
end

# === Queries demonstrating polymorphism ===

println("\nQ1: movies with an Info row of type 'countries' and info text 'USA'")
println("    movie : (Movie.info → ((Info.type == \"countries\") ∧ (Info.info == \"USA\"))) → title")
q1 = (movie : (Movie.info → ((Info.type == "countries") ∧ (Info.info == "USA")))) → title
print_pairs(q1)

println("\nQ2: movies with at least one Info row of type 'release dates' (title only)")
println("    movie : (Movie.info → (Info.type == \"release dates\")) → title")
q2 = (movie : (Movie.info → (Info.type == "release dates"))) → title
print_pairs(q2)

println("\nQ3: bare alias still works for a unique field name")
println("    movie : (production_year > 2005) → title")
q3 = (movie : (production_year > 2005)) → title
print_pairs(q3)

println("\nQ4: Movie.info vs Info.info — same name, different relations")
println("    Movie.info: ", typeof(Movie.info))
println("    Info.info:  ", typeof(Info.info))

println("\nQ5: compose two polymorphic hops — Movie.info → Info.type, matched by name")
println("    movie : ((Movie.info → Info.type) ~ r\"countries\") → title")
q5 = (movie : ((Movie.info → Info.type) ~ r"countries")) → title
print_pairs(q5)
