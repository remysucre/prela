use crate::concepts::normalized_input__stg_medical_claim::NormalizedInputStgMedicalClaim;
use crate::schema::Db;
use harness::prelude::*;

#[derive(Clone, Copy)]
pub struct IntNormalizedInputProcedureCodeIntermediate {
    pub claim_id: Str,
    pub data_source: Str,
    pub procedure_code_type: Option<Str>,
    pub column_name: Str,
    pub procedure_code: Str,
    pub tuva_last_run: Ts,
}

pub const COLUMNS: [Str; 25] = ["procedure_code_1", "procedure_code_2", "procedure_code_3", "procedure_code_4", "procedure_code_5", "procedure_code_6", "procedure_code_7", "procedure_code_8", "procedure_code_9", "procedure_code_10", "procedure_code_11", "procedure_code_12", "procedure_code_13", "procedure_code_14", "procedure_code_15", "procedure_code_16", "procedure_code_17", "procedure_code_18", "procedure_code_19", "procedure_code_20", "procedure_code_21", "procedure_code_22", "procedure_code_23", "procedure_code_24", "procedure_code_25"];

fn codes(m: &NormalizedInputStgMedicalClaim) -> [Option<Str>; 25] {
    [m.procedure_code_1, m.procedure_code_2, m.procedure_code_3, m.procedure_code_4, m.procedure_code_5, m.procedure_code_6, m.procedure_code_7, m.procedure_code_8, m.procedure_code_9, m.procedure_code_10, m.procedure_code_11, m.procedure_code_12, m.procedure_code_13, m.procedure_code_14, m.procedure_code_15, m.procedure_code_16, m.procedure_code_17, m.procedure_code_18, m.procedure_code_19, m.procedure_code_20, m.procedure_code_21, m.procedure_code_22, m.procedure_code_23, m.procedure_code_24, m.procedure_code_25]
}

fn nodot(s: Str) -> Str {
    if s.contains('.') { Box::leak(s.replace('.', "").into_boxed_str()) } else { s }
}

type Piv = (((Str, Str), (Str, Option<Str>)), (Str, Str));

fn pivot(m: NormalizedInputStgMedicalClaim) -> Vec<Piv> {
    let cs = codes(&m);
    (0..25)
        .flat_map(|i| cs[i].map(|c| (((m.claim_id, m.claim_type), (m.data_source, m.procedure_code_type)), (COLUMNS[i], c))))
        .collect()
}

pub fn int_normalized_input_procedure_code_intermediate(
    _db: &'static Db,
    stg: &[NormalizedInputStgMedicalClaim],
) -> Vec<IntNormalizedInputProcedureCodeIntermediate> {
    let med = rel(stg.to_vec());
    let piv = rel(drain((&med).flat_map(pivot)).into_iter().map(|(_, p)| p).collect::<Vec<Piv>>());
    let distinct = (&piv)
        .filt(|(((_, ty), _), _): Piv| ty == "institutional")
        .map(|(((claim_id, _), (data_source, pct)), (col, code)): Piv| ((claim_id, data_source), (pct, (col, nodot(code)))))
        .inv()
        .fold((), |a, _| a);
    let run = ts(2026, 1, 1, 0, 0, 0);
    drain(&distinct)
        .into_iter()
        .map(|(((claim_id, data_source), (procedure_code_type, (column_name, procedure_code))), _)| {
            IntNormalizedInputProcedureCodeIntermediate {
                claim_id,
                data_source,
                procedure_code_type,
                column_name,
                procedure_code,
                tuva_last_run: run,
            }
        })
        .collect()
}

pub fn fmt(v: &IntNormalizedInputProcedureCodeIntermediate) -> String {
    row(vec![
        V::S(v.claim_id),
        V::S(v.data_source),
        ostr(v.procedure_code_type),
        V::S(v.column_name),
        V::S(v.procedure_code),
        V::T(v.tuva_last_run),
    ])
}

pub fn q(db: &'static Db) -> String {
    let mc = crate::concepts::medical_claim::medical_claim(db);
    let il = crate::concepts::input_layer__medical_claim::input_layer__medical_claim(db, &mc);
    let stg = crate::concepts::normalized_input__stg_medical_claim::normalized_input__stg_medical_claim(db, &il);
    let inter = int_normalized_input_procedure_code_intermediate(db, &stg);
    rows(inter.iter().map(fmt))
}
