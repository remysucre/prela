// The schema of data/imdb/parquet (the JOB benchmark's IMDB tables) as
// ColumnRefs over the files: what job_queries/queries_loaded.rs is written
// over and `.load()`s. The TPC-H counterpart is fileSchema.rs, whose Catalog
// this file reuses.
//
// The entities and field names are job_schema.rs's, so that queries.rs's 113
// queries read the same over the files. Most fields are one file column, as
// in fileSchema.rs (`Movie.title` is title.parquet's `id -> title`).
//
// Lookups: job_schema has string columns that no single file holds. regen
// built them by joining a code column to its lookup table: `Movie.kind` is
// title's `kind_id`, looked up in kind_type. Here those fields are the two
// ColumnRefs composed (`Lookup`), which `.load()` reads as any other Compose:
//
//     Movie.kind = title (id -> kind_id)  then  kind_type (id -> kind)
//
// Schema rule (fileSchema.rs, section 6): every column has its file's primary
// key on one side, so a loaded (key, value) pair is one file row. Three fields
// break it, to match how regen built them: `Movie.keyword` (movie_keyword's
// `movie_id -> keyword_id`), `Movie.aka` (aka_title's `movie_id -> title`) and
// `Person.alias` (aka_name's `person_id -> name`). Two rows can then give the
// same pair, and `union` keeps it once. That cannot change a JOB answer, which
// is a MIN over sets of values. `to_memory` (below) rebuilds an in-memory db
// from the sinks, for plans that load first and then query in memory
// (job_queries/queries_loaded_optimized.rs).
//
// Ids are the raw file ids (regen's cache shifts them to 0-based; the loader
// does not), and JOB's answers contain no ids, so nothing needs shifting.

use prela::engine::*;
use prela::loader::{Set, SparseKey, Str};

use crate::file_schema::Catalog;
use crate::trial_loader::{ColumnRef, RowIndex};

/// A code column looked up in its lookup table: `E -> L` in one file, then
/// `L -> T` in L's file.
pub type Lookup<E, L, T> = Compose<ColumnRef<E, Id<L>>, ColumnRef<L, T>>;

// ===== Lookup tables: entity tags only ==========================================
//
// Their rows are only ever reached through a `Lookup`, so they need no struct
// of columns, just a tag for the ids (`Id<KindType>`).

pub struct KindType; // kind_type.parquet
pub struct Keyword; // keyword.parquet
pub struct RoleType; // role_type.parquet
pub struct CharName; // char_name.parquet
pub struct CompanyName; // company_name.parquet
pub struct CompanyType; // company_type.parquet
pub struct InfoType; // info_type.parquet
pub struct LinkType; // link_type.parquet
pub struct CompCastType; // comp_cast_type.parquet

// ===== Entities ==================================================================

#[derive(IntoQuery)]
pub struct Movie {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub title: ColumnRef<Self, Str>,
    pub kind: Lookup<Self, KindType, Str>,
    pub production_year: ColumnRef<Self, i64>,
    pub episode_nr: ColumnRef<Self, i64>,
    pub keyword: Lookup<Self, Keyword, Str>, // breaks the schema rule (see top)
    pub company: ColumnRef<Self, Id<Company>>, // reverse FK: read from movie_companies.parquet
    pub cast: ColumnRef<Self, Id<Cast>>, // reverse FK: read from cast_info.parquet
    pub info: ColumnRef<Self, Id<Info>>, // reverse FK: read from movie_info.parquet
    pub data: ColumnRef<Self, Id<Data>>, // reverse FK: read from movie_info_idx.parquet
    pub complete_cast: ColumnRef<Self, Id<CompleteCast>>, // reverse FK: read from complete_cast.parquet
    pub link: ColumnRef<Self, Id<MovieLink>>, // reverse FK: links from this movie
    pub linked_by: ColumnRef<Self, Id<MovieLink>>, // reverse FK: links to this movie
    pub aka: ColumnRef<Self, Str>, // breaks the schema rule (see top)
}

/// A cast_info row: one person in one movie.
#[derive(IntoQuery)]
pub struct Cast {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub person: ColumnRef<Self, Id<Person>>,
    pub role: Lookup<Self, RoleType, Str>,
    pub note: ColumnRef<Self, Str>,
    pub character: Lookup<Self, CharName, Str>,
}

