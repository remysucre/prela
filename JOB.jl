# One-shot JOB data loader. include("JOB.jl") once — afterwards every JOB
# entity is declared, every relation is populated from `../jobdata/parquet/`,
# and Movie's fields are bare-name accessible (`title`, `keyword`, ...).

include("Prela.jl")
using .Prela
using Parquet2, DataFrames

# === Forward-declare every entity so cyclic refs work ===

@declare Movie Keyword Kind LinkType RoleType CompCastType InfoType CompanyType
@declare Character AkaName AkaTitle Person Company Info Data MovieLink Cast CompleteCast
@declare PersonInfo

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
    aka            :: ID{AkaName}
    info           :: ID{PersonInfo}
end

@entity PersonInfo begin
    info :: String
    note :: String
    type :: ID{InfoType}
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
    movie     :: ID{Movie}
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

    # ---- name (Person) ----
    t = time()
    df = load_df("name")
    _load!(Prela.lookup_field(ID{Person}, Val(:name)),
           col(df, 0), col(df, 1), ID{Person}, identity)
    _load!(Prela.lookup_field(ID{Person}, Val(:gender)),
           col(df, 0), col(df, 4), ID{Person}, identity)
    _load!(Prela.lookup_field(ID{Person}, Val(:name_pcode_cf)),
           col(df, 0), col(df, 5), ID{Person}, identity)
    println("  name (Person): $(length(Prela.lookup_field(ID{Person}, Val(:name)).pairs)) persons ($(round(time()-t; digits=1))s)")

    # ---- char_name (Character) ----
    t = time()
    df = load_df("char_name")
    _load!(Prela.lookup_field(ID{Character}, Val(:name)),
           col(df, 0), col(df, 1), ID{Character}, identity)
    println("  char_name (Character): $(length(Prela.lookup_field(ID{Character}, Val(:name)).pairs)) characters ($(round(time()-t; digits=1))s)")

    # ---- role_type ----
    t = time()
    df = load_df("role_type")
    _load!(Prela.lookup_field(ID{RoleType}, Val(:role)),
           col(df, 0), col(df, 1), ID{RoleType}, identity)
    println("  role_type: $(length(Prela.lookup_field(ID{RoleType}, Val(:role)).pairs)) roles ($(round(time()-t; digits=1))s)")

    # ---- aka_name ----
    # Columns: id, person_id, name, ... → AkaName.name + Person.aka link.
    t = time()
    df = load_df("aka_name")
    _load!(Prela.lookup_field(ID{AkaName}, Val(:name)),
           col(df, 0), col(df, 2), ID{AkaName}, identity)
    _load!(Prela.lookup_field(ID{Person}, Val(:aka)),
           col(df, 1), col(df, 0), ID{Person}, ID{AkaName})
    println("  aka_name: $(length(Prela.lookup_field(ID{AkaName}, Val(:name)).pairs)) aka names ($(round(time()-t; digits=1))s)")

    # ---- comp_cast_type ----
    t = time()
    df = load_df("comp_cast_type")
    _load!(Prela.lookup_field(ID{CompCastType}, Val(:kind)),
           col(df, 0), col(df, 1), ID{CompCastType}, identity)
    println("  comp_cast_type: $(length(Prela.lookup_field(ID{CompCastType}, Val(:kind)).pairs)) kinds ($(round(time()-t; digits=1))s)")

    # ---- complete_cast → CompleteCast ----
    # Columns: id, movie_id, subject_id, status_id.
    t = time()
    cc_df = load_df("complete_cast")
    cc_ids     = col(cc_df, 0)
    cc_movie   = col(cc_df, 1)
    cc_subject = col(cc_df, 2)
    cc_status  = col(cc_df, 3)
    cc_subj_rel = Prela.lookup_field(ID{CompleteCast}, Val(:subject))
    cc_stat_rel = Prela.lookup_field(ID{CompleteCast}, Val(:status))
    n = length(cc_ids)
    sizehint!(complete_cast.pairs, n)
    sizehint!(cc_subj_rel.pairs, n); sizehint!(cc_stat_rel.pairs, n)
    for i in 1:n
        ccid_raw = cc_ids[i]; ismissing(ccid_raw) && continue
        ccid = ID{CompleteCast}(ccid_raw)
        mid = cc_movie[i]
        ismissing(mid) || push!(complete_cast.pairs, ID{Movie}(mid) => ccid)
        sj = cc_subject[i]
        ismissing(sj) || push!(cc_subj_rel.pairs, ccid => ID{CompCastType}(sj))
        st = cc_status[i]
        ismissing(st) || push!(cc_stat_rel.pairs, ccid => ID{CompCastType}(st))
    end
    println("  complete_cast: $(length(complete_cast.pairs)) movie-completecast pairs ($(round(time()-t; digits=1))s)")

    # ---- person_info → PersonInfo ----
    # Columns: id, person_id, info_type_id, info, note.
    t = time()
    pi_df = load_df("person_info")
    pi_ids   = col(pi_df, 0)
    pi_person= col(pi_df, 1)
    pi_type  = col(pi_df, 2)
    pi_info  = col(pi_df, 3)
    pi_note  = col(pi_df, 4)
    person_aka_info = Prela.lookup_field(ID{Person}, Val(:info))
    pinfo_info = Prela.lookup_field(ID{PersonInfo}, Val(:info))
    pinfo_note = Prela.lookup_field(ID{PersonInfo}, Val(:note))
    pinfo_type = Prela.lookup_field(ID{PersonInfo}, Val(:type))
    n = length(pi_ids)
    sizehint!(person_aka_info.pairs, n)
    sizehint!(pinfo_info.pairs, n); sizehint!(pinfo_note.pairs, n); sizehint!(pinfo_type.pairs, n)
    for i in 1:n
        piid_raw = pi_ids[i]; ismissing(piid_raw) && continue
        piid = ID{PersonInfo}(piid_raw)
        pid = pi_person[i]
        ismissing(pid) || push!(person_aka_info.pairs, ID{Person}(pid) => piid)
        ty = pi_type[i]
        ismissing(ty) || push!(pinfo_type.pairs, piid => ID{InfoType}(ty))
        inf = pi_info[i]
        ismissing(inf) || push!(pinfo_info.pairs, piid => inf)
        nt = pi_note[i]
        ismissing(nt) || push!(pinfo_note.pairs, piid => nt)
    end
    println("  person_info: $(length(pinfo_info.pairs)) person-info pairs ($(round(time()-t; digits=1))s)")

    # ---- cast_info (Cast) — the big one (~37M rows) ----
    # Columns: id, person_id, movie_id, person_role_id, note, nr_order, role_id.
    t = time()
    ci_df = load_df("cast_info")
    ci_ids     = col(ci_df, 0)
    ci_person  = col(ci_df, 1)
    ci_movie   = col(ci_df, 2)
    ci_charrole= col(ci_df, 3)
    ci_note    = col(ci_df, 4)
    ci_role    = col(ci_df, 6)

    cast_movie     = Prela.lookup_field(ID{Cast}, Val(:movie))
    cast_person    = Prela.lookup_field(ID{Cast}, Val(:person))
    cast_character = Prela.lookup_field(ID{Cast}, Val(:character))
    cast_role      = Prela.lookup_field(ID{Cast}, Val(:role))
    cast_note      = Prela.lookup_field(ID{Cast}, Val(:note))

    n = length(ci_ids)
    sizehint!(cast_movie.pairs, n)
    sizehint!(cast_person.pairs, n)
    sizehint!(cast_character.pairs, n)
    sizehint!(cast_role.pairs, n)
    sizehint!(cast_note.pairs, n)

    for i in 1:n
        cid_raw = ci_ids[i]
        ismissing(cid_raw) && continue
        cid = ID{Cast}(cid_raw)
        m  = ci_movie[i];   ismissing(m)  || push!(cast_movie.pairs,     cid => ID{Movie}(m))
        p  = ci_person[i];  ismissing(p)  || push!(cast_person.pairs,    cid => ID{Person}(p))
        c  = ci_charrole[i];ismissing(c)  || push!(cast_character.pairs, cid => ID{Character}(c))
        r  = ci_role[i];    ismissing(r)  || push!(cast_role.pairs,      cid => ID{RoleType}(r))
        nt = ci_note[i];    ismissing(nt) || push!(cast_note.pairs,      cid => nt)
    end
    println("  cast_info: $(length(cast_movie.pairs)) cast→movie, $(length(cast_person.pairs)) cast→person, $(length(cast_note.pairs)) notes ($(round(time()-t; digits=1))s)")

    println("  TOTAL: $(round(time() - t_total; digits=1))s")
