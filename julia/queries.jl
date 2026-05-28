# JOB benchmark — all 113 queries (templates 1-33) in one file.
#
# Idiomatic Prela: movie-rooted queries start from `movie`, cast-rooted ones
# from `cast`; filters are fused onto the column they constrain so every
# intermediate stays small. `:` restricts a projection to a predicate's
# domain, `×` joins output columns on the shared key, `∧`/`∨` combine
# predicates, `→` navigates / composes.
#
# Run after include("JOB.jl"), then call runall(). Queries evaluate in
# parallel when Julia is started with threads (e.g. `julia -t8 ...`); each
# result's MIN tuple is checked against its recorded reference.
@assert isdefined(Main, :movie) "Run include(\"JOB.jl\") first."

# A query registers a (name, reference, thunk); runall() evaluates them.
const _Q = Tuple{String,String,Function}[]
function _q(f, name, oracle)
    entry = (name, oracle, f)
    idx = findfirst(t -> t[1] == name, _Q)
    idx === nothing ? push!(_Q, entry) : (_Q[idx] = entry)
    nothing
end

# Recursively flatten nested tuples at compile time — left-associative Prod
# yields `((((a, b), c), d), ...)`; we want a flat NTuple so the emit hot loop
# can iterate it without allocating a Vector{Any} per row.
@generated function _flatten(y)
    if y <: Tuple
        exprs = Any[]
        for i in 1:fieldcount(y)
            ti = fieldtype(y, i)
            if ti <: Tuple
                push!(exprs, :(_flatten(y[$i])...))
            else
                push!(exprs, :(y[$i]))
            end
        end
        Expr(:tuple, exprs...)
    else
        :((y,))
    end
end

# Drive the query node, folding a running minimum per output column — the
# terminal continuation. Never materializes the result. `cur` is a mutable
# typed Vector{Any} sized once at first emit; later emits compare in place
# without allocating.
function _vals(q)
    cur = Ref{Any}(nothing)
    emit = y -> begin
        fl = _flatten(y)
        c = cur[]
        if c === nothing
            cur[] = collect(Any, fl)
        else
            cc = c::Vector{Any}
            @inbounds for i in eachindex(cc)
                fi = fl[i]
                isless(fi, cc[i]) && (cc[i] = fi)
            end
        end
    end
    if Prela._rangeof(q) === Tuple{}
        Prela.drive(q, (x, _) -> emit(x))
    else
        Prela.drive(q, (_, y) -> emit(y))
    end
    c = cur[]
    c === nothing ? "(empty)" : join(string.(c::Vector{Any}), " || ")
end

const _PRINT_LOCK = ReentrantLock()

# Evaluate every registered query (parallel across threads when Julia is
# started with -t N), printing each result with its reference as it finishes,
# then a final match count.
function runall()
    res = Vector{Any}(undef, length(_Q))
    done = Threads.Atomic{Int}(0)
    log_file = open("/tmp/prela_progress.log", "w")
    Threads.@threads for i in eachindex(_Q)
        name, oracle, f = _Q[i]
        t = time()
        got = try
            _vals(f())
        catch e
            "ERROR: " * sprint(showerror, e)
        end
        dt = round(time() - t; digits=1)
        res[i] = (name, got, oracle, dt)
        k = Threads.atomic_add!(done, 1) + 1
        ok = got == oracle
        msg = rpad("[$k/$(length(_Q))]", 9) * rpad(name, 6) *
              (ok ? "ok   " : "DIFF ") * rpad("$(dt)s", 9) * got
        ok || (msg *= "\n       expected: " * oracle)
        lock(_PRINT_LOCK) do
            println(msg)
            flush(stdout)
            println(log_file, msg)
            flush(log_file)
        end
    end
    pass = count(t -> t[2] == t[3], res)
    summary = "\n$pass / $(length(_Q)) queries match reference"
    println(summary)
    flush(stdout)
    println(log_file, summary)
    flush(log_file)
    close(log_file)
end

const _KW8 = ("superhero", "sequel", "second-part", "marvel-comics",
              "based-on-comic", "tv-special", "fight", "violence")
const _KW10 = ("superhero", "marvel-comics", "based-on-comic", "tv-special",
               "fight", "violence", "magnet", "web", "claw", "laser")
const _KW7 = ("murder", "violence", "blood", "gore", "death",
              "female-nudity", "hospital")
const _VOICE4 = ("(voice)", "(voice: Japanese version)",
                 "(voice) (uncredited)", "(voice: English version)")
const _VOICE3 = ("(voice)", "(voice) (uncredited)", "(voice: English version)")
const _WRITER5 = ("(writer)", "(head writer)", "(written by)",
                  "(story)", "(story editor)")
const _GENRE6 = ("Horror", "Action", "Sci-Fi", "Thriller", "Crime", "War")
const _MURDER4 = ("murder", "murder-in-title", "blood", "violence")
const _NORDIC8 = ("Sweden", "Norway", "Germany", "Denmark",
                  "Swedish", "Denish", "Norwegian", "German")
const _NORDIC9 = ("Sweden", "Norway", "Germany", "Denmark", "Swedish",
                  "Denish", "Norwegian", "German", "English")
const _NORDIC10 = ("Sweden", "Norway", "Germany", "Denmark", "Swedish",
                   "Danish", "Norwegian", "German", "USA", "American")
const _LINK3 = ("sequel", "follows", "followed by")

# ---- templates 1-5, 11-15, 22 (movie-only) ----

_q("2a", "'Doc'") do
    (movie
        : (keyword == "character-name-in-title") ∧
          (company → (Company.country == "[de]"))
        → title)
end

_q("2d", "& Teller") do
    (movie
        : (keyword == "character-name-in-title") ∧
          (company → (Company.country == "[us]"))
        → title)
end

_q("3b", "300: Rise of an Empire") do
    (movie
        : (keyword ~ r"sequel") ∧
          (info → (Info.info == "Bulgaria")) ∧
          (production_year > 2010)
        → title)
end

_q("4a", "5.1 || & Teller 2") do
    (movie
        : (keyword ~ r"sequel") ∧
          (production_year > 2005)
        → ((data : ((Data.type == "rating") ∧ (Data.data > "5.0"))) → Data.data)
        × title)
end

_q("13a", "Afghanistan:24 June 2012 || 1.0 || &Me") do
    (movie
        : (company → ((Company.country == "[de]") ∧ (Company.type == "production companies"))) ∧
          (kind == "movie")
        → (info : (Info.type == "release dates") → Info.info)
        × (data : (Data.type == "rating") → Data.data)
        × title)
