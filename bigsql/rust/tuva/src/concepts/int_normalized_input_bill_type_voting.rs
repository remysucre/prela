use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::{BillType, Db};
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputBillTypeVoting {
    pub claim_id: Str,
    pub data_source: Str,
    pub column_name: Str,
    pub normalized_code: Str,
    pub normalized_description: Str,
    pub occurrence_count: i64,
    pub occurrence_row_count: i64,
    pub tuva_last_run: Ts,
}

type Dc = (((Str, Str), (Str, Str)), i64);

pub fn int_normalized_input_bill_type_voting(
    db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputBillTypeVoting> {
    let t = &db.bill_type;
    let med = rel(stg.to_vec());
    let by_code: HashIdx<Str, Id<BillType>> = (&t.bill_type_code).inv().collect();
    let normalize = (&med)
        .filt(|m: NormalizedInputStgMedicalClaim| m.claim_type == "institutional")
        .map(|m: NormalizedInputStgMedicalClaim| (m.claim_id, m.data_source))
        .and(
            (&med)
                .filt(|m: NormalizedInputStgMedicalClaim| m.bill_type_code.is_some())
                .map(|m: NormalizedInputStgMedicalClaim| m.bill_type_code.unwrap().trim_start_matches('0'))
                .select(&by_code)
                .select((&t.bill_type_code).and(&t.bill_type_description)),
        );
    let counts = (&normalize).inv().fold(0i64, |a, _| a + 1);
    let dc = rel(drain(&counts));
    let ranked = (&dc)
        .map(|((k, _), _): Dc| k)
        .inv()
        .select(&dc)
        .window(row_number, |(((_, _), (code, _)), n): Dc| (n, code), |a: &(i64, Str), b: &(i64, Str)| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&ranked)
        .into_iter()
        .map(|((claim_id, data_source), ((((_, _), (code, desc)), n), rn))| {
            IntNormalizedInputBillTypeVoting {
                claim_id,
                data_source,
                column_name: "bill_type_code",
                normalized_code: code,
                normalized_description: desc,
                occurrence_count: n,
                occurrence_row_count: rn,
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputBillTypeVoting) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        V::S(v.column_name),
        V::S(v.normalized_code),
        V::S(v.normalized_description),
        V::I(v.occurrence_count),
        V::I(v.occurrence_row_count),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_bill_type_voting(db, &stg).iter().map(fmt))
}
