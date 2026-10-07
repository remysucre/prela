use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::{DischargeDisposition, Db};
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputDischargeDispositionVoting {
    pub claim_id: Str,
    pub data_source: Str,
    pub column_name: Str,
    pub normalized_code: Str,
    pub normalized_description: Str,
    pub occurrence_count: i64,
    pub next_occurrence_count: i64,
    pub occurrence_row_count: i64,
    pub tuva_last_run: Ts,
}

type Dc = (((Str, Str), (Str, Str)), i64);

fn vote(g: &[(i64, Dc)], out: &mut Vec<(i64, i64)>) {
    for i in 0..g.len() {
        out.push((g.get(i + 1).map_or(0, |n| n.0), i as i64 + 1));
    }
}

pub fn int_normalized_input_discharge_disposition_voting(
    db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputDischargeDispositionVoting> {
    let t = &db.discharge_disposition;
    let med = rel(stg.to_vec());
    let by_code: HashIdx<Str, Id<DischargeDisposition>> = (&t.discharge_disposition_code).inv().collect();
    let normalize = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.claim_type == "institutional")
        .map(|m: NormalizedInputStgMedicalClaim| (m.claim_id, m.data_source))
        .and(
            (&med)
                .filt(|m: NormalizedInputStgMedicalClaim| m.discharge_disposition_code.is_some())
                .map(|m: NormalizedInputStgMedicalClaim| m.discharge_disposition_code.unwrap())
                .select(&by_code)
                .select((&t.discharge_disposition_code).and(&t.discharge_disposition_description)),
        );
    let counts = (&normalize).inv().fold(0i64, |a, _| a + 1);
    let dc = rel(drain(&counts));
    let ranked = (&dc)
        .map(|((k, _), _): Dc| k)
        .inv()
        .select(&dc)
        .window(vote, |(_, n): Dc| n, |a: &i64, b: &i64| b.cmp(a));
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&ranked)
        .into_iter()
        .map(|((claim_id, data_source), ((((_, _), (code, desc)), n), (next, rn)))| {
            IntNormalizedInputDischargeDispositionVoting {
                claim_id,
                data_source,
                column_name: "discharge_disposition_code",
                normalized_code: code,
                normalized_description: desc,
                occurrence_count: n,
                next_occurrence_count: next,
                occurrence_row_count: rn,
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputDischargeDispositionVoting) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        V::S(v.column_name),
        V::S(v.normalized_code),
        V::S(v.normalized_description),
        V::I(v.occurrence_count),
        V::I(v.next_occurrence_count),
        V::I(v.occurrence_row_count),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_discharge_disposition_voting(db, &stg).iter().map(fmt))
}
