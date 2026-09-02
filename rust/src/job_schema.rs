use crate::engine::{Id, IntoQuery, Universe, Value};
use crate::loader::{Col, Loader, Set, Str};
use std::path::Path;
use std::sync::OnceLock;

// =====================================================================
// Entities
// =====================================================================
//
// An entity struct reads like a SQL table: the first field `key` is the
// identity column over the entity's ids (its id space, sized at load), and
// every other field is a column keyed by those ids. `#[derive(IntoQuery)]`
// implements `IntoQuery for &Self` by returning `key`, and `QueryExt` is
// blanket-implemented over `IntoQuery`, so an entity drives directly:
// `db.movie.select(..)` resolves by autoref on `&Movie`. Reach for
// `db.movie.key` itself when you want the `Universe`, e.g. for its `.n`.

#[derive(IntoQuery)]
pub struct Movie {
    pub key: Universe<Id<Movie>>,
    pub title: Col<Movie, Str>,
    pub kind: Col<Movie, Id<Kind>>,
    pub production_year: Set<Movie, i64>,
    pub episode_nr: Set<Movie, i64>,
    pub keyword: Set<Movie, Id<Keyword>>,
    pub company: Set<Movie, Id<Company>>,
    pub cast: Set<Movie, Id<Cast>>,
    pub info: Set<Movie, Id<Info>>,
    pub data: Set<Movie, Id<Data>>,
    pub complete_cast: Set<Movie, Id<CompleteCast>>,
    pub link: Set<Movie, Id<MovieLink>>,
    pub linked_by: Set<Movie, Id<MovieLink>>,
    pub aka: Set<Movie, Id<AkaTitle>>,
}

#[derive(IntoQuery)]
pub struct Cast {
    pub key: Universe<Id<Cast>>,
    pub person: Col<Cast, Id<Person>>,
    pub role: Col<Cast, Id<RoleType>>,
    pub note: Set<Cast, Str>,
    pub character: Set<Cast, Id<Character>>,
}

#[derive(IntoQuery)]
pub struct Person {
    pub key: Universe<Id<Person>>,
    pub name: Col<Person, Str>,
    pub gender: Set<Person, Str>,
    pub alias: Set<Person, Id<AkaName>>,
    pub bio: Set<Person, Id<PersonInfo>>,
    pub name_pcode_cf: Set<Person, Str>,
}

#[derive(IntoQuery)]
pub struct Keyword {
    pub key: Universe<Id<Keyword>>,
    pub text: Col<Keyword, Str>,
}
#[derive(IntoQuery)]
pub struct Kind {
    pub key: Universe<Id<Kind>>,
    pub text: Col<Kind, Str>,
}
#[derive(IntoQuery)]
pub struct RoleType {
    pub key: Universe<Id<RoleType>>,
    pub text: Col<RoleType, Str>,
}
#[derive(IntoQuery)]
pub struct Character {
    pub key: Universe<Id<Character>>,
    pub text: Col<Character, Str>,
}

#[derive(IntoQuery)]
pub struct Company {
    pub key: Universe<Id<Company>>,
    pub name: Col<Company, Str>,
    pub country: Set<Company, Str>,
    pub note: Set<Company, Str>,
    pub ty: Col<Company, Id<CompanyType>>,
}
#[derive(IntoQuery)]
pub struct CompanyType {
    pub key: Universe<Id<CompanyType>>,
    pub text: Col<CompanyType, Str>,
}

#[derive(IntoQuery)]
pub struct Info {
    pub key: Universe<Id<Info>>,
    pub info: Col<Info, Str>,
    pub ty: Col<Info, Id<InfoType>>,
    pub note: Set<Info, Str>,
}
#[derive(IntoQuery)]
pub struct InfoType {
    pub key: Universe<Id<InfoType>>,
    pub text: Col<InfoType, Str>,
}

#[derive(IntoQuery)]
pub struct Data {
    pub key: Universe<Id<Data>>,
    pub text: Col<Data, Str>,
    pub ty: Col<Data, Id<InfoType>>,
}

