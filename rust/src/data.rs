// The JOB tables, loaded from the binary cache (../cache/*.bin, produced
// by `regen job`; originally by Julia's JOB.jl/cache.jl — julia-engine
// branch) via the shared loaders in cache.rs.

use crate::cache::{ids, ids_fk, load_bits, load_strs, max_key, max_val};
use crate::engine::{MultiRel, Universe, VecRel, NO_ID};

// ===== the loaded dataset ===============================================

pub struct Data {
    // universes
    pub movie:   Universe,
    pub persons: Universe,

    // Movie.* (movie → ...)
    pub movie_title:           VecRel<&'static str>,
    pub movie_kind:            VecRel<usize>,
    pub movie_production_year: MultiRel<i64>,
    pub movie_episode_nr:      MultiRel<i64>,
    pub movie_keyword:         MultiRel<usize>,
    pub movie_company:         MultiRel<usize>,
    pub movie_cast:            MultiRel<usize>,
    pub movie_info:            MultiRel<usize>,
    pub movie_data:            MultiRel<usize>,
    pub movie_complete_cast:   MultiRel<usize>,
    pub movie_link:            MultiRel<usize>,
    pub movie_linked_by:       MultiRel<usize>,
    pub movie_aka:             MultiRel<usize>,

    // Cast.*
    pub cast_person:           VecRel<usize>,
    pub cast_role:             VecRel<usize>,
    pub cast_note:             MultiRel<&'static str>,
    pub cast_character:        MultiRel<usize>,

    // Person.*
    pub person_name:           VecRel<&'static str>,
    pub person_gender:         MultiRel<&'static str>,
    pub person_aka:            MultiRel<usize>,
    pub person_info:           MultiRel<usize>,
    pub person_name_pcode:     MultiRel<&'static str>,

    // Keyword, Kind, RoleType, Character
    pub keyword_keyword:       VecRel<&'static str>,
    pub kind_kind:             VecRel<&'static str>,
    pub roletype_role:         VecRel<&'static str>,
    pub character_name:        VecRel<&'static str>,

    // Company, CompanyType
    pub company_country:       MultiRel<&'static str>,
    pub company_name:          VecRel<&'static str>,
    pub company_note:          MultiRel<&'static str>,
    pub company_type:          VecRel<usize>,
    pub companytype_kind:      VecRel<&'static str>,

    // Info, Data, PersonInfo
    pub info_info:             VecRel<&'static str>,
    pub info_type:             VecRel<usize>,
    pub info_note:             MultiRel<&'static str>,
    pub infotype_info:         VecRel<&'static str>,
    pub data_data:             VecRel<&'static str>,
    pub data_type:             VecRel<usize>,
    pub personinfo_info:       VecRel<&'static str>,
    pub personinfo_type:       VecRel<usize>,
    pub personinfo_note:       MultiRel<&'static str>,

    // Aka tables
    pub akaname_name:          VecRel<&'static str>,
    pub akatitle_title:        VecRel<&'static str>,

    // MovieLink, LinkType
    pub movielink_target:      VecRel<usize>,
    pub movielink_type:        VecRel<usize>,
    pub linktype_link:         VecRel<&'static str>,

    // CompleteCast, CompCastType
    pub completecast_status:   VecRel<usize>,
    pub completecast_subject:  VecRel<usize>,
    pub compcasttype_kind:     VecRel<&'static str>,
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
            // FK-valued VecRel columns fill holes with NO_ID (a dead id) so a
            // key with no row never aliases entity 0 — see the VecRel invariant.
            movie_title:           VecRel::from_pairs(n_movie, ids(&mt)),
            movie_kind:            VecRel::from_pairs_fill(n_movie, NO_ID, ids_fk(&mki)),
            movie_production_year: MultiRel::from_pairs(n_movie, ids(&py)),
            movie_episode_nr:      MultiRel::from_pairs(n_movie, ids(&men)),
            movie_keyword:         MultiRel::from_pairs(n_movie, ids_fk(&mk)),
            movie_company:         MultiRel::from_pairs(n_movie, ids_fk(&mcmp)),
            movie_cast:            MultiRel::from_pairs(n_movie, ids_fk(&mcst)),
            movie_info:            MultiRel::from_pairs(n_movie, ids_fk(&mif)),
            movie_data:            MultiRel::from_pairs(n_movie, ids_fk(&mdt)),
            movie_complete_cast:   MultiRel::from_pairs(n_movie, ids_fk(&mcc)),
            movie_link:            MultiRel::from_pairs(n_movie, ids_fk(&mln)),
            movie_linked_by:       MultiRel::from_pairs(n_movie, ids_fk(&mlnby)),
            movie_aka:             MultiRel::from_pairs(n_movie, ids_fk(&mak)),

