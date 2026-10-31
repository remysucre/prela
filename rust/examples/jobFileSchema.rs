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
// is a MIN over sets of values, and nothing here rebuilds an in-memory db
// from the sinks (there is no `to_memory`).
//
// Ids are the raw file ids (regen's cache shifts them to 0-based; the loader
// does not), and JOB's answers contain no ids, so nothing needs shifting.

use prela::engine::*;
use prela::loader::Str;

use crate::file_schema::Catalog;
use crate::trial_loader::ColumnRef;

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
    JobFiles {
        movie: Movie {
            id: c.key("title", "id"),
            title: c.col("title", "id", "title"),
            kind: Compose {
                a: c.col("title", "id", "kind_id"),
                b: c.col("kind_type", "id", "kind"),
            },
            production_year: c.col("title", "id", "production_year"),
            episode_nr: c.col("title", "id", "episode_nr"),
            keyword: Compose {
                a: c.col("movie_keyword", "movie_id", "keyword_id"),
                b: c.col("keyword", "id", "keyword"),
            },
            company: c.col("movie_companies", "movie_id", "id"), // reverse FK
            cast: c.col("cast_info", "movie_id", "id"), // reverse FK
            info: c.col("movie_info", "movie_id", "id"), // reverse FK
            data: c.col("movie_info_idx", "movie_id", "id"), // reverse FK
            complete_cast: c.col("complete_cast", "movie_id", "id"), // reverse FK
            link: c.col("movie_link", "movie_id", "id"), // reverse FK
            linked_by: c.col("movie_link", "linked_movie_id", "id"), // reverse FK
            aka: c.col("aka_title", "movie_id", "title"),
        },
        cast: Cast {
            id: c.key("cast_info", "id"),
            person: c.col("cast_info", "id", "person_id"),
            role: Compose {
                a: c.col("cast_info", "id", "role_id"),
                b: c.col("role_type", "id", "role"),
            },
            note: c.col("cast_info", "id", "note"),
            character: Compose {
                a: c.col("cast_info", "id", "person_role_id"),
                b: c.col("char_name", "id", "name"),
            },
        },
        person: Person {
            id: c.key("name", "id"),
            name: c.col("name", "id", "name"),
            gender: c.col("name", "id", "gender"),
            alias: c.col("aka_name", "person_id", "name"),
            bio: c.col("person_info", "person_id", "id"), // reverse FK
            name_pcode_cf: c.col("name", "id", "name_pcode_cf"),
        },
        company: Company {
            id: c.key("movie_companies", "id"),
            name: Compose {
                a: c.col("movie_companies", "id", "company_id"),
                b: c.col("company_name", "id", "name"),
            },
            country: Compose {
                a: c.col("movie_companies", "id", "company_id"),
                b: c.col("company_name", "id", "country_code"),
            },
            note: c.col("movie_companies", "id", "note"),
            ty: Compose {
                a: c.col("movie_companies", "id", "company_type_id"),
                b: c.col("company_type", "id", "kind"),
            },
        },
        info: Info {
            id: c.key("movie_info", "id"),
            info: c.col("movie_info", "id", "info"),
            ty: Compose {
                a: c.col("movie_info", "id", "info_type_id"),
                b: c.col("info_type", "id", "info"),
            },
            note: c.col("movie_info", "id", "note"),
        },
        data: Data {
            id: c.key("movie_info_idx", "id"),
            text: c.col("movie_info_idx", "id", "info"),
            ty: Compose {
                a: c.col("movie_info_idx", "id", "info_type_id"),
                b: c.col("info_type", "id", "info"),
            },
        },
        person_info: PersonInfo {
            id: c.key("person_info", "id"),
            info: c.col("person_info", "id", "info"),
            ty: Compose {
                a: c.col("person_info", "id", "info_type_id"),
                b: c.col("info_type", "id", "info"),
            },
            note: c.col("person_info", "id", "note"),
        },
        movie_link: MovieLink {
            id: c.key("movie_link", "id"),
            target: c.col("movie_link", "id", "linked_movie_id"),
            ty: Compose {
                a: c.col("movie_link", "id", "link_type_id"),
                b: c.col("link_type", "id", "link"),
            },
        },
        complete_cast: CompleteCast {
            id: c.key("complete_cast", "id"),
            status: Compose {
                a: c.col("complete_cast", "id", "status_id"),
                b: c.col("comp_cast_type", "id", "kind"),
            },
            subject: Compose {
                a: c.col("complete_cast", "id", "subject_id"),
                b: c.col("comp_cast_type", "id", "kind"),
            },
        },
    }
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
