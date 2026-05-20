# JOB queries. Assumes `include("start.jl")` (or `include("JOB.jl")`) has run.
@assert isdefined(Main, :movie) "Run `include(\"JOB.jl\")` first."

# Helper: flatten a left-associated tuple into a flat vector of leaf values.
function _flatten_leaves(v)
    if v isa Tuple
        out = Any[]
        function go(x)
            if x isa Tuple
                for el in x
                    go(el)
                end
            else
                push!(out, x)
            end
        end
        go(v)
        out
    else
        Any[v]
    end
end

function _min_cols(q::Rel)
    isempty(q.pairs) && return Any[]
    sample = _flatten_leaves(q.pairs[1].second)
    n = length(sample)
    cols = [Vector{Any}(undef, 0) for _ in 1:n]
    for p in q.pairs
        flat = _flatten_leaves(p.second)
        for i in 1:n
            push!(cols[i], flat[i])
        end
    end
    [minimum(c) for c in cols]
end

_fmt_mins(q::Rel) = isempty(q.pairs) ? "(empty)" :
    join(("MIN[$i]=$(repr(v))" for (i, v) in enumerate(_min_cols(q))), ", ")

println()

# === 2a ===
println("=== 2a ===  (expected MIN(title): \"'Doc'\")")
let t = time()
    q = (movie
        → (keyword == "character-name-in-title")
        ∧ (company → (Company.country == "[de]"))
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 2d ===
println("\n=== 2d ===")
let t = time()
    q = (movie
        → (keyword == "character-name-in-title")
        ∧ (company → (Company.country == "[us]"))
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 3b ===
println("\n=== 3b ===")
let t = time()
    q = (movie
        → (keyword ~ r"sequel")
        ∧ (info → (Info.info == "Bulgaria"))
        ∧ (production_year > 2010)
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 4a ===
println("\n=== 4a ===")
let t = time()
    q = (movie
        → (keyword ~ r"sequel")
        ∧ (production_year > 2005)
        → (data → (Data.type == "rating") ∧ (Data.data > "5.0")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 13a ===
println("\n=== 13a ===")
let t = time()
    q = (movie
        → (company → (Company.country == "[de]") ∧ (Company.type == "production companies"))
        ∧ (kind == "movie")
        → (info → (Info.type == "release dates") : Info.info)
        × (data → (Data.type == "rating") : Data.data)
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 11a ===
println("\n=== 11a ===")
let t = time()
    q = (movie
        → (keyword == "sequel")
        ∧ (production_year >= 1950)
        ∧ (production_year <= 2000)
        → (company → (Company.country != "[pl]")
                   ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner"))
                   ∧ (Company.type == "production companies") - Company.note).name
        × (link → (MovieLink.type ~ r"follow"))
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22a === (empty: verified vs DuckDB)
println("\n=== 22a ===  (empty)")
let t = time()
    q = (movie
        → (info → (Info.type == "countries")
                ∧ (Info.info in ("Germany", "German", "USA", "American")))
        ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
        ∧ (production_year > 2008)
        ∧ (kind in ("movie", "episode"))
        → title
        × (data → (Data.data < "7.0") ∧ (Data.type == "rating")).data
        × (company → (Company.note ≁ r"\(USA\)")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.country != "[us]")
                   ∧ (Company.type == "production companies")).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1a ===
println("\n=== 1a ===")
let t = time()
    q = (movie
        → (data → (Data.type == "top 250 rank"))
        : (company → (Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")
                   ∧ ((Company.note ~ r"\(co-production\)") ∨ (Company.note ~ r"\(presents\)"))).note
        × title
        × production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 5a === (empty)
println("\n=== 5a ===  (empty)")
let t = time()
    q = (movie
        → (company → (Company.type == "production companies")
                   ∧ (Company.note ~ r"\(theatrical\)")
                   ∧ (Company.note ~ r"\(France\)"))
        ∧ (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German")))
        ∧ (production_year > 2005)
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 12a ===
println("\n=== 12a ===")
let t = time()
    q = (movie
        → (info → (Info.type == "genres") ∧ (Info.info in ("Drama", "Horror")))
        ∧ (production_year >= 2005)
        ∧ (production_year <= 2008)
        → (company → (Company.country == "[us]") ∧ (Company.type == "production companies")).name
        × (data → (Data.type == "rating") ∧ (Data.data > "8.0")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 14a ===
println("\n=== 14a ===")
let t = time()
    q = (movie
        → (keyword in ("murder", "murder-in-title", "blood", "violence"))
        ∧ (kind == "movie")
        ∧ (info → (Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (production_year > 2010)
        → (data → (Data.type == "rating") ∧ (Data.data < "8.5")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1b ===
println("\n=== 1b ===")
let t = time()
    q = (movie
        → (data → (Data.type == "bottom 10 rank"))
        ∧ (production_year >= 2005)
        ∧ (production_year <= 2010)
        → (company → (Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")).note
        × title
        × production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 2b ===
println("\n=== 2b ===")
let t = time()
    q = (movie
        → (keyword == "character-name-in-title")
        ∧ (company → (Company.country == "[nl]"))
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 2c === (empty)
println("\n=== 2c ===  (empty)")
let t = time()
    q = (movie
        → (keyword == "character-name-in-title")
        ∧ (company → (Company.country == "[sm]"))
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 3a ===
println("\n=== 3a ===")
let t = time()
    q = (movie
        → (keyword ~ r"sequel")
        ∧ (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German")))
        ∧ (production_year > 2005)
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 3c ===
println("\n=== 3c ===")
let t = time()
    q = (movie
        → (keyword ~ r"sequel")
        ∧ (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (production_year > 1990)
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 4b ===
println("\n=== 4b ===")
let t = time()
    q = (movie
        → (keyword ~ r"sequel")
        ∧ (production_year > 2010)
        → (data → (Data.type == "rating") ∧ (Data.data > "9.0")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 11b ===
println("\n=== 11b ===")
let t = time()
    q = (movie
        → (keyword == "sequel")
        ∧ (production_year == 1998)
        ∧ (title ~ r"Money")
        → (company → (Company.country != "[pl]")
                   ∧ ((Company.name ~ r"Film") ∨ (Company.name ~ r"Warner"))
                   ∧ (Company.type == "production companies") - Company.note).name
        × (link → (MovieLink.type ~ r"follows"))
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 13b ===
println("\n=== 13b ===")
let t = time()
    q = (movie
        → (kind == "movie")
        ∧ (info → (Info.type == "release dates"))
        ∧ (title != "")
        ∧ ((title ~ r"Champion") ∨ (title ~ r"Loser"))
        → (company → (Company.country == "[us]") ∧ (Company.type == "production companies")).name
        × (data → (Data.type == "rating") : Data.data)
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1c ===
println("\n=== 1c ===")
let t = time()
    q = (movie
        → (data → (Data.type == "top 250 rank"))
        ∧ (production_year > 2010)
        → (company → (Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")
                   ∧ (Company.note ~ r"\(co-production\)")).note
        × title
        × production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1d ===
println("\n=== 1d ===")
let t = time()
    q = (movie
        → (data → (Data.type == "bottom 10 rank"))
        ∧ (production_year > 2000)
        → (company → (Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")).note
        × title
        × production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 4c ===
println("\n=== 4c ===")
let t = time()
    q = (movie
        → (keyword ~ r"sequel")
        ∧ (production_year > 1990)
        → (data → (Data.type == "rating") ∧ (Data.data > "2.0")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 12b ===
println("\n=== 12b ===")
let t = time()
    q = (movie
        → (company → (Company.country == "[us]")
                   ∧ (Company.type in ("production companies", "distributors")))
        ∧ (data → (Data.type == "bottom 10 rank"))
        ∧ (production_year > 2000)
        ∧ ((title ~ r"^Birdemic") ∨ (title ~ r"Movie"))
        → (info → (Info.type == "budget") : Info.info)
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 12c ===
println("\n=== 12c ===")
let t = time()
    q = (movie
        → (info → (Info.type == "genres")
                ∧ (Info.info in ("Drama", "Horror", "Western", "Family")))
        ∧ (production_year >= 2000)
        ∧ (production_year <= 2010)
        → (company → (Company.country == "[us]") ∧ (Company.type == "production companies")).name
        × (data → (Data.type == "rating") ∧ (Data.data > "7.0")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 13c ===
println("\n=== 13c ===")
let t = time()
    q = (movie
        → (kind == "movie")
        ∧ (info → (Info.type == "release dates"))
        ∧ (title != "")
        ∧ ((title ~ r"^Champion") ∨ (title ~ r"^Loser"))
        → (company → (Company.country == "[us]") ∧ (Company.type == "production companies")).name
        × (data → (Data.type == "rating") : Data.data)
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 14b ===
println("\n=== 14b ===")
let t = time()
    q = (movie
        → (keyword in ("murder", "murder-in-title"))
        ∧ (kind == "movie")
        ∧ (info → (Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (production_year > 2010)
        ∧ ((title ~ r"murder") ∨ (title ~ r"Murder") ∨ (title ~ r"Mord"))
        → (data → (Data.type == "rating") ∧ (Data.data > "6.0")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 14c ===
println("\n=== 14c ===")
let t = time()
    q = (movie
        → (keyword in ("murder", "murder-in-title", "blood", "violence"))
        ∧ (kind in ("movie", "episode"))
        ∧ (info → (Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Danish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (production_year > 2005)
        → (data → (Data.type == "rating") ∧ (Data.data < "8.5")).data
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22b === (empty)
println("\n=== 22b ===  (empty)")
let t = time()
    q = (movie
        → (info → (Info.type == "countries")
                ∧ (Info.info in ("Germany", "German", "USA", "American")))
        ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
        ∧ (production_year > 2009)
        ∧ (kind in ("movie", "episode"))
        → title
        × (data → (Data.data < "7.0") ∧ (Data.type == "rating")).data
        × (company → (Company.note ≁ r"\(USA\)")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.country != "[us]")
                   ∧ (Company.type == "production companies")).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22c === (empty)
println("\n=== 22c ===  (empty)")
let t = time()
    q = (movie
        → (info → (Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Danish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
        ∧ (production_year > 2005)
        ∧ (kind in ("movie", "episode"))
        → title
        × (data → (Data.data < "8.5") ∧ (Data.type == "rating")).data
        × (company → (Company.note ≁ r"\(USA\)")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.country != "[us]")
                   ∧ (Company.type == "production companies")).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22d ===
println("\n=== 22d ===")
let t = time()
    q = (movie
        → (info → (Info.type == "countries")
                ∧ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Danish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (keyword in ("murder", "murder-in-title", "blood", "violence"))
        ∧ (production_year > 2005)
        ∧ (kind in ("movie", "episode"))
        → title
        × (data → (Data.data < "8.5") ∧ (Data.type == "rating")).data
        × (company → (Company.country != "[us]") ∧ (Company.type == "production companies")).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 5b === (empty)
println("\n=== 5b ===  (empty)")
let t = time()
    q = (movie
        → (company → (Company.type == "production companies")
                   ∧ (Company.note ~ r"\(VHS\)")
                   ∧ (Company.note ~ r"\(USA\)")
                   ∧ (Company.note ~ r"\(1994\)"))
        ∧ (info → (Info.info in ("USA", "America")))
        ∧ (production_year > 2010)
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 5c ===
println("\n=== 5c ===")
let t = time()
    q = (movie
        → (company → (Company.type == "production companies")
                   ∧ (Company.note ≁ r"\(TV\)")
                   ∧ (Company.note ~ r"\(USA\)"))
        ∧ (info → (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                 "Swedish", "Denish", "Norwegian", "German",
                                 "USA", "American")))
        ∧ (production_year > 1990)
        → title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15a ===
println("\n=== 15a ===")
let t = time()
    q = (movie
        → (production_year > 2000)
        ∧ (company → (Company.country == "[us]")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.note ~ r"\(worldwide\)"))
        ∧ keyword
        ∧ aka
        → (info → (Info.type == "release dates")
                ∧ (Info.info ~ r"^USA:.* 200")
                ∧ (Info.note ~ r"internet")).info
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15b ===
println("\n=== 15b ===")
let t = time()
    q = (movie
        → (company → (Company.country == "[us]")
                   ∧ (Company.name == "YouTube")
                   ∧ (Company.note ~ r"\(200.*\)")
                   ∧ (Company.note ~ r"\(worldwide\)"))
        ∧ keyword
        ∧ aka
        ∧ (production_year >= 2005)
        ∧ (production_year <= 2010)
        → (info → (Info.type == "release dates")
                ∧ (Info.info ~ r"^USA:.* 200")
                ∧ (Info.note ~ r"internet")).info
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15c ===
println("\n=== 15c ===")
let t = time()
    q = (movie
        → (company → (Company.country == "[us]"))
        ∧ keyword
        ∧ aka
        ∧ (production_year > 1990)
        → (info → (Info.type == "release dates")
                ∧ ((Info.info ~ r"^USA:.* 199") ∨ (Info.info ~ r"^USA:.* 200"))
                ∧ (Info.note ~ r"internet")).info
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15d ===
println("\n=== 15d ===")
let t = time()
    q = (movie
        → (company → (Company.country == "[us]"))
        ∧ keyword
        ∧ (info → (Info.type == "release dates") ∧ (Info.note ~ r"internet"))
        ∧ (production_year > 1990)
        → aka.title
        × title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end