end

_q("11a", "Churchill Films || followed by || Batman Beyond") do
    (movie
        : (keyword == "sequel") ∧
          (production_year >= 1950) ∧
          (production_year <= 2000)
        → ((company : ((Company.country != "[pl]")
                   ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner"))
                   ∧ (Company.type == "production companies") - Company.note)) → Company.name)
        × (link → (MovieLink.type ~ r"follow"))
        × title)
end

_q("22a", "(empty)") do
    (movie
        : (info → ((Info.type == "countries")
                ∧ (Info.info in ("Germany", "German", "USA", "American")))) ∧
          (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
          (production_year > 2008) ∧
          (kind in ("movie", "episode"))
        → title
        × ((data : ((Data.data < "7.0") ∧ (Data.type == "rating"))) → Data.data)
        × ((company : ((Company.note ≁ r"\(USA\)")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.country != "[us]")
                   ∧ (Company.type == "production companies"))) → Company.name))
end

_q("1a", "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934") do
    (movie
        : (data → (Data.type == "top 250 rank"))
        → ((company : ((Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")
                   ∧ ((Company.note ~ r"\(co-production\)") ∨ (Company.note ~ r"\(presents\)")))) → Company.note)
        × title
        × production_year)
end

_q("5a", "(empty)") do
    (movie
        : (company → ((Company.type == "production companies")
                   ∧ (Company.note ~ r"\(theatrical\)")
                   ∧ (Company.note ~ r"\(France\)"))) ∧
          (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German"))) ∧
          (production_year > 2005)
        → title)
end

_q("12a", "10th Grade Reunion Films || 8.1 || 3:20") do
    (movie
        : (info → ((Info.type == "genres") ∧ (Info.info in ("Drama", "Horror")))) ∧
          (production_year >= 2005) ∧
          (production_year <= 2008)
        → ((company : ((Company.country == "[us]") ∧ (Company.type == "production companies"))) → Company.name)
        × ((data : ((Data.type == "rating") ∧ (Data.data > "8.0"))) → Data.data)
        × title)
end

_q("14a", "1.0 || \$lowdown") do
    (movie
        : (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
          (kind == "movie") ∧
          (info → ((Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American")))) ∧
          (production_year > 2010)
        → ((data : ((Data.type == "rating") ∧ (Data.data < "8.5"))) → Data.data)
        × title)
end

_q("1b", "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008") do
    (movie
        : (data → (Data.type == "bottom 10 rank")) ∧
          (production_year >= 2005) ∧
          (production_year <= 2010)
        → ((company : ((Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)"))) → Company.note)
        × title
        × production_year)
end

_q("2b", "'Doc'") do
    (movie
        : (keyword == "character-name-in-title") ∧
          (company → (Company.country == "[nl]"))
        → title)
end

_q("2c", "(empty)") do
    (movie
        : (keyword == "character-name-in-title") ∧
          (company → (Company.country == "[sm]"))
        → title)
end

_q("3a", "2 Days in New York") do
    (movie
        : (keyword ~ r"sequel") ∧
          (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German"))) ∧
          (production_year > 2005)
        → title)
end

_q("3c", "& Teller 2") do
    (movie
        : (keyword ~ r"sequel") ∧
          (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American"))) ∧
          (production_year > 1990)
        → title)
end

_q("4b", "9.1 || Batman: Arkham City") do
    (movie
        : (keyword ~ r"sequel") ∧
          (production_year > 2010)
        → ((data : ((Data.type == "rating") ∧ (Data.data > "9.0"))) → Data.data)
        × title)
end

_q("11b", "Filmlance International AB || follows || The Money Man") do
    (movie
        : (keyword == "sequel") ∧
          (production_year == 1998) ∧
          (title ~ r"Money")
        → ((company : ((Company.country != "[pl]")
                   ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner"))
                   ∧ (Company.type == "production companies") - Company.note)) → Company.name)
        × (link → (MovieLink.type ~ r"follows"))
        × title)
end

_q("13b", "501audio || 1.8 || 5 Time Champion") do
    (movie
        : (kind == "movie") ∧
          (info → (Info.type == "release dates")) ∧
          (title != "") ∧
          ((title ~ r"Champion") ∨ (title ~ r"Loser"))
        → ((company : ((Company.country == "[us]") ∧ (Company.type == "production companies"))) → Company.name)
        × (data : (Data.type == "rating") → Data.data)
        × title)
end

_q("1c", "(co-production) || Intouchables || 2011") do
    (movie
        : (data → (Data.type == "top 250 rank")) ∧
          (production_year > 2010)
        → ((company : ((Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")
                   ∧ (Company.note ~ r"\(co-production\)"))) → Company.note)
        × title
        × production_year)
end

_q("1d", "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004") do
    (movie
        : (data → (Data.type == "bottom 10 rank")) ∧
          (production_year > 2000)
        → ((company : ((Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)"))) → Company.note)
        × title
        × production_year)
end

_q("4c", "2.1 || & Teller 2") do
    (movie
        : (keyword ~ r"sequel") ∧
          (production_year > 1990)
        → ((data : ((Data.type == "rating") ∧ (Data.data > "2.0"))) → Data.data)
        × title)
end

_q("12b", "\$10,000 || Birdemic: Shock and Terror") do
    (movie
        : (company → ((Company.country == "[us]")
                   ∧ (Company.type in ("production companies", "distributors")))) ∧
          (data → (Data.type == "bottom 10 rank")) ∧
          (production_year > 2000) ∧
          ((title ~ r"^Birdemic") ∨ (title ~ r"Movie"))
        → (info : (Info.type == "budget") → Info.info)
        × title)
end

_q("12c", "\"Oh That Gus!\" || 7.1 || \$1.11") do
    (movie
        : (info → ((Info.type == "genres")
                ∧ (Info.info in ("Drama", "Horror", "Western", "Family")))) ∧
          (production_year >= 2000) ∧
          (production_year <= 2010)
        → ((company : ((Company.country == "[us]") ∧ (Company.type == "production companies"))) → Company.name)
        × ((data : ((Data.type == "rating") ∧ (Data.data > "7.0"))) → Data.data)
        × title)
end

_q("13c", "DL Sites || 1.8 || Champion") do
    (movie
        : (kind == "movie") ∧
          (info → (Info.type == "release dates")) ∧
          (title != "") ∧
          ((title ~ r"^Champion") ∨ (title ~ r"^Loser"))
        → ((company : ((Company.country == "[us]") ∧ (Company.type == "production companies"))) → Company.name)
        × (data : (Data.type == "rating") → Data.data)
        × title)
end

_q("14b", "6.4 || Of Dolls and Murder") do
    (movie
        : (keyword in ("murder", "murder-in-title")) ∧
          (kind == "movie") ∧
          (info → ((Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American")))) ∧
          (production_year > 2010) ∧
          ((title ~ r"murder") ∨ (title ~ r"Murder") ∨ (title ~ r"Mord"))
        → ((data : ((Data.type == "rating") ∧ (Data.data > "6.0"))) → Data.data)
        × title)
end

_q("14c", "1.0 || \$lowdown") do
    (movie
        : (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
          (kind in ("movie", "episode")) ∧
          (info → ((Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Danish", "Norwegian", "German",
                                 "USA", "American")))) ∧
          (production_year > 2005)
        → ((data : ((Data.type == "rating") ∧ (Data.data < "8.5"))) → Data.data)
        × title)
end

_q("22b", "(empty)") do
    (movie
        : (info → ((Info.type == "countries")
                ∧ (Info.info in ("Germany", "German", "USA", "American")))) ∧
          (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
          (production_year > 2009) ∧
          (kind in ("movie", "episode"))
        → title
        × ((data : ((Data.data < "7.0") ∧ (Data.type == "rating"))) → Data.data)
        × ((company : ((Company.note ≁ r"\(USA\)")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.country != "[us]")
                   ∧ (Company.type == "production companies"))) → Company.name))
end

_q("22c", "(empty)") do
    (movie
        : (info → ((Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Danish", "Norwegian", "German",
                                 "USA", "American")))) ∧
          (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
          (production_year > 2005) ∧
          (kind in ("movie", "episode"))
        → title
        × ((data : ((Data.data < "8.5") ∧ (Data.type == "rating"))) → Data.data)
        × ((company : ((Company.note ≁ r"\(USA\)")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.country != "[us]")
                   ∧ (Company.type == "production companies"))) → Company.name))
end

_q("22d", "(#1.1) || 2.0 || 13 Productions") do
    (movie
        : (info → ((Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Danish", "Norwegian", "German",
                                 "USA", "American")))) ∧
          (keyword in ("murder", "murder-in-title", "blood", "violence")) ∧
          (production_year > 2005) ∧
          (kind in ("movie", "episode"))
        → title
        × ((data : ((Data.data < "8.5") ∧ (Data.type == "rating"))) → Data.data)
        × ((company : ((Company.country != "[us]") ∧ (Company.type == "production companies"))) → Company.name))
end

_q("5b", "(empty)") do
    (movie
        : (company → ((Company.type == "production companies")
                   ∧ (Company.note ~ r"\(VHS\)")
                   ∧ (Company.note ~ r"\(USA\)")
                   ∧ (Company.note ~ r"\(1994\)"))) ∧
          (info → (Info.info in ("USA", "America"))) ∧
          (production_year > 2010)
        → title)
end

_q("5c", "11,830,420") do
    (movie
        : (company → ((Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(TV\)")
                   ∧ (Company.note ~ r"\(USA\)"))) ∧
          (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American"))) ∧
          (production_year > 1990)
        → title)
end

_q("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance") do
    (movie
        : (production_year > 2000) ∧
          (company → ((Company.country == "[us]")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.note ~ r"\(worldwide\)"))) ∧
          keyword ∧
          aka
        → ((info : ((Info.type == "release dates")
                ∧ (Info.info ~ r"^USA:.* 200")
                ∧ (Info.note ~ r"internet"))) → Info.info)
        × title)
end

_q("15b", "USA:27 April 2007 || RoboCop vs Terminator") do
    (movie
        : (company → ((Company.country == "[us]")
                   ∧ (Company.name == "YouTube")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.note ~ r"\(worldwide\)"))) ∧
          keyword ∧
          aka ∧
          (production_year >= 2005) ∧
          (production_year <= 2010)
        → ((info : ((Info.type == "release dates")
                ∧ (Info.info ~ r"^USA:.* 200")
                ∧ (Info.note ~ r"internet"))) → Info.info)
        × title)
end

_q("15c", "USA:1 April 2003 || 24: Day Six - Debrief") do
    (movie
        : (company → (Company.country == "[us]")) ∧
          keyword ∧
          aka ∧
          (production_year > 1990)
        → ((info : ((Info.type == "release dates")
                ∧ ((Info.info ~ r"^USA:.* 199") ∨ (Info.info ~ r"^USA:.* 200"))
                ∧ (Info.note ~ r"internet"))) → Info.info)
        × title)
end

_q("15d", "(Not So) Instant Photo || 06/05") do
    (movie
        : (company → (Company.country == "[us]")) ∧
          keyword ∧
          (info → ((Info.type == "release dates") ∧ (Info.note ~ r"internet"))) ∧
          (production_year > 1990)
        → (aka → AkaTitle.title)
        × title)
end

_q("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24") do
    (movie
        : (keyword in ("sequel", "revenge", "based-on-novel")) ∧
          (production_year > 1950) ∧
          link
        → ((company : ((Company.country != "[pl]")
                    ∧ ((Company.name ~ r"^20th Century Fox") ∨ (Company.name ~ r"^Twentieth Century Fox"))
                    ∧ (Company.type != "production companies")
                    ∧ Company.note)) → (Company.name × Company.note))
        × title)
end

_q("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun") do
    (movie
        : (keyword in ("sequel", "revenge", "based-on-novel")) ∧
          (production_year > 1950) ∧
          link
        → ((company : ((Company.country != "[pl]")
                    ∧ (Company.type != "production companies")
                    ∧ Company.note)) → (Company.name × Company.note))
        × title)
end

_q("13d", "\"O\" Films || 1.0 || #54 Meets #47") do
    (movie
        : (kind == "movie") ∧
          (info → (Info.type == "release dates"))
        → ((company : ((Company.country == "[us]") ∧ (Company.type == "production companies"))) → Company.name)
        × (data : (Data.type == "rating") → Data.data)
        × title)
end

# ---- templates 6-10, 16-33 (cast / complete_cast / person_info) ----

_q("6a", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert") do
    let kw = keyword == "marvel-cinematic-universe"
        (movie
            : (production_year > 2010) ∧ kw
            → kw × title × (cast → person → (Person.name ~ r"Downey.*Robert")))
    end
end

_q("6b", "based-on-comic || The Avengers 2 || Downey Jr., Robert") do
    let kw = keyword in _KW8
        (movie
            : (production_year > 2014) ∧ kw
            → kw × title × (cast → person → (Person.name ~ r"Downey.*Robert")))
    end
end

_q("6c", "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert") do
    let kw = keyword == "marvel-cinematic-universe"
        (movie
            : (production_year > 2014) ∧ kw
            → kw × title × (cast → person → (Person.name ~ r"Downey.*Robert")))
    end
end

_q("6d", "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert") do
    let kw = keyword in _KW8
        (movie
            : (production_year > 2000) ∧ kw
            → kw × title × (cast → person → (Person.name ~ r"Downey.*Robert")))
    end
end

_q("6e", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert") do
    let kw = keyword == "marvel-cinematic-universe"
        (movie
            : (production_year > 2000) ∧ kw
            → kw × title × (cast → person → (Person.name ~ r"Downey.*Robert")))
    end
end

_q("6f", "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha") do
    let kw = keyword in _KW8
        (movie
            : (production_year > 2000) ∧ kw
            → kw × title × (cast → person → Person.name))
    end
end

# ===================================================================
_q("7a", "Antonioni, Michelangelo || Dressed to Kill") do
    (movie
        : (production_year >= 1980) ∧ (production_year <= 1995) ∧
          (linked_by → (MovieLink.type == "features"))
        → (cast
            : (person → (((Person.aka → AkaName.name ~ r"a")
                       ∧ (Person.name_pcode_cf >= "A") ∧ (Person.name_pcode_cf <= "F")
                       ∧ ((Person.gender == "m") ∨ ((Person.gender == "f") ∧ (Person.name ~ r"^B")))
                       ∧ (Person.info → ((PersonInfo.type == "mini biography") ∧ (PersonInfo.note == "Volker Boehm"))))))
            → person → Person.name)
        × title)
end

_q("7b", "De Palma, Brian || Dressed to Kill") do
    (movie
        : (production_year >= 1980) ∧ (production_year <= 1984) ∧
          (linked_by → (MovieLink.type == "features"))
        → (cast
            : (person → (((Person.aka → AkaName.name ~ r"a") ∧ (Person.name_pcode_cf ~ r"^D") ∧ (Person.gender == "m")
                       ∧ (Person.info → ((PersonInfo.type == "mini biography") ∧ (PersonInfo.note == "Volker Boehm"))))))
            → person → Person.name)
        × title)
end

_q("7c", "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.") do
    let bio_filter = (PersonInfo.type == "mini biography") ∧ PersonInfo.note,
        pf = ((Person.aka → AkaName.name ~ r"a|^A")
            ∧ (Person.name_pcode_cf >= "A") ∧ (Person.name_pcode_cf <= "F")
            ∧ ((Person.gender == "m") ∨ ((Person.gender == "f") ∧ (Person.name ~ r"^A"))))
        (movie
            : (production_year >= 1980) ∧ (production_year <= 2010) ∧
              (linked_by → (MovieLink.type in ("references", "referenced in", "features", "featured in")))
            → (cast
                : (person → ((pf ∧ (Person.info → bio_filter))))
                → (person → Person.name)
                × (person → Person.info : bio_filter → PersonInfo.info)))
    end
end

# ===================================================================
_q("8a", "Chambers, Linda || .hack//Quantum") do
    (movie
        : (company → (((Company.country == "[jp]") ∧ (Company.note ~ r"\(Japan\)") ∧ (Company.note ≁ r"\(USA\)"))))
        → (cast
            : (note == "(voice: English version)") ∧
              (role == "actress") ∧
              (person → (((Person.name ~ r"Yo") ∧ (Person.name ≁ r"Yu"))))
            → person → Person.aka → AkaName.name)
        × title)
end

_q("8b", "Chambers, Linda || Dragon Ball Z: Shin Budokai") do
    (movie
        : (company → (((Company.country == "[jp]") ∧ (Company.note ~ r"\(Japan\)") ∧ (Company.note ≁ r"\(USA\)") ∧ ((Company.note ~ r"\(2006\)") ∨ (Company.note ~ r"\(2007\)"))))) ∧
          (production_year >= 2006) ∧ (production_year <= 2007) ∧
          ((title ~ r"^One Piece") ∨ (title ~ r"^Dragon Ball Z"))
        → (cast
            : (note == "(voice: English version)") ∧
              (role == "actress") ∧
              (person → (((Person.name ~ r"Yo") ∧ (Person.name ≁ r"Yu"))))
            → person → Person.aka → AkaName.name)
        × title)
end

_q("8c", "\"A.J.\" || #1 Cheerleader Camp") do
    (movie
        : (company → (Company.country == "[us]"))
        → (cast : (role == "writer") → person → Person.aka → AkaName.name)
        × title)
end

_q("8d", "\"Jenny from the Block\" || #1 Cheerleader Camp") do
    (movie
        : (company → (Company.country == "[us]"))
        → (cast : (role == "costume designer") → person → Person.aka → AkaName.name)
        × title)
end

# ===================================================================
_q("9a", "AJ || Airport Announcer || Blue Harvest") do
    (movie
        : (company → (((Company.country == "[us]") ∧ ((Company.note ~ r"\(USA\)") ∨ (Company.note ~ r"\(worldwide\)"))))) ∧
          (production_year >= 2005) ∧ (production_year <= 2015)
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"Ang"))))
            → (person → Person.aka → AkaName.name)
            × (character → Character.name))
        × title)
end

_q("9b", "AJ || Airport Announcer || Bassett, Angela || Blue Harvest") do
    (movie
        : (company → (((Company.country == "[us]") ∧ (Company.note ~ r"\(200.*\)") ∧ ((Company.note ~ r"\(USA\)") ∨ (Company.note ~ r"\(worldwide\)"))))) ∧
          (production_year >= 2007) ∧ (production_year <= 2010)
        → (cast
            : (note == "(voice)") ∧
              (role == "actress") ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"Angel"))))
            → (person → Person.aka → AkaName.name)
            × (character → Character.name)
            × (person → Person.name))
        × title)
end

_q("9c", "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)") do
    (movie
        : (company → (Company.country == "[us]"))
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An"))))
            → (person → Person.aka → AkaName.name)
            × (character → Character.name)
            × (person → Person.name))
        × title)
end

_q("9d", "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || \$15,000.00 Error") do
    (movie
        : (company → (Company.country == "[us]"))
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              (person → (Person.gender == "f"))
            → (person → Person.aka → AkaName.name)
            × (person → Person.name)
            × (character → Character.name))
        × title)
end

# ===================================================================
_q("10a", "Actor || 12 Rounds") do
    (movie
        : (company → (Company.country == "[ru]")) ∧
          (production_year > 2005)
        → (cast
            : (note ~ r"\(voice\)") ∧
              (note ~ r"\(uncredited\)") ∧
              (role == "actor")
            → character → Character.name)
        × title)
end

_q("10b", "(empty)") do
    (movie
        : (company → (Company.country == "[ru]")) ∧
          (production_year > 2010)
        → (cast
            : (note ~ r"\(producer\)") ∧
              (role == "actor")
            → character → Character.name)
        × title)
end

_q("10c", "Himself || Evil Eyes: Behind the Scenes") do
    (movie
        : (company → (Company.country == "[us]")) ∧
          (production_year > 1990)
        → (cast : (note ~ r"\(producer\)") → character → Character.name)
        × title)
end

# ===================================================================
_q("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins") do
    (movie
        : (company → (Company.country == "[us]")) ∧ (keyword == "character-name-in-title") ∧
          (episode_nr >= 50) ∧ (episode_nr < 100)
        → (cast → person → Person.aka → AkaName.name)
        × title)
end

_q("16b", "!!!, Toy || & Teller") do
    (movie
        : (company → (Company.country == "[us]")) ∧ (keyword == "character-name-in-title")
        → (cast → person → Person.aka → AkaName.name)
        × title)
end

_q("16c", "\"Brooklyn\" Tony Danza || (#1.5)") do
    (movie
        : (company → (Company.country == "[us]")) ∧ (keyword == "character-name-in-title") ∧
          (episode_nr < 100)
        → (cast → person → Person.aka → AkaName.name)
        × title)
end

_q("16d", "\"Brooklyn\" Tony Danza || (#1.5)") do
    (movie
        : (company → (Company.country == "[us]")) ∧ (keyword == "character-name-in-title") ∧
          (episode_nr >= 5) ∧ (episode_nr < 100)
        → (cast → person → Person.aka → AkaName.name)
        × title)
end

# ===================================================================
_q("17a", "B, Khaz") do
    (movie
        : (company → (Company.country == "[us]")) ∧ (keyword == "character-name-in-title")
        → (cast → person → (Person.name ~ r"^B")))
end

_q("17b", "Z'Dar, Robert") do
    (movie
        : company ∧ (keyword == "character-name-in-title")
        → (cast → person → (Person.name ~ r"^Z")))
end

_q("17c", "X'Volaitis, John") do
    (movie
        : company ∧ (keyword == "character-name-in-title")
        → (cast → person → (Person.name ~ r"^X")))
end

_q("17d", "Abrahamsson, Bertil") do
    (movie
        : company ∧ (keyword == "character-name-in-title")
        → (cast → person → (Person.name ~ r"Bert")))
end

_q("17e", "\$hort, Too") do
    (movie
        : (company → (Company.country == "[us]")) ∧ (keyword == "character-name-in-title")
        → (cast → person → Person.name))
end

_q("17f", "'El Galgo PornoStar', Blanquito") do
    (movie
        : company ∧ (keyword == "character-name-in-title")
        → (cast → person → (Person.name ~ r"B")))
end

# ===================================================================
_q("18a", "\$1,000 || 10 || 40 Days and 40 Nights") do
    let ib = info : (Info.type == "budget") → Info.info,
        dv = data : (Data.type == "votes") → Data.data
        (movie
            : ib
            ∧ (cast → ((note in ("(producer)", "(executive producer)"))
                    ∧ (person → ((Person.gender == "m") ∧ (Person.name ~ r"Tim")))))
            → ib × dv × title)
    end
end

_q("18b", "Horror || 8.1 || Agorable") do
    let gf = ((Info.type == "genres") ∧ (Info.info in ("Horror", "Thriller"))) - Info.note,
        ig = info : gf → Info.info,
        dr = data : ((Data.type == "rating") ∧ (Data.data > "8.0")) → Data.data
        (movie
            : (info → gf) ∧ (production_year >= 2008) ∧ (production_year <= 2014) ∧
              (cast → ((note in _WRITER5) ∧ (person → (Person.gender == "f"))))
            → ig × dr × title)
    end
end

_q("18c", "Action || 10 || #PostModem") do
    let gf = (Info.type == "genres") ∧ (Info.info in _GENRE6),
        ig = info : gf → Info.info,
        dv = data : (Data.type == "votes") → Data.data
        (movie
            : (info → gf) ∧
              (cast → ((note in _WRITER5) ∧ (person → (Person.gender == "m"))))
            → ig × dv × title)
    end
end

# ===================================================================
_q("19a", "Angeline, Moriah || Blue Harvest") do
    (movie
        : (company → (((Company.country == "[us]") ∧ ((Company.note ~ r"\(USA\)") ∨ (Company.note ~ r"\(worldwide\)"))))) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*200") ∨ (Info.info ~ r"^USA:.*200"))))) ∧
          (production_year >= 2005) ∧ (production_year <= 2009)
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              character ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"Ang") ∧ Person.aka)))
            → person → Person.name)
        × title)
end

_q("19b", "Jolie, Angelina || Kung Fu Panda") do
    (movie
        : (company → (((Company.country == "[us]") ∧ (Company.note ~ r"\(200.*\)") ∧ ((Company.note ~ r"\(USA\)") ∨ (Company.note ~ r"\(worldwide\)"))))) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*2007") ∨ (Info.info ~ r"^USA:.*2008"))))) ∧
          (production_year >= 2007) ∧ (production_year <= 2008) ∧
          (title ~ r"Kung.*Fu.*Panda")
        → (cast
            : (note == "(voice)") ∧
              (role == "actress") ∧
              character ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"Angel") ∧ Person.aka)))
            → person → Person.name)
        × title)
