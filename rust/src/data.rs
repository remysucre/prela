// Load the JOB tables from Julia's binary cache (../prela/cache/*.bin).
//
// File formats (from cache.jl):
//   bits  — [u64 n][n × (i64,i64)]         ID×ID or ID×Int pairs
//   str   — [u64 n][n × i64 keys][(n+1) × u32 offsets][bytes]
//
// Strings are returned as &'static str — the mmap is leaked, so the bytes live
// for the program. No per-string allocation.

#![allow(dead_code)]

use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::engine::{Many, Universe, Vec1};

fn cache_dir() -> PathBuf {
    PathBuf::from("../cache")
}

fn mmap_static(path: &Path) -> &'static [u8] {
    let f = File::open(path).unwrap_or_else(|e| panic!("open {path:?}: {e}"));
    let mmap = unsafe { Mmap::map(&f).unwrap() };
    let leaked: &'static Mmap = Box::leak(Box::new(mmap));
    &**leaked
}

fn load_bits(name: &str) -> &'static [[i64; 2]] {
    let bytes = mmap_static(&cache_dir().join(format!("{name}.bin")));
    let n = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    let ptr = unsafe { bytes.as_ptr().add(8) as *const [i64; 2] };
    unsafe { std::slice::from_raw_parts(ptr, n) }
}

fn load_strs(name: &str) -> Vec<(i64, &'static str)> {
    let bytes = mmap_static(&cache_dir().join(format!("{name}.bin")));
    let n = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    let keys_off = 8;
    let offsets_off = keys_off + n * 8;
    let bytes_off = offsets_off + (n + 1) * 4;
    let keys: &'static [i64] = unsafe {
        std::slice::from_raw_parts(bytes.as_ptr().add(keys_off) as *const i64, n)
    };
    let offsets: &'static [u32] = unsafe {
        std::slice::from_raw_parts(bytes.as_ptr().add(offsets_off) as *const u32, n + 1)
    };
    let data: &'static [u8] = &bytes[bytes_off..];
    (0..n).map(|i| {
        let lo = offsets[i] as usize;
        let hi = offsets[i + 1] as usize;
        let s: &'static str = unsafe { std::str::from_utf8_unchecked(&data[lo..hi]) };
        (keys[i], s)
    }).collect()
}