#[derive(IntoQuery)]
pub struct PersonInfo {
    pub key: Universe<Id<PersonInfo>>,
    pub info: Col<PersonInfo, Str>,
    pub ty: Col<PersonInfo, Id<InfoType>>,
    pub note: Set<PersonInfo, Str>,
}

#[derive(IntoQuery)]
pub struct AkaName {
    pub key: Universe<Id<AkaName>>,
    pub text: Col<AkaName, Str>,
}
#[derive(IntoQuery)]
pub struct AkaTitle {
    pub key: Universe<Id<AkaTitle>>,
    pub text: Col<AkaTitle, Str>,
}

#[derive(IntoQuery)]
pub struct MovieLink {
    pub key: Universe<Id<MovieLink>>,
    pub target: Col<MovieLink, Id<Movie>>,
    pub ty: Col<MovieLink, Id<LinkType>>,
}
#[derive(IntoQuery)]
pub struct LinkType {
    pub key: Universe<Id<LinkType>>,
    pub text: Col<LinkType, Str>,
}

#[derive(IntoQuery)]
pub struct CompleteCast {
    pub key: Universe<Id<CompleteCast>>,
    pub status: Col<CompleteCast, Id<CompCastType>>,
    pub subject: Col<CompleteCast, Id<CompCastType>>,
}
#[derive(IntoQuery)]
pub struct CompCastType {
    pub key: Universe<Id<CompCastType>>,
    pub text: Col<CompCastType, Str>,
}

macro_rules! value {
    ($Db:ty; $($E:ident, $STATIC:ident, $dbfield:ident . $field:ident, $Scalar:ty);* $(;)?) => {
        $(
            static $STATIC: OnceLock<&'static Col<$E, $Scalar>> = OnceLock::new();
            impl Value for $E {
                type Scalar = $Scalar;
                type Col = Col<$E, $Scalar>;
                #[inline]
                fn value() -> &'static Self::Col {
                    $STATIC.get().expect("value column read before load()")
                }
            }
        )*

        pub fn register_values(db: &'static $Db) {
            $(
                let _ = $STATIC.set(&db.$dbfield.$field);
            )*
        }
    };
}

value! {
    Job;
    Movie, MOVIE_VALUE, movie.title, Str;
    Person, PERSON_VALUE, person.name, Str;
    Keyword, KEYWORD_VALUE, keyword.text, Str;
    Kind, KIND_VALUE, kind.text, Str;
    RoleType, ROLE_TYPE_VALUE, role_type.text, Str;
    Character, CHARACTER_VALUE, character.text, Str;
    Company, COMPANY_VALUE, company.name, Str;
    CompanyType, COMPANY_TYPE_VALUE, company_type.text, Str;
    Info, INFO_VALUE, info.info, Str;
    InfoType, INFO_TYPE_VALUE, info_type.text, Str;
    Data, DATA_VALUE, data.text, Str;
    PersonInfo, PERSON_INFO_VALUE, person_info.info, Str;
    AkaName, AKA_NAME_VALUE, aka_name.text, Str;
    AkaTitle, AKA_TITLE_VALUE, aka_title.text, Str;
    LinkType, LINK_TYPE_VALUE, link_type.text, Str;
    CompCastType, COMP_CAST_TYPE_VALUE, comp_cast_type.text, Str;
}

// =====================================================================
// The database
// =====================================================================

pub struct Job {
    pub movie: Movie,
    pub cast: Cast,
    pub person: Person,
    pub keyword: Keyword,
    pub kind: Kind,
    pub role_type: RoleType,
    pub character: Character,
    pub company: Company,
    pub company_type: CompanyType,
    pub info: Info,
    pub info_type: InfoType,
    pub data: Data,
    pub person_info: PersonInfo,
    pub aka_name: AkaName,
    pub aka_title: AkaTitle,
    pub movie_link: MovieLink,
    pub link_type: LinkType,
    pub complete_cast: CompleteCast,
    pub comp_cast_type: CompCastType,
}