end

_q("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2") do
    (movie
        : (company → (Company.country == "[us]")) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*200") ∨ (Info.info ~ r"^USA:.*200"))))) ∧
          (production_year > 2000)
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              character ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An") ∧ Person.aka)))
            → person → Person.name)
        × title)
end

_q("19d", "Aaron, Caroline || \$9.99") do
    (movie
        : (company → (Company.country == "[us]")) ∧
          (info → (Info.type == "release dates")) ∧
          (production_year > 2000)
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              character ∧
              (person → (((Person.gender == "f") ∧ Person.aka)))
            → person → Person.name)
        × title)
end

# ===================================================================
_q("20a", "Disaster Movie") do
    (movie
        : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"complete")))) ∧
          (keyword in _KW8) ∧ (kind == "movie") ∧ (production_year > 1950) ∧
          (cast → ((character : ((Character.name ≁ r"Sherlock")
                              ∧ ((Character.name ~ r"Tony.*Stark") ∨ (Character.name ~ r"Iron.*Man"))))))
        → title)
end

_q("20b", "Iron Man") do
    (movie
        : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"complete")))) ∧
          (keyword in _KW8) ∧ (kind == "movie") ∧ (production_year > 2000) ∧
          (cast → ((character : ((Character.name ≁ r"Sherlock")
                              ∧ ((Character.name ~ r"Tony.*Stark") ∨ (Character.name ~ r"Iron.*Man"))))
                ∧ (person → (Person.name ~ r"Downey.*Robert"))))
        → title)
