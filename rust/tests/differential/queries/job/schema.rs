//! Canonical normalized IMDB schema used only by JOB differential tests.

use crate::schema::{ColumnId, PrimaryKey, Schema, TableMeta, constraints, entities};
use prela::engine::Id;
use prela::loader::{Col, Str};

entities! {
    pub struct CompCastType {
        pub kind: Col<CompCastType, Str>,
    }

    pub struct CompanyName {
        pub name: Col<CompanyName, Str>,
        pub country_code: Col<CompanyName, Str>,
        pub imdb_id: Col<CompanyName, i64>,
        pub name_pcode_nf: Col<CompanyName, Str>,
        pub name_pcode_sf: Col<CompanyName, Str>,
        pub md5sum: Col<CompanyName, Str>,
    }

    pub struct CompanyType {
        pub kind: Col<CompanyType, Str>,
    }

    pub struct InfoType {
        pub info: Col<InfoType, Str>,
    }

    pub struct Keyword {
        pub keyword: Col<Keyword, Str>,
        pub phonetic_code: Col<Keyword, Str>,
    }

    pub struct KindType {
        pub kind: Col<KindType, Str>,
    }

    pub struct LinkType {
        pub link: Col<LinkType, Str>,
    }

    pub struct Name {
        pub name: Col<Name, Str>,
        pub imdb_index: Col<Name, Str>,
        pub imdb_id: Col<Name, i64>,
        pub gender: Col<Name, Str>,
        pub name_pcode_cf: Col<Name, Str>,
        pub name_pcode_nf: Col<Name, Str>,
        pub surname_pcode: Col<Name, Str>,
        pub md5sum: Col<Name, Str>,
    }

    pub struct RoleType {
        pub role: Col<RoleType, Str>,
    }

    pub struct CharName {
        pub name: Col<CharName, Str>,
        pub imdb_index: Col<CharName, Str>,
        pub imdb_id: Col<CharName, i64>,
        pub name_pcode_nf: Col<CharName, Str>,
        pub surname_pcode: Col<CharName, Str>,
        pub md5sum: Col<CharName, Str>,
    }

    pub struct Title {
        pub title: Col<Title, Str>,
        pub imdb_index: Col<Title, Str>,
        pub kind_id: Col<Title, Id<KindType>>,
        pub production_year: Col<Title, i64>,
        pub imdb_id: Col<Title, i64>,
        pub phonetic_code: Col<Title, Str>,
        // Kept scalar in differential fixtures: DuckDB 1.1 cannot load a
        // generated row which references itself while enforcing this FK.
        pub episode_of_id: Col<Title, i64>,
        pub season_nr: Col<Title, i64>,
        pub episode_nr: Col<Title, i64>,
        pub series_years: Col<Title, Str>,
        pub md5sum: Col<Title, Str>,
    }

    pub struct AkaName {
        pub person_id: Col<AkaName, Id<Name>>,
        pub name: Col<AkaName, Str>,
        pub imdb_index: Col<AkaName, Str>,
        pub name_pcode_cf: Col<AkaName, Str>,
        pub name_pcode_nf: Col<AkaName, Str>,
        pub surname_pcode: Col<AkaName, Str>,
        pub md5sum: Col<AkaName, Str>,
    }

    pub struct AkaTitle {
        pub movie_id: Col<AkaTitle, Id<Title>>,
        pub title: Col<AkaTitle, Str>,
        pub imdb_index: Col<AkaTitle, Str>,
        pub kind_id: Col<AkaTitle, Id<KindType>>,
        pub production_year: Col<AkaTitle, i64>,
        pub phonetic_code: Col<AkaTitle, Str>,
        pub episode_of_id: Col<AkaTitle, i64>,
        pub season_nr: Col<AkaTitle, i64>,
        pub episode_nr: Col<AkaTitle, i64>,
        pub note: Col<AkaTitle, Str>,
        pub md5sum: Col<AkaTitle, Str>,
    }

    pub struct CastInfo {
        pub person_id: Col<CastInfo, Id<Name>>,
        pub movie_id: Col<CastInfo, Id<Title>>,
        pub person_role_id: Col<CastInfo, Id<CharName>>,
        pub note: Col<CastInfo, Str>,
        pub nr_order: Col<CastInfo, i64>,
        pub role_id: Col<CastInfo, Id<RoleType>>,
    }

    pub struct CompleteCast {
        pub movie_id: Col<CompleteCast, Id<Title>>,
        pub subject_id: Col<CompleteCast, Id<CompCastType>>,
        pub status_id: Col<CompleteCast, Id<CompCastType>>,
    }

    pub struct MovieCompanies {
        pub movie_id: Col<MovieCompanies, Id<Title>>,
        pub company_id: Col<MovieCompanies, Id<CompanyName>>,
        pub company_type_id: Col<MovieCompanies, Id<CompanyType>>,
        pub note: Col<MovieCompanies, Str>,
    }

    pub struct MovieInfo {
        pub movie_id: Col<MovieInfo, Id<Title>>,
        pub info_type_id: Col<MovieInfo, Id<InfoType>>,
        pub info: Col<MovieInfo, Str>,
        pub note: Col<MovieInfo, Str>,
    }

    pub struct MovieInfoIdx {
        pub movie_id: Col<MovieInfoIdx, Id<Title>>,
        pub info_type_id: Col<MovieInfoIdx, Id<InfoType>>,
        pub info: Col<MovieInfoIdx, Str>,
        pub note: Col<MovieInfoIdx, Str>,
    }

    pub struct MovieKeyword {
        pub movie_id: Col<MovieKeyword, Id<Title>>,
        pub keyword_id: Col<MovieKeyword, Id<Keyword>>,
    }

    pub struct MovieLink {
        pub movie_id: Col<MovieLink, Id<Title>>,
        pub linked_movie_id: Col<MovieLink, Id<Title>>,
        pub link_type_id: Col<MovieLink, Id<LinkType>>,
    }

    pub struct PersonInfo {
        pub person_id: Col<PersonInfo, Id<Name>>,
        pub info_type_id: Col<PersonInfo, Id<InfoType>>,
        pub info: Col<PersonInfo, Str>,
        pub note: Col<PersonInfo, Str>,
    }
}

