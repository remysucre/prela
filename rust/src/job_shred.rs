//! Shared shredding of normalized JOB rows into Prela relations.
//!
//! The Parquet regeneration binary and generated differential tests feed the
//! same row methods below. A missing (`None`) value emits no pair, which is the
//! single definition of how SQL `NULL` is represented by the JOB relations.

use crate::cache_writer::{CacheColumn, StringColumn, WordColumn};
#[cfg(feature = "test")]
use crate::cache_writer::{CacheColumns, MemoryColumn};
#[cfg(feature = "test")]
use crate::engine::{DictMultiRel, DictRel, Id, MultiRel, Universe, VecRel};
use crate::format::{KIND_DENSE_I64, NO_ID_WORD};
#[cfg(feature = "test")]
use crate::job_schema::{Job, JobSource};
#[cfg(feature = "test")]
use crate::loader::{Col, Dict, DictSet, Key, Set, Str};
#[cfg(feature = "test")]
use std::any::Any;
#[cfg(feature = "test")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

fn internal(id: i64) -> usize {
    usize::try_from(id - 1).expect("JOB ids must be positive and fit usize")
}

fn id_word(id: i64) -> u64 {
    u64::try_from(internal(id)).expect("internal JOB id must fit u64")
}

/// Stateful receiver for normalized JOB rows.
///
/// IDs use the source database's one-based convention. Every optional value
/// independently contributes a pair when present and contributes nothing when
/// absent. Lookup rows must be supplied before rows which reference them.
#[derive(Default)]
pub struct JobShredder {
    movie_title: StringColumn,
    movie_kind: WordColumn,
    movie_production_year: WordColumn,
    movie_episode_nr: WordColumn,
    keyword_keyword: StringColumn,
    movie_keyword: WordColumn,
    kind_kind: StringColumn,
    roletype_role: StringColumn,
    character_name: StringColumn,
    compcasttype_kind: StringColumn,
    infotype_info: StringColumn,
    linktype_link: StringColumn,
    companytype_kind: StringColumn,
    movie_company: WordColumn,
    company_name: StringColumn,
    company_country: StringColumn,
    company_note: StringColumn,
    company_type: WordColumn,
    movie_info: WordColumn,
    info_info: StringColumn,
    info_type: WordColumn,
    info_note: StringColumn,
    movie_data: WordColumn,
    data_data: StringColumn,
    data_type: WordColumn,
    movie_link: WordColumn,
    movie_linked_by: WordColumn,
    movielink_target: WordColumn,
    movielink_type: WordColumn,
    movie_aka: WordColumn,
    akatitle_title: StringColumn,
    person_aka: WordColumn,
    akaname_name: StringColumn,
    person_name: StringColumn,
    person_gender: StringColumn,
    person_name_pcode: StringColumn,
    movie_complete_cast: WordColumn,
    completecast_subject: WordColumn,
    completecast_status: WordColumn,
    person_info: WordColumn,
    personinfo_type: WordColumn,
    personinfo_info: StringColumn,
    personinfo_note: StringColumn,
    movie_cast: WordColumn,
    cast_person: WordColumn,
    cast_character: WordColumn,
    cast_role: WordColumn,
    cast_note: StringColumn,
    company_names: HashMap<i64, String>,
    company_countries: HashMap<i64, String>,
}

impl JobShredder {
    pub fn title(
        &mut self,
        id: Option<i64>,
        title: Option<&str>,
        kind_id: Option<i64>,
        production_year: Option<i64>,
        episode_nr: Option<i64>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(value) = title {
            self.movie_title.push(key, value);
        }
        if let Some(value) = kind_id {
            self.movie_kind.push(key, id_word(value));
        }
        if let Some(value) = production_year {
            self.movie_production_year.push(key, value as u64);
        }
        if let Some(value) = episode_nr {
            self.movie_episode_nr.push(key, value as u64);
        }
    }

