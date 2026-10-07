use crate::concepts::int_normalized_input_procedure_date_voting::IntNormalizedInputProcedureDateVoting;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputProcedureDateFinal {
    pub claim_id: Str,
    pub data_source: Str,
    pub procedure_date_1: Option<Date>,
    pub procedure_date_2: Option<Date>,
    pub procedure_date_3: Option<Date>,
    pub procedure_date_4: Option<Date>,
    pub procedure_date_5: Option<Date>,
    pub procedure_date_6: Option<Date>,
    pub procedure_date_7: Option<Date>,
    pub procedure_date_8: Option<Date>,
    pub procedure_date_9: Option<Date>,
    pub procedure_date_10: Option<Date>,
    pub procedure_date_11: Option<Date>,
    pub procedure_date_12: Option<Date>,
    pub procedure_date_13: Option<Date>,
    pub procedure_date_14: Option<Date>,
    pub procedure_date_15: Option<Date>,
    pub procedure_date_16: Option<Date>,
    pub procedure_date_17: Option<Date>,
    pub procedure_date_18: Option<Date>,
    pub procedure_date_19: Option<Date>,
    pub procedure_date_20: Option<Date>,
    pub procedure_date_21: Option<Date>,
    pub procedure_date_22: Option<Date>,
    pub procedure_date_23: Option<Date>,
    pub procedure_date_24: Option<Date>,
    pub procedure_date_25: Option<Date>,
    pub tuva_last_run: Ts,
}

const COLUMNS: [Str; 25] = ["procedure_date_1", "procedure_date_2", "procedure_date_3", "procedure_date_4", "procedure_date_5", "procedure_date_6", "procedure_date_7", "procedure_date_8", "procedure_date_9", "procedure_date_10", "procedure_date_11", "procedure_date_12", "procedure_date_13", "procedure_date_14", "procedure_date_15", "procedure_date_16", "procedure_date_17", "procedure_date_18", "procedure_date_19", "procedure_date_20", "procedure_date_21", "procedure_date_22", "procedure_date_23", "procedure_date_24", "procedure_date_25"];

pub fn int_normalized_input_procedure_date_final(_db: &'static Db, src: &[IntNormalizedInputProcedureDateVoting]) -> Vec<IntNormalizedInputProcedureDateFinal> {
    let r = rel(src.to_vec());
    let per = (&r)
        .filt(|v: IntNormalizedInputProcedureDateVoting| v.occurrence_row_count == 1 && v.occurrence_count > v.next_occurrence_count)
        .map(|v: IntNormalizedInputProcedureDateVoting| (v.claim_id, v.data_source))
        .inv()
        .select(&r)
        .fold([None::<Date>; 25], |mut a, v: IntNormalizedInputProcedureDateVoting| {
            for (i, c) in COLUMNS.iter().enumerate() {
                if v.column_name.eq_ignore_ascii_case(c) {
                    a[i] = a[i].max(v.normalized_code);
                }
            }
            a
        });
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&per)
        .into_iter()
        .map(|((claim_id, data_source), [procedure_date_1, procedure_date_2, procedure_date_3, procedure_date_4, procedure_date_5, procedure_date_6, procedure_date_7, procedure_date_8, procedure_date_9, procedure_date_10, procedure_date_11, procedure_date_12, procedure_date_13, procedure_date_14, procedure_date_15, procedure_date_16, procedure_date_17, procedure_date_18, procedure_date_19, procedure_date_20, procedure_date_21, procedure_date_22, procedure_date_23, procedure_date_24, procedure_date_25])| IntNormalizedInputProcedureDateFinal {
            claim_id,
            data_source,
            procedure_date_1, procedure_date_2, procedure_date_3, procedure_date_4, procedure_date_5, procedure_date_6, procedure_date_7, procedure_date_8, procedure_date_9, procedure_date_10, procedure_date_11, procedure_date_12, procedure_date_13, procedure_date_14, procedure_date_15, procedure_date_16, procedure_date_17, procedure_date_18, procedure_date_19, procedure_date_20, procedure_date_21, procedure_date_22, procedure_date_23, procedure_date_24, procedure_date_25,
            tuva_last_run: run,
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputProcedureDateFinal) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        odate(v.procedure_date_1),
        odate(v.procedure_date_2),
        odate(v.procedure_date_3),
        odate(v.procedure_date_4),
        odate(v.procedure_date_5),
        odate(v.procedure_date_6),
        odate(v.procedure_date_7),
        odate(v.procedure_date_8),
        odate(v.procedure_date_9),
        odate(v.procedure_date_10),
        odate(v.procedure_date_11),
        odate(v.procedure_date_12),
        odate(v.procedure_date_13),
        odate(v.procedure_date_14),
        odate(v.procedure_date_15),
        odate(v.procedure_date_16),
        odate(v.procedure_date_17),
        odate(v.procedure_date_18),
        odate(v.procedure_date_19),
        odate(v.procedure_date_20),
        odate(v.procedure_date_21),
        odate(v.procedure_date_22),
        odate(v.procedure_date_23),
        odate(v.procedure_date_24),
        odate(v.procedure_date_25),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let norm = crate::concepts::int_normalized_input_procedure_date_normalize::int_normalized_input_procedure_date_normalize(db, &stg);
    let up = crate::concepts::int_normalized_input_procedure_date_voting::int_normalized_input_procedure_date_voting(db, &norm);
    rows(int_normalized_input_procedure_date_final(db, &up).iter().map(fmt))
}