macro_rules! ids {
    ($($entity:ident.$field:ident),* $(,)?) => {
        &[$(ColumnId::new(stringify!($entity), stringify!($field))),*]
    };
}

macro_rules! primary_keys {
    ($($entity:ident),* $(,)?) => {
        &[$(PrimaryKey::new(
            stringify!($entity),
            ids![$entity.__id],
        )),*]
    };
}

pub static SQL_TABLES: &[TableMeta] = &[
    TableMeta::new("CompCastType", "comp_cast_type", Some("id")),
    TableMeta::new("CompanyName", "company_name", Some("id")),
    TableMeta::new("CompanyType", "company_type", Some("id")),
    TableMeta::new("InfoType", "info_type", Some("id")),
    TableMeta::new("Keyword", "keyword", Some("id")),
    TableMeta::new("KindType", "kind_type", Some("id")),
    TableMeta::new("LinkType", "link_type", Some("id")),
    TableMeta::new("Name", "name", Some("id")),
    TableMeta::new("RoleType", "role_type", Some("id")),
    TableMeta::new("CharName", "char_name", Some("id")),
    TableMeta::new("Title", "title", Some("id")),
    TableMeta::new("AkaName", "aka_name", Some("id")),
    TableMeta::new("AkaTitle", "aka_title", Some("id")),
    TableMeta::new("CastInfo", "cast_info", Some("id")),
    TableMeta::new("CompleteCast", "complete_cast", Some("id")),
    TableMeta::new("MovieCompanies", "movie_companies", Some("id")),
    TableMeta::new("MovieInfo", "movie_info", Some("id")),
    TableMeta::new("MovieInfoIdx", "movie_info_idx", Some("id")),
    TableMeta::new("MovieKeyword", "movie_keyword", Some("id")),
    TableMeta::new("MovieLink", "movie_link", Some("id")),
    TableMeta::new("PersonInfo", "person_info", Some("id")),
];

