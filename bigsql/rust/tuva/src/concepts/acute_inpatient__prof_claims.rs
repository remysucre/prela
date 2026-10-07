use crate::concepts::acute_inpatient__generate_encounter_id::{AcuteInpatientGenerateEncounterId, asc_nl};
use crate::concepts::acute_inpatient__start_end_dates::AcuteInpatientStartEndDates;
use crate::concepts::encounters__prof_and_lower_priority::EncountersProfAndLowerPriority;
use crate::concepts::encounters__stg_medical_claim::EncountersStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct AcuteInpatientProfClaims {
    pub encounter_id: Str,
    pub encounter_start_date: Option<Date>,
    pub encounter_end_date: Option<Date>,
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub data_source: Str,
    pub claim_attribution_number: i64,
}

type G = AcuteInpatientGenerateEncounterId;
type D = AcuteInpatientStartEndDates;
type M = EncountersStgMedicalClaim;
type P = EncountersProfAndLowerPriority;

#[derive(Clone, Copy)]
struct Jfc {
    encounter_id: Str,
    pdsi: Str,
    anchor_claim_id: Option<Str>,
    end: Option<Date>,
    start: Option<Date>,
}

fn between(x: Option<Date>, lo: Option<Date>, hi: Option<Date>) -> bool {
    matches!((x, lo, hi), (Some(x), Some(l), Some(h)) if x >= l && x <= h)
}

pub fn acute_inpatient__prof_claims(
    _db: &'static Db,
    stg: &[M],
    plp: &[P],
    genc: &[G],
    sed: &[D],
) -> Vec<AcuteInpatientProfClaims> {
    let g = rel(genc.to_vec());
    let d = rel(sed.to_vec());
    let dx: HashIdx<Str, usize> = (&d).map(|x: D| x.encounter_id).inv().collect();
    let jfc = (&g)
        .filt(|x: G| x.encounter_claim_number == 1)
        .and((&g).map(|x: G| x.encounter_id).select(&dx).select(&d))
        .map(|(f, x): (G, D)| Jfc {
            encounter_id: f.encounter_id,
            pdsi: f.patient_data_source_id,
            anchor_claim_id: x.anchor_claim_id,
            end: x.encounter_end_date,
            start: x.encounter_start_date,
        });
    let jfc = rel(drain(&jfc).into_iter().map(|(_, x)| x).collect::<Vec<Jfc>>());
    let jx: HashIdx<Str, usize> = (&jfc).map(|x: Jfc| x.pdsi).inv().collect();

    let m = rel(stg.to_vec());
    let p = rel(plp.to_vec());
    let px: HashIdx<(Str, i64, Str), usize> =
        (&p).map(|x: P| (x.claim_id, x.claim_line_number, x.data_source)).inv().collect();
    let j = (&m)
        .and((&m).map(|x: M| (x.claim_id, x.claim_line_number, x.data_source)).select(&px).select(&p))
        .and((&m).map(|x: M| x.patient_data_source_id).select(&jx).select(&jfc))
        .filt(|((x, _), dat): ((M, P), Jfc)| between(x.start_date, dat.start, dat.end))
        .map(|((x, _), dat): ((M, P), Jfc)| (x, dat));
    let j = rel(drain(&j).into_iter().map(|(_, x)| x).collect::<Vec<(M, Jfc)>>());
    let w = (&j)
        .map(|(x, _): (M, Jfc)| (x.claim_id, x.claim_line_number, x.data_source))
        .inv()
        .select(&j)
        .window(row_number, |(_, dat): (M, Jfc)| dat.anchor_claim_id, asc_nl);
    drain(&w)
        .into_iter()
        .map(|(_, ((x, dat), n))| AcuteInpatientProfClaims {
            encounter_id: dat.encounter_id,
            encounter_start_date: dat.start,
            encounter_end_date: dat.end,
            claim_id: x.claim_id,
            claim_line_number: x.claim_line_number,
            data_source: x.data_source,
            claim_attribution_number: n,
        })
        .collect()
}

pub fn fmt(v: &AcuteInpatientProfClaims) -> String {
    row(vec![
        V::S(v.encounter_id),
        odate(v.encounter_start_date),
        odate(v.encounter_end_date),
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.data_source),
        V::I(v.claim_attribution_number),
    ])
}

pub fn q(db: &'static Db) -> String {
    let stg = crate::concepts::acute_inpatient__generate_encounter_id::stg_medical_claim(db);
    let genc = crate::concepts::acute_inpatient__generate_encounter_id::acute_inpatient__generate_encounter_id(db, &stg);
    let sed = crate::concepts::acute_inpatient__start_end_dates::acute_inpatient__start_end_dates(db, &genc);
    let prof = crate::concepts::encounters__stg_professional::encounters__stg_professional(db);
    let plp = crate::concepts::encounters__prof_and_lower_priority::encounters__prof_and_lower_priority(db, &prof);
    rows(acute_inpatient__prof_claims(db, &stg, &plp, &genc, &sed).iter().map(fmt))
}
