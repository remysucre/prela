// The JOB tables, loaded from the v2 binary cache (../cache/*.bin,
// produced by `regen job` — see src/format.rs for the format). Every file
// already holds the final physical layout (0-based ids, NO_ID holes, CSR),
// so loading is just "read each file into its typed field"; universe sizes
// are the column lengths.

use crate::cache::{load_ids, load_multi_i64, load_multi_ids, load_multi_strs, load_strs};
use crate::engine::{MultiRel, Universe, VecRel};

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
        let d = Data {
            movie:   Universe { n: 0 }, // patched below from column lengths
            persons: Universe { n: 0 },

            movie_title:           load_strs("Movie_title"),
            movie_kind:            load_ids("Movie_kind"),
            movie_production_year: load_multi_i64("Movie_production_year"),
            movie_episode_nr:      load_multi_i64("Movie_episode_nr"),
            movie_keyword:         load_multi_ids("Movie_keyword"),
            movie_company:         load_multi_ids("Movie_company"),
            movie_cast:            load_multi_ids("Movie_cast"),
            movie_info:            load_multi_ids("Movie_info"),
            movie_data:            load_multi_ids("Movie_data"),
            movie_complete_cast:   load_multi_ids("Movie_complete_cast"),
            movie_link:            load_multi_ids("Movie_link"),
            movie_linked_by:       load_multi_ids("Movie_linked_by"),
            movie_aka:             load_multi_ids("Movie_aka"),

            cast_person:    load_ids("Cast_person"),
            cast_role:      load_ids("Cast_role"),
            cast_note:      load_multi_strs("Cast_note"),
            cast_character: load_multi_ids("Cast_character"),

            person_name:       load_strs("Person_name"),
            person_gender:     load_multi_strs("Person_gender"),
            person_aka:        load_multi_ids("Person_aka"),
            person_info:       load_multi_ids("Person_info"),
            person_name_pcode: load_multi_strs("Person_name_pcode_cf"),

            keyword_keyword: load_strs("Keyword_keyword"),
            kind_kind:       load_strs("Kind_kind"),
            roletype_role:   load_strs("RoleType_role"),
            character_name:  load_strs("Character_name"),

            company_country:  load_multi_strs("Company_country"),
            company_name:     load_strs("Company_name"),
            company_note:     load_multi_strs("Company_note"),
            company_type:     load_ids("Company_type"),
            companytype_kind: load_strs("CompanyType_kind"),

            info_info:       load_strs("Info_info"),
            info_type:       load_ids("Info_type"),
            info_note:       load_multi_strs("Info_note"),
            infotype_info:   load_strs("InfoType_info"),
            data_data:       load_strs("Data_data"),
            data_type:       load_ids("Data_type"),
            personinfo_info: load_strs("PersonInfo_info"),
            personinfo_type: load_ids("PersonInfo_type"),
            personinfo_note: load_multi_strs("PersonInfo_note"),

            akaname_name:   load_strs("AkaName_name"),
            akatitle_title: load_strs("AkaTitle_title"),

            movielink_target: load_ids("MovieLink_target"),
            movielink_type:   load_ids("MovieLink_type"),
            linktype_link:    load_strs("LinkType_link"),

            completecast_status:  load_ids("CompleteCast_status"),
            completecast_subject: load_ids("CompleteCast_subject"),
            compcasttype_kind:    load_strs("CompCastType_kind"),
        };

        // Universe sizes ARE the dense column lengths (regen sizes every
        // column of an entity to the same n); cross-check a few siblings.
        let movie = Universe { n: d.movie_title.values.len() };
        let persons = Universe { n: d.person_name.values.len() };
        assert_eq!(movie.n + 1, d.movie_cast.offsets.len());
        assert_eq!(d.cast_person.values.len() + 1, d.cast_note.offsets.len());
        assert_eq!(persons.n + 1, d.person_aka.offsets.len());
        Data { movie, persons, ..d }
    }
}