end

_q("20c", "Abell, Alistair || ...And Then I...") do
    (movie
        : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"complete")))) ∧
          (keyword in _KW10) ∧ (kind == "movie") ∧ (production_year > 2000)
        → (cast : (character → (Character.name ~ r"[Mm]an")) → person → Person.name)
        × title)
end

# ===================================================================
_q("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes") do
    let co = (company : (((Company.country != "[pl]") ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner")) ∧ ((Company.type == "production companies") - Company.note)))),
        lk = (link → (MovieLink.type ~ r"follow"))
        (movie
           : co ∧ (keyword == "sequel") ∧ lk
           ∧ (info → (Info.info in _NORDIC8))
           ∧ (production_year >= 1950) ∧ (production_year <= 2000)
           → (co → Company.name) × lk × title)
    end
end

_q("21b", "Filmlance International AB || followed by || Hämndens pris") do
    let co = (company : (((Company.country != "[pl]") ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner")) ∧ ((Company.type == "production companies") - Company.note)))),
        lk = (link → (MovieLink.type ~ r"follow"))
        (movie
           : co ∧ (keyword == "sequel") ∧ lk
           ∧ (info → (Info.info in ("Germany", "German")))
           ∧ (production_year >= 2000) ∧ (production_year <= 2010)
           → (co → Company.name) × lk × title)
    end
end

_q("21c", "Churchill Films || followed by || Batman Beyond") do
    let co = (company : (((Company.country != "[pl]") ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner")) ∧ ((Company.type == "production companies") - Company.note)))),
        lk = (link → (MovieLink.type ~ r"follow"))
        (movie
           : co ∧ (keyword == "sequel") ∧ lk
           ∧ (info → (Info.info in _NORDIC9))
           ∧ (production_year >= 1950) ∧ (production_year <= 2010)
           → (co → Company.name) × lk × title)
    end
end

# ===================================================================
_q("23a", "movie || The Analysts") do
    let k = (kind == "movie")
        (movie
           : (complete_cast → (CompleteCast.status == "complete+verified")) ∧
             (company → (Company.country == "[us]")) ∧
             (info → (((Info.type == "release dates") ∧ (Info.note ~ r"internet") ∧ ((Info.info ~ r"^USA:.* 199") ∨ (Info.info ~ r"^USA:.* 200"))))) ∧
             k ∧ keyword ∧ (production_year > 2000)
           → k × title)
    end
end

_q("23b", "movie || The Big Mope") do
    let k = (kind == "movie")
        (movie
           : (complete_cast → (CompleteCast.status == "complete+verified")) ∧
             (company → (Company.country == "[us]")) ∧
             (info → (((Info.type == "release dates") ∧ (Info.note ~ r"internet") ∧ (Info.info ~ r"^USA:.* 200")))) ∧
             k ∧
             (keyword in ("nerd", "loner", "alienation", "dignity")) ∧
             (production_year > 2000)
           → k × title)
    end
end

_q("23c", "movie || Dirt Merchant") do
    let k = (kind in ("movie", "tv movie", "video movie", "video game"))
        (movie
           : (complete_cast → (CompleteCast.status == "complete+verified")) ∧
             (company → (Company.country == "[us]")) ∧
             (info → (((Info.type == "release dates") ∧ (Info.note ~ r"internet") ∧ ((Info.info ~ r"^USA:.* 199") ∨ (Info.info ~ r"^USA:.* 200"))))) ∧
             k ∧ keyword ∧ (production_year > 1990)
           → k × title)
    end
end

# ===================================================================
_q("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6") do
    (movie
        : (company → (Company.country == "[us]")) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*201") ∨ (Info.info ~ r"^USA:.*201"))))) ∧
          (keyword in ("hero", "martial-arts", "hand-to-hand-combat")) ∧ (production_year > 2010)
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An") ∧ Person.aka)))
            → (character → Character.name)
            × (person → Person.name))
        × title)
end

_q("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2") do
    (movie
        : (company → (((Company.country == "[us]") ∧ (Company.name == "DreamWorks Animation")))) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*201") ∨ (Info.info ~ r"^USA:.*201"))))) ∧
          (keyword in ("hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie")) ∧
          (production_year > 2010) ∧ (title ~ r"^Kung Fu Panda")
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An") ∧ Person.aka)))
            → (character → Character.name)
            × (person → Person.name))
        × title)
end

# ===================================================================
_q("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon") do
    let gf = (Info.type == "genres") ∧ (Info.info == "Horror")
        (movie
            : (info → gf) ∧ (keyword in ("murder", "blood", "gore", "death", "female-nudity"))
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

_q("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah") do
    let gf = (Info.type == "genres") ∧ (Info.info == "Horror")
        (movie
            : (info → gf) ∧ (keyword in ("murder", "blood", "gore", "death", "female-nudity")) ∧
              (production_year > 2010) ∧ (title ~ r"^Vampire")
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

_q("25c", "Action || 10 || \$ || Aakeson, Kim Fupz") do
    let gf = (Info.type == "genres") ∧ (Info.info in _GENRE6)
        (movie
            : (info → gf) ∧ (keyword in _KW7)
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

# ===================================================================
_q("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma") do
    let rd = data : ((Data.type == "rating") ∧ (Data.data > "7.0"))
        (movie
            : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"complete")))) ∧
              (keyword in _KW10) ∧ (kind == "movie") ∧ (production_year > 2000)
            → (cast : (character → (Character.name ~ r"[Mm]an")) → ((character → Character.name) × (person → Person.name)))
            × (rd → Data.data)
            × title)
    end
end

_q("26b", "Bank Manager || 8.2 || Inception") do
    let rd = data : ((Data.type == "rating") ∧ (Data.data > "8.0"))
        (movie
            : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"complete")))) ∧
              (keyword in ("superhero", "marvel-comics", "based-on-comic", "fight")) ∧ (kind == "movie") ∧
              (production_year > 2005)
            → (cast : (character → (Character.name ~ r"[Mm]an")) → character → Character.name)
            × (rd → Data.data)
            × title)
    end
end

_q("26c", "'Agua' Man || 1.9 || 12 Rounds") do
    let rd = data : (Data.type == "rating") → Data.data
        (movie
            : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"complete")))) ∧
              (keyword in _KW10) ∧ (kind == "movie") ∧ (production_year > 2000)
            → (cast : (character → (Character.name ~ r"[Mm]an")) → character → Character.name)
            × rd
            × title)
    end