    pub fn kind_type(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.kind_kind, id, value);
    }

    pub fn role_type(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.roletype_role, id, value);
    }

    pub fn char_name(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.character_name, id, value);
    }

    pub fn info_type(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.infotype_info, id, value);
    }

    pub fn link_type(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.linktype_link, id, value);
    }

    pub fn comp_cast_type(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.compcasttype_kind, id, value);
    }

    pub fn company_type(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.companytype_kind, id, value);
    }

    pub fn keyword(&mut self, id: Option<i64>, value: Option<&str>) {
        push_string(&mut self.keyword_keyword, id, value);
    }

    pub fn movie_keyword(&mut self, movie_id: Option<i64>, keyword_id: Option<i64>) {
        push_id(&mut self.movie_keyword, movie_id, keyword_id);
    }

    pub fn company_name(&mut self, id: Option<i64>, name: Option<&str>, country: Option<&str>) {
        let Some(id) = id else { return };
        if let Some(name) = name {
            self.company_names.insert(id, name.to_owned());
        }
        if let Some(country) = country {
            self.company_countries.insert(id, country.to_owned());
        }
    }

    pub fn movie_company(
        &mut self,
        id: Option<i64>,
        movie_id: Option<i64>,
        company_id: Option<i64>,
        company_type_id: Option<i64>,
        note: Option<&str>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_company.push(internal(movie_id), key as u64);
        }
        if let Some(company_id) = company_id {
            if let Some(value) = self.company_names.get(&company_id) {
                self.company_name.push(key, value);
            }
            if let Some(value) = self.company_countries.get(&company_id) {
                self.company_country.push(key, value);
            }
        }
        if let Some(value) = company_type_id {
            self.company_type.push(key, id_word(value));
        }
        if let Some(value) = note {
            self.company_note.push(key, value);
        }
    }

    pub fn movie_info(
        &mut self,
        id: Option<i64>,
        movie_id: Option<i64>,
        info_type_id: Option<i64>,
        info: Option<&str>,
        note: Option<&str>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_info.push(internal(movie_id), key as u64);
        }
        if let Some(value) = info_type_id {
            self.info_type.push(key, id_word(value));
        }
        if let Some(value) = info {
            self.info_info.push(key, value);
        }
        if let Some(value) = note {
            self.info_note.push(key, value);
        }
    }

    pub fn movie_info_idx(
        &mut self,
        id: Option<i64>,
        movie_id: Option<i64>,
        info_type_id: Option<i64>,
        info: Option<&str>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_data.push(internal(movie_id), key as u64);
        }
        if let Some(value) = info_type_id {
            self.data_type.push(key, id_word(value));
        }
        if let Some(value) = info {
            self.data_data.push(key, value);
        }
    }

    pub fn movie_link(
        &mut self,
        id: Option<i64>,
        movie_id: Option<i64>,
        linked_movie_id: Option<i64>,
        link_type_id: Option<i64>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_link.push(internal(movie_id), key as u64);
        }
        if let Some(target) = linked_movie_id {
            self.movie_linked_by.push(internal(target), key as u64);
            self.movielink_target.push(key, id_word(target));
        }
        if let Some(value) = link_type_id {
            self.movielink_type.push(key, id_word(value));
        }
    }

    pub fn aka_title(&mut self, id: Option<i64>, movie_id: Option<i64>, title: Option<&str>) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_aka.push(internal(movie_id), key as u64);
        }
        if let Some(value) = title {
            self.akatitle_title.push(key, value);
        }
    }

    pub fn name(
        &mut self,
        id: Option<i64>,
        name: Option<&str>,
        gender: Option<&str>,
        name_pcode_cf: Option<&str>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(value) = name {
            self.person_name.push(key, value);
        }
        if let Some(value) = gender {
            self.person_gender.push(key, value);
        }
        if let Some(value) = name_pcode_cf {
            self.person_name_pcode.push(key, value);
        }
    }

    pub fn aka_name(&mut self, id: Option<i64>, person_id: Option<i64>, name: Option<&str>) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(person_id) = person_id {
            self.person_aka.push(internal(person_id), key as u64);
        }
        if let Some(value) = name {
            self.akaname_name.push(key, value);
        }
    }

    pub fn complete_cast(
        &mut self,
        id: Option<i64>,
        movie_id: Option<i64>,
        subject_id: Option<i64>,
        status_id: Option<i64>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_complete_cast
                .push(internal(movie_id), key as u64);
        }
        if let Some(value) = subject_id {
            self.completecast_subject.push(key, id_word(value));
        }
        if let Some(value) = status_id {
            self.completecast_status.push(key, id_word(value));
        }
    }

    pub fn person_info(
        &mut self,
        id: Option<i64>,
        person_id: Option<i64>,
        info_type_id: Option<i64>,
        info: Option<&str>,
        note: Option<&str>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(person_id) = person_id {
            self.person_info.push(internal(person_id), key as u64);
        }
        if let Some(value) = info_type_id {
            self.personinfo_type.push(key, id_word(value));
        }
        if let Some(value) = info {
            self.personinfo_info.push(key, value);
        }
        if let Some(value) = note {
            self.personinfo_note.push(key, value);
        }
    }

    pub fn cast_info(
        &mut self,
        id: Option<i64>,
        person_id: Option<i64>,
        movie_id: Option<i64>,
        character_id: Option<i64>,
        note: Option<&str>,
        role_id: Option<i64>,
    ) {
        let Some(id) = id else { return };
        let key = internal(id);
        if let Some(movie_id) = movie_id {
            self.movie_cast.push(internal(movie_id), key as u64);
        }
        if let Some(value) = person_id {
            self.cast_person.push(key, id_word(value));
        }
        if let Some(value) = character_id {
            self.cast_character.push(key, id_word(value));
        }
        if let Some(value) = role_id {
            self.cast_role.push(key, id_word(value));
        }
        if let Some(value) = note {
            self.cast_note.push(key, value);
        }
    }

    /// Materialize the shredded pairs directly as an owned production JOB
    /// schema. The wrapper retains the backing storage borrowed by its CSR
    /// relations and string values.
    #[cfg(feature = "test")]
    pub fn into_job(self) -> OwnedJob {
        let mut source = MemoryJobSource::new(self.into_columns());
        let job = crate::job_schema::build(&mut source);
        OwnedJob {
            job,
            _source: source,
        }
    }

    /// Write the same shredded pairs in the production cache format.
    pub fn write_cache(self, cache_dir: &Path) -> Vec<String> {
        std::fs::create_dir_all(cache_dir).unwrap();
        let mut written = Vec::new();
        self.emit_columns(|name, column| {
            column.write(&cache_dir.join(format!("{name}.bin")));
            written.push(name.to_owned());
        });
        written
    }

    /// Finalize every logical pair stream once. Both the in-memory source and
    /// the cache writer consume this exact named physical-column collection.
    #[cfg(feature = "test")]
    fn into_columns(self) -> CacheColumns {
        let mut columns = CacheColumns::default();
        self.emit_columns(|name, column| columns.push(name, column));
        columns
    }

    /// Define the physical columns once while allowing production regeneration
    /// to serialize and release each finalized column immediately.
    fn emit_columns(self, mut emit: impl FnMut(&str, CacheColumn)) {
        let sizes = self.sizes();
        macro_rules! dense_words {
            ($name:literal, $column:expr, $n:expr) => {
                emit($name, $column.dense($n, KIND_DENSE_I64, NO_ID_WORD))
            };
        }
        macro_rules! dense_strings {
            ($name:literal, $column:expr, $n:expr) => {
                emit($name, $column.dense($n))
            };
        }
        macro_rules! multi_words {
            ($name:literal, $column:expr, $n:expr) => {
                emit($name, $column.multi($n))
            };
        }
        macro_rules! multi_strings {
            ($name:literal, $column:expr, $n:expr) => {
                emit($name, $column.multi($n))
            };
        }

        dense_strings!("Movie_title", self.movie_title, sizes.movie);
        dense_words!("Movie_kind", self.movie_kind, sizes.movie);
        multi_words!(
            "Movie_production_year",
            self.movie_production_year,
            sizes.movie
        );
        multi_words!("Movie_episode_nr", self.movie_episode_nr, sizes.movie);
        multi_words!("Movie_keyword", self.movie_keyword, sizes.movie);
        multi_words!("Movie_company", self.movie_company, sizes.movie);
        multi_words!("Movie_cast", self.movie_cast, sizes.movie);
        multi_words!("Movie_info", self.movie_info, sizes.movie);
        multi_words!("Movie_data", self.movie_data, sizes.movie);
        multi_words!("Movie_complete_cast", self.movie_complete_cast, sizes.movie);
        multi_words!("Movie_link", self.movie_link, sizes.movie);
        multi_words!("Movie_linked_by", self.movie_linked_by, sizes.movie);
        multi_words!("Movie_aka", self.movie_aka, sizes.movie);
        dense_words!("Cast_person", self.cast_person, sizes.cast);
        dense_words!("Cast_role", self.cast_role, sizes.cast);
        multi_strings!("Cast_note", self.cast_note, sizes.cast);
        multi_words!("Cast_character", self.cast_character, sizes.cast);
        dense_strings!("Person_name", self.person_name, sizes.person);
        multi_strings!("Person_gender", self.person_gender, sizes.person);
        multi_words!("Person_alias", self.person_aka, sizes.person);
        multi_words!("Person_bio", self.person_info, sizes.person);
        multi_strings!("Person_name_pcode_cf", self.person_name_pcode, sizes.person);
        dense_strings!("Keyword_text", self.keyword_keyword, sizes.keyword);
        dense_strings!("Kind_text", self.kind_kind, sizes.kind);
        dense_strings!("RoleType_text", self.roletype_role, sizes.role_type);
        dense_strings!("Character_text", self.character_name, sizes.character);
        multi_strings!("Company_country", self.company_country, sizes.company);
        dense_strings!("Company_name", self.company_name, sizes.company);
        multi_strings!("Company_note", self.company_note, sizes.company);
        dense_words!("Company_ty", self.company_type, sizes.company);
        dense_strings!(
            "CompanyType_text",
            self.companytype_kind,
            sizes.company_type
        );
        dense_strings!("Info_info", self.info_info, sizes.info);
        dense_words!("Info_ty", self.info_type, sizes.info);
        multi_strings!("Info_note", self.info_note, sizes.info);
        dense_strings!("InfoType_text", self.infotype_info, sizes.info_type);
        dense_strings!("Data_text", self.data_data, sizes.data);
        dense_words!("Data_ty", self.data_type, sizes.data);
        dense_strings!("PersonInfo_info", self.personinfo_info, sizes.person_info);
        dense_words!("PersonInfo_ty", self.personinfo_type, sizes.person_info);
        multi_strings!("PersonInfo_note", self.personinfo_note, sizes.person_info);
        dense_strings!("AkaName_text", self.akaname_name, sizes.aka_name);
        dense_strings!("AkaTitle_text", self.akatitle_title, sizes.aka_title);
        dense_words!("MovieLink_target", self.movielink_target, sizes.movie_link);
        dense_words!("MovieLink_ty", self.movielink_type, sizes.movie_link);
        dense_strings!("LinkType_text", self.linktype_link, sizes.link_type);
        dense_words!(
            "CompleteCast_status",
            self.completecast_status,
            sizes.complete_cast
        );
        dense_words!(
            "CompleteCast_subject",
            self.completecast_subject,
            sizes.complete_cast
        );
        dense_strings!(
            "CompCastType_text",
            self.compcasttype_kind,
            sizes.comp_cast_type
        );
    }

    fn sizes(&self) -> Sizes {
        Sizes {
            movie: self
                .movie_title
                .n_from_keys()
                .max(self.movielink_target.n_from_values()),
            person: self.person_name.n_from_keys(),
            cast: self.cast_person.n_from_keys(),
            keyword: self
                .keyword_keyword
                .n_from_keys()
                .max(self.movie_keyword.n_from_values()),
            kind: self
                .kind_kind
                .n_from_keys()
                .max(self.movie_kind.n_from_values()),
            role_type: self
                .roletype_role
                .n_from_keys()
                .max(self.cast_role.n_from_values()),
            character: self
                .character_name
                .n_from_keys()
                .max(self.cast_character.n_from_values()),
            company: self
                .company_name
                .n_from_keys()
                .max(self.company_country.n_from_keys())
                .max(self.company_note.n_from_keys())
                .max(self.company_type.n_from_keys())
                .max(self.movie_company.n_from_values()),
            company_type: self
                .companytype_kind
                .n_from_keys()
                .max(self.company_type.n_from_values()),
            info: self
                .info_info
                .n_from_keys()
                .max(self.info_type.n_from_keys())
                .max(self.info_note.n_from_keys())
                .max(self.movie_info.n_from_values()),
            info_type: self
                .infotype_info
                .n_from_keys()
                .max(self.info_type.n_from_values()),
            data: self
                .data_data
                .n_from_keys()
                .max(self.data_type.n_from_keys())
                .max(self.movie_data.n_from_values()),
            person_info: self
                .personinfo_info
                .n_from_keys()
                .max(self.personinfo_type.n_from_keys())
                .max(self.personinfo_note.n_from_keys())
                .max(self.person_info.n_from_values()),
            aka_name: self
                .akaname_name
                .n_from_keys()
                .max(self.person_aka.n_from_values()),
            aka_title: self
                .akatitle_title
                .n_from_keys()
                .max(self.movie_aka.n_from_values()),
            movie_link: self
                .movielink_target
                .n_from_keys()
                .max(self.movielink_type.n_from_keys())
                .max(self.movie_link.n_from_values())
                .max(self.movie_linked_by.n_from_values()),
            link_type: self
                .linktype_link
                .n_from_keys()
                .max(self.movielink_type.n_from_values()),
            complete_cast: self
                .completecast_status
                .n_from_keys()
                .max(self.completecast_subject.n_from_keys())
                .max(self.movie_complete_cast.n_from_values()),
            comp_cast_type: self
                .compcasttype_kind
                .n_from_keys()
                .max(self.completecast_status.n_from_values())
                .max(self.completecast_subject.n_from_values()),
        }
    }
}

