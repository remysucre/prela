# One-shot JOB data loader. include("JOB.jl") once — afterwards every JOB
# entity is declared, every relation is populated from `../jobdata/parquet/`,
# and Movie's fields are bare-name accessible (`title`, `keyword`, ...).

include("Prela.jl")
using .Prela
using Parquet2, DataFrames

# === Forward-declare every entity so cyclic refs work ===

@declare Movie Keyword Kind LinkType RoleType CompCastType InfoType CompanyType
@declare Character AkaName AkaTitle Person Company Info Data MovieLink Cast CompleteCast

# === Entity declarations ===

@entity Keyword      begin keyword :: String end
@entity Kind         begin kind    :: String end
@entity LinkType     begin link    :: String end
@entity RoleType     begin role    :: String end
@entity CompCastType begin kind    :: String end
@entity InfoType     begin info    :: String end
@entity CompanyType  begin kind    :: String end
@entity Character    begin name    :: String end
@entity AkaName      begin name    :: String end
@entity AkaTitle     begin title   :: String end

@entity Person begin
    name           :: String
    gender         :: String
    name_pcode_cf  :: String
end

@entity Company begin
    name    :: String
    note    :: String
    country :: String
    type    :: ID{CompanyType}
end

@entity Info begin
    info :: String
    type :: ID{InfoType}
    note :: String
end

@entity Data begin
    data :: String
    type :: ID{InfoType}
end

@entity MovieLink begin
    type   :: ID{LinkType}
    target :: ID{Movie}
end

@entity Cast begin
    person    :: ID{Person}
    character :: ID{Character}
    role      :: ID{RoleType}
    note      :: String
end

@entity CompleteCast begin
    subject :: ID{CompCastType}
    status  :: ID{CompCastType}
end

@entity Movie begin
    title           :: String
    production_year :: Int
    episode_nr      :: Int
    kind            :: ID{Kind}
    info            :: ID{Info}
    keyword         :: ID{Keyword}
    data            :: ID{Data}
    company         :: ID{Company}
    cast            :: ID{Cast}
    complete_cast   :: ID{CompleteCast}
    link            :: ID{MovieLink}
    linked_by       :: ID{MovieLink}
    aka             :: ID{AkaTitle}
end

@expose Movie

# === Loader helpers ===

const BASE = "../jobdata/parquet"

load_df(name) = DataFrame(Parquet2.Dataset(joinpath(BASE, name * ".parquet")); copycols=false)
col(df, i) = df[!, i + 1]   # 0-based to 1-based

# Load a Rel{D, R} from two parquet columns. `idctor` / `valctor` wrap the raw
# Int into entity IDs (or `identity` for scalars).
function _load!(rel::Rel{D, R}, ids, vals, idctor, valctor) where {D, R}
    n = length(ids)
    pairs = Vector{Pair{D, R}}(undef, 0)
    sizehint!(pairs, n)
    for i in 1:n
        id = ids[i]; val = vals[i]
        (ismissing(id) || ismissing(val)) && continue
        push!(pairs, idctor(id) => valctor(val))
    end
    append!(rel.pairs, pairs)
end

# Build a Dict for FK lookups (e.g. company_id → country).
function build_lookup(df, key_idx, val_idx)
    ks = col(df, key_idx); vs = col(df, val_idx)
    d = Dict{Int64, Any}()
    n = length(ks)
    sizehint!(d, n)
    for i in 1:n
        k = ks[i]; v = vs[i]
        (ismissing(k) || ismissing(v)) && continue
        d[k] = v
    end
    d
end