end

# ===================================================================
_q("27a", "Det Danske Filminstitut || followed by || Spår i mörker") do
    let co = (company : (((Company.country != "[pl]") ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner")) ∧ ((Company.type == "production companies") - Company.note)))),
        lk = (link → (MovieLink.type ~ r"follow"))
        (movie
           : (complete_cast → (((CompleteCast.subject in ("cast", "crew")) ∧ (CompleteCast.status == "complete")))) ∧
             co ∧ (keyword == "sequel") ∧ lk ∧
             (info → (Info.info in ("Sweden", "Germany", "Swedish", "German"))) ∧
             (production_year >= 1950) ∧ (production_year <= 2000)
           → (co → Company.name) × lk × title)
    end
end

_q("27b", "Filmlance International AB || followed by || Vita nätter") do
    let co = (company : (((Company.country != "[pl]") ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner")) ∧ ((Company.type == "production companies") - Company.note)))),
        lk = (link → (MovieLink.type ~ r"follow"))
        (movie
           : (complete_cast → (((CompleteCast.subject in ("cast", "crew")) ∧ (CompleteCast.status == "complete")))) ∧
             co ∧ (keyword == "sequel") ∧ lk ∧
             (info → (Info.info in ("Sweden", "Germany", "Swedish", "German"))) ∧
             (production_year == 1998)
           → (co → Company.name) × lk × title)
    end
end

_q("27c", "Det Danske Filminstitut || followed by || Spår i mörker") do
    let co = (company : (((Company.country != "[pl]") ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner")) ∧ ((Company.type == "production companies") - Company.note)))),
        lk = (link → (MovieLink.type ~ r"follow"))
        (movie
           : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status ~ r"^complete")))) ∧
             co ∧ (keyword == "sequel") ∧ lk ∧
             (info → (Info.info in _NORDIC9)) ∧
             (production_year >= 1950) ∧ (production_year <= 2010)
           → (co → Company.name) × lk × title)
    end
end

# ===================================================================
_q("28a", "01 Distribuzione || 2.9 || (#1.1)") do
    let co = (company : (((Company.country != "[us]") ∧ (Company.note ≁ r"\(USA\)") ∧ (Company.note ~ r"\(200.*\)")))),
        dt = (data : (((Data.type == "rating") ∧ (Data.data < "8.5"))))
        (movie
           : (complete_cast → (((CompleteCast.subject == "crew") ∧ (CompleteCast.status != "complete+verified")))) ∧
             co ∧
             (info → (((Info.type == "countries") ∧ (Info.info in _NORDIC10)))) ∧
             dt ∧
             (keyword in _MURDER4) ∧ (kind in ("movie", "episode")) ∧ (production_year > 2000)
           → (co → Company.name) × (dt → Data.data) × title)
    end
end

_q("28b", "20th Century Fox || 6.6 || (#1.1)") do
    let co = (company : (((Company.country != "[us]") ∧ (Company.note ≁ r"\(USA\)") ∧ (Company.note ~ r"\(200.*\)")))),
        dt = (data : (((Data.type == "rating") ∧ (Data.data > "6.5"))))
        (movie
           : (complete_cast → (((CompleteCast.subject == "crew") ∧ (CompleteCast.status != "complete+verified")))) ∧
             co ∧
             (info → (((Info.type == "countries") ∧ (Info.info in ("Sweden", "Germany", "Swedish", "German"))))) ∧
             dt ∧
             (keyword in _MURDER4) ∧ (kind in ("movie", "episode")) ∧ (production_year > 2005)
           → (co → Company.name) × (dt → Data.data) × title)
    end
end

_q("28c", "01 Distribuzione || 1.9 || (#1.1)") do
    let co = (company : (((Company.country != "[us]") ∧ (Company.note ≁ r"\(USA\)") ∧ (Company.note ~ r"\(200.*\)")))),
        dt = (data : (((Data.type == "rating") ∧ (Data.data < "8.5"))))
        (movie
           : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status == "complete")))) ∧
             co ∧
             (info → (((Info.type == "countries") ∧ (Info.info in _NORDIC10)))) ∧
             dt ∧
             (keyword in _MURDER4) ∧ (kind in ("movie", "episode")) ∧ (production_year > 2005)
           → (co → Company.name) × (dt → Data.data) × title)
    end
end

# ===================================================================
_q("29a", "Queen || Andrews, Julie || Shrek 2") do
    (movie
        : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status == "complete+verified")))) ∧
          (company → (Company.country == "[us]")) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*200") ∨ (Info.info ~ r"^USA:.*200"))))) ∧
          (keyword == "computer-animation") ∧ (title == "Shrek 2") ∧
          (production_year >= 2000) ∧ (production_year <= 2010)
        → (cast
            : (note in _VOICE3) ∧
              (role == "actress") ∧
              (character → (Character.name == "Queen")) ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An") ∧ Person.aka
                       ∧ (Person.info → (PersonInfo.type == "trivia")))))
            → (character → Character.name)
            × (person → Person.name))
        × title)
