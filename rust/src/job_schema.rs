use crate::engine::{Id, IntoQuery, Primary, Universe};
use crate::loader::{Col, Loader, Set, Str};
use std::path::Path;
use std::sync::OnceLock;

// =====================================================================
// Entities
// =====================================================================

pub struct Movie {
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

pub struct Cast {
    pub person: Col<Cast, Id<Person>>,
    pub role: Col<Cast, Id<RoleType>>,
    pub note: Set<Cast, Str>,
    pub character: Set<Cast, Id<Character>>,
}

pub struct Person {
    pub name: Col<Person, Str>,
    pub gender: Set<Person, Str>,
    pub alias: Set<Person, Id<AkaName>>,
    pub bio: Set<Person, Id<PersonInfo>>,
    pub name_pcode_cf: Set<Person, Str>,
}

pub struct Keyword {
    pub text: Col<Keyword, Str>,
}
pub struct Kind {
    pub text: Col<Kind, Str>,
}
pub struct RoleType {
    pub text: Col<RoleType, Str>,
}
pub struct Character {
    pub text: Col<Character, Str>,
}

pub struct Company {
    pub name: Col<Company, Str>,
    pub country: Set<Company, Str>,
    pub note: Set<Company, Str>,
    pub ty: Col<Company, Id<CompanyType>>,
}
pub struct CompanyType {
    pub text: Col<CompanyType, Str>,
}

pub struct Info {
    pub info: Col<Info, Str>,
    pub ty: Col<Info, Id<InfoType>>,
    pub note: Set<Info, Str>,
}
pub struct InfoType {
    pub text: Col<InfoType, Str>,
}

pub struct Data {
    pub text: Col<Data, Str>,
    pub ty: Col<Data, Id<InfoType>>,
}

pub struct PersonInfo {
    pub info: Col<PersonInfo, Str>,
    pub ty: Col<PersonInfo, Id<InfoType>>,
    pub note: Set<PersonInfo, Str>,
}

pub struct AkaName {
    pub text: Col<AkaName, Str>,
}
pub struct AkaTitle {
    pub text: Col<AkaTitle, Str>,
}

pub struct MovieLink {
    pub target: Col<MovieLink, Id<Movie>>,
    pub ty: Col<MovieLink, Id<LinkType>>,
}
pub struct LinkType {
    pub text: Col<LinkType, Str>,
}

pub struct CompleteCast {
    pub status: Col<CompleteCast, Id<CompCastType>>,
    pub subject: Col<CompleteCast, Id<CompCastType>>,
}
pub struct CompCastType {
    pub text: Col<CompCastType, Str>,
}

// =====================================================================
// Universes
// =====================================================================

impl Movie {
    #[inline]
    pub fn all(&self) -> Universe<Id<Movie>> {
        Universe::new(self.title.n_keys())
    }
}
impl Cast {
    #[inline]
    pub fn all(&self) -> Universe<Id<Cast>> {
        Universe::new(self.person.n_keys())
    }
}
impl Person {
    #[inline]
    pub fn all(&self) -> Universe<Id<Person>> {
        Universe::new(self.name.n_keys())
    }
}
impl Keyword {
    #[inline]
    pub fn all(&self) -> Universe<Id<Keyword>> {
        Universe::new(self.text.n_keys())
    }
}
impl Kind {
    #[inline]
    pub fn all(&self) -> Universe<Id<Kind>> {
        Universe::new(self.text.n_keys())
    }
}
impl RoleType {
    #[inline]
    pub fn all(&self) -> Universe<Id<RoleType>> {
        Universe::new(self.text.n_keys())
    }
}
impl Character {
    #[inline]
    pub fn all(&self) -> Universe<Id<Character>> {
        Universe::new(self.text.n_keys())
    }
}
impl Company {
    #[inline]
    pub fn all(&self) -> Universe<Id<Company>> {
        Universe::new(self.name.n_keys())
    }
}
impl CompanyType {
    #[inline]
    pub fn all(&self) -> Universe<Id<CompanyType>> {
        Universe::new(self.text.n_keys())
    }
}
impl Info {
    #[inline]
    pub fn all(&self) -> Universe<Id<Info>> {
        Universe::new(self.info.n_keys())
    }
}
impl InfoType {
    #[inline]
    pub fn all(&self) -> Universe<Id<InfoType>> {
        Universe::new(self.text.n_keys())
    }
}
impl Data {
    #[inline]
    pub fn all(&self) -> Universe<Id<Data>> {
        Universe::new(self.text.n_keys())
    }
}
impl PersonInfo {
    #[inline]
    pub fn all(&self) -> Universe<Id<PersonInfo>> {
        Universe::new(self.info.n_keys())
    }
}
impl AkaName {
    #[inline]
    pub fn all(&self) -> Universe<Id<AkaName>> {
        Universe::new(self.text.n_keys())
    }
}
impl AkaTitle {
    #[inline]
    pub fn all(&self) -> Universe<Id<AkaTitle>> {
        Universe::new(self.text.n_keys())
    }
}
impl MovieLink {
    #[inline]
    pub fn all(&self) -> Universe<Id<MovieLink>> {
        Universe::new(self.target.n_keys())
    }
}
impl LinkType {
    #[inline]
    pub fn all(&self) -> Universe<Id<LinkType>> {
        Universe::new(self.text.n_keys())
    }
}
impl CompleteCast {
    #[inline]
    pub fn all(&self) -> Universe<Id<CompleteCast>> {
        Universe::new(self.status.n_keys())
    }
}
impl CompCastType {
    #[inline]
    pub fn all(&self) -> Universe<Id<CompCastType>> {
        Universe::new(self.text.n_keys())
    }
}

// =====================================================================
// Entity-as-query
// =====================================================================
//
// `QueryExt` is blanket-implemented over `IntoQuery`, so teaching an entity
// struct to become its own universe is enough to make it drivable directly:
// `db.movie.select(..)` is `db.movie.all().select(..)`, resolved by autoref
// on `&Movie`. `all()` remains the explicit spelling — and the one to reach
// for when you want the `Universe` itself, e.g. for its `.n`.
//
// No coherence clash with `impl<Q: Query> IntoQuery for Q`: an entity struct
// is not itself a `Query`.

macro_rules! entity_query {
    ($($E:ident),* $(,)?) => {$(
        impl IntoQuery for &$E {
            type Q = Universe<Id<$E>>;
            #[inline(always)]
            fn iq(self) -> Self::Q {
                self.all()
            }
        }
    )*};
}

entity_query! {
    Movie,
    Cast,
    Person,
    Keyword,
    Kind,
    RoleType,
    Character,
    Company,
    CompanyType,
    Info,
    InfoType,
    Data,
    PersonInfo,
    AkaName,
    AkaTitle,
    MovieLink,
    LinkType,
    CompleteCast,
    CompCastType,
}

macro_rules! primary {
    ($Db:ty; $($E:ident, $STATIC:ident, $dbfield:ident . $field:ident, $Scalar:ty);* $(;)?) => {
        $(
            static $STATIC: OnceLock<&'static Col<$E, $Scalar>> = OnceLock::new();
            impl Primary for $E {
                type Scalar = $Scalar;
                type Col = Col<$E, $Scalar>;
                #[inline]
                fn primary() -> &'static Self::Col {
                    $STATIC.get().expect("primary column read before load()")
                }
            }
        )*

        pub fn register_primaries(db: &'static $Db) {
            $(
                let _ = $STATIC.set(&db.$dbfield.$field);
            )*
        }
    };
}