/// A name.parquet row.
#[derive(IntoQuery)]
pub struct Person {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: ColumnRef<Self, Str>,
    pub gender: ColumnRef<Self, Str>,
    pub alias: ColumnRef<Self, Str>, // breaks the schema rule (see top)
    pub bio: ColumnRef<Self, Id<PersonInfo>>, // reverse FK: read from person_info.parquet
    pub name_pcode_cf: ColumnRef<Self, Str>,
}

/// A movie_companies row: one company's part in one movie. Its name and
/// country are the company_name row it points to.
#[derive(IntoQuery)]
pub struct Company {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub name: Lookup<Self, CompanyName, Str>,
    pub country: Lookup<Self, CompanyName, Str>,
    pub note: ColumnRef<Self, Str>,
    pub ty: Lookup<Self, CompanyType, Str>,
}

/// A movie_info row.
#[derive(IntoQuery)]
pub struct Info {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub info: ColumnRef<Self, Str>,
    pub ty: Lookup<Self, InfoType, Str>,
    pub note: ColumnRef<Self, Str>,
}

/// A movie_info_idx row (ratings, votes, top/bottom ranks).
#[derive(IntoQuery)]
pub struct Data {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub text: ColumnRef<Self, Str>,
    pub ty: Lookup<Self, InfoType, Str>,
}

/// A person_info row.
#[derive(IntoQuery)]
pub struct PersonInfo {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub info: ColumnRef<Self, Str>,
    pub ty: Lookup<Self, InfoType, Str>,
    pub note: ColumnRef<Self, Str>,
}

/// A movie_link row: `movie_id` links to `linked_movie_id` (the target).
#[derive(IntoQuery)]
pub struct MovieLink {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub target: ColumnRef<Self, Id<Movie>>,
    pub ty: Lookup<Self, LinkType, Str>,
}

/// A complete_cast row.
#[derive(IntoQuery)]
pub struct CompleteCast {
    #[primary_key]
    pub id: ColumnRef<Self, Id<Self>>,
    pub status: Lookup<Self, CompCastType, Str>,
    pub subject: Lookup<Self, CompCastType, Str>,
}

pub struct JobFiles {
    pub movie: Movie,
    pub cast: Cast,
    pub person: Person,
    pub company: Company,
    pub info: Info,
    pub data: Data,
    pub person_info: PersonInfo,
    pub movie_link: MovieLink,
    pub complete_cast: CompleteCast,
}

