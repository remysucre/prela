use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EncountersStgOutpatientInstitutional {
    pub claim_id: Str,
    pub data_source: Str,
    pub service_type: Str,
    pub tuva_last_run: Ts,
}

pub fn encounters__stg_outpatient_institutional(db: &'static Db) -> Vec<EncountersStgOutpatientInstitutional> {
    let a = &db.service_category_stg_outpatient_institutional;
    let run = ts(2026, 1, 1, 0, 0, 0);
    let j = (&a.claim_id).and(&a.data_source).and(&a.service_type);
    drain(&j)
        .into_iter()
        .map(|(_, ((claim_id, data_source), service_type))| EncountersStgOutpatientInstitutional {
            claim_id,
            data_source,
            service_type,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &EncountersStgOutpatientInstitutional) -> String {
    row(vec![V::S(v.claim_id), V::S(v.data_source), V::S(v.service_type), V::T(v.tuva_last_run)])
}

pub fn q(db: &'static Db) -> String {
    rows(encounters__stg_outpatient_institutional(db).iter().map(fmt))
}
