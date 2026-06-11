// The JOB tables, loaded from Julia's binary cache (../cache/*.bin) via the
// shared loaders in cache.rs.

use crate::cache::{ids, ids_fk, load_bits, load_strs, max_key, max_val};
use crate::engine::{Many, Universe, Vec1, NO_ID};

// ===== the loaded dataset ===============================================

pub struct Data {
    // universes
    pub movie:   Universe,
    pub persons: Universe,

    // Movie.* (movie → ...)
    pub movie_title:           Vec1<&'static str>,
    pub movie_kind:            Vec1<usize>,
    pub movie_production_year: Many<i64>,
    pub movie_episode_nr:      Many<i64>,
    pub movie_keyword:         Many<usize>,
    pub movie_company:         Many<usize>,
    pub movie_cast:            Many<usize>,
    pub movie_info:            Many<usize>,
    pub movie_data:            Many<usize>,
    pub movie_complete_cast:   Many<usize>,
    pub movie_link:            Many<usize>,
    pub movie_linked_by:       Many<usize>,
    pub movie_aka:             Many<usize>,

    // Cast.*
    pub cast_person:           Vec1<usize>,
    pub cast_role:             Vec1<usize>,
    pub cast_note:             Many<&'static str>,
    pub cast_character:        Many<usize>,

    // Person.*
    pub person_name:           Vec1<&'static str>,
    pub person_gender:         Many<&'static str>,
    pub person_aka:            Many<usize>,
    pub person_info:           Many<usize>,
    pub person_name_pcode:     Many<&'static str>,

    // Keyword, Kind, RoleType, Character
    pub keyword_keyword:       Vec1<&'static str>,
    pub kind_kind:             Vec1<&'static str>,
    pub roletype_role:         Vec1<&'static str>,
    pub character_name:        Vec1<&'static str>,

    // Company, CompanyType
    pub company_country:       Many<&'static str>,
    pub company_name:          Vec1<&'static str>,
    pub company_note:          Many<&'static str>,
    pub company_type:          Vec1<usize>,
    pub companytype_kind:      Vec1<&'static str>,

    // Info, Data, PersonInfo
    pub info_info:             Vec1<&'static str>,
    pub info_type:             Vec1<usize>,
    pub info_note:             Many<&'static str>,
    pub infotype_info:         Vec1<&'static str>,
    pub data_data:             Vec1<&'static str>,
    pub data_type:             Vec1<usize>,
    pub personinfo_info:       Vec1<&'static str>,
    pub personinfo_type:       Vec1<usize>,
    pub personinfo_note:       Many<&'static str>,

    // Aka tables
    pub akaname_name:          Vec1<&'static str>,
    pub akatitle_title:        Vec1<&'static str>,

    // MovieLink, LinkType
    pub movielink_target:      Vec1<usize>,
    pub movielink_type:        Vec1<usize>,
    pub linktype_link:         Vec1<&'static str>,

    // CompleteCast, CompCastType
    pub completecast_status:   Vec1<usize>,
    pub completecast_subject:  Vec1<usize>,
    pub compcasttype_kind:     Vec1<&'static str>,
}