end

include("cache.jl")

if isdir(CACHE_DIR) && load_cache!()
    println("Loaded JOB tables from cache (binary + mmap).")
else
    println("Cache miss; loading JOB tables from parquet...")
    load_all!()
    println("Saving cache for next time...")
    save_cache!()
end

# Universe of movies = the keys of the title relation.
const movie = Unary{ID{Movie}}(unique(p.first for p in title.pairs))
println("Universe: $(length(movie.values)) movies")

# Universe of casts. Parallels `movie` for Movie. Use `cast → ...` to root a
# cast-side query; `Cast.movie`/`Cast.person`/etc. when qualification is needed.
const cast = Unary{ID{Cast}}(unique(p.first for p in _Cast_movie.pairs))
println("Cast universe: $(length(cast.values)) casts")

# === Promote dense one-to-one leaf rels to VecRel (column-store) ========
# Cache is already saved at this point (always as MapRel). Promotion only
# changes the in-memory representation; the next session reloads MapRel from
# cache, then promotes again.
let
    n_cast  = length(cast.values)
    n_movie = length(movie.values)
    function _promote!(qual_sym::Symbol, n::Int)
        old = getfield(@__MODULE__, qual_sym)
        old isa Prela.VecRel && return  # already promoted
        try
            new = Prela.vectorize(old, n)
            Core.eval(@__MODULE__, :(const $qual_sym = $new))
            push!(Prela._LEAF_RELS, new)
        catch e
            @info "skip promote $qual_sym: $(sprint(showerror, e))"
        end
    end
    _promote!(:_Cast_movie,   n_cast)
    _promote!(:_Cast_person,  n_cast)
    _promote!(:_Cast_role,    n_cast)
    _promote!(:_Movie_title,  n_movie)
    _promote!(:_Movie_kind,   n_movie)
end

# Bare-expose Cast's non-conflicting field rels for navigation. `movie` clashes
# with the Movie universe above, so callers use `Cast.movie` for that FK.
# Bindings point at the (possibly promoted) leaf rels.
const note      = _Cast_note
const role      = _Cast_role
const person    = _Cast_person
const character = _Cast_character

# Re-bind Movie's bare-exposed fields whose backing rel was promoted. The
# original `@expose Movie` captured the MapRel object by reference; const
# redefinition swaps in the VecRel-backed binding so bare `title`/`kind`
# get the fast path.
const title = _Movie_title
const kind  = _Movie_kind
