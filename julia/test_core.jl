# Operator-coverage tests on tiny inline data — no external datasets.
#
#   julia --project=. test_core.jl
#
# Every Query node type is exercised through `collect`-style terminals on BOTH
# engines (interpreted CPS drive and the @generated staged scan), with results
# checked against hand-computed expectations. This is the safety net for
# engine/prepare refactors: green here means the algebra's semantics survived.

include("Prela.jl")
using .Prela
using .Prela: prepare, scan, Interp, Staged, ID

# ===== tiny schema ======================================================

@entity Person begin
    name :: String
end

@entity Film begin
    title    :: String
    year     :: Int
    director :: ID{Person}
    cast     :: Multi{ID{Person}}
end

# Award: ids {1, 3} with universe stretched to 3 by `winner` — makes
# `Award.name` seal as SparseRel (gap at 2).
@entity Award begin
    name   :: String
    winner :: ID{Film}
end

P(i) = ID{Person}(i); F(i) = ID{Film}(i); A(i) = ID{Award}(i)

append!(Person.name.pairs, [P(1) => "kurosawa", P(2) => "mifune", P(3) => "kyo"])
append!(Film.title.pairs,  [F(1) => "rashomon", F(2) => "ikiru", F(3) => "ran"])
append!(Film.year.pairs,   [F(1) => 1950, F(2) => 1952, F(3) => 1985])
append!(Film.director.pairs, [F(1) => P(1), F(2) => P(1), F(3) => P(1)])
append!(Film.cast.pairs,   [F(1) => P(2), F(1) => P(3), F(3) => P(2)])
append!(Award.name.pairs,  [A(1) => "lion", A(3) => "palme"])
append!(Award.winner.pairs, [A(1) => F(1), A(2) => F(2), A(3) => F(2)])

seal_entities!()
@expose Film
@expose Person : name
@expose Award : winner

const film   = UnaryVec{ID{Film}}(F.(1:3))
const people = Universe{Person}(3)

# sanity: the seal produced the layouts the tests assume
@assert title isa Prela.VecRel
@assert cast isa Prela.MultiRel
@assert Award.name isa Prela.SparseRel

# ===== harness ==========================================================

failures = Ref(0)

# Collect (x, y) pairs from a query through one engine — prepare AND scan
# both go through that engine.
function pairs_via(q, eng)
    out = Tuple{Any, Any}[]
    scan(eng, prepare(eng, q), (x, y) -> push!(out, (x, y)))
    sort!(out; by = repr)
end

function expect(label, q, want)
    wanted = sort!(Tuple{Any, Any}[(x, y) for (x, y) in want]; by = repr)
    for eng in (Interp(), Staged())
        got = pairs_via(q, eng)
        if got != wanted
            failures[] += 1
            println("FAIL [$(typeof(eng))] $label")
            println("  want: ", wanted)
            println("  got:  ", got)
        end
    end
end

function expect_scalar(label, q, want)
    for eng in (Interp(), Staged())
        got = Prela.unwrap(q, eng)
        if got != want
            failures[] += 1
            println("FAIL [unwrap/$(typeof(eng))] $label: want $want, got $got")
        end
    end
end

# ===== leaves ===========================================================

expect("VecRel drive", title,
       [F(1) => "rashomon", F(2) => "ikiru", F(3) => "ran"])

expect("SparseRel drive (gap at 2)", Award.name,
       [A(1) => "lion", A(3) => "palme"])

expect("MultiRel drive", cast,
       [F(1) => P(2), F(1) => P(3), F(3) => P(2)])

expect("UnaryVec drive (identity pairs)", film,
       [F(1) => F(1), F(2) => F(2), F(3) => F(3)])

expect("Universe drive", people,
       [P(1) => P(1), P(2) => P(2), P(3) => P(3)])

# ===== compose / navigate ==============================================

expect("Compose probe (film → title)", film → title,
       [F(1) => "rashomon", F(2) => "ikiru", F(3) => "ran"])

expect("Compose chain (film → director → name)", film → director → Person.name,
       [F(1) => "kurosawa", F(2) => "kurosawa", F(3) => "kurosawa"])

expect("Compose through MultiRel (film → cast → name)", film → cast → Person.name,
       [F(1) => "mifune", F(1) => "kyo", F(3) => "mifune"])

expect("Compose probe misses (winner → cast)", winner → cast,
       [A(1) => P(2), A(1) => P(3)])   # films 2 has no cast rows

# ===== filters ==========================================================

expect("Filter callable (year > 1951)", year > 1951,
       [F(2) => 1952, F(3) => 1985])

expect("Filter EqP (title == ran)", title == "ran",
       [F(3) => "ran"])

expect("Filter InP (year in tuple)", year in (1950, 1985),
       [F(1) => 1950, F(3) => 1985])

