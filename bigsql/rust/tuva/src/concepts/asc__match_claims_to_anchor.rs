use crate::concepts::anchor_tmpl_b as k;
use crate::concepts::asc__start_end_dates::AscStartEndDates;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

type M = EncountersStgMedicalClaim;
type D = AscStartEndDates;
type P = (Str, Date, Date, Str, i64, Str);

#[derive(Clone, Copy)]
pub struct AscMatchClaimsToAnchor {
    pub old_encounter_id: Str,
    pub encounter_start_date: Date,
    pub encounter_end_date: Date,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub claim_attribution_number: i64,
}

pub fn asc__match_claims_to_anchor(_db: &'static Db, stg: &[M], dates: &[D]) -> Vec<AscMatchClaimsToAnchor> {
    let m = rel(stg.to_vec());
    let d = rel(dates.to_vec());
    let dx: HashIdx<Str, usize> = (&d).map(|x: D| x.patient_data_source_id).inv().collect();
    let j = (&m)
        .and((&m).map(|x: M| x.patient_data_source_id).select(&dx).select(&d))
        .flat_map(|(x, e): (M, D)| match (x.start_date, e.encounter_start_date, e.encounter_end_date) {
            (Some(s), Some(a), Some(b)) if s >= a && s <= b => {
                Some((e.old_encounter_id, a, b, x.claim_id, x.claim_line_number, x.data_source))
            }
            _ => None,
        });
    let j = rel(drain(&j).into_iter().map(|(_, p)| p).collect::<Vec<P>>());
    let w = (&j)
        .map(|p: P| (p.3, p.4, p.5))
        .inv()
        .select(&j)
        .window(row_number, |p: P| (p.1, p.0), |a: &(Date, Str), b: &(Date, Str)| a.cmp(b));
    drain(&w)
        .into_iter()
        .map(|(_, (p, n))| AscMatchClaimsToAnchor {
            old_encounter_id: p.0,
            encounter_start_date: p.1,
            encounter_end_date: p.2,
            claim_id: p.3,
            claim_line_number: p.4,
            data_source: p.5,
            claim_attribution_number: n,
        })
        .collect()
}

pub fn fmt(v: &AscMatchClaimsToAnchor) -> String {
    row(vec![
        V::S(v.old_encounter_id),
        V::D(v.encounter_start_date),
        V::D(v.encounter_end_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::I(v.claim_attribution_number),
    ])
}

pub fn q(db: &'static Db) -> String {
    let s = k::stg(db);
    let a = crate::concepts::asc__anchor_events::asc__anchor_events(db, &s);
    let g = crate::concepts::asc__generate_encounter_id::asc__generate_encounter_id(db, &s, &a);
    let d = crate::concepts::asc__start_end_dates::asc__start_end_dates(db, &g);
    rows(asc__match_claims_to_anchor(db, &s, &d).iter().map(fmt))
}