end

_q("29b", "Queen || Andrews, Julie || Shrek 2") do
    (movie
        : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status == "complete+verified")))) ∧
          (company → (Company.country == "[us]")) ∧
          (info → (((Info.type == "release dates") ∧ (Info.info ~ r"^USA:.*200")))) ∧
          (keyword == "computer-animation") ∧ (title == "Shrek 2") ∧
          (production_year >= 2000) ∧ (production_year <= 2005)
        → (cast
            : (note in _VOICE3) ∧
              (role == "actress") ∧
              (character → (Character.name == "Queen")) ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An") ∧ Person.aka
                       ∧ (Person.info → (PersonInfo.type == "height")))))
            → (character → Character.name)
            × (person → Person.name))
        × title)
end

_q("29c", "Lola || Andrews, Julie || Hoodwinked!") do
    (movie
        : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status == "complete+verified")))) ∧
          (company → (Company.country == "[us]")) ∧
          (info → (((Info.type == "release dates") ∧ ((Info.info ~ r"^Japan:.*200") ∨ (Info.info ~ r"^USA:.*200"))))) ∧
          (keyword == "computer-animation") ∧
          (production_year >= 2000) ∧ (production_year <= 2010)
        → (cast
            : (note in _VOICE4) ∧
              (role == "actress") ∧
              (person → (((Person.gender == "f") ∧ (Person.name ~ r"An") ∧ Person.aka
                       ∧ (Person.info → (PersonInfo.type == "trivia")))))
            → (character → Character.name)
            × (person → Person.name))
        × title)
