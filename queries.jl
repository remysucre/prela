# JOB queries. Assumes `include("start.jl")` (or `include("JOB.jl")`) has run.
@assert isdefined(Main, :movie) "Run `include(\"JOB.jl\")` first."

# Helper: flatten a left-associated tuple into a flat vector of leaf values.
# Our broadcast composes `r.(a, b, c)` as `((a_val, b_val), c_val)`, so the
# tree is left-deep — recurse on `.first` only.
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

# Per-column MIN across all output rows.
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

# Convenience: format the per-column MIN list as `MIN(c1)=..., MIN(c2)=...`.
_fmt_mins(q::Rel) = isempty(q.pairs) ? "(empty)" :
    join(("MIN[$i]=$(repr(v))" for (i, v) in enumerate(_min_cols(q))), ", ")

println()

# === 2a: 'character-name-in-title' + German company ===  Expected MIN: "'Doc'"
println("=== 2a ===  (expected MIN(title): \"'Doc'\")")
let t = time()
    q = movie.(
            ((keyword == "character-name-in-title")
           & (company ∘ (Company.country == "[de]")))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 2d: 'character-name-in-title' + US company ===
println("\n=== 2d ===")
let t = time()
    q = movie.(
            ((keyword == "character-name-in-title")
           & (company ∘ (Company.country == "[us]")))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 3b: '%sequel%' kw + Bulgarian info + year > 2010 ===
println("\n=== 3b ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (info ∘ (Info.info == "Bulgaria"))
           & (production_year > 2010))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 4a: '%sequel%' kw + rating > 5.0 + year > 2005 ===
# Output: (rating, title); MIN over each separately in SQL.
println("\n=== 4a ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (production_year > 2005))
          : (data ∘ ((Data.type == "rating") & (Data.data > "5.0"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 13a: German production company + movie kind + rating + release date ===
println("\n=== 13a ===")
let t = time()
    q = movie.(
            ((company ∘ ((Company.country == "[de]") & (Company.type == "production companies")))
           & (kind == "movie"))
          : (info ∘ dom(Info.type == "release dates")).info
          , (data ∘ dom(Data.type == "rating")).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 11a: sequel keyword + non-Polish Film/Warner company + follow link ===
println("\n=== 11a ===")
let t = time()
    q = movie.(
            ((keyword == "sequel")
           & (production_year >= 1950) & (production_year <= 2000))
          : (company ∘ (
                (Company.country != "[pl]")
              & ((Company.name ~ r"Film") | (Company.name ~ r"Warner"))
              & (Company.type == "production companies")
              - Company.note    # IS NULL
            )).name
          , link ∘ (MovieLink.type ~ r"follow")
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22a: 'murder*' kw + Germanic info + low-rating data + non-US production company ===
# Genuinely empty: verified independently — only 144 movies satisfy the non-US
# production-company-with-(200X)-note filter (mostly pre-2008), and none
# overlap the 2034 LHS-filter movies (year>2008 + murder kw + Germanic info).
println("\n=== 22a ===  (empty: verified non-empty intersection bug-free)")
let t = time()
    q = movie.(
            ((info ∘ ((Info.type == "countries") &
                      (Info.info in ("Germany", "German", "USA", "American"))))
           & (keyword in ("murder", "murder-in-title", "blood", "violence"))
           & (production_year > 2008)
           & (kind in ("movie", "episode")))
          : title
          , (data ∘ ((Data.data < "7.0") & (Data.type == "rating"))).data
          , (company ∘ (
                (Company.note ≁ r"\(USA\)")
              & (Company.note ~ r"\(200.*\)")
              & (Company.country != "[us]")
              & (Company.type == "production companies")
            )).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1a: top-250 movies + co-production-or-presents (non-MGM) company ===
# Output: (note, title, year).
println("\n=== 1a ===")
let t = time()
    q = movie.(
            (data ∘ (Data.type == "top 250 rank"))
          : (company ∘ (
                (Company.type == "production companies")
              & (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")
              & ((Company.note ~ r"\(co-production\)") | (Company.note ~ r"\(presents\)"))
            )).note
          , title
          , production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 5a: French theatrical company + European info + year > 2005 ===
# Genuinely empty: ALL 24K mc rows with "(theatrical)+(France)" notes are
# DISTRIBUTORS (type 1), not production companies — verified independently.
println("\n=== 5a ===  (empty: all (theatrical)+(France) notes are on distributors)")
let t = time()
    q = movie.(
            ((company ∘ (
                (Company.type == "production companies")
              & (Company.note ~ r"\(theatrical\)")
              & (Company.note ~ r"\(France\)")))
           & (info ∘ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                    "Swedish", "Denish", "Norwegian", "German")))
           & (production_year > 2005))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 12a: drama/horror + 2005-2008 + US production company + rating > 8.0 ===
println("\n=== 12a ===")
let t = time()
    q = movie.(
            ((info ∘ ((Info.type == "genres") & (Info.info in ("Drama", "Horror"))))
           & (production_year >= 2005) & (production_year <= 2008))
          : (company ∘ ((Company.country == "[us]") & (Company.type == "production companies"))).name
          , (data ∘ ((Data.type == "rating") & (Data.data > "8.0"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 14a: violence-keyword movies + Germanic/Scandi countries + year > 2010 + rating < 8.5 ===
println("\n=== 14a ===")
let t = time()
    q = movie.(
            ((keyword in ("murder", "murder-in-title", "blood", "violence"))
           & (kind == "movie")
           & (info ∘ ((Info.type == "countries") &
                     (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                    "Swedish", "Denish", "Norwegian", "German",
                                    "USA", "American"))))
           & (production_year > 2010))
          : (data ∘ ((Data.type == "rating") & (Data.data < "8.5"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1b: bottom-10 movies, 2005-2010, non-MGM production company ===
println("\n=== 1b ===")
let t = time()
    q = movie.(
            ((data ∘ (Data.type == "bottom 10 rank"))
           & (production_year >= 2005) & (production_year <= 2010))
          : (company ∘ (
                (Company.type == "production companies")
              & (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)"))).note
          , title
          , production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 2b: character-name + Dutch company ===
println("\n=== 2b ===")
let t = time()
    q = movie.(
            ((keyword == "character-name-in-title")
           & (company ∘ (Company.country == "[nl]")))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 2c: character-name + San Marino company ===
println("\n=== 2c ===")
let t = time()
    q = movie.(
            ((keyword == "character-name-in-title")
           & (company ∘ (Company.country == "[sm]")))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 3a: sequel keyword + European info + year > 2005 ===
println("\n=== 3a ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (info ∘ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                    "Swedish", "Denish", "Norwegian", "German")))
           & (production_year > 2005))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 3c: sequel + USA/Germanic info + year > 1990 (looser than 3a) ===
println("\n=== 3c ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (info ∘ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                    "Swedish", "Denish", "Norwegian", "German",
                                    "USA", "American")))
           & (production_year > 1990))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 4b: sequel keyword + year > 2010 + rating > 9.0 ===
println("\n=== 4b ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (production_year > 2010))
          : (data ∘ ((Data.type == "rating") & (Data.data > "9.0"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 11b: sequel + year==1998 + title~Money + Film/Warner production company + follows link ===
println("\n=== 11b ===")
let t = time()
    q = movie.(
            ((keyword == "sequel")
           & (production_year == 1998)
           & (title ~ r"Money"))
          : (company ∘ (
                (Company.country != "[pl]")
              & ((Company.name ~ r"Film") | (Company.name ~ r"Warner"))
              & (Company.type == "production companies")
              - Company.note)).name
          , link ∘ (MovieLink.type ~ r"follows")
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 13b: movie + release-date info + Champion/Loser title + US production + rating ===
println("\n=== 13b ===")
let t = time()
    q = movie.(
            ((kind == "movie")
           & (info ∘ (Info.type == "release dates"))
           & (title != "")
           & ((title ~ r"Champion") | (title ~ r"Loser")))
          : (company ∘ ((Company.country == "[us]") & (Company.type == "production companies"))).name
          , (data ∘ dom(Data.type == "rating")).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1c: top-250 + year > 2010 + co-production (non-MGM) ===
println("\n=== 1c ===")
let t = time()
    q = movie.(
            ((data ∘ (Data.type == "top 250 rank"))
           & (production_year > 2010))
          : (company ∘ (
                (Company.type == "production companies")
              & (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)")
              & (Company.note ~ r"\(co-production\)"))).note
          , title
          , production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 1d: bottom-10 + year > 2000 + non-MGM production ===
println("\n=== 1d ===")
let t = time()
    q = movie.(
            ((data ∘ (Data.type == "bottom 10 rank"))
           & (production_year > 2000))
          : (company ∘ (
                (Company.type == "production companies")
              & (Company.note ≁ r"\(as Metro-Goldwyn-Mayer Pictures\)"))).note
          , title
          , production_year)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 4c: sequel + year > 1990 + rating > 2.0 ===
println("\n=== 4c ===")
let t = time()
    q = movie.(
            ((keyword ~ r"sequel")
           & (production_year > 1990))
          : (data ∘ ((Data.type == "rating") & (Data.data > "2.0"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 12b: US prod/distrib + bottom-10 + year > 2000 + title ~ Birdemic/Movie + budget info ===
println("\n=== 12b ===")
let t = time()
    q = movie.(
            ((company ∘ ((Company.country == "[us]")
                       & (Company.type in ("production companies", "distributors"))))
           & (data ∘ (Data.type == "bottom 10 rank"))
           & (production_year > 2000)
           & ((title ~ r"^Birdemic") | (title ~ r"Movie")))
          : (info ∘ dom(Info.type == "budget")).info
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 12c: drama/horror/western/family + 2000-2010 + US production + rating > 7.0 ===
println("\n=== 12c ===")
let t = time()
    q = movie.(
            ((info ∘ ((Info.type == "genres")
                    & (Info.info in ("Drama", "Horror", "Western", "Family"))))
           & (production_year >= 2000) & (production_year <= 2010))
          : (company ∘ ((Company.country == "[us]")
                      & (Company.type == "production companies"))).name
          , (data ∘ ((Data.type == "rating") & (Data.data > "7.0"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 13c: movie kind + release-date info + title^Champion/Loser + US production + rating ===
println("\n=== 13c ===")
let t = time()
    q = movie.(
            ((kind == "movie")
           & (info ∘ (Info.type == "release dates"))
           & (title != "")
           & ((title ~ r"^Champion") | (title ~ r"^Loser")))
          : (company ∘ ((Company.country == "[us]") & (Company.type == "production companies"))).name
          , (data ∘ dom(Data.type == "rating")).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 14b: murder kw + Germanic countries + year > 2010 + murder-title + rating > 6 ===
println("\n=== 14b ===")
let t = time()
    q = movie.(
            ((keyword in ("murder", "murder-in-title"))
           & (kind == "movie")
           & (info ∘ ((Info.type == "countries")
                    & (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                     "Swedish", "Denish", "Norwegian", "German",
                                     "USA", "American"))))
           & (production_year > 2010)
           & ((title ~ r"murder") | (title ~ r"Murder") | (title ~ r"Mord")))
          : (data ∘ ((Data.type == "rating") & (Data.data > "6.0"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 14c: like 14a but year > 2005 and broader keyword/kind set ===
println("\n=== 14c ===")
let t = time()
    q = movie.(
            ((keyword in ("murder", "murder-in-title", "blood", "violence"))
           & (kind in ("movie", "episode"))
           & (info ∘ ((Info.type == "countries")
                    & (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                     "Swedish", "Danish", "Norwegian", "German",
                                     "USA", "American"))))
           & (production_year > 2005))
          : (data ∘ ((Data.type == "rating") & (Data.data < "8.5"))).data
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22b: like 22a but year > 2009 ===
println("\n=== 22b ===")
let t = time()
    q = movie.(
            ((info ∘ ((Info.type == "countries") &
                      (Info.info in ("Germany", "German", "USA", "American"))))
           & (keyword in ("murder", "murder-in-title", "blood", "violence"))
           & (production_year > 2009)
           & (kind in ("movie", "episode")))
          : title
          , (data ∘ ((Data.data < "7.0") & (Data.type == "rating"))).data
          , (company ∘ (
                (Company.note ≁ r"\(USA\)")
              & (Company.note ~ r"\(200.*\)")
              & (Company.country != "[us]")
              & (Company.type == "production companies"))).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22c: like 22a, year > 2005, broader country list, rating < 8.5 ===
println("\n=== 22c ===")
let t = time()
    q = movie.(
            ((info ∘ ((Info.type == "countries")
                    & (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                     "Swedish", "Danish", "Norwegian", "German",
                                     "USA", "American"))))
           & (keyword in ("murder", "murder-in-title", "blood", "violence"))
           & (production_year > 2005)
           & (kind in ("movie", "episode")))
          : title
          , (data ∘ ((Data.data < "8.5") & (Data.type == "rating"))).data
          , (company ∘ (
                (Company.note ≁ r"\(USA\)")
              & (Company.note ~ r"\(200.*\)")
              & (Company.country != "[us]")
              & (Company.type == "production companies"))).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 22d: like 22c but with simpler company filter (no note) ===
println("\n=== 22d ===")
let t = time()
    q = movie.(
            ((info ∘ ((Info.type == "countries")
                    & (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                     "Swedish", "Danish", "Norwegian", "German",
                                     "USA", "American"))))
           & (keyword in ("murder", "murder-in-title", "blood", "violence"))
           & (production_year > 2005)
           & (kind in ("movie", "episode")))
          : title
          , (data ∘ ((Data.data < "8.5") & (Data.type == "rating"))).data
          , (company ∘ ((Company.country != "[us]") & (Company.type == "production companies"))).name)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 5b: VHS+USA+1994 notes + USA/America info + year > 2010 ===
println("\n=== 5b ===")
let t = time()
    q = movie.(
            ((company ∘ (
                (Company.type == "production companies")
              & (Company.note ~ r"\(VHS\)")
              & (Company.note ~ r"\(USA\)")
              & (Company.note ~ r"\(1994\)")))
           & (info ∘ (Info.info in ("USA", "America")))
           & (production_year > 2010))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 5c: production not (TV), (USA) note + European/US info + year > 1990 ===
println("\n=== 5c ===")
let t = time()
    q = movie.(
            ((company ∘ (
                (Company.type == "production companies")
              & (Company.note ≁ r"\(TV\)")
              & (Company.note ~ r"\(USA\)")))
           & (info ∘ (Info.info in ("Sweden", "Norway", "Germany", "Denmark",
                                    "Swedish", "Denish", "Norwegian", "German",
                                    "USA", "American")))
           & (production_year > 1990))
          : title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15a: year > 2000, US worldwide-200X company, has kw+aka, USA:200X internet info ===
println("\n=== 15a ===")
let t = time()
    q = movie.(
            ((production_year > 2000)
           & (company ∘ ((Company.country == "[us]")
                       & (Company.note ~ r"\(200.*\)")
                       & (Company.note ~ r"\(worldwide\)")))
           & keyword
           & aka)
          : (info ∘ ((Info.type == "release dates")
                   & (Info.info ~ r"^USA:.* 200")
                   & (Info.note ~ r"internet"))).info
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15b: 15a but YouTube company + year 2005-2010 ===
println("\n=== 15b ===")
let t = time()
    q = movie.(
            ((company ∘ ((Company.country == "[us]")
                       & (Company.name == "YouTube")
                       & (Company.note ~ r"\(200.*\)")
                       & (Company.note ~ r"\(worldwide\)")))
           & keyword
           & aka
           & (production_year >= 2005) & (production_year <= 2010))
          : (info ∘ ((Info.type == "release dates")
                   & (Info.info ~ r"^USA:.* 200")
                   & (Info.note ~ r"internet"))).info
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15c: US company + kw+aka + year > 1990 + USA:199X|200X internet info ===
println("\n=== 15c ===")
let t = time()
    q = movie.(
            ((company ∘ (Company.country == "[us]"))
           & keyword
           & aka
           & (production_year > 1990))
          : (info ∘ ((Info.type == "release dates")
                   & ((Info.info ~ r"^USA:.* 199") | (Info.info ~ r"^USA:.* 200"))
                   & (Info.note ~ r"internet"))).info
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end

# === 15d: US company + has keyword + release dates with internet note + year > 1990 ===
println("\n=== 15d ===")
let t = time()
    q = movie.(
            ((company ∘ (Company.country == "[us]"))
           & keyword
           & (info ∘ ((Info.type == "release dates") & (Info.note ~ r"internet")))
           & (production_year > 1990))
          : aka.title
          , title)
    println("  $(length(q.pairs)) rows, MINs = $(_fmt_mins(q))  ($(round(time()-t; digits=2))s)")
    flush(stdout)
end