            cast_person:     VecRel::from_pairs_fill(n_cast, NO_ID, ids_fk(&cp)),
            cast_role:       VecRel::from_pairs_fill(n_cast, NO_ID, ids_fk(&cr)),
            cast_note:       MultiRel::from_pairs(n_cast, ids(&cnt)),
            cast_character:  MultiRel::from_pairs(n_cast, ids_fk(&cch)),

            person_name:       VecRel::from_pairs(n_person, ids(&pn)),
            person_gender:     MultiRel::from_pairs(n_person, ids(&pg)),
            person_aka:        MultiRel::from_pairs(n_person, ids_fk(&pa)),
            person_info:       MultiRel::from_pairs(n_person, ids_fk(&pif)),
            person_name_pcode: MultiRel::from_pairs(n_person, ids(&pnp)),

            keyword_keyword: VecRel::from_pairs(n_keyword,   ids(&kk)),
            kind_kind:       VecRel::from_pairs(n_kind,      ids(&kik)),
            roletype_role:   VecRel::from_pairs(n_roletype,  ids(&rt)),
            character_name:  VecRel::from_pairs(n_character, ids(&chn)),

            company_country: MultiRel::from_pairs(n_company, ids(&cc)),
            company_name:    VecRel::from_pairs(n_company, ids(&cmn_)),
            company_note:    MultiRel::from_pairs(n_company, ids(&cmnt)),
            company_type:    VecRel::from_pairs_fill(n_company, NO_ID, ids_fk(&cty)),
            companytype_kind: VecRel::from_pairs(n_comptype, ids(&cyk)),

            info_info:    VecRel::from_pairs(n_info,     ids(&ii)),
            info_type:    VecRel::from_pairs_fill(n_info, NO_ID,     ids_fk(&ity)),
            info_note:    MultiRel::from_pairs(n_info,     ids(&in_)),
            infotype_info: VecRel::from_pairs(n_infotype, ids(&ityp)),
            data_data:    VecRel::from_pairs(n_data,     ids(&dd)),
            data_type:    VecRel::from_pairs_fill(n_data, NO_ID,     ids_fk(&dty)),
            personinfo_info: VecRel::from_pairs(n_pinfo,  ids(&pi)),
            personinfo_type: VecRel::from_pairs_fill(n_pinfo, NO_ID,  ids_fk(&pity)),
            personinfo_note: MultiRel::from_pairs(n_pinfo,  ids(&pin)),

            akaname_name:    VecRel::from_pairs(n_akaname,  ids(&an)),
            akatitle_title:  VecRel::from_pairs(n_akatitle, ids(&at)),

            movielink_target: VecRel::from_pairs_fill(n_mlink, NO_ID, ids_fk(&mlt)),
            movielink_type:   VecRel::from_pairs_fill(n_mlink, NO_ID, ids_fk(&mlty)),
            linktype_link:    VecRel::from_pairs(n_ltype, ids(&lty)),

            completecast_status:  VecRel::from_pairs_fill(n_ccast, NO_ID, ids_fk(&ccst)),
            completecast_subject: VecRel::from_pairs_fill(n_ccast, NO_ID, ids_fk(&ccsub)),
            compcasttype_kind:    VecRel::from_pairs(n_ccktype, ids(&cck)),
        }
    }
}
