use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputMedicalClaimDateNormalize {
    pub claim_id: Str,
    pub claim_line_number: i64,
    pub claim_type: Str,
    pub data_source: Str,
    pub normalized_claim_start_date: Date,
    pub normalized_claim_end_date: Date,
    pub normalized_claim_line_start_date: Option<Date>,
    pub normalized_claim_line_end_date: Option<Date>,
    pub normalized_admission_date: Option<Date>,
    pub normalized_discharge_date: Option<Date>,
    pub tuva_last_run: Ts,
}

type Key = (((Str, i64), (Str, Str)), ((Date, Date), ((Option<Date>, Option<Date>), (Option<Date>, Option<Date>))));

pub fn int_normalized_input_medical_claim_date_normalize(
    _db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputMedicalClaimDateNormalize> {
    let med = rel(stg.to_vec());
    let distinct = (&med)
        .map(|m: NormalizedInputStgMedicalClaim| -> Key {
            (
                ((m.claim_id, m.claim_line_number), (m.claim_type, m.data_source)),
                (
                    (m.claim_start_date, m.claim_end_date),
                    ((m.claim_line_start_date, m.claim_line_end_date), (m.admission_date, m.discharge_date)),
                ),
            )
        })
        .inv()
        .map(|_| ())
        .fold((), |a, _| a);
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&distinct)
        .into_iter()
        .map(|((((claim_id, claim_line_number), (claim_type, data_source)), ((s, e), ((ls, le), (ad, dc)))), _)| {
            IntNormalizedInputMedicalClaimDateNormalize {
                claim_id,
                claim_line_number,
                claim_type,
                data_source,
                normalized_claim_start_date: s,
                normalized_claim_end_date: e,
                normalized_claim_line_start_date: ls,
                normalized_claim_line_end_date: le,
                normalized_admission_date: ad,
                normalized_discharge_date: dc,
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputMedicalClaimDateNormalize) -> String {
    row(vec![
        V::S(v.claim_id),
        V::I(v.claim_line_number),
        V::S(v.claim_type),
        V::S(v.data_source),
        V::D(v.normalized_claim_start_date),
        V::D(v.normalized_claim_end_date),
        odate(v.normalized_claim_line_start_date),
        odate(v.normalized_claim_line_end_date),
        odate(v.normalized_admission_date),
        odate(v.normalized_discharge_date),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_medical_claim_date_normalize(db, &stg).iter().map(fmt))
}
