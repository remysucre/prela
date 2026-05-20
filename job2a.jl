include("Prela.jl")
using .Prela
using Parquet2, DataFrames

# === Schema (subset of JOB for query 2a) ===

@entity Keyword begin
    keyword :: String
end

@entity Company begin
    country :: String
end

@entity Movie begin
    title   :: String
    keyword :: ID{Keyword}
    company :: ID{Company}
end

# Bare access to Movie's fields
@expose Movie

# === Loader ===

load_parquet(path) = DataFrame(Parquet2.Dataset(path); copycols=false)

# Column padding in parquet output varies with column count; index by position.
col(df, i) = df[!, i + 1]   # 0-based input, 1-based DataFrame

function load_pairs!(rel::Rel{D, R}, ids, vals, idctor) where {D, R}
    n = length(ids)
    pairs = Vector{Pair{D, R}}(undef, 0)
    sizehint!(pairs, n)
    for i in 1:n
        id  = ids[i]
        val = vals[i]
        (ismissing(id) || ismissing(val)) && continue
        push!(pairs, idctor(id) => val)
    end
    append!(rel.pairs, pairs)
end

function load_all!()
    base = "../jobdata/parquet"

    t = time()
    println("Loading title...")
    df = load_parquet("$base/title.parquet")
    load_pairs!(title, col(df, 0), col(df, 1), ID{Movie})
    println("  $(length(title.pairs)) movies loaded ($(round(time()-t; digits=1))s)")

    t = time()
    println("Loading keyword...")
    df = load_parquet("$base/keyword.parquet")
    kw_text = Prela.lookup_field(ID{Keyword}, Val(:keyword))
    load_pairs!(kw_text, col(df, 0), col(df, 1), ID{Keyword})
    println("  $(length(kw_text.pairs)) keywords loaded ($(round(time()-t; digits=1))s)")

    t = time()
    println("Loading movie_keyword...")
    df = load_parquet("$base/movie_keyword.parquet")
    # col1=movie_id, col2=keyword_id
    n = nrow(df)
    pairs = Vector{Pair{ID{Movie}, ID{Keyword}}}(undef, 0)
    sizehint!(pairs, n)
    movie_ids = col(df, 1); kw_ids = col(df, 2)
    for i in 1:n
        mid = movie_ids[i]; kid = kw_ids[i]
        (ismissing(mid) || ismissing(kid)) && continue
        push!(pairs, ID{Movie}(mid) => ID{Keyword}(kid))
    end
    append!(keyword.pairs, pairs)
    println("  $(length(keyword.pairs)) movie-keyword pairs loaded ($(round(time()-t; digits=1))s)")

    t = time()
    println("Loading company_name...")
    df = load_parquet("$base/company_name.parquet")
    cn_country = Prela.lookup_field(ID{Company}, Val(:country))
    # col0=id, col2=country_code
    load_pairs!(cn_country, col(df, 0), col(df, 2), ID{Company})
    println("  $(length(cn_country.pairs)) company countries loaded ($(round(time()-t; digits=1))s)")

    t = time()
    println("Loading movie_companies...")
    df = load_parquet("$base/movie_companies.parquet")
    # col1=movie_id, col2=company_id
    n = nrow(df)
    pairs = Vector{Pair{ID{Movie}, ID{Company}}}(undef, 0)
    sizehint!(pairs, n)
    movie_ids = col(df, 1); comp_ids = col(df, 2)
    for i in 1:n
        mid = movie_ids[i]; cid = comp_ids[i]
        (ismissing(mid) || ismissing(cid)) && continue
        push!(pairs, ID{Movie}(mid) => ID{Company}(cid))
    end
    append!(company.pairs, pairs)
    println("  $(length(company.pairs)) movie-company pairs loaded ($(round(time()-t; digits=1))s)")
end

# === Build the universe and run 2a ===

load_all!()
const movie = Unary{ID{Movie}}([p.first for p in title.pairs])
println("Universe: $(length(movie.values)) movies")

# JOB 2a:
#   SELECT MIN(t.title)
#   FROM cn, k, mc, mk, t
#   WHERE cn.country_code = '[de]'
#     AND k.keyword = 'character-name-in-title'
#     AND <joins>;
#
# Prela:
println("\nRunning 2a: movies with keyword 'character-name-in-title' and a German company")
t0 = time()
q = movie.(((keyword == "character-name-in-title") & (company.country == "[de]")) : title)
elapsed = round(time() - t0; digits=2)
println("  $(length(q.pairs)) matching pairs ($(elapsed)s)")

if !isempty(q.pairs)
    n_movies   = length(unique(p.first  for p in q.pairs))
    n_titles   = length(unique(p.second for p in q.pairs))
    titles_srt = sort!(unique(p.second for p in q.pairs))
    println("  $n_movies distinct movies, $n_titles distinct titles")
    println("  MIN(title) = $(repr(first(titles_srt)))   (JOB 2a canonical: \"Doc\")")
    println("  first 5 distinct titles: ", titles_srt[1:min(5, end)])
end
