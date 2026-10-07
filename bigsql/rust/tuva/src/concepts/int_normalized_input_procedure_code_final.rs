use crate::concepts::int_normalized_input_procedure_code_intermediate::IntNormalizedInputProcedureCodeIntermediate;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputProcedureCodeFinal {
    pub claim_id: Str,
    pub data_source: Str,
    pub procedure_code_1: Option<Str>,
    pub procedure_code_2: Option<Str>,
    pub procedure_code_3: Option<Str>,
    pub procedure_code_4: Option<Str>,
    pub procedure_code_5: Option<Str>,
    pub procedure_code_6: Option<Str>,
    pub procedure_code_7: Option<Str>,
    pub procedure_code_8: Option<Str>,
    pub procedure_code_9: Option<Str>,
    pub procedure_code_10: Option<Str>,
    pub procedure_code_11: Option<Str>,
    pub procedure_code_12: Option<Str>,
    pub procedure_code_13: Option<Str>,
    pub procedure_code_14: Option<Str>,
    pub procedure_code_15: Option<Str>,
    pub procedure_code_16: Option<Str>,
    pub procedure_code_17: Option<Str>,
    pub procedure_code_18: Option<Str>,
    pub procedure_code_19: Option<Str>,
    pub procedure_code_20: Option<Str>,
    pub procedure_code_21: Option<Str>,
    pub procedure_code_22: Option<Str>,
    pub procedure_code_23: Option<Str>,
    pub procedure_code_24: Option<Str>,
    pub procedure_code_25: Option<Str>,
    pub tuva_last_run: Ts,
}

const COLUMNS: [Str; 25] = ["procedure_code_1", "procedure_code_2", "procedure_code_3", "procedure_code_4", "procedure_code_5", "procedure_code_6", "procedure_code_7", "procedure_code_8", "procedure_code_9", "procedure_code_10", "procedure_code_11", "procedure_code_12", "procedure_code_13", "procedure_code_14", "procedure_code_15", "procedure_code_16", "procedure_code_17", "procedure_code_18", "procedure_code_19", "procedure_code_20", "procedure_code_21", "procedure_code_22", "procedure_code_23", "procedure_code_24", "procedure_code_25"];

pub fn int_normalized_input_procedure_code_final(_db: &'static Db, src: &[IntNormalizedInputProcedureCodeIntermediate]) -> Vec<IntNormalizedInputProcedureCodeFinal> {
    let r = rel(src.to_vec());
    let per = (&r)
        .map(|v: IntNormalizedInputProcedureCodeIntermediate| (v.claim_id, v.data_source))
        .inv()
        .select(&r)
        .fold([None::<Str>; 25], |mut a, v: IntNormalizedInputProcedureCodeIntermediate| {
            for (i, c) in COLUMNS.iter().enumerate() {
                if v.column_name == *c {
                    a[i] = a[i].max(Some(v.procedure_code));
                }
            }
            a
        });
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&per)
        .into_iter()
        .map(|((claim_id, data_source), [procedure_code_1, procedure_code_2, procedure_code_3, procedure_code_4, procedure_code_5, procedure_code_6, procedure_code_7, procedure_code_8, procedure_code_9, procedure_code_10, procedure_code_11, procedure_code_12, procedure_code_13, procedure_code_14, procedure_code_15, procedure_code_16, procedure_code_17, procedure_code_18, procedure_code_19, procedure_code_20, procedure_code_21, procedure_code_22, procedure_code_23, procedure_code_24, procedure_code_25])| IntNormalizedInputProcedureCodeFinal {
            claim_id,
            data_source,
            procedure_code_1, procedure_code_2, procedure_code_3, procedure_code_4, procedure_code_5, procedure_code_6, procedure_code_7, procedure_code_8, procedure_code_9, procedure_code_10, procedure_code_11, procedure_code_12, procedure_code_13, procedure_code_14, procedure_code_15, procedure_code_16, procedure_code_17, procedure_code_18, procedure_code_19, procedure_code_20, procedure_code_21, procedure_code_22, procedure_code_23, procedure_code_24, procedure_code_25,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputProcedureCodeFinal) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        ostr(v.procedure_code_1),
        ostr(v.procedure_code_2),
        ostr(v.procedure_code_3),
        ostr(v.procedure_code_4),
        ostr(v.procedure_code_5),
        ostr(v.procedure_code_6),
        ostr(v.procedure_code_7),
        ostr(v.procedure_code_8),
        ostr(v.procedure_code_9),
        ostr(v.procedure_code_10),
        ostr(v.procedure_code_11),
        ostr(v.procedure_code_12),
        ostr(v.procedure_code_13),
        ostr(v.procedure_code_14),
        ostr(v.procedure_code_15),
        ostr(v.procedure_code_16),
        ostr(v.procedure_code_17),
        ostr(v.procedure_code_18),
        ostr(v.procedure_code_19),
        ostr(v.procedure_code_20),
        ostr(v.procedure_code_21),
        ostr(v.procedure_code_22),
        ostr(v.procedure_code_23),
        ostr(v.procedure_code_24),
        ostr(v.procedure_code_25),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let up = crate::concepts::int_normalized_input_procedure_code_intermediate::int_normalized_input_procedure_code_intermediate(db, &stg);
    rows(int_normalized_input_procedure_code_final(db, &up).iter().map(fmt))
}