fn push_string(column: &mut StringColumn, id: Option<i64>, value: Option<&str>) {
    if let (Some(id), Some(value)) = (id, value) {
        column.push(internal(id), value);
    }
}

fn push_id(column: &mut WordColumn, key: Option<i64>, value: Option<i64>) {
    if let (Some(key), Some(value)) = (key, value) {
        column.push(internal(key), id_word(value));
    }
}

#[derive(Clone, Copy)]
struct Sizes {
    movie: usize,
    person: usize,
    cast: usize,
    keyword: usize,
    kind: usize,
    role_type: usize,
    character: usize,
    company: usize,
    company_type: usize,
    info: usize,
    info_type: usize,
    data: usize,
    person_info: usize,
    aka_name: usize,
    aka_title: usize,
    movie_link: usize,
    link_type: usize,
    complete_cast: usize,
    comp_cast_type: usize,
}

/// An in-memory JOB and all allocations referenced by its relations.
///
/// Field order matters: Rust drops fields in declaration order, so `job` is
/// gone before `_source` releases the buffers to which it contains views.
#[cfg(feature = "test")]
pub struct OwnedJob {
    job: Job,
    _source: MemoryJobSource,
}

#[cfg(feature = "test")]
impl OwnedJob {
    /// Run a differential query without exposing the schema's internally
    /// lifetime-extended string views.
    pub fn differential(
        &self,
        name: &str,
    ) -> Result<Vec<crate::job_queries::helpers::Result>, String> {
        crate::job_queries::differential(name, &self.job)
    }
}