function load_all!()
    t_total = time()

    # ---- title (Movie) ----
    t = time()
    df = load_df("title")
    _load!(Prela.lookup_field(ID{Movie}, Val(:title)),
           col(df, 0), col(df, 1), ID{Movie}, identity)
    # kind_id is col 3 → Movie.kind = ID{Kind}
    _load!(Prela.lookup_field(ID{Movie}, Val(:kind)),
           col(df, 0), col(df, 3), ID{Movie}, ID{Kind})
    # production_year is col 4
    _load!(Prela.lookup_field(ID{Movie}, Val(:production_year)),
           col(df, 0), col(df, 4), ID{Movie}, identity)
    # episode_nr is col 9
    _load!(Prela.lookup_field(ID{Movie}, Val(:episode_nr)),
           col(df, 0), col(df, 9), ID{Movie}, identity)
    println("  title (Movie): $(length(title.pairs)) movies ($(round(time()-t; digits=1))s)")

    # ---- kind_type ----
    t = time()
    df = load_df("kind_type")
    _load!(Prela.lookup_field(ID{Kind}, Val(:kind)),
           col(df, 0), col(df, 1), ID{Kind}, identity)
    println("  kind_type: $(length(Prela.lookup_field(ID{Kind}, Val(:kind)).pairs)) kinds ($(round(time()-t; digits=1))s)")

    # ---- keyword + movie_keyword ----
    t = time()
    df = load_df("keyword")
    _load!(Prela.lookup_field(ID{Keyword}, Val(:keyword)),
           col(df, 0), col(df, 1), ID{Keyword}, identity)
    df = load_df("movie_keyword")
    _load!(keyword, col(df, 1), col(df, 2), ID{Movie}, ID{Keyword})
    println("  keyword: $(length(keyword.pairs)) movie-keyword pairs ($(round(time()-t; digits=1))s)")

    # ---- company_name + company_type + movie_companies → Company ----
    # Company entities are movie_companies rows, with name/country/type joined from FKs.
    t = time()
    cn_df = load_df("company_name")
    cn_name    = build_lookup(cn_df, 0, 1)
    cn_country = build_lookup(cn_df, 0, 2)

    ct_df = load_df("company_type")
    _load!(Prela.lookup_field(ID{CompanyType}, Val(:kind)),
           col(ct_df, 0), col(ct_df, 1), ID{CompanyType}, identity)

    mc_df = load_df("movie_companies")
    mc_ids       = col(mc_df, 0)
    mc_movie     = col(mc_df, 1)
    mc_company   = col(mc_df, 2)
    mc_comptype  = col(mc_df, 3)
    mc_note      = col(mc_df, 4)

    co_name    = Prela.lookup_field(ID{Company}, Val(:name))
    co_note    = Prela.lookup_field(ID{Company}, Val(:note))
    co_country = Prela.lookup_field(ID{Company}, Val(:country))
    co_type    = Prela.lookup_field(ID{Company}, Val(:type))

    n = length(mc_ids)
    sizehint!(company.pairs, n)
    sizehint!(co_name.pairs, n); sizehint!(co_country.pairs, n)
    sizehint!(co_type.pairs, n); sizehint!(co_note.pairs, n)
    for i in 1:n
        cid_raw = mc_ids[i]
        ismissing(cid_raw) && continue
        cid = ID{Company}(cid_raw)
        mid = mc_movie[i]
        ismissing(mid) || push!(company.pairs, ID{Movie}(mid) => cid)
        cn = mc_company[i]
        if !ismissing(cn)
            nm = get(cn_name, cn, missing)
            ismissing(nm) || push!(co_name.pairs, cid => nm)
            ct = get(cn_country, cn, missing)
            ismissing(ct) || push!(co_country.pairs, cid => ct)
        end
        ctid = mc_comptype[i]
        ismissing(ctid) || push!(co_type.pairs, cid => ID{CompanyType}(ctid))
        nt = mc_note[i]
        ismissing(nt) || push!(co_note.pairs, cid => nt)
    end
    println("  company: $(length(company.pairs)) movie-company pairs ($(round(time()-t; digits=1))s)")

    # ---- info_type ----
    t = time()
    df = load_df("info_type")
    _load!(Prela.lookup_field(ID{InfoType}, Val(:info)),
           col(df, 0), col(df, 1), ID{InfoType}, identity)
    println("  info_type: $(length(Prela.lookup_field(ID{InfoType}, Val(:info)).pairs)) types ($(round(time()-t; digits=1))s)")

    # ---- movie_info → Info ----
    t = time()
    mi_df = load_df("movie_info")
    mi_ids      = col(mi_df, 0)
    mi_movie    = col(mi_df, 1)
    mi_type     = col(mi_df, 2)
    mi_info     = col(mi_df, 3)
    mi_note     = col(mi_df, 4)

    info_text = Prela.lookup_field(ID{Info}, Val(:info))
    info_type = Prela.lookup_field(ID{Info}, Val(:type))
    info_note = Prela.lookup_field(ID{Info}, Val(:note))
    n = length(mi_ids)
    sizehint!(info.pairs, n); sizehint!(info_text.pairs, n)
    sizehint!(info_type.pairs, n); sizehint!(info_note.pairs, n)
    for i in 1:n
        iid_raw = mi_ids[i]; ismissing(iid_raw) && continue
        iid = ID{Info}(iid_raw)
        mid = mi_movie[i]
        ismissing(mid) || push!(info.pairs, ID{Movie}(mid) => iid)
        it = mi_type[i]
        ismissing(it) || push!(info_type.pairs, iid => ID{InfoType}(it))
        txt = mi_info[i]
        ismissing(txt) || push!(info_text.pairs, iid => txt)
        nt = mi_note[i]
        ismissing(nt) || push!(info_note.pairs, iid => nt)
    end
    println("  movie_info: $(length(info.pairs)) movie-info pairs ($(round(time()-t; digits=1))s)")

    # ---- movie_info_idx → Data ----
    t = time()
    di_df = load_df("movie_info_idx")
    di_ids   = col(di_df, 0)
    di_movie = col(di_df, 1)
    di_type  = col(di_df, 2)
    di_data  = col(di_df, 3)

    data_text = Prela.lookup_field(ID{Data}, Val(:data))
    data_type = Prela.lookup_field(ID{Data}, Val(:type))
    n = length(di_ids)
    sizehint!(data.pairs, n)
    sizehint!(data_text.pairs, n); sizehint!(data_type.pairs, n)
    for i in 1:n
        did_raw = di_ids[i]; ismissing(did_raw) && continue
        did = ID{Data}(did_raw)
        mid = di_movie[i]
        ismissing(mid) || push!(data.pairs, ID{Movie}(mid) => did)
        dt = di_type[i]
        ismissing(dt) || push!(data_type.pairs, did => ID{InfoType}(dt))
        dx = di_data[i]
        ismissing(dx) || push!(data_text.pairs, did => dx)
    end
    println("  movie_info_idx: $(length(data.pairs)) movie-data pairs ($(round(time()-t; digits=1))s)")

    # ---- link_type + movie_link → MovieLink ----
    t = time()
    df = load_df("link_type")
    _load!(Prela.lookup_field(ID{LinkType}, Val(:link)),
           col(df, 0), col(df, 1), ID{LinkType}, identity)

    ml_df = load_df("movie_link")
    ml_ids       = col(ml_df, 0)
    ml_movie     = col(ml_df, 1)
    ml_linked    = col(ml_df, 2)
    ml_linktype  = col(ml_df, 3)

    mlink_type   = Prela.lookup_field(ID{MovieLink}, Val(:type))
    mlink_target = Prela.lookup_field(ID{MovieLink}, Val(:target))
    n = length(ml_ids)
    sizehint!(link.pairs, n); sizehint!(linked_by.pairs, n)
    sizehint!(mlink_type.pairs, n); sizehint!(mlink_target.pairs, n)
    for i in 1:n
        mlid_raw = ml_ids[i]; ismissing(mlid_raw) && continue
        mlid = ID{MovieLink}(mlid_raw)
        src = ml_movie[i]; tgt = ml_linked[i]
        ismissing(src) || push!(link.pairs, ID{Movie}(src) => mlid)
        ismissing(tgt) || push!(linked_by.pairs, ID{Movie}(tgt) => mlid)
        ismissing(tgt) || push!(mlink_target.pairs, mlid => ID{Movie}(tgt))
        lt = ml_linktype[i]
        ismissing(lt) || push!(mlink_type.pairs, mlid => ID{LinkType}(lt))
    end
    println("  movie_link: $(length(link.pairs)) outgoing, $(length(linked_by.pairs)) incoming ($(round(time()-t; digits=1))s)")

    # ---- aka_title ----
    t = time()
    df = load_df("aka_title")
    aka_title = Prela.lookup_field(ID{AkaTitle}, Val(:title))
    _load!(aka, col(df, 1), col(df, 0), ID{Movie}, ID{AkaTitle})
    _load!(aka_title, col(df, 0), col(df, 2), ID{AkaTitle}, identity)
    println("  aka_title: $(length(aka.pairs)) movie-aka pairs ($(round(time()-t; digits=1))s)")

    println("  TOTAL: $(round(time() - t_total; digits=1))s")
end

println("Loading JOB tables from parquet...")
load_all!()

# Universe of movies = the keys of the title relation.
const movie = Unary{ID{Movie}}(unique(p.first for p in title.pairs))
println("Universe: $(length(movie.values)) movies")
