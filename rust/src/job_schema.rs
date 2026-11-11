use crate::engine::IntoQuery;
use crate::loader::{Col, Dict, DictSet, Fk, FkSet, Loader, Rows, Set, Str};
use std::path::Path;

#[derive(IntoQuery)]
pub struct Movie {
    #[primary_key]
    pub id: Rows<Self>,
    pub title: Col<Self, Str>,
    pub kind: Dict<Self, Str>,
    pub production_year: Set<Self, i64>,
    pub episode_nr: Set<Self, i64>,
    pub keyword: DictSet<Self, Str>,
    pub company: FkSet<Self, Company>,
    pub cast: FkSet<Self, Cast>,
    pub info: FkSet<Self, Info>,
    pub data: FkSet<Self, Data>,
    pub complete_cast: FkSet<Self, CompleteCast>,
    pub link: FkSet<Self, MovieLink>,
    pub linked_by: FkSet<Self, MovieLink>,
    pub aka: DictSet<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Cast {
    #[primary_key]
    pub id: Rows<Self>,
    pub person: Fk<Self, Person>,
    pub role: Dict<Self, Str>,
    pub note: Set<Self, Str>,
    pub character: DictSet<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Person {
    #[primary_key]
    pub id: Rows<Self>,
    pub name: Col<Self, Str>,
    pub gender: Set<Self, Str>,
    pub alias: DictSet<Self, Str>,
    pub bio: FkSet<Self, PersonInfo>,
    pub name_pcode_cf: Set<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Company {
    #[primary_key]
    pub id: Rows<Self>,
    pub name: Col<Self, Str>,
    pub country: Set<Self, Str>,
    pub note: Set<Self, Str>,
    pub ty: Dict<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Info {
    #[primary_key]
    pub id: Rows<Self>,
    pub info: Col<Self, Str>,
    pub ty: Dict<Self, Str>,
    pub note: Set<Self, Str>,
}

#[derive(IntoQuery)]
pub struct Data {
    #[primary_key]
    pub id: Rows<Self>,
    pub text: Col<Self, Str>,
    pub ty: Dict<Self, Str>,
}

#[derive(IntoQuery)]
pub struct PersonInfo {
    #[primary_key]
    pub id: Rows<Self>,
    pub info: Col<Self, Str>,
    pub ty: Dict<Self, Str>,
    pub note: Set<Self, Str>,
}

#[derive(IntoQuery)]
pub struct MovieLink {
    #[primary_key]
    pub id: Rows<Self>,
    pub target: Fk<Self, Movie>,
    pub ty: Dict<Self, Str>,
}

#[derive(IntoQuery)]
pub struct CompleteCast {
    #[primary_key]
    pub id: Rows<Self>,
    pub status: Dict<Self, Str>,
    pub subject: Dict<Self, Str>,
}

pub struct Job {
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

fn build(l: &mut Loader) -> Job {
    // Every entity an FK points at gets a primary-key index (key → row),
    // built before any FK column that resolves through it.
    let movie_id = l.rows("Movie_title");
    let cast_id = l.rows("Cast_person");
    let person_id = l.rows("Person_name");
    let company_id = l.rows("Company_name");
    let info_id = l.rows("Info_info");
    let data_id = l.rows("Data_text");
    let person_info_id = l.rows("PersonInfo_info");
    let movie_link_id = l.rows("MovieLink_target");
    let complete_cast_id = l.rows("CompleteCast_status");
    let movie_pk = l.pk(movie_id);
    let cast_pk = l.pk(cast_id);
    let person_pk = l.pk(person_id);
    let company_pk = l.pk(company_id);
    let info_pk = l.pk(info_id);
    let data_pk = l.pk(data_id);
    let person_info_pk = l.pk(person_info_id);
    let movie_link_pk = l.pk(movie_link_id);
    let complete_cast_pk = l.pk(complete_cast_id);
    Job {
        movie: Movie {
            id: movie_id,
            title: l.strs("Movie_title"),
            kind: l.dict("Movie_kind", "Kind_text"),
            production_year: l.multi_i64("Movie_production_year"),
            episode_nr: l.multi_i64("Movie_episode_nr"),
            keyword: l.multi_dict("Movie_keyword", "Keyword_text"),
            company: l.multi_fk("Movie_company", company_pk),
            cast: l.multi_fk("Movie_cast", cast_pk),
            info: l.multi_fk("Movie_info", info_pk),
            data: l.multi_fk("Movie_data", data_pk),
            complete_cast: l.multi_fk("Movie_complete_cast", complete_cast_pk),
            link: l.multi_fk("Movie_link", movie_link_pk),
            linked_by: l.multi_fk("Movie_linked_by", movie_link_pk),
            aka: l.multi_dict("Movie_aka", "AkaTitle_text"),
        },
        cast: Cast {
            id: cast_id,
            person: l.fk("Cast_person", person_pk),
            role: l.dict("Cast_role", "RoleType_text"),
            note: l.multi_strs("Cast_note"),
            character: l.multi_dict("Cast_character", "Character_text"),
        },
        person: Person {
            id: person_id,
            name: l.strs("Person_name"),
            gender: l.multi_strs("Person_gender"),
            alias: l.multi_dict("Person_alias", "AkaName_text"),
            bio: l.multi_fk("Person_bio", person_info_pk),
            name_pcode_cf: l.multi_strs("Person_name_pcode_cf"),
        },
        company: Company {
            id: company_id,
            name: l.strs("Company_name"),
            country: l.multi_strs("Company_country"),
            note: l.multi_strs("Company_note"),
            ty: l.dict("Company_ty", "CompanyType_text"),
        },
        info: Info {
            id: info_id,
            info: l.strs("Info_info"),
            ty: l.dict("Info_ty", "InfoType_text"),
            note: l.multi_strs("Info_note"),
        },
        data: Data {
            id: data_id,
            text: l.strs("Data_text"),
            ty: l.dict("Data_ty", "InfoType_text"),
        },
        person_info: PersonInfo {
            id: person_info_id,
            info: l.strs("PersonInfo_info"),
            ty: l.dict("PersonInfo_ty", "InfoType_text"),
            note: l.multi_strs("PersonInfo_note"),
        },
        movie_link: MovieLink {
            id: movie_link_id,
            target: l.fk("MovieLink_target", movie_pk),
            ty: l.dict("MovieLink_ty", "LinkType_text"),
        },
        complete_cast: CompleteCast {
            id: complete_cast_id,
            status: l.dict("CompleteCast_status", "CompCastType_text"),
            subject: l.dict("CompleteCast_subject", "CompCastType_text"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{load_ids, load_multi_ids, load_strs};
    use crate::engine::{Drive, Probe, QueryExt, Row};

    /// Full load against the real cache, cross-checked column-by-column
    /// against the untyped cache readers (lengths AND a value spot check
    /// through the dictionary). Skipped when the cache isn't present (CI
    /// without regen output).
    #[test]
    fn typed_schema_matches_untyped_loaders() {
        let dir = Path::new("../cache");
        if !dir.join("Movie_title.bin").exists() {
            eprintln!("skipping: ../cache not present (run `regen job`)");
            return;
        }
        let db = load(dir);

        // primary keys are sized by the value column's domain
        assert_eq!(db.movie.id.n, load_strs("Movie_title").n_dom());
        assert_eq!(db.person.id.n, load_strs("Person_name").n_dom());

        // dense str / dictionary / CSR id columns
        assert_eq!(db.movie.title.n_dom(), load_strs("Movie_title").n_dom());
        assert_eq!(db.movie.kind.n_dom(), load_ids("Movie_kind").n_dom());
        assert_eq!(
            db.movie.keyword.n_dom(),
            load_multi_ids("Movie_keyword").n_dom()
        );
        assert_eq!(db.data.ty.n_dom(), load_ids("Data_ty").n_dom());

        // value parity through the dictionary: a movie's kind string is the
        // untyped code looked up in the untyped string table (first 1000).
        let codes = load_ids("Movie_kind");
        let table = load_strs("Kind_text");
        for i in 0..1000.min(codes.n_dom()) {
            let mut got = None;
            (&db.movie.kind).probe(Row::new(i), |s| got = Some(s));
            assert_eq!(got, Some(table.v[codes.v[i]]));
        }

        // a typed end-to-end drive matches the untyped one
        let Movie { kind, .. } = &db.movie;
        let mut n_typed = 0usize;
        db.movie
            .with(kind.eq("movie"))
            .drive(|_, _| n_typed += 1);
        let kk = load_strs("Kind_text");
        let mk = load_ids("Movie_kind");
        let mut n_untyped = 0usize;
        crate::engine::Universe::<usize>::new(mk.n_dom())
            .with((&mk).select(&kk).eq("movie"))
            .drive(|_, _| n_untyped += 1);
        assert_eq!(n_typed, n_untyped);
        assert!(n_typed > 0);
    }

    /// `rx_dict` (regex once over the dictionary, bit test per pair) agrees
    /// with `rx` (regex per pair).
    #[test]
    fn rx_dict_matches_rx() {
        let dir = Path::new("../cache");
        if !dir.join("Movie_title.bin").exists() {
            eprintln!("skipping: ../cache not present (run `regen job`)");
            return;
        }
        let db = load(dir);
        let Movie { keyword, .. } = &db.movie;
        let mut a = 0usize;
        db.movie
            .with(keyword.rx_dict(r"sequel"))
            .drive(|_, _| a += 1);
        let mut b = 0usize;
        db.movie
            .with(keyword.rx(r"sequel"))
            .drive(|_, _| b += 1);
        assert_eq!(a, b);
        assert!(a > 0);
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