// =====================================================================
// Loading
// =====================================================================

fn build(l: &mut Loader) -> Job {
    Job {
        movie: Movie {
            key: l.key("Movie_title"),
            title: l.strs("Movie_title"),
            kind: l.ids("Movie_kind"),
            production_year: l.multi_i64("Movie_production_year"),
            episode_nr: l.multi_i64("Movie_episode_nr"),
            keyword: l.multi_ids("Movie_keyword"),
            company: l.multi_ids("Movie_company"),
            cast: l.multi_ids("Movie_cast"),
            info: l.multi_ids("Movie_info"),
            data: l.multi_ids("Movie_data"),
            complete_cast: l.multi_ids("Movie_complete_cast"),
            link: l.multi_ids("Movie_link"),
            linked_by: l.multi_ids("Movie_linked_by"),
            aka: l.multi_ids("Movie_aka"),
        },
        cast: Cast {
            key: l.key("Cast_person"),
            person: l.ids("Cast_person"),
            role: l.ids("Cast_role"),
            note: l.multi_strs("Cast_note"),
            character: l.multi_ids("Cast_character"),
        },
        person: Person {
            key: l.key("Person_name"),
            name: l.strs("Person_name"),
            gender: l.multi_strs("Person_gender"),
            alias: l.multi_ids("Person_alias"),
            bio: l.multi_ids("Person_bio"),
            name_pcode_cf: l.multi_strs("Person_name_pcode_cf"),
        },
        keyword: Keyword {
            key: l.key("Keyword_text"),
            text: l.strs("Keyword_text"),
        },
        kind: Kind {
            key: l.key("Kind_text"),
            text: l.strs("Kind_text"),
        },
        role_type: RoleType {
            key: l.key("RoleType_text"),
            text: l.strs("RoleType_text"),
        },
        character: Character {
            key: l.key("Character_text"),
            text: l.strs("Character_text"),
        },
        company: Company {
            key: l.key("Company_name"),
            name: l.strs("Company_name"),
            country: l.multi_strs("Company_country"),
            note: l.multi_strs("Company_note"),
            ty: l.ids("Company_ty"),
        },
        company_type: CompanyType {
            key: l.key("CompanyType_text"),
            text: l.strs("CompanyType_text"),
        },
        info: Info {
            key: l.key("Info_info"),
            info: l.strs("Info_info"),
            ty: l.ids("Info_ty"),
            note: l.multi_strs("Info_note"),
        },
        info_type: InfoType {
            key: l.key("InfoType_text"),
            text: l.strs("InfoType_text"),
        },
        data: Data {
            key: l.key("Data_text"),
            text: l.strs("Data_text"),
            ty: l.ids("Data_ty"),
        },
        person_info: PersonInfo {
            key: l.key("PersonInfo_info"),
            info: l.strs("PersonInfo_info"),
            ty: l.ids("PersonInfo_ty"),
            note: l.multi_strs("PersonInfo_note"),
        },
        aka_name: AkaName {
            key: l.key("AkaName_text"),
            text: l.strs("AkaName_text"),
        },
        aka_title: AkaTitle {
            key: l.key("AkaTitle_text"),
            text: l.strs("AkaTitle_text"),
        },
        movie_link: MovieLink {
            key: l.key("MovieLink_target"),
            target: l.ids("MovieLink_target"),
            ty: l.ids("MovieLink_ty"),
        },
        link_type: LinkType {
            key: l.key("LinkType_text"),
            text: l.strs("LinkType_text"),
        },
        complete_cast: CompleteCast {
            key: l.key("CompleteCast_status"),
            status: l.ids("CompleteCast_status"),
            subject: l.ids("CompleteCast_subject"),
        },
        comp_cast_type: CompCastType {
            key: l.key("CompCastType_text"),
            text: l.strs("CompCastType_text"),
        },
    }
}

