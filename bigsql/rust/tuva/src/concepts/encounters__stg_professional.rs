use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EncountersStgProfessional {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub claim_line_id: Str,
    pub service_type: Str,
    pub data_source: Str,
    pub tuva_last_run: Ts,
}

pub fn encounters__stg_professional(db: &'static Db) -> Vec<EncountersStgProfessional> {
    let a = &db.service_category_stg_professional;
    let run = ts(2026, 1, 1, 0, 0, 0);
    let j = (&a.claim_id).and(&a.claim_line_number).and(&a.claim_line_id).and(&a.service_type).and(&a.data_source);
    drain(&j)
        .into_iter()
        .map(|(_, ((((claim_id, claim_line_number), claim_line_id), service_type), data_source))| EncountersStgProfessional {
            claim_id,
            claim_line_number,
            claim_line_id,
            service_type,
            data_source,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &EncountersStgProfessional) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.claim_line_id),
        V::S(v.service_type),
        V::S(v.data_source),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    rows(encounters__stg_professional(db).iter().map(fmt))
}
