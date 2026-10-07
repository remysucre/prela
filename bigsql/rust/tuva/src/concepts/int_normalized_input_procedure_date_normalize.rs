use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputProcedureDateNormalize {
    pub claim_id: Str,
    pub data_source: Str,
    pub procedure_column: Str,
    pub procedure_date: Option<Date>,
    pub procedure_date_occurrence_count: i64,
    pub tuva_last_run: Ts,
}

pub const COLUMNS: [Str; 25] = ["procedure_date_1", "procedure_date_2", "procedure_date_3", "procedure_date_4", "procedure_date_5", "procedure_date_6", "procedure_date_7", "procedure_date_8", "procedure_date_9", "procedure_date_10", "procedure_date_11", "procedure_date_12", "procedure_date_13", "procedure_date_14", "procedure_date_15", "procedure_date_16", "procedure_date_17", "procedure_date_18", "procedure_date_19", "procedure_date_20", "procedure_date_21", "procedure_date_22", "procedure_date_23", "procedure_date_24", "procedure_date_25"];

fn dates(m: &NormalizedInputStgMedicalClaim) -> [Option<Date>; 25] {
    [m.procedure_date_1, m.procedure_date_2, m.procedure_date_3, m.procedure_date_4, m.procedure_date_5, m.procedure_date_6, m.procedure_date_7, m.procedure_date_8, m.procedure_date_9, m.procedure_date_10, m.procedure_date_11, m.procedure_date_12, m.procedure_date_13, m.procedure_date_14, m.procedure_date_15, m.procedure_date_16, m.procedure_date_17, m.procedure_date_18, m.procedure_date_19, m.procedure_date_20, m.procedure_date_21, m.procedure_date_22, m.procedure_date_23, m.procedure_date_24, m.procedure_date_25]
}

type Piv = ((Str, Str), (Str, (Str, Option<Date>)));

fn pivot(m: NormalizedInputStgMedicalClaim) -> [Piv; 25] {
    let ds = dates(&m);
    std::array::from_fn(|i| ((m.claim_id, m.claim_type), (m.data_source, (COLUMNS[i], ds[i]))))
}

pub fn int_normalized_input_procedure_date_normalize(
    _db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputProcedureDateNormalize> {
    let med = rel(stg.to_vec());
    let piv = rel(drain((&med).flat_map(pivot)).into_iter().map(|(_, p)| p).collect::<Vec<Piv>>());
    let counts = (&piv)
        .filt(|((_, ty), _): Piv| ty == "institutional")
        .map(|((claim_id, _), (data_source, (col, d))): Piv| ((claim_id, data_source), (col, d)))
        .inv()
        .fold(0i64, |a, _| a + 1);
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&counts)
        .into_iter()
        .map(|(((claim_id, data_source), (procedure_column, procedure_date)), n)| IntNormalizedInputProcedureDateNormalize {
            claim_id,
            data_source,
            procedure_column,
            procedure_date,
            procedure_date_occurrence_count: n,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputProcedureDateNormalize) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        V::S(v.procedure_column),
        odate(v.procedure_date),
        V::I(v.procedure_date_occurrence_count),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    rows(int_normalized_input_procedure_date_normalize(db, &stg).iter().map(fmt))
}