pub fn job(c: &mut Catalog) -> JobFiles {

    let title = RowIndex::new(c.path("title"));
    let name = RowIndex::new(c.path("name"));
    let char_name = RowIndex::new(c.path("char_name"));
    let company_name = RowIndex::new(c.path("company_name"));
    let keyword = RowIndex::new(c.path("keyword"));
    let kind_type = RowIndex::new(c.path("kind_type"));
    let role_type = RowIndex::new(c.path("role_type"));
    let company_type = RowIndex::new(c.path("company_type"));
    let info_type = RowIndex::new(c.path("info_type"));
    let link_type = RowIndex::new(c.path("link_type"));
    let comp_cast_type = RowIndex::new(c.path("comp_cast_type"));

    JobFiles {
        movie: Movie {
            id: c.key("title", "id"),
            title: c.col("title", "id", "title"),
            kind: Compose {
                a: c.col("title", "id", "kind_id").val_via(kind_type),
                b: c.col("kind_type", "id", "kind"),
            },
            production_year: c.col("title", "id", "production_year"),
            episode_nr: c.col("title", "id", "episode_nr"),
            keyword: Compose {
                a: c.col("movie_keyword", "movie_id", "keyword_id").key_via(title).val_via(keyword),
                b: c.col("keyword", "id", "keyword"),
            },
            company: c.col("movie_companies", "movie_id", "id").key_via(title), // reverse FK
            cast: c.col("cast_info", "movie_id", "id").key_via(title), // reverse FK
            info: c.col("movie_info", "movie_id", "id").key_via(title), // reverse FK
            data: c.col("movie_info_idx", "movie_id", "id").key_via(title), // reverse FK
            complete_cast: c.col("complete_cast", "movie_id", "id").key_via(title), // reverse FK
            link: c.col("movie_link", "movie_id", "id").key_via(title), // reverse FK
            linked_by: c.col("movie_link", "linked_movie_id", "id").key_via(title), // reverse FK
            aka: c.col("aka_title", "movie_id", "title").key_via(title),
        },
        cast: Cast {
            id: c.key("cast_info", "id"),
            person: c.col("cast_info", "id", "person_id").val_via(name),
            role: Compose {
                a: c.col("cast_info", "id", "role_id").val_via(role_type),
                b: c.col("role_type", "id", "role"),
            },
            note: c.col("cast_info", "id", "note"),
            character: Compose {
                a: c.col("cast_info", "id", "person_role_id").val_via(char_name),
                b: c.col("char_name", "id", "name"),
            },
        },
        person: Person {
            id: c.key("name", "id"),
            name: c.col("name", "id", "name"),
            gender: c.col("name", "id", "gender"),
            alias: c.col("aka_name", "person_id", "name").key_via(name),
            bio: c.col("person_info", "person_id", "id").key_via(name), // reverse FK
            name_pcode_cf: c.col("name", "id", "name_pcode_cf"),
        },
        company: Company {
            id: c.key("movie_companies", "id"),
            name: Compose {
                a: c.col("movie_companies", "id", "company_id").val_via(company_name),
                b: c.col("company_name", "id", "name"),
            },
            country: Compose {
                a: c.col("movie_companies", "id", "company_id").val_via(company_name),
                b: c.col("company_name", "id", "country_code"),
            },
            note: c.col("movie_companies", "id", "note"),
            ty: Compose {
                a: c.col("movie_companies", "id", "company_type_id").val_via(company_type),
                b: c.col("company_type", "id", "kind"),
            },
        },
        info: Info {
            id: c.key("movie_info", "id"),
            info: c.col("movie_info", "id", "info"),
            ty: Compose {
                a: c.col("movie_info", "id", "info_type_id").val_via(info_type),
                b: c.col("info_type", "id", "info"),
            },
            note: c.col("movie_info", "id", "note"),
        },
        data: Data {
            id: c.key("movie_info_idx", "id"),
            text: c.col("movie_info_idx", "id", "info"),
            ty: Compose {
                a: c.col("movie_info_idx", "id", "info_type_id").val_via(info_type),
                b: c.col("info_type", "id", "info"),
            },
        },
        person_info: PersonInfo {
            id: c.key("person_info", "id"),
            info: c.col("person_info", "id", "info"),
            ty: Compose {
                a: c.col("person_info", "id", "info_type_id").val_via(info_type),
                b: c.col("info_type", "id", "info"),
            },
            note: c.col("person_info", "id", "note"),
        },
        movie_link: MovieLink {
            id: c.key("movie_link", "id"),
            target: c.col("movie_link", "id", "linked_movie_id").val_via(title),
            ty: Compose {
                a: c.col("movie_link", "id", "link_type_id").val_via(link_type),
                b: c.col("link_type", "id", "link"),
            },
        },
        complete_cast: CompleteCast {
            id: c.key("complete_cast", "id"),
            status: Compose {
                a: c.col("complete_cast", "id", "status_id").val_via(comp_cast_type),
                b: c.col("comp_cast_type", "id", "kind"),
            },
            subject: Compose {
                a: c.col("complete_cast", "id", "subject_id").val_via(comp_cast_type),
                b: c.col("comp_cast_type", "id", "kind"),
            },
        },
    }
}

// ===== In memory: what the sinks become (to_memory) ==============================
//
// As fileSchema.rs's Mem structs: one struct per entity, the same field names,
// so a query over the loaded data reads the same. Entity tags are written out
// (`Movie`, not `Self`), so the ids are the `Id<Movie>` the ColumnRefs load.
// A Lookup stays two relations composed.

pub type MemLookup<E, L, T> = Compose<Set<E, Id<L>>, Set<L, T>>;

#[derive(IntoQuery)]
pub struct MemMovie {
    #[primary_key]
    pub id: SparseKey<Movie>,
    pub title: Set<Movie, Str>,
    pub kind: MemLookup<Movie, KindType, Str>,
    pub production_year: Set<Movie, i64>,
    pub episode_nr: Set<Movie, i64>,
    pub keyword: MemLookup<Movie, Keyword, Str>,
    pub company: Set<Movie, Id<Company>>,
    pub cast: Set<Movie, Id<Cast>>,
    pub info: Set<Movie, Id<Info>>,
    pub data: Set<Movie, Id<Data>>,
    pub complete_cast: Set<Movie, Id<CompleteCast>>,
    pub link: Set<Movie, Id<MovieLink>>,
    pub linked_by: Set<Movie, Id<MovieLink>>,
    pub aka: Set<Movie, Str>,
}

#[derive(IntoQuery)]
pub struct MemCast {
    #[primary_key]
    pub id: SparseKey<Cast>,
    pub person: Set<Cast, Id<Person>>,
    pub role: MemLookup<Cast, RoleType, Str>,
    pub note: Set<Cast, Str>,
    pub character: MemLookup<Cast, CharName, Str>,
}