impl Data {
    pub fn load() -> Self {
        // ---- bits ----
        let py    = load_bits("Movie_production_year");
        let men   = load_bits("Movie_episode_nr");
        let mki   = load_bits("Movie_kind");
        let mk    = load_bits("Movie_keyword");
        let mcmp  = load_bits("Movie_company");
        let mcst  = load_bits("Movie_cast");
        let mif   = load_bits("Movie_info");
        let mdt   = load_bits("Movie_data");
        let mcc   = load_bits("Movie_complete_cast");
        let mln   = load_bits("Movie_link");
        let mlnby = load_bits("Movie_linked_by");
        let mak   = load_bits("Movie_aka");
        let cp    = load_bits("Cast_person");
        let cr    = load_bits("Cast_role");
        let cch   = load_bits("Cast_character");
        let pa    = load_bits("Person_aka");
        let pif   = load_bits("Person_info");
        let cty   = load_bits("Company_type");
        let ity   = load_bits("Info_type");
        let dty   = load_bits("Data_type");
        let pity  = load_bits("PersonInfo_type");
        let mlt   = load_bits("MovieLink_target");
        let mlty  = load_bits("MovieLink_type");
        let ccst  = load_bits("CompleteCast_status");
        let ccsub = load_bits("CompleteCast_subject");

        // ---- strs ----
        let mt   = load_strs("Movie_title");
        let kk   = load_strs("Keyword_keyword");
        let kik  = load_strs("Kind_kind");
        let rt   = load_strs("RoleType_role");
        let chn  = load_strs("Character_name");
        let cc   = load_strs("Company_country");
        let cmn_ = load_strs("Company_name");
        let cmnt = load_strs("Company_note");
        let cyk  = load_strs("CompanyType_kind");
        let ii   = load_strs("Info_info");
        let in_  = load_strs("Info_note");
        let ityp = load_strs("InfoType_info");
        let dd   = load_strs("Data_data");
        let pi   = load_strs("PersonInfo_info");
        let pin  = load_strs("PersonInfo_note");
        let pn   = load_strs("Person_name");
        let pg   = load_strs("Person_gender");
        let pnp  = load_strs("Person_name_pcode_cf");
        let cnt  = load_strs("Cast_note");
        let an   = load_strs("AkaName_name");
        let at   = load_strs("AkaTitle_title");
        let lty  = load_strs("LinkType_link");
        let cck  = load_strs("CompCastType_kind");

        // ---- entity sizes (max id seen across all references) ----
        let n_movie     = max_key(&mt)
                          .max(max_val(&mlt));
        let n_person    = max_key(&pn);
        let n_cast      = max_key(&cp);
        let n_keyword   = max_key(&kk)
                          .max(max_val(&mk));
        let n_kind      = max_key(&kik).max(max_val(&mki));
        let n_roletype  = max_key(&rt).max(max_val(&cr));
        let n_character = max_key(&chn).max(max_val(&cch));
        let n_company   = max_key(&cmn_)
                          .max(max_key(&cc))
                          .max(max_key(&cmnt))
                          .max(max_key(&cty))
                          .max(max_val(&mcmp));
        let n_comptype  = max_key(&cyk).max(max_val(&cty));
        let n_info      = max_key(&ii)
                          .max(max_key(&ity))
                          .max(max_key(&in_))
                          .max(max_val(&mif));
        let n_infotype  = max_key(&ityp).max(max_val(&ity));
        let n_data      = max_key(&dd)
                          .max(max_key(&dty))
                          .max(max_val(&mdt));
        let n_pinfo     = max_key(&pi)
                          .max(max_key(&pity))
                          .max(max_key(&pin))
                          .max(max_val(&pif));
        let n_akaname   = max_key(&an).max(max_val(&pa));
        let n_akatitle  = max_key(&at).max(max_val(&mak));
        let n_mlink     = max_key(&mlt)
                          .max(max_key(&mlty))
                          .max(max_val(&mln))
                          .max(max_val(&mlnby));
        let n_ltype     = max_key(&lty).max(max_val(&mlty));
        let n_ccast     = max_key(&ccst)
                          .max(max_key(&ccsub))
                          .max(max_val(&mcc));
        let n_ccktype   = max_key(&cck)
                          .max(max_val(&ccst))
                          .max(max_val(&ccsub));

        Data {
            movie:   Universe { n: n_movie  },
            persons: Universe { n: n_person },

            // `ids` shifts keys to 0-based; `ids_fk` also shifts the value
            // (FK columns). Year/episode-nr/string values are untouched.
            // FK-valued Vec1 columns fill holes with NO_ID (a dead id) so a
            // key with no row never aliases entity 0 — see the Vec1 invariant.
            movie_title:           Vec1::from_pairs(n_movie, ids(&mt)),
            movie_kind:            Vec1::from_pairs_fill(n_movie, NO_ID, ids_fk(&mki)),
            movie_production_year: Many::from_pairs(n_movie, ids(&py)),
            movie_episode_nr:      Many::from_pairs(n_movie, ids(&men)),
            movie_keyword:         Many::from_pairs(n_movie, ids_fk(&mk)),
            movie_company:         Many::from_pairs(n_movie, ids_fk(&mcmp)),
            movie_cast:            Many::from_pairs(n_movie, ids_fk(&mcst)),
            movie_info:            Many::from_pairs(n_movie, ids_fk(&mif)),
            movie_data:            Many::from_pairs(n_movie, ids_fk(&mdt)),
            movie_complete_cast:   Many::from_pairs(n_movie, ids_fk(&mcc)),
            movie_link:            Many::from_pairs(n_movie, ids_fk(&mln)),
            movie_linked_by:       Many::from_pairs(n_movie, ids_fk(&mlnby)),
            movie_aka:             Many::from_pairs(n_movie, ids_fk(&mak)),

            cast_person:     Vec1::from_pairs_fill(n_cast, NO_ID, ids_fk(&cp)),
            cast_role:       Vec1::from_pairs_fill(n_cast, NO_ID, ids_fk(&cr)),
            cast_note:       Many::from_pairs(n_cast, ids(&cnt)),
            cast_character:  Many::from_pairs(n_cast, ids_fk(&cch)),

            person_name:       Vec1::from_pairs(n_person, ids(&pn)),
            person_gender:     Many::from_pairs(n_person, ids(&pg)),
            person_aka:        Many::from_pairs(n_person, ids_fk(&pa)),
            person_info:       Many::from_pairs(n_person, ids_fk(&pif)),
            person_name_pcode: Many::from_pairs(n_person, ids(&pnp)),

            keyword_keyword: Vec1::from_pairs(n_keyword,   ids(&kk)),
            kind_kind:       Vec1::from_pairs(n_kind,      ids(&kik)),
            roletype_role:   Vec1::from_pairs(n_roletype,  ids(&rt)),
            character_name:  Vec1::from_pairs(n_character, ids(&chn)),

            company_country: Many::from_pairs(n_company, ids(&cc)),
            company_name:    Vec1::from_pairs(n_company, ids(&cmn_)),
            company_note:    Many::from_pairs(n_company, ids(&cmnt)),
            company_type:    Vec1::from_pairs_fill(n_company, NO_ID, ids_fk(&cty)),
            companytype_kind: Vec1::from_pairs(n_comptype, ids(&cyk)),

            info_info:    Vec1::from_pairs(n_info,     ids(&ii)),
            info_type:    Vec1::from_pairs_fill(n_info, NO_ID,     ids_fk(&ity)),
            info_note:    Many::from_pairs(n_info,     ids(&in_)),
            infotype_info: Vec1::from_pairs(n_infotype, ids(&ityp)),
            data_data:    Vec1::from_pairs(n_data,     ids(&dd)),
            data_type:    Vec1::from_pairs_fill(n_data, NO_ID,     ids_fk(&dty)),
            personinfo_info: Vec1::from_pairs(n_pinfo,  ids(&pi)),
            personinfo_type: Vec1::from_pairs_fill(n_pinfo, NO_ID,  ids_fk(&pity)),
            personinfo_note: Many::from_pairs(n_pinfo,  ids(&pin)),

            akaname_name:    Vec1::from_pairs(n_akaname,  ids(&an)),
            akatitle_title:  Vec1::from_pairs(n_akatitle, ids(&at)),

            movielink_target: Vec1::from_pairs_fill(n_mlink, NO_ID, ids_fk(&mlt)),
            movielink_type:   Vec1::from_pairs_fill(n_mlink, NO_ID, ids_fk(&mlty)),
            linktype_link:    Vec1::from_pairs(n_ltype, ids(&lty)),

            completecast_status:  Vec1::from_pairs_fill(n_ccast, NO_ID, ids_fk(&ccst)),
            completecast_subject: Vec1::from_pairs_fill(n_ccast, NO_ID, ids_fk(&ccsub)),
            compcasttype_kind:    Vec1::from_pairs(n_ccktype, ids(&cck)),
        }
    }
}