fn max_src_bits(p: &[[i64; 2]]) -> usize {
    p.iter().map(|x| x[0]).max().unwrap_or(0) as usize
}
fn max_src_str(p: &[(i64, &str)]) -> usize {
    p.iter().map(|x| x.0).max().unwrap_or(0) as usize
}
fn max_val_bits(p: &[[i64; 2]]) -> usize {
    p.iter().map(|x| x[1]).max().unwrap_or(0) as usize
}

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
        let n_movie     = max_src_str(&mt)
                          .max(mlt.iter().map(|x| x[1]).max().unwrap_or(0) as usize);
        let n_person    = max_src_str(&pn);
        let n_cast      = max_src_bits(cp);
        let n_keyword   = max_src_str(&kk)
                          .max(max_val_bits(mk));
        let n_kind      = max_src_str(&kik).max(max_val_bits(mki));
        let n_roletype  = max_src_str(&rt).max(max_val_bits(cr));
        let n_character = max_src_str(&chn).max(max_val_bits(cch));
        let n_company   = max_src_str(&cmn_)
                          .max(max_src_str(&cc))
                          .max(max_src_str(&cmnt))
                          .max(max_src_bits(cty))
                          .max(max_val_bits(mcmp));
        let n_comptype  = max_src_str(&cyk).max(max_val_bits(cty));
        let n_info      = max_src_str(&ii)
                          .max(max_src_bits(ity))
                          .max(max_src_str(&in_))
                          .max(max_val_bits(mif));
        let n_infotype  = max_src_str(&ityp).max(max_val_bits(ity));
        let n_data      = max_src_str(&dd)
                          .max(max_src_bits(dty))
                          .max(max_val_bits(mdt));
        let n_pinfo     = max_src_str(&pi)
                          .max(max_src_bits(pity))
                          .max(max_src_str(&pin))
                          .max(max_val_bits(pif));
        let n_akaname   = max_src_str(&an).max(max_val_bits(pa));
        let n_akatitle  = max_src_str(&at).max(max_val_bits(mak));
        let n_mlink     = max_src_bits(mlt)
                          .max(max_src_bits(mlty))
                          .max(max_val_bits(mln))
                          .max(max_val_bits(mlnby));
        let n_ltype     = max_src_str(&lty).max(max_val_bits(mlty));
        let n_ccast     = max_src_bits(ccst)
                          .max(max_src_bits(ccsub))
                          .max(max_val_bits(mcc));
        let n_ccktype   = max_src_str(&cck)
                          .max(ccst.iter().map(|x| x[1]).max().unwrap_or(0) as usize)
                          .max(ccsub.iter().map(|x| x[1]).max().unwrap_or(0) as usize);

        Data {
            movie:   Universe { n: n_movie  as i64 },
            persons: Universe { n: n_person as i64 },

            movie_title:           Vec1::from_pairs(n_movie, mt.iter().copied()),
            movie_kind:            Vec1::from_pairs(n_movie, mki.iter().map(|p| (p[0], p[1]))),
            movie_production_year: Many::from_pairs(n_movie, py.iter().map(|p| (p[0], p[1]))),
            movie_episode_nr:      Many::from_pairs(n_movie, men.iter().map(|p| (p[0], p[1]))),
            movie_keyword:         Many::from_pairs(n_movie, mk.iter().map(|p| (p[0], p[1]))),
            movie_company:         Many::from_pairs(n_movie, mcmp.iter().map(|p| (p[0], p[1]))),
            movie_cast:            Many::from_pairs(n_movie, mcst.iter().map(|p| (p[0], p[1]))),
            movie_info:            Many::from_pairs(n_movie, mif.iter().map(|p| (p[0], p[1]))),
            movie_data:            Many::from_pairs(n_movie, mdt.iter().map(|p| (p[0], p[1]))),
            movie_complete_cast:   Many::from_pairs(n_movie, mcc.iter().map(|p| (p[0], p[1]))),
            movie_link:            Many::from_pairs(n_movie, mln.iter().map(|p| (p[0], p[1]))),
            movie_linked_by:       Many::from_pairs(n_movie, mlnby.iter().map(|p| (p[0], p[1]))),
            movie_aka:             Many::from_pairs(n_movie, mak.iter().map(|p| (p[0], p[1]))),

            cast_person:     Vec1::from_pairs(n_cast, cp.iter().map(|p| (p[0], p[1]))),
            cast_role:       Vec1::from_pairs(n_cast, cr.iter().map(|p| (p[0], p[1]))),
            cast_note:       Many::from_pairs(n_cast, cnt.iter().copied()),
            cast_character:  Many::from_pairs(n_cast, cch.iter().map(|p| (p[0], p[1]))),

            person_name:       Vec1::from_pairs(n_person, pn.iter().copied()),
            person_gender:     Many::from_pairs(n_person, pg.iter().copied()),
            person_aka:        Many::from_pairs(n_person, pa.iter().map(|p| (p[0], p[1]))),
            person_info:       Many::from_pairs(n_person, pif.iter().map(|p| (p[0], p[1]))),
            person_name_pcode: Many::from_pairs(n_person, pnp.iter().copied()),

            keyword_keyword: Vec1::from_pairs(n_keyword,   kk.iter().copied()),
            kind_kind:       Vec1::from_pairs(n_kind,      kik.iter().copied()),
            roletype_role:   Vec1::from_pairs(n_roletype,  rt.iter().copied()),
            character_name:  Vec1::from_pairs(n_character, chn.iter().copied()),

            company_country: Many::from_pairs(n_company, cc.iter().copied()),
            company_name:    Vec1::from_pairs(n_company, cmn_.iter().copied()),
            company_note:    Many::from_pairs(n_company, cmnt.iter().copied()),
            company_type:    Vec1::from_pairs(n_company, cty.iter().map(|p| (p[0], p[1]))),
            companytype_kind: Vec1::from_pairs(n_comptype, cyk.iter().copied()),

            info_info:    Vec1::from_pairs(n_info,     ii.iter().copied()),
            info_type:    Vec1::from_pairs(n_info,     ity.iter().map(|p| (p[0], p[1]))),
            info_note:    Many::from_pairs(n_info,     in_.iter().copied()),
            infotype_info: Vec1::from_pairs(n_infotype, ityp.iter().copied()),
            data_data:    Vec1::from_pairs(n_data,     dd.iter().copied()),
            data_type:    Vec1::from_pairs(n_data,     dty.iter().map(|p| (p[0], p[1]))),
            personinfo_info: Vec1::from_pairs(n_pinfo,  pi.iter().copied()),
            personinfo_type: Vec1::from_pairs(n_pinfo,  pity.iter().map(|p| (p[0], p[1]))),
            personinfo_note: Many::from_pairs(n_pinfo,  pin.iter().copied()),

            akaname_name:    Vec1::from_pairs(n_akaname,  an.iter().copied()),
            akatitle_title:  Vec1::from_pairs(n_akatitle, at.iter().copied()),

            movielink_target: Vec1::from_pairs(n_mlink, mlt.iter().map(|p| (p[0], p[1]))),
            movielink_type:   Vec1::from_pairs(n_mlink, mlty.iter().map(|p| (p[0], p[1]))),
            linktype_link:    Vec1::from_pairs(n_ltype, lty.iter().copied()),

            completecast_status:  Vec1::from_pairs(n_ccast, ccst.iter().map(|p| (p[0], p[1]))),
            completecast_subject: Vec1::from_pairs(n_ccast, ccsub.iter().map(|p| (p[0], p[1]))),
            compcasttype_kind:    Vec1::from_pairs(n_ccktype, cck.iter().copied()),
        }
    }
}