end

# ===================================================================
_q("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.") do
    let gf = (Info.type == "genres") ∧ (Info.info in ("Horror", "Thriller"))
        (movie
            : (complete_cast → (((CompleteCast.subject in ("cast", "crew")) ∧ (CompleteCast.status == "complete+verified")))) ∧
              (info → gf) ∧ (keyword in _KW7) ∧ (production_year > 2000)
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

_q("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian") do
    let gf = (Info.type == "genres") ∧ (Info.info in ("Horror", "Thriller"))
        (movie
            : (complete_cast → (((CompleteCast.subject in ("cast", "crew")) ∧ (CompleteCast.status == "complete+verified")))) ∧
              (info → gf) ∧ (keyword in _KW7) ∧ (production_year > 2000) ∧
              ((title ~ r"Freddy") ∨ (title ~ r"Jason") ∨ (title ~ r"^Saw"))
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

_q("30c", "Action || 100356 || \$ || Abernathy, Lewis") do
    let gf = (Info.type == "genres") ∧ (Info.info in _GENRE6)
        (movie
            : (complete_cast → (((CompleteCast.subject == "cast") ∧ (CompleteCast.status == "complete+verified")))) ∧
              (info → gf) ∧ (keyword in _KW7)
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

# ===================================================================
_q("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim") do
    let gf = (Info.type == "genres") ∧ (Info.info in ("Horror", "Thriller"))
        (movie
            : (company → (Company.name ~ r"^Lionsgate")) ∧
              (info → gf) ∧ (keyword in _KW7)
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

_q("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn") do
    let gf = (Info.type == "genres") ∧ (Info.info in ("Horror", "Thriller"))
        (movie
            : (company → (((Company.name ~ r"^Lionsgate") ∧ (Company.note ~ r"\(Blu-ray\)")))) ∧
              (info → gf) ∧ (keyword in _KW7) ∧ (production_year > 2000) ∧
              ((title ~ r"Freddy") ∨ (title ~ r"Jason") ∨ (title ~ r"^Saw"))
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : ((note in _WRITER5) ∧ (person → (Person.gender == "m"))) → person → Person.name))
    end
end

_q("31c", "Action || 1008 || 11:14 || Abraham, Brad") do
    let gf = (Info.type == "genres") ∧ (Info.info in _GENRE6)
        (movie
            : (company → (Company.name ~ r"^Lionsgate")) ∧
              (info → gf) ∧ (keyword in _KW7)
            → (info : gf → Info.info)
            × (data : (Data.type == "votes") → Data.data)
            × title
            × (cast : (note in _WRITER5) → person → Person.name))
    end
end

# ===================================================================
_q("32a", "(empty)") do
    (movie
       : (keyword == "10,000-mile-club") ∧ link
       → ((link → MovieLink.type) → LinkType.link)
       × title
       × (link → MovieLink.target → title))
end

_q("32b", "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview") do
    (movie
       : (keyword == "character-name-in-title") ∧ link
       → ((link → MovieLink.type) → LinkType.link)
       × title
       × (link → MovieLink.target → title))
end

# ===================================================================
# 33 is a movie self-join (t1 linked to t2). `qlink` is the *qualifying* link
# (right type, target satisfying the t2 predicate); `t1` is the filtered
# source domain. Both t1- and t2-side projections restrict to those, so the
# 6-way `×` runs over a few hundred rows, not the whole universe.
_q("33a", "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila") do
    let co  = company : (Company.country == "[us]") → Company.name,
        rd  = data : (Data.type    == "rating") → Data.data,
        rdlt = data   : ((Data.type   == "rating") ∧ (Data.data < "3.0")) → Data.data,
        t2f = ((kind == "tv series") ∧ company ∧ rdlt
                                     ∧ (production_year >= 2005) ∧ (production_year <= 2008)),
        qlk = link : ((MovieLink.type in _LINK3) ∧ (MovieLink.target → t2f)),
        t2  = qlk → MovieLink.target
        (movie
            : (kind == "tv series") ∧ co ∧ qlk
            → co × (t2 → company → Company.name) × rd × (t2 → rdlt) × title × (t2 → title))
    end
end

_q("33b", "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila") do
    let co  = company : (Company.country == "[nl]") → Company.name,
        rd  = data : (Data.type    == "rating") → Data.data,
        rdlt = data   : ((Data.type   == "rating") ∧ (Data.data < "3.0")) → Data.data,
        t2f = (kind == "tv series") ∧ company ∧ rdlt ∧ (production_year == 2007),
        qlk = link : ((MovieLink.type ~ r"follow") ∧ (MovieLink.target → t2f)),
        t2  = qlk → MovieLink.target
        (movie
            : (kind == "tv series") ∧ co ∧ qlk
            → co × (t2 → company → Company.name) × rd × (t2 → rdlt) × title × (t2 → title))
    end
end

_q("33c", "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love") do
    let co  = company : (Company.country != "[us]") → Company.name,
        rd  = data : (Data.type    == "rating") → Data.data,
        rdlt = data   : ((Data.type   == "rating") ∧ (Data.data < "3.5")) → Data.data,
        t2f = ((kind in ("tv series", "episode")) ∧ company ∧ rdlt
                                                  ∧ (production_year >= 2000) ∧ (production_year <= 2010)),
        qlk = link : ((MovieLink.type in _LINK3) ∧ (MovieLink.target → t2f)),
        t2  = qlk → MovieLink.target
        (movie
            : (kind in ("tv series", "episode")) ∧ co ∧ qlk
            → co × (t2 → company → Company.name) × rd × (t2 → rdlt) × title × (t2 → title))
    end
end

get(ENV, "PRELA_SKIP_RUNALL", "0") == "0" && runall()
