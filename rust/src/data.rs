// The JOB tables, loaded from Julia's binary cache (../cache/*.bin) via the
// shared loaders in cache.rs.

use crate::cache::{load_bits, load_strs, max_key, max_val};
use crate::engine::{Many, Universe, Vec1};

// ===== the loaded dataset ===============================================

pub struct Data {
    // universes
    pub movie:   Universe,
    pub persons: Universe,

    // Movie.* (movie → ...)
    pub movie_title:           Vec1<&'static str>,
    pub movie_kind:            Vec1<i64>,
    pub movie_production_year: Many<i64>,
    pub movie_episode_nr:      Many<i64>,
    pub movie_keyword:         Many<i64>,
    pub movie_company:         Many<i64>,
    pub movie_cast:            Many<i64>,
    pub movie_info:            Many<i64>,
    pub movie_data:            Many<i64>,
    pub movie_complete_cast:   Many<i64>,
    pub movie_link:            Many<i64>,
    pub movie_linked_by:       Many<i64>,
    pub movie_aka:             Many<i64>,

    // Cast.*
    pub cast_person:           Vec1<i64>,
    pub cast_role:             Vec1<i64>,
    pub cast_note:             Many<&'static str>,
    pub cast_character:        Many<i64>,

    // Person.*
    pub person_name:           Vec1<&'static str>,
    pub person_gender:         Many<&'static str>,
    pub person_aka:            Many<i64>,
    pub person_info:           Many<i64>,
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
    pub company_type:          Vec1<i64>,
    pub companytype_kind:      Vec1<&'static str>,

    // Info, Data, PersonInfo
    pub info_info:             Vec1<&'static str>,
    pub info_type:             Vec1<i64>,
    pub info_note:             Many<&'static str>,
    pub infotype_info:         Vec1<&'static str>,
    pub data_data:             Vec1<&'static str>,
    pub data_type:             Vec1<i64>,
    pub personinfo_info:       Vec1<&'static str>,
    pub personinfo_type:       Vec1<i64>,
    pub personinfo_note:       Many<&'static str>,

    // Aka tables
    pub akaname_name:          Vec1<&'static str>,
    pub akatitle_title:        Vec1<&'static str>,

    // MovieLink, LinkType
    pub movielink_target:      Vec1<i64>,
    pub movielink_type:        Vec1<i64>,
    pub linktype_link:         Vec1<&'static str>,

    // CompleteCast, CompCastType
    pub completecast_status:   Vec1<i64>,
    pub completecast_subject:  Vec1<i64>,
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
            movie:   Universe { n: n_movie  as i64 },
            persons: Universe { n: n_person as i64 },

            movie_title:           Vec1::from_pairs(n_movie, mt.iter().copied()),
            movie_kind:            Vec1::from_pairs(n_movie, mki.iter().copied()),
            movie_production_year: Many::from_pairs(n_movie, py.iter().copied()),
            movie_episode_nr:      Many::from_pairs(n_movie, men.iter().copied()),
            movie_keyword:         Many::from_pairs(n_movie, mk.iter().copied()),
            movie_company:         Many::from_pairs(n_movie, mcmp.iter().copied()),
            movie_cast:            Many::from_pairs(n_movie, mcst.iter().copied()),
            movie_info:            Many::from_pairs(n_movie, mif.iter().copied()),
            movie_data:            Many::from_pairs(n_movie, mdt.iter().copied()),
            movie_complete_cast:   Many::from_pairs(n_movie, mcc.iter().copied()),
            movie_link:            Many::from_pairs(n_movie, mln.iter().copied()),
            movie_linked_by:       Many::from_pairs(n_movie, mlnby.iter().copied()),
            movie_aka:             Many::from_pairs(n_movie, mak.iter().copied()),

            cast_person:     Vec1::from_pairs(n_cast, cp.iter().copied()),
            cast_role:       Vec1::from_pairs(n_cast, cr.iter().copied()),
            cast_note:       Many::from_pairs(n_cast, cnt.iter().copied()),
            cast_character:  Many::from_pairs(n_cast, cch.iter().copied()),

            person_name:       Vec1::from_pairs(n_person, pn.iter().copied()),
            person_gender:     Many::from_pairs(n_person, pg.iter().copied()),
            person_aka:        Many::from_pairs(n_person, pa.iter().copied()),
            person_info:       Many::from_pairs(n_person, pif.iter().copied()),
            person_name_pcode: Many::from_pairs(n_person, pnp.iter().copied()),

            keyword_keyword: Vec1::from_pairs(n_keyword,   kk.iter().copied()),
            kind_kind:       Vec1::from_pairs(n_kind,      kik.iter().copied()),
            roletype_role:   Vec1::from_pairs(n_roletype,  rt.iter().copied()),
            character_name:  Vec1::from_pairs(n_character, chn.iter().copied()),

            company_country: Many::from_pairs(n_company, cc.iter().copied()),
            company_name:    Vec1::from_pairs(n_company, cmn_.iter().copied()),
            company_note:    Many::from_pairs(n_company, cmnt.iter().copied()),
            company_type:    Vec1::from_pairs(n_company, cty.iter().copied()),
            companytype_kind: Vec1::from_pairs(n_comptype, cyk.iter().copied()),

            info_info:    Vec1::from_pairs(n_info,     ii.iter().copied()),
            info_type:    Vec1::from_pairs(n_info,     ity.iter().copied()),
            info_note:    Many::from_pairs(n_info,     in_.iter().copied()),
            infotype_info: Vec1::from_pairs(n_infotype, ityp.iter().copied()),
            data_data:    Vec1::from_pairs(n_data,     dd.iter().copied()),
            data_type:    Vec1::from_pairs(n_data,     dty.iter().copied()),
            personinfo_info: Vec1::from_pairs(n_pinfo,  pi.iter().copied()),
            personinfo_type: Vec1::from_pairs(n_pinfo,  pity.iter().copied()),
            personinfo_note: Many::from_pairs(n_pinfo,  pin.iter().copied()),

            akaname_name:    Vec1::from_pairs(n_akaname,  an.iter().copied()),
            akatitle_title:  Vec1::from_pairs(n_akatitle, at.iter().copied()),

            movielink_target: Vec1::from_pairs(n_mlink, mlt.iter().copied()),
            movielink_type:   Vec1::from_pairs(n_mlink, mlty.iter().copied()),
            linktype_link:    Vec1::from_pairs(n_ltype, lty.iter().copied()),

            completecast_status:  Vec1::from_pairs(n_ccast, ccst.iter().copied()),
            completecast_subject: Vec1::from_pairs(n_ccast, ccsub.iter().copied()),
            compcasttype_kind:    Vec1::from_pairs(n_ccktype, cck.iter().copied()),
        }
    }
}