#[derive(IntoQuery)]
pub struct MemPerson {
    #[primary_key]
    pub id: SparseKey<Person>,
    pub name: Set<Person, Str>,
    pub gender: Set<Person, Str>,
    pub alias: Set<Person, Str>,
    pub bio: Set<Person, Id<PersonInfo>>,
    pub name_pcode_cf: Set<Person, Str>,
}

#[derive(IntoQuery)]
pub struct MemCompany {
    #[primary_key]
    pub id: SparseKey<Company>,
    pub name: MemLookup<Company, CompanyName, Str>,
    pub country: MemLookup<Company, CompanyName, Str>,
    pub note: Set<Company, Str>,
    pub ty: MemLookup<Company, CompanyType, Str>,
}

#[derive(IntoQuery)]
pub struct MemInfo {
    #[primary_key]
    pub id: SparseKey<Info>,
    pub info: Set<Info, Str>,
    pub ty: MemLookup<Info, InfoType, Str>,
    pub note: Set<Info, Str>,
}

#[derive(IntoQuery)]
pub struct MemData {
    #[primary_key]
    pub id: SparseKey<Data>,
    pub text: Set<Data, Str>,
    pub ty: MemLookup<Data, InfoType, Str>,
}

#[derive(IntoQuery)]
pub struct MemPersonInfo {
    #[primary_key]
    pub id: SparseKey<PersonInfo>,
    pub info: Set<PersonInfo, Str>,
    pub ty: MemLookup<PersonInfo, InfoType, Str>,
    pub note: Set<PersonInfo, Str>,
}

#[derive(IntoQuery)]
pub struct MemMovieLink {
    #[primary_key]
    pub id: SparseKey<MovieLink>,
    pub target: Set<MovieLink, Id<Movie>>,
    pub ty: MemLookup<MovieLink, LinkType, Str>,
}

#[derive(IntoQuery)]
pub struct MemCompleteCast {
    #[primary_key]
    pub id: SparseKey<CompleteCast>,
    pub status: MemLookup<CompleteCast, CompCastType, Str>,
    pub subject: MemLookup<CompleteCast, CompCastType, Str>,
}

pub struct MemJob {
    pub movie: MemMovie,
    pub cast: MemCast,
    pub person: MemPerson,
    pub company: MemCompany,
    pub info: MemInfo,
    pub data: MemData,
    pub person_info: MemPersonInfo,
    pub movie_link: MemMovieLink,
    pub complete_cast: MemCompleteCast,
}

impl JobFiles {
    /// The loaded columns as an in-memory db. An entity's `id` is exactly its
    /// id column's loaded keys (not the union of every column's keys), so a
    /// plan driving `mem.movie` drives only the movies the load kept.
    /// Unloaded columns come back empty (quietly: a JOB plan loads a handful
    /// of the ~60 columns). The sinks are left as they are; `clear()` resets them.
    pub fn to_memory(&self) -> MemJob {
        let (m, c, p) = (&self.movie, &self.cast, &self.person);
        let (co, i, d) = (&self.company, &self.info, &self.data);
        let (pi, ml, cc) = (&self.person_info, &self.movie_link, &self.complete_cast);
        MemJob {
            movie: MemMovie {
                id: mem_key(&m.id.keys()),
                title: mem(&m.title),
                kind: mem_lookup(&m.kind),
                production_year: mem(&m.production_year),
                episode_nr: mem(&m.episode_nr),
                keyword: mem_lookup(&m.keyword),
                company: mem(&m.company),
                cast: mem(&m.cast),
                info: mem(&m.info),
                data: mem(&m.data),
                complete_cast: mem(&m.complete_cast),
                link: mem(&m.link),
                linked_by: mem(&m.linked_by),
                aka: mem(&m.aka),
            },
            cast: MemCast {
                id: mem_key(&c.id.keys()),
                person: mem(&c.person),
                role: mem_lookup(&c.role),
                note: mem(&c.note),
                character: mem_lookup(&c.character),
            },
            person: MemPerson {
                id: mem_key(&p.id.keys()),
                name: mem(&p.name),
                gender: mem(&p.gender),
                alias: mem(&p.alias),
                bio: mem(&p.bio),
                name_pcode_cf: mem(&p.name_pcode_cf),
            },
            company: MemCompany {
                id: mem_key(&co.id.keys()),
                name: mem_lookup(&co.name),
                country: mem_lookup(&co.country),
                note: mem(&co.note),
                ty: mem_lookup(&co.ty),
            },
            info: MemInfo {
                id: mem_key(&i.id.keys()),
                info: mem(&i.info),
                ty: mem_lookup(&i.ty),
                note: mem(&i.note),
            },
            data: MemData { id: mem_key(&d.id.keys()), text: mem(&d.text), ty: mem_lookup(&d.ty) },
            person_info: MemPersonInfo {
                id: mem_key(&pi.id.keys()),
                info: mem(&pi.info),
                ty: mem_lookup(&pi.ty),
                note: mem(&pi.note),
            },
            movie_link: MemMovieLink {
                id: mem_key(&ml.id.keys()),
                target: mem(&ml.target),
                ty: mem_lookup(&ml.ty),
            },
            complete_cast: MemCompleteCast {
                id: mem_key(&cc.id.keys()),
                status: mem_lookup(&cc.status),
                subject: mem_lookup(&cc.subject),
            },
        }
    }
}