#[cfg(feature = "test")]
struct MemoryJobSource {
    columns: BTreeMap<String, MemoryColumn>,
    derived: Vec<Box<dyn Any>>,
}

#[cfg(feature = "test")]
impl MemoryJobSource {
    fn new(columns: CacheColumns) -> Self {
        Self {
            columns: columns.into_memory(),
            derived: Vec::new(),
        }
    }

    fn column(&self, name: &str) -> &MemoryColumn {
        self.columns
            .get(name)
            .unwrap_or_else(|| panic!("missing in-memory JOB column {name}"))
    }

    fn dense_words(&self, name: &str) -> &[u64] {
        match self.column(name) {
            MemoryColumn::DenseWords(values) => values,
            _ => panic!("{name} is not a dense word column"),
        }
    }

    fn dense_strings(&self, name: &str) -> &[&'static str] {
        match self.column(name) {
            MemoryColumn::DenseStrings { values, .. } => values,
            _ => panic!("{name} is not a dense string column"),
        }
    }

    fn multi_words(&self, name: &str) -> (&'static [u32], &'static [u64]) {
        match self.column(name) {
            MemoryColumn::CsrWords { offsets, values } => unsafe {
                (extend_slice(offsets), extend_slice(values))
            },
            _ => panic!("{name} is not a CSR word column"),
        }
    }