pub fn load(dir: &Path) -> Job {
    build(&mut Loader::new(dir))
}

pub fn manifest() -> Vec<(String, u32)> {
    let mut l = Loader::probing();
    let _ = build(&mut l);
    l.manifest()
}

// ===== tests — typed loading agrees with the untyped loaders =============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{load_ids, load_multi_ids, load_strs};
    use crate::engine::{Drive, Probe, QueryExt};

    /// Full load against the real cache, cross-checked column-by-column
    /// against the untyped cache readers (lengths AND a value spot check
    /// through the `repr(transparent)` id reinterpret). Skipped when the
    /// cache isn't present (CI without regen output).
    #[test]
    fn typed_schema_matches_untyped_loaders() {
        let dir = Path::new("../cache");
        if !dir.join("Movie_title.bin").exists() {
            eprintln!("skipping: ../cache not present (run `regen job`)");
            return;
        }
        let db = load(dir);

        // key columns are sized by the value column's domain
        assert_eq!(db.movie.key.n, load_strs("Movie_title").n_dom());
        assert_eq!(db.person.key.n, load_strs("Person_name").n_dom());

        // dense str / dense id / CSR id columns
        assert_eq!(db.movie.title.n_dom(), load_strs("Movie_title").n_dom());
        assert_eq!(db.movie.kind.n_dom(), load_ids("Movie_kind").n_dom());
        assert_eq!(
            db.movie.keyword.n_dom(),
            load_multi_ids("Movie_keyword").n_dom()
        );
        assert_eq!(db.data.ty.n_dom(), load_ids("Data_ty").n_dom());

        // value parity through the bulk reinterpret: typed Id<Kind> indexes
        // equal the untyped words, row for row (first 1000 rows).
        let untyped = load_ids("Movie_kind");
        for i in 0..1000.min(untyped.n_dom()) {
            let mut got = None;
            (&db.movie.kind).probe(Id::new(i), |k| got = Some(k.0));
            assert_eq!(got, Some(untyped.v[i]));
        }

        // a typed end-to-end drive matches the untyped one
        let Movie { kind, .. } = &db.movie;
        let Kind { text: kind_text, .. } = &db.kind;
        let mut n_typed = 0usize;
        db.movie
            .with(kind.select(kind_text).eq("movie"))
            .drive(|_, _| n_typed += 1);
        let kk = load_strs("Kind_text");
        let mk = load_ids("Movie_kind");
        let mut n_untyped = 0usize;
        crate::engine::Universe::<usize>::new(mk.n_dom())
            .with((&mk).select(&kk).eq("movie"))
            .drive(|_, _| n_untyped += 1);
        assert_eq!(n_typed, n_untyped);
    }

    #[test]
    fn value_elision_matches_explicit_select() {
        let dir = Path::new("../cache");
        if !dir.join("Movie_title.bin").exists() {
            eprintln!("skipping: ../cache not present (run `regen job`)");
            return;
        }
        let db: &'static Job = Box::leak(Box::new(load(dir)));
        register_values(db);

        let Movie { kind, .. } = &db.movie;
        let mut n_elided = 0usize;
        db.movie
            .with(kind.eq("movie"))
            .drive(|_, _| n_elided += 1);

        let Kind { text: kind_text, .. } = &db.kind;
        let mut n_explicit = 0usize;
        db.movie
            .with(kind.select(kind_text).eq("movie"))
            .drive(|_, _| n_explicit += 1);

        assert_eq!(n_elided, n_explicit);
        assert!(n_elided > 0);
    }

    /// The manifest lists every column exactly once and matches the file
    /// set the JOB cache actually holds.
    #[test]
    fn manifest_is_complete_and_unique() {
        let m = manifest();
        assert_eq!(m.len(), 48, "JOB declares 48 columns");
        let mut names: Vec<&str> = m.iter().map(|(n, _)| n.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate column in the manifest");
    }
}