// A column's sink as a relation: a view of the loaded rows (no copy), or
// empty if the column wasn't loaded. (`ColumnRef::loaded` without its warning.)
fn mem<E: 'static, T: Copy + 'static>(c: &ColumnRef<E, T>) -> Set<E, T> {
    match &*c.sink.borrow() {
        Some(rel) => MultiRel::from_csr(rel.offsets, rel.v),
        None => MultiRel::from_pairs(0, []),
    }
}

fn mem_lookup<E: 'static, L: 'static, T: Copy + 'static>(l: &Lookup<E, L, T>) -> MemLookup<E, L, T> {
    Compose { a: mem(&l.a), b: mem(&l.b) }
}

// The ids of E present in memory, as a SparseKey over 0..=max id with only
// these ids valid (as fileSchema.rs's `mem_key`). The Bitset is leaked because
// SparseUniverse holds a `&'static`.
fn mem_key<E: 'static>(ids: &[usize]) -> SparseKey<E> {
    let n = ids.iter().max().map_or(0, |&m| m + 1);
    let mut valid: Bitset<Id<E>> = Bitset::empty(Universe::<Id<E>>::new(n));
    for &i in ids {
        valid.set(Id::new(i));
    }
    SparseUniverse::new(n, Box::leak(Box::new(valid)))
}

// ===== clear =====================================================================

impl JobFiles {
    /// Forget every column's loaded rows. The JOB runner calls this after each
    /// query, so one query's sinks don't carry over into the next.
    pub fn clear(&self) {
        let Movie { id, title, kind, production_year, episode_nr, keyword, company, cast,
                    info, data, complete_cast, link, linked_by, aka } = &self.movie;
        id.clear();
        title.clear();
        kind.a.clear();
        kind.b.clear();
        production_year.clear();
        episode_nr.clear();
        keyword.a.clear();
        keyword.b.clear();
        company.clear();
        cast.clear();
        info.clear();
        data.clear();
        complete_cast.clear();
        link.clear();
        linked_by.clear();
        aka.clear();

        let Cast { id, person, role, note, character } = &self.cast;
        id.clear();
        person.clear();
        role.a.clear();
        role.b.clear();
        note.clear();
        character.a.clear();
        character.b.clear();

        let Person { id, name, gender, alias, bio, name_pcode_cf } = &self.person;
        id.clear();
        name.clear();
        gender.clear();
        alias.clear();
        bio.clear();
        name_pcode_cf.clear();

        let Company { id, name, country, note, ty } = &self.company;
        id.clear();
        name.a.clear();
        name.b.clear();
        country.a.clear();
        country.b.clear();
        note.clear();
        ty.a.clear();
        ty.b.clear();

        let Info { id, info, ty, note } = &self.info;
        id.clear();
        info.clear();
        ty.a.clear();
        ty.b.clear();
        note.clear();

        let Data { id, text, ty } = &self.data;
        id.clear();
        text.clear();
        ty.a.clear();
        ty.b.clear();

        let PersonInfo { id, info, ty, note } = &self.person_info;
        id.clear();
        info.clear();
        ty.a.clear();
        ty.b.clear();
        note.clear();

        let MovieLink { id, target, ty } = &self.movie_link;
        id.clear();
        target.clear();
        ty.a.clear();
        ty.b.clear();

        let CompleteCast { id, status, subject } = &self.complete_cast;
        id.clear();
        status.a.clear();
        status.b.clear();
        subject.a.clear();
        subject.b.clear();
    }
}