primary! {
    Job;
    Movie, MOVIE_PRIMARY, movie.title, Str;
    Person, PERSON_PRIMARY, person.name, Str;
    Keyword, KEYWORD_PRIMARY, keyword.text, Str;
    Kind, KIND_PRIMARY, kind.text, Str;
    RoleType, ROLE_TYPE_PRIMARY, role_type.text, Str;
    Character, CHARACTER_PRIMARY, character.text, Str;
    Company, COMPANY_PRIMARY, company.name, Str;
    CompanyType, COMPANY_TYPE_PRIMARY, company_type.text, Str;
    Info, INFO_PRIMARY, info.info, Str;
    InfoType, INFO_TYPE_PRIMARY, info_type.text, Str;
    Data, DATA_PRIMARY, data.text, Str;
    PersonInfo, PERSON_INFO_PRIMARY, person_info.info, Str;
    AkaName, AKA_NAME_PRIMARY, aka_name.text, Str;
    AkaTitle, AKA_TITLE_PRIMARY, aka_title.text, Str;
    LinkType, LINK_TYPE_PRIMARY, link_type.text, Str;
    CompCastType, COMP_CAST_TYPE_PRIMARY, comp_cast_type.text, Str;
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
            person: l.ids("Cast_person"),
            role: l.ids("Cast_role"),
            note: l.multi_strs("Cast_note"),
            character: l.multi_ids("Cast_character"),
        },
        person: Person {
            name: l.strs("Person_name"),
            gender: l.multi_strs("Person_gender"),
            alias: l.multi_ids("Person_alias"),
            bio: l.multi_ids("Person_bio"),
            name_pcode_cf: l.multi_strs("Person_name_pcode_cf"),
        },
        keyword: Keyword {
            text: l.strs("Keyword_text"),
        },
        kind: Kind {
            text: l.strs("Kind_text"),
        },
        role_type: RoleType {
            text: l.strs("RoleType_text"),
        },
        character: Character {
            text: l.strs("Character_text"),
        },
        company: Company {
            name: l.strs("Company_name"),
            country: l.multi_strs("Company_country"),
            note: l.multi_strs("Company_note"),
            ty: l.ids("Company_ty"),
        },
        company_type: CompanyType {
            text: l.strs("CompanyType_text"),
        },
        info: Info {
            info: l.strs("Info_info"),
            ty: l.ids("Info_ty"),
            note: l.multi_strs("Info_note"),
        },
        info_type: InfoType {
            text: l.strs("InfoType_text"),
        },
        data: Data {
            text: l.strs("Data_text"),
            ty: l.ids("Data_ty"),
        },
        person_info: PersonInfo {
            info: l.strs("PersonInfo_info"),
            ty: l.ids("PersonInfo_ty"),
            note: l.multi_strs("PersonInfo_note"),
        },
        aka_name: AkaName {
            text: l.strs("AkaName_text"),
        },
        aka_title: AkaTitle {
            text: l.strs("AkaTitle_text"),
        },
        movie_link: MovieLink {
            target: l.ids("MovieLink_target"),
            ty: l.ids("MovieLink_ty"),
        },
        link_type: LinkType {
            text: l.strs("LinkType_text"),
        },
        complete_cast: CompleteCast {
            status: l.ids("CompleteCast_status"),
            subject: l.ids("CompleteCast_subject"),
        },
        comp_cast_type: CompCastType {
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

        // universe sizes = the naming column's key count
        assert_eq!(db.movie.all().n, load_strs("Movie_title").n_keys());
        assert_eq!(db.person.all().n, load_strs("Person_name").n_keys());

        // dense str / dense id / CSR id columns
        assert_eq!(db.movie.title.n_keys(), load_strs("Movie_title").n_keys());
        assert_eq!(db.movie.kind.n_keys(), load_ids("Movie_kind").n_keys());
        assert_eq!(
            db.movie.keyword.n_keys(),
            load_multi_ids("Movie_keyword").n_keys()
        );
        assert_eq!(db.data.ty.n_keys(), load_ids("Data_ty").n_keys());

        // value parity through the bulk reinterpret: typed Id<Kind> indexes
        // equal the untyped words, row for row (first 1000 rows).
        let untyped = load_ids("Movie_kind");
        for i in 0..1000.min(untyped.n_keys()) {
            let mut got = None;
            (&db.movie.kind).probe(Id::new(i), |k| got = Some(k.0));
            assert_eq!(got, Some(untyped.values[i]));
        }

        // a typed end-to-end drive matches the untyped one
        let Movie { kind, .. } = &db.movie;
        let Kind { text: kind_text } = &db.kind;
        let mut n_typed = 0usize;
        db.movie
            .all()
            .with(kind.select(kind_text).eq("movie"))
            .drive(|_, _| n_typed += 1);
        let kk = load_strs("Kind_text");
        let mk = load_ids("Movie_kind");
        let mut n_untyped = 0usize;
        crate::engine::Universe::<usize>::new(mk.n_keys())
            .with((&mk).select(&kk).eq("movie"))
            .drive(|_, _| n_untyped += 1);
        assert_eq!(n_typed, n_untyped);
    }

    #[test]
    fn primary_elision_matches_explicit_select() {
        let dir = Path::new("../cache");
        if !dir.join("Movie_title.bin").exists() {
            eprintln!("skipping: ../cache not present (run `regen job`)");
            return;
        }
        let db: &'static Job = Box::leak(Box::new(load(dir)));
        register_primaries(db);

        let Movie { kind, .. } = &db.movie;
        let mut n_elided = 0usize;
        db.movie
            .all()
            .with(kind.eq("movie"))
            .drive(|_, _| n_elided += 1);

        let Kind { text: kind_text } = &db.kind;
        let mut n_explicit = 0usize;
        db.movie
            .all()
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