expect("Filter regex (~)", title ~ r"^ra",
       [F(1) => "rashomon", F(3) => "ran"])

expect("Filter interval (in 1950..1952)", year in (1950 .. 1952),
       [F(1) => 1950, F(2) => 1952])

expect("entity-elision predicate (director == name)", director == "kurosawa",
       [F(1) => "kurosawa", F(2) => "kurosawa", F(3) => "kurosawa"])

# ===== restrict / diff / product / disj ================================

expect("Restrict (film : (year > 1951) → title)", film : (year > 1951) → title,
       [F(2) => "ikiru", F(3) => "ran"])

expect("Diff (film - (year > 1951))", film - (year > 1951),
       [F(1) => F(1)])

expect("Prod ⊗ (title ⊗ year)", title ⊗ year,
       [F(1) => ("rashomon", 1950), F(2) => ("ikiru", 1952), F(3) => ("ran", 1985)])

expect("Prod via ∧ in Restrict", film : (year > 1949) ∧ (title ~ r"n") → title,
       [F(1) => "rashomon", F(3) => "ran"])

expect("Disj ∨ membership", film : ((title == "ikiru") ∨ (year > 1984)) → title,
       [F(2) => "ikiru", F(3) => "ran"])

# cross-column compare (Filter ∘ Prod with tuple-destructuring predicate)
expect("cross-column predicate (year < year+: trivially empty)", year < year,
       Tuple{Any, Any}[])

# ===== Inv (both modes) =================================================

# driven: streaming flip
expect("Inv driven (title')", (title)',
       ["rashomon" => F(1), "ikiru" => F(2), "ran" => F(3)])

# probed: eager index — compose onto it so it sits in Probed position
expect("Inv probed (winner → winner' )", winner → (winner)',
       [A(1) => A(1), A(2) => A(2), A(2) => A(3), A(3) => A(2), A(3) => A(3)])

# ===== LeftCompose (both modes) =========================================

# driven: drive s, probe r — group year by director-name
expect("LeftCompose driven (director ← year)", director ← year,
       [P(1) => 1950, P(1) => 1952, P(1) => 1985])

# probed: concrete index — put the ← under a Compose rhs
expect("LeftCompose probed (people → (director ← title))",
       people → (director ← title),
       [P(1) => "rashomon", P(1) => "ikiru", P(1) => "ran"])

# ===== ⩘ (restrict by materialized value-set) ==========================

expect("⩘ (cast ⩘ people: people appearing in some cast)",
       cast ⩘ people,
       [P(2) => P(2), P(3) => P(3)])

# ===== materialize (both modes), MatSet, Bitset ========================

expect("Materialized driven (!q scanned)", !(film : (year > 1951) → title),
       [F(2) => "ikiru", F(3) => "ran"])

expect("Materialized probed (film → !title)", film → !title,
       [F(1) => "rashomon", F(2) => "ikiru", F(3) => "ran"])

expect("MatSet driven", !(film : (year > 1951)),
       [F(2) => F(2), F(3) => F(3)])

expect("MatSet probed (restrict by materialized set)", film : !(film : (year > 1951)),
       [F(2) => F(2), F(3) => F(3)])

expect("Bitset membership", film : bitset(film : (year > 1951), 3),
       [F(2) => F(2), F(3) => F(3)])

# ===== folds / map / scalar ============================================

expect("Fold ▷ sum of years by director", (director ← year) ▷ (+, 0),
       [P(1) => 1950 + 1952 + 1985])

expect("Fold ▷ count", (director ← year) ▷ ((a, _) -> a + 1, 0),
       [P(1) => 3])

expect("DenseFold ▷ (op, init, n)", (director ← year) ▷ (+, 0, 3),
       [P(1) => 1950 + 1952 + 1985])

expect("BufFold ▷ callable", (director ← year) ▷ length,
       [P(1) => 3])

expect("Map ↦ (year + 1)", (film → year) ↦ (v -> v + 1),
       [F(1) => 1951, F(2) => 1953, F(3) => 1986])

expect_scalar("Scalar ⊵ total years", year ⊵ (+, 0), 1950 + 1952 + 1985)

expect("Fold probed (film → year-sum-by-director via director → fold)",
       film → director → ((director ← year) ▷ (+, 0)),
       [F(1) => 5887, F(2) => 5887, F(3) => 5887])

# ScalarP probe via ↦ on Scalar
expect("Scalar ↦ chain", (year ⊵ (+, 0)) ↦ (v -> v ÷ 3),
       [nothing => 1962])

# ===== MapRel terminal reuse ===========================================

let collected = collect(film : (year > 1951) → title)
    expect("MapRel re-drive of collected result", collected,
           [F(2) => "ikiru", F(3) => "ran"])
end

# ===== report ===========================================================

if failures[] == 0
    println("test_core: all passed")
else
    println("test_core: $(failures[]) FAILURES")
    exit(1)
end
