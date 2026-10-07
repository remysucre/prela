use crate::concepts::int_normalized_input_medical_claim_date_normalize::IntNormalizedInputMedicalClaimDateNormalize as Dn;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputMedicalDateAggregation {
    pub claim_id: Str,
    pub data_source: Str,
    pub minimum_claim_start_date: Date,
    pub maximum_claim_end_date: Date,
    pub minimum_admission_date: Option<Date>,
    pub maximum_discharge_date: Option<Date>,
    pub tuva_last_run: Ts,
}

type Acc = (((Date, Date), Option<Date>), Option<Date>);

fn omin(a: Option<Date>, x: Option<Date>) -> Option<Date> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.min(x)),
        _ => a.or(x),
    }
}

fn omax(a: Option<Date>, x: Option<Date>) -> Option<Date> {
    match (a, x) {
        (Some(a), Some(x)) => Some(a.max(x)),
        _ => a.or(x),
    }
}

fn agg(a: Acc, d: Dn) -> Acc {
    (
        (
            (a.0.0.0.min(d.normalized_claim_start_date), a.0.0.1.max(d.normalized_claim_end_date)),
            omin(a.0.1, d.normalized_admission_date),
        ),
        omax(a.1, d.normalized_discharge_date),
    )
}

pub fn int_normalized_input_medical_date_aggregation(
    _db: &'static Db,
    dn: &[Dn],
) -> Vec<IntNormalizedInputMedicalDateAggregation> {
    let r = rel(dn.to_vec());
    let init: Acc = (((Date::MAX, Date::MIN), None), None);
    let key = |d: Dn| (d.claim_id, d.data_source);
    let inst = (&r)
        .filt(|d: Dn| d.claim_type == "institutional")
        .map(key)
        .inv()
        .select(&r)
        .fold(init, agg);
    let prof = (&r)
        .filt(|d: Dn| d.claim_type == "professional")
        .map(key)
        .inv()
        .select(&r)
        .fold(init, agg);
    let run = ts(2026, 1, 1, 0, 0, 0);
    let mut out: Vec<IntNormalizedInputMedicalDateAggregation> = drain(&inst)
        .into_iter()
        .map(|((claim_id, data_source), (((s, e), a), d))| IntNormalizedInputMedicalDateAggregation {
            claim_id,
            data_source,
            minimum_claim_start_date: s,
            maximum_claim_end_date: e,
            minimum_admission_date: a,
            maximum_discharge_date: d,
            tuva_last_run: run,
        })
        .collect();
    out.extend(drain(&prof).into_iter().map(|((claim_id, data_source), (((s, e), _), _))| {
        IntNormalizedInputMedicalDateAggregation {
            claim_id,
            data_source,
            minimum_claim_start_date: s,
            maximum_claim_end_date: e,
            minimum_admission_date: None,
            maximum_discharge_date: None,
            tuva_last_run: run,
        }
    }));
    out
}

pub fn fmt(v: &IntNormalizedInputMedicalDateAggregation) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        V::D(v.minimum_claim_start_date),
        V::D(v.maximum_claim_end_date),
        odate(v.minimum_admission_date),
        odate(v.maximum_discharge_date),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let dn = crate::concepts::int_normalized_input_medical_claim_date_normalize::int_normalized_input_medical_claim_date_normalize(db, &stg);
    rows(int_normalized_input_medical_date_aggregation(db, &dn).iter().map(fmt))
}
