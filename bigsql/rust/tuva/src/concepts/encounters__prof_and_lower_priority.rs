use crate::concepts::encounters__stg_professional::EncountersStgProfessional;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct EncountersProfAndLowerPriority {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub tuva_last_run: Ts,
}

const LOW: &[&str] = &["lab", "durable medical equipment", "ambulance"];

pub fn encounters__prof_and_lower_priority(db: &'static Db, prof: &[EncountersStgProfessional]) -> Vec<EncountersProfAndLowerPriority> {
    let a = rel(prof.to_vec());
    let g = &db.service_category_grouper;
    let low = (&g.id)
        .with((&g.duplicate_row_number).filt(|x: i64| x == 1))
        .with((&g.service_category_2).is_in(LOW.iter().copied()))
        .select((&g.claim_id).and(&g.claim_line_number).and(&g.data_source))
        .map(|((c, l), d): ((Str, i64), Str)| (c, l, d));
    let run = ts(2026, 1, 1, 0, 0, 0);
    let u = (&a)
        .map(|x: EncountersStgProfessional| (x.claim_id, x.claim_line_number, x.data_source))
        .inv()
        .map(|_| ())
        .union((&low).inv().map(|_| ()))
        .fold((), |a, _| a);
    drain(&u)
        .into_iter()
        .map(|((claim_id, claim_line_number, data_source), _)| EncountersProfAndLowerPriority {
            claim_id,
            claim_line_number,
            data_source,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &EncountersProfAndLowerPriority) -> String {
    row(vec![V::S(v.claim_id), V::I(v.claim_line_number), V::S(v.data_source), V::T(v.tuva_last_run)])
}

pub fn q(db: &'static Db) -> String {
    let prof = crate::concepts::encounters__stg_professional::encounters__stg_professional(db);
    rows(encounters__prof_and_lower_priority(db, &prof).iter().map(fmt))
}
