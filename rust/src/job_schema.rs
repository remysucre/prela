// The typed JOB schema — the `schema!` declaration of the JOB cache
// (entities and fields per the cache file list; v2 readers, every column
// keyed by `Id<Entity>`).
//
// `pub` marks the fields that get bare top-level leaf handles — names that
// are unambiguous across this schema (and don't collide with the universe
// handles `movie`/`persons`). Lookup-table labels are uniformly `text`
// (Keyword, Kind, RoleType, Character, CompanyType, InfoType, AkaName,
// AkaTitle, LinkType, CompCastType — and Data, whose payload is its label),
// so the interesting names (`kind`, `keyword`, `link`, `aka`, …) are free
// for Movie's edges to claim bare. `name` (Person/Company), `note`
// (Cast/Company/Info/PersonInfo), `ty` and Info/PersonInfo's `info` still
// collide and stay entity-qualified (`Info::ty`) — mid-chain they're
// navigation methods anyway. Each `Entity / EntityNav` pair also names the generated
// navigation trait (see src/schema.rs). The FIRST field of each entity
// sizes its universe.

use crate::schema::schema;

schema! { JOB / JobSchema / job_init:
    // Universe is singular `movie` — Cast has no stored movie back-pointer
    // (movie→cast traversal uses the Movie.cast edge), so the name is free.
    // `persons` IS plural: `person` is the hot bare Cast.person handle.
    Movie(movie) / MovieNav {
        pub title: str,
        pub kind: Kind,
        pub production_year: Multi<i64>,
        pub episode_nr: Multi<i64>,
        pub keyword: Multi<Keyword>,
        pub company: Multi<Company>,
        pub cast: Multi<Cast>,
        pub info: Multi<Info>,
        pub data: Multi<Data>,
        pub complete_cast: Multi<CompleteCast>,
        pub link: Multi<MovieLink>,
        pub linked_by: Multi<MovieLink>,
        pub aka: Multi<AkaTitle>,
    }
    Cast / CastNav {
        pub person: Person,
        pub role: RoleType,
        note: Multi<str>,
        pub character: Multi<Character>,
    }
    Person(persons) / PersonNav {
        name: str,
        pub gender: Multi<str>,
        pub alias: Multi<AkaName>,
        pub bio: Multi<PersonInfo>,
        pub name_pcode_cf: Multi<str>,
    }
    Keyword / KeywordNav { text: str }
    Kind / KindNav { text: str }
    RoleType / RoleTypeNav { text: str }
    Character / CharacterNav { text: str }
    Company / CompanyNav {
        name: str,
        pub country: Multi<str>,
        note: Multi<str>,
        ty: CompanyType,
    }
    CompanyType / CompanyTypeNav { text: str }
    Info / InfoNav { info: str, ty: InfoType, note: Multi<str> }
    InfoType / InfoTypeNav { text: str }
    Data / DataNav { text: str, ty: InfoType }
    PersonInfo / PersonInfoNav { info: str, ty: InfoType, note: Multi<str> }
    AkaName / AkaNameNav { text: str }
    AkaTitle / AkaTitleNav { text: str }
    MovieLink / MovieLinkNav { pub target: Movie, ty: LinkType }
    LinkType / LinkTypeNav { text: str }
    CompleteCast / CompleteCastNav { pub status: CompCastType, pub subject: CompCastType }
    CompCastType / CompCastTypeNav { text: str }
}

// ===== tests — typed loading agrees with the untyped loaders =============

#[cfg(test)]
mod tests {
    // Selective imports, NOT `use super::*`: glob-importing a schema that
    // exports a bare `kind` handle breaks `assert_eq!` — the core macro
    // internally binds `let kind = AssertKind::…`, and an in-scope unit
    // struct named `kind` turns that binding pattern into a (mismatched)
    // const match. Bare handles capture ANY same-named binding pattern in
    // scope, so test modules import handles selectively/qualified.
    use super::{job_init, movie, persons, Data, KindNav, Movie};
    use crate::cache::{load_ids, load_multi_ids, load_strs};
    use crate::engine::{Drive, IntoQuery, Probe, QueryExt};
    use std::path::Path;

    /// Full typed init against the real cache, cross-checked column-by-
    /// column against the untyped v2 readers (lengths AND a value spot
    /// check through the `repr(transparent)` id reinterpret). Skipped when
    /// the cache isn't present (CI without regen output).
    #[test]
    fn typed_schema_matches_untyped_loaders() {
        let dir = Path::new("../cache");
        if !dir.join("Movie_title.bin").exists() {
            eprintln!("skipping: ../cache not present (run `regen job`)");
            return;
        }
        job_init(dir);

        // universe sizes = first-column key counts (handles resolve via iq)
        assert_eq!(movie.iq().n, load_strs("Movie_title").n_keys());
        assert_eq!(persons.iq().n, load_strs("Person_name").n_keys());

        // dense str / dense id / CSR id columns
        assert_eq!(Movie::title.iq().n_keys(), load_strs("Movie_title").n_keys());
        assert_eq!(Movie::kind.iq().n_keys(), load_ids("Movie_kind").n_keys());
        assert_eq!(Movie::keyword.iq().n_keys(), load_multi_ids("Movie_keyword").n_keys());
        assert_eq!(Data::ty.iq().n_keys(), load_ids("Data_ty").n_keys());

        // value parity through the bulk reinterpret: typed Id<Kind> indexes
        // equal the untyped words, row for row (first 1000 rows).
        let untyped = load_ids("Movie_kind");
        for i in 0..1000.min(untyped.n_keys()) {
            let mut got = None;
            Movie::kind.iq().probe(crate::engine::Id::new(i), |k| got = Some(k.0));
            assert_eq!(got, Some(untyped.values[i]));
        }

        // a typed end-to-end drive (nav spelling) matches the untyped one
        // (`super::kind` qualified — see the import note above)
        let typed = movie.with(super::kind.text().eq("movie"));
        let mut n_typed = 0usize;
        typed.drive(|_, _| n_typed += 1);
        let kk = load_strs("Kind_text");
        let mk = load_ids("Movie_kind");
        let mut n_untyped = 0usize;
        crate::engine::Universe::<usize>::new(mk.n_keys())
            .with((&mk).select(&kk).eq("movie"))
            .drive(|_, _| n_untyped += 1);
        assert_eq!(n_typed, n_untyped);
    }
}