    fn multi_strings(&self, name: &str) -> (&'static [u32], &'static [&'static str]) {
        match self.column(name) {
            MemoryColumn::CsrStrings {
                offsets, values, ..
            } => unsafe { (extend_slice(offsets), extend_slice(values)) },
            _ => panic!("{name} is not a CSR string column"),
        }
    }

    fn keep<T: 'static>(&mut self, values: Vec<T>) -> &'static [T] {
        let values = values.into_boxed_slice();
        // SAFETY: `derived` retains this box until its enclosing `OwnedJob`
        // has dropped the schema that contains the returned view.
        let slice = unsafe { std::slice::from_raw_parts(values.as_ptr(), values.len()) };
        self.derived.push(Box::new(values));
        slice
    }
}

/// Extend a view into storage owned by `MemoryJobSource`. These references are
/// used only inside the associated `OwnedJob`, whose field order guarantees
/// that the relations are destroyed before their storage.
#[cfg(feature = "test")]
unsafe fn extend_slice<T>(slice: &[T]) -> &'static [T] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr(), slice.len()) }
}

#[cfg(feature = "test")]
impl JobSource for MemoryJobSource {
    fn key<E: 'static>(&self, name: &str) -> Key<E> {
        Universe::new(self.column(name).n_dom())
    }

    fn strs<E: 'static>(&mut self, name: &str) -> Col<E, Str> {
        VecRel::new(self.dense_strings(name).to_vec())
    }

    fn ids<T: 'static, E: 'static>(&mut self, name: &str) -> Col<E, Id<T>> {
        VecRel::new(
            self.dense_words(name)
                .iter()
                .map(|&value| Id::new(value as usize))
                .collect(),
        )
    }

    fn dict<E: 'static>(&mut self, codes: &str, table: &str) -> Dict<E, Str> {
        DictRel::new(
            VecRel::new(
                self.dense_words(codes)
                    .iter()
                    .map(|&value| value as usize)
                    .collect(),
            ),
            VecRel::new(self.dense_strings(table).to_vec()),
        )
    }

    fn multi_dict<E: 'static>(&mut self, codes: &str, table: &str) -> DictSet<E, Str> {
        let (offsets, values) = self.multi_words(codes);
        let values = values
            .iter()
            .map(|&value| value as usize)
            .collect::<Vec<_>>();
        let values = self.keep(values);
        DictMultiRel::new(
            MultiRel::from_csr(offsets, values),
            VecRel::new(self.dense_strings(table).to_vec()),
        )
    }

    fn multi_ids<T: 'static, E: 'static>(&mut self, name: &str) -> Set<E, Id<T>> {
        let (offsets, values) = self.multi_words(name);
        let values = values
            .iter()
            .map(|&value| Id::new(value as usize))
            .collect::<Vec<_>>();
        let values = self.keep(values);
        MultiRel::from_csr(offsets, values)
    }

    fn multi_i64<E: 'static>(&mut self, name: &str) -> Set<E, i64> {
        let (offsets, values) = self.multi_words(name);
        let values = values.iter().map(|&value| value as i64).collect::<Vec<_>>();
        let values = self.keep(values);
        MultiRel::from_csr(offsets, values)
    }

    fn multi_strs<E: 'static>(&mut self, name: &str) -> Set<E, Str> {
        let (offsets, values) = self.multi_strings(name);
        MultiRel::from_csr(offsets, values)
    }
}