pub static REQUIRED: &[ColumnId] = ids![
    CompCastType.kind,
    CompanyName.name,
    CompanyType.kind,
    InfoType.info,
    Keyword.keyword,
    KindType.kind,
    LinkType.link,
    Name.name,
    RoleType.role,
    CharName.name,
    Title.title,
    Title.kind_id,
    AkaName.person_id,
    AkaName.name,
    AkaTitle.movie_id,
    AkaTitle.title,
    AkaTitle.kind_id,
    CastInfo.person_id,
    CastInfo.movie_id,
    CastInfo.role_id,
    CompleteCast.subject_id,
    CompleteCast.status_id,
    MovieCompanies.movie_id,
    MovieCompanies.company_id,
    MovieCompanies.company_type_id,
    MovieInfo.movie_id,
    MovieInfo.info_type_id,
    MovieInfo.info,
    MovieInfoIdx.movie_id,
    MovieInfoIdx.info_type_id,
    MovieInfoIdx.info,
    MovieKeyword.movie_id,
    MovieKeyword.keyword_id,
    MovieLink.movie_id,
    MovieLink.linked_movie_id,
    MovieLink.link_type_id,
    PersonInfo.person_id,
    PersonInfo.info_type_id,
    PersonInfo.info,
];

pub static PRIMARY_KEYS: &[PrimaryKey] = primary_keys![
    CompCastType,
    CompanyName,
    CompanyType,
    InfoType,
    Keyword,
    KindType,
    LinkType,
    Name,
    RoleType,
    CharName,
    Title,
    AkaName,
    AkaTitle,
    CastInfo,
    CompleteCast,
    MovieCompanies,
    MovieInfo,
    MovieInfoIdx,
    MovieKeyword,
    MovieLink,
    PersonInfo,
];

static CANONICAL_SCHEMA: Schema =
    Schema::canonical(TABLES, COLUMNS, SQL_TABLES, REQUIRED, PRIMARY_KEYS);

// These small vocabularies make successful joins and failed predicates
// both common in generated cases. Nullability remains schema-derived.
constraints! {
    pub static SCHEMA for CANONICAL_SCHEMA;
    CompCastType.kind: str => values(["complete", "complete+verified"]);
    CompanyName.country_code: str => values(["[de]", "[ru]", "[us]"]);
    CompanyType.kind: str => values(["production companies", "distributors"]);
    InfoType.info: str => values(["countries", "rating"]);
    Keyword.keyword: str => values(["character-name-in-title",
        "marvel-cinematic-universe", "hero-sequel"]);
    KindType.kind: str => values(["movie", "episode"]);
    LinkType.link: str => values(["follows", "references"]);
    Name.name: str => values(["Downey Jr., Robert", "Other Person"]);
    RoleType.role: str => values(["actor", "actress"]);
    CharName.name: str => values(["Voice Character", "Other Character"]);
    Title.title: str => length(1, 32);
    Title.production_year: i64 => range(2005, 2012);
    CastInfo.note: str => values(["(voice) (uncredited)", "(voice)",
        "(uncredited)"]);
    MovieInfo.info: str => values(["Bulgaria", "USA"]);
    MovieInfoIdx.info: str => values(["8.0", "4.0"]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_job_shape_is_valid() {
        SCHEMA.validate().unwrap();
        assert_eq!(TABLES.len(), 21);
        assert_eq!(SQL_TABLES.len(), 21);
        assert_eq!(PRIMARY_KEYS.len(), 21);
        assert_eq!(SCHEMA.foreign_key_count(), 25);
        assert!(SCHEMA.is_nullable(ColumnId::new("CastInfo", "note")));
        assert!(SCHEMA.is_nullable(ColumnId::new("Title", "production_year")));
        assert!(!SCHEMA.is_nullable(ColumnId::new("MovieKeyword", "movie_id")));
    }

    #[hegel::test(test_cases = 5)]
    fn generated_job_fixture_contains_null_and_loads(tc: hegel::TestCase) {
        use crate::generate::{Cell, generator};
        use crate::sql::to_sql;

        let database = tc.draw(generator(&SCHEMA));
        assert!(database.tables.iter().any(|table| {
            table
                .rows
                .iter()
                .any(|row| row.cells.values().any(|cell| matches!(cell, Cell::Null)))
        }));

        let fixture = to_sql(&SCHEMA, &database);
        assert!(fixture.contains("CREATE TABLE title"));
        assert!(fixture.contains("CREATE TABLE cast_info"));
        assert!(fixture.contains("NULL"));
        duckdb::Connection::open_in_memory()
            .unwrap()
            .execute_batch(&fixture)
            .unwrap();
    }
}
